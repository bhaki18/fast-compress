use std::fs;
use std::path::Path;

use crate::libs::archive::{
    ArchiveEntry, CompressionMethod, EntryKind, crc32, validate_archive_rel_path,
};
use crate::lz77::lz77::lz77_decompress;
use crate::rle::rle::rle_decompress;

#[cfg(unix)]
fn apply_mode(path: &Path, mode: u32) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    if mode != 0 {
        let perms = fs::Permissions::from_mode(mode & 0o777);
        fs::set_permissions(path, perms)
            .map_err(|e| format!("failed to restore permissions on {}: {e}", path.display()))?;
    }
    Ok(())
}

#[cfg(not(unix))]
fn apply_mode(_path: &Path, _mode: u32) -> Result<(), String> {
    Ok(())
}

/// Decompresses `data` according to `method` and verifies both `expected_size` and `expected_crc32`.
pub fn decompress_bytes(
    data: &[u8],
    method: CompressionMethod,
    expected_size: u64,
    expected_crc32: u32,
) -> Result<Vec<u8>, String> {
    let expected_usize = usize::try_from(expected_size)
        .map_err(|_| "decompressed entry exceeds platform memory limits".to_string())?;

    let raw = match method {
        CompressionMethod::Stored => {
            if data.len() != expected_usize {
                return Err(format!(
                    "stored payload size mismatch: expected {expected_usize} bytes, got {}",
                    data.len()
                ));
            }
            data.to_vec()
        }
        CompressionMethod::Rle => rle_decompress(data, Some(expected_usize))?,
        CompressionMethod::Lz77 => lz77_decompress(data, Some(expected_usize))?,
    };

    if raw.len() as u64 != expected_size {
        return Err(format!(
            "decompressed size mismatch: expected {expected_size} bytes, got {}",
            raw.len()
        ));
    }

    let actual_crc = crc32(&raw);
    if actual_crc != expected_crc32 {
        return Err(format!(
            "CRC-32 mismatch (expected {expected_crc32:#010x}, got {actual_crc:#010x})"
        ));
    }

    Ok(raw)
}

/// Validates and extracts a single `ArchiveEntry` into `dest_dir`.
pub fn decompress_file(entry: &ArchiveEntry, dest_dir: &Path) -> Result<(), String> {
    let rel_path = validate_archive_rel_path(&entry.path)?;
    let target = dest_dir.join(&rel_path);

    match entry.kind {
        EntryKind::Directory => {
            fs::create_dir_all(&target).map_err(|e| {
                format!("failed to create directory {}: {e}", target.display())
            })?;
            apply_mode(&target, entry.mode)?;
        }
        EntryKind::File => {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(|e| {
                    format!(
                        "failed to create parent directory {}: {e}",
                        parent.display()
                    )
                })?;
            }
            let raw = decompress_bytes(
                &entry.data,
                entry.method,
                entry.original_size,
                entry.crc32,
            )
            .map_err(|e| format!("{}: {e}", entry.path))?;

            fs::write(&target, &raw)
                .map_err(|e| format!("failed to write {}: {e}", target.display()))?;
            apply_mode(&target, entry.mode)?;
        }
    }

    Ok(())
}

