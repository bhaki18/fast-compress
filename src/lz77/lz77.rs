const MIN_MATCH: usize = 4;
const MAX_OFFSET: usize = 65_535;
const HASH_BITS: u32 = 16;
const HASH_SIZE: usize = 1 << HASH_BITS;
const MAX_DECOMPRESS_PREALLOC: usize = 64 * 1024 * 1024;

#[inline]
fn hash4(data: &[u8], pos: usize) -> usize {
    let v = u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
    (v.wrapping_mul(2_654_435_761) >> (32 - HASH_BITS)) as usize
}

#[inline]
fn write_extra_len(out: &mut Vec<u8>, mut extra: usize) {
    while extra >= 255 {
        out.push(255);
        extra -= 255;
    }
    out.push(extra as u8);
}

#[inline]
fn emit_literals_only(out: &mut Vec<u8>, lits: &[u8]) {
    let lit_len = lits.len();
    out.push((lit_len.min(15) as u8) << 4);
    if lit_len >= 15 {
        write_extra_len(out, lit_len - 15);
    }
    out.extend_from_slice(lits);
}

#[inline]
fn emit_match_sequence(out: &mut Vec<u8>, lits: &[u8], offset: u16, match_len: usize) {
    let lit_len = lits.len();
    let ml_code = match_len - MIN_MATCH;
    let token = ((lit_len.min(15) as u8) << 4) | (ml_code.min(15) as u8);
    out.push(token);

    if lit_len >= 15 {
        write_extra_len(out, lit_len - 15);
    }
    out.extend_from_slice(lits);
    out.extend_from_slice(&offset.to_le_bytes());

    if ml_code >= 15 {
        write_extra_len(out, ml_code - 15);
    }
}

/// Compresses arbitrary bytes in `O(n)` time using an LZ4-style hash-table LZ77 encoder.
pub fn lz77_compress(input: &[u8]) -> Vec<u8> {
    if input.is_empty() {
        return Vec::new();
    }

    if input.len() < MIN_MATCH {
        let mut out = Vec::with_capacity(1 + input.len());
        emit_literals_only(&mut out, input);
        return out;
    }

    let mut out = Vec::with_capacity(input.len() / 2 + 16);
    let mut table = vec![usize::MAX; HASH_SIZE];
    let limit = input.len() - MIN_MATCH;
    let mut anchor = 0usize;
    let mut pos = 0usize;

    while pos <= limit {
        let h = hash4(input, pos);
        let cand = table[h];
        table[h] = pos;

        if cand != usize::MAX
            && pos - cand <= MAX_OFFSET
            && input[cand..cand + MIN_MATCH] == input[pos..pos + MIN_MATCH]
        {
            let mut match_len = MIN_MATCH;
            while pos + match_len < input.len() && input[cand + match_len] == input[pos + match_len]
            {
                match_len += 1;
            }

            emit_match_sequence(&mut out, &input[anchor..pos], (pos - cand) as u16, match_len);
            pos += match_len;
            anchor = pos;

            if pos >= 2 && pos - 2 <= limit {
                let h_prev = hash4(input, pos - 2);
                table[h_prev] = pos - 2;
            }
        } else {
            pos += 1;
        }
    }

    if anchor < input.len() {
        emit_literals_only(&mut out, &input[anchor..]);
    }

    out
}

#[inline]
fn read_extra_len(input: &[u8], pos: &mut usize, base: usize) -> Result<usize, String> {
    let mut total = base;
    if base == 15 {
        loop {
            if *pos >= input.len() {
                return Err("truncated LZ77 length extension".to_string());
            }
            let b = usize::from(input[*pos]);
            *pos += 1;
            total = total
                .checked_add(b)
                .ok_or_else(|| "LZ77 length overflow".to_string())?;
            if b < 255 {
                break;
            }
        }
    }
    Ok(total)
}

/// Decompresses a byte slice encoded with [`lz77_compress`].
///
/// Supports overlapping match copies (`offset < match_len`) and validates every
/// offset and bound before reading or extending the output buffer.
pub fn lz77_decompress(input: &[u8], expected_size: Option<usize>) -> Result<Vec<u8>, String> {
    let prealloc = expected_size
        .unwrap_or(input.len())
        .min(MAX_DECOMPRESS_PREALLOC);
    let mut out = Vec::with_capacity(prealloc);
    let mut pos = 0usize;

    while pos < input.len() {
        let token = input[pos];
        pos += 1;

        let lit_len = read_extra_len(input, &mut pos, usize::from(token >> 4))?;
        if input.len() - pos < lit_len {
            return Err("truncated LZ77 literal data".to_string());
        }
        if let Some(exp) = expected_size
            && out.len().checked_add(lit_len).is_none_or(|n| n > exp)
        {
            return Err(format!(
                "LZ77 literals exceed expected output size ({exp} bytes)"
            ));
        }

        out.extend_from_slice(&input[pos..pos + lit_len]);
        pos += lit_len;

        if pos == input.len() {
            if (token & 0x0F) != 0 {
                return Err("truncated LZ77 match offset".to_string());
            }
            break;
        }

        if input.len() - pos < 2 {
            return Err("truncated LZ77 match offset".to_string());
        }
        let offset = usize::from(u16::from_le_bytes([input[pos], input[pos + 1]]));
        pos += 2;

        if offset == 0 || offset > out.len() {
            return Err(format!(
                "invalid LZ77 match offset {offset} for output length {}",
                out.len()
            ));
        }

        let ml_extra = read_extra_len(input, &mut pos, usize::from(token & 0x0F))?;
        let match_len = ml_extra
            .checked_add(MIN_MATCH)
            .ok_or_else(|| "LZ77 match length overflow".to_string())?;

        if let Some(exp) = expected_size
            && out.len().checked_add(match_len).is_none_or(|n| n > exp)
        {
            return Err(format!(
                "LZ77 match exceeds expected output size ({exp} bytes)"
            ));
        }

        let start = out.len() - offset;
        out.reserve(match_len);
        for idx in 0..match_len {
            let byte = out[start + idx];
            out.push(byte);
        }
    }

    if let Some(exp) = expected_size
        && out.len() != exp
    {
        return Err(format!(
            "LZ77 size mismatch: expected {exp} bytes, got {}",
            out.len()
        ));
    }

    Ok(out)
}

