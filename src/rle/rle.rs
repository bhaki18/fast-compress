const MAX_RUN_LEN: usize = 130;
const MIN_RUN_LEN: usize = 3;
const MAX_LIT_LEN: usize = 128;
const MAX_DECOMPRESS_PREALLOC: usize = 64 * 1024 * 1024;

#[inline]
fn flush_literals(out: &mut Vec<u8>, lits: &[u8]) {
    if !lits.is_empty() {
        debug_assert!(lits.len() <= MAX_LIT_LEN);
        out.push((lits.len() - 1) as u8);
        out.extend_from_slice(lits);
    }
}

/// Compresses arbitrary bytes using a PackBits-style lossless Run-Length Encoding.
///
/// Packet layout:
/// - `0x00..=0x7F` (`n`): literal chunk of `n + 1` bytes (`1..=128`) followed by those bytes.
/// - `0x80..=0xFF` (`0x80 | n`): run of `n + 3` identical bytes (`3..=130`) followed by 1 byte.
pub fn rle_compress(input: &[u8]) -> Vec<u8> {
    if input.is_empty() {
        return Vec::new();
    }

    let mut out = Vec::with_capacity(input.len() / 2 + 8);
    let mut lit_start = 0usize;
    let mut i = 0usize;

    while i < input.len() {
        let byte = input[i];
        let mut run = 1usize;
        while i + run < input.len() && run < MAX_RUN_LEN && input[i + run] == byte {
            run += 1;
        }

        if run >= MIN_RUN_LEN {
            flush_literals(&mut out, &input[lit_start..i]);
            out.push(0x80 | ((run - MIN_RUN_LEN) as u8));
            out.push(byte);
            i += run;
            lit_start = i;
        } else {
            i += 1;
            if i - lit_start == MAX_LIT_LEN {
                flush_literals(&mut out, &input[lit_start..i]);
                lit_start = i;
            }
        }
    }

    flush_literals(&mut out, &input[lit_start..]);
    out
}

/// Decompresses a byte slice encoded with [`rle_compress`].
///
/// When `expected_size` is provided, bounds are checked before every write and the
/// final decompressed length must match it exactly.
pub fn rle_decompress(input: &[u8], expected_size: Option<usize>) -> Result<Vec<u8>, String> {
    let prealloc = expected_size
        .unwrap_or(input.len())
        .min(MAX_DECOMPRESS_PREALLOC);
    let mut out = Vec::with_capacity(prealloc);
    let mut pos = 0usize;

    while pos < input.len() {
        let header = input[pos];
        pos += 1;

        if (header & 0x80) != 0 {
            let run_len = usize::from(header & 0x7F) + MIN_RUN_LEN;
            if pos >= input.len() {
                return Err("truncated RLE run packet".to_string());
            }
            let byte = input[pos];
            pos += 1;

            if let Some(exp) = expected_size
                && out.len().checked_add(run_len).is_none_or(|n| n > exp)
            {
                return Err(format!(
                    "RLE run packet exceeds expected output size ({exp} bytes)"
                ));
            }

            out.resize(out.len() + run_len, byte);
        } else {
            let lit_len = usize::from(header) + 1;
            if input.len() - pos < lit_len {
                return Err("truncated RLE literal packet".to_string());
            }

            if let Some(exp) = expected_size
                && out.len().checked_add(lit_len).is_none_or(|n| n > exp)
            {
                return Err(format!(
                    "RLE literal packet exceeds expected output size ({exp} bytes)"
                ));
            }

            out.extend_from_slice(&input[pos..pos + lit_len]);
            pos += lit_len;
        }
    }

    if let Some(exp) = expected_size
        && out.len() != exp
    {
        return Err(format!(
            "RLE size mismatch: expected {exp} bytes, got {}",
            out.len()
        ));
    }

    Ok(out)
}
