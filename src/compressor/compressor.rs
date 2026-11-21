use owo_colors::OwoColorize;
use std::fs;
use std::path::{Path, PathBuf};

use crate::libs::archive::{Algorithm, format_bytes, normalize_rel_path};
use crate::libs::compress_file::compress_file;
use crate::libs::compress_folder::{CompressionStats, compress_folder};
use crate::libs::create_archive::create_archive;
use crate::libs::push_to_archive::push_file;

fn resolve_dest_exclusion(dest: &Path) -> Option<PathBuf> {
    if let Ok(canon) = fs::canonicalize(dest) {
        return Some(canon);
    }
    let abs = if dest.is_absolute() {
        dest.to_path_buf()
    } else {
        std::env::current_dir().ok()?.join(dest)
    };
    if let Some(parent) = abs.parent()
        && let Some(file_name) = abs.file_name()
        && let Ok(canon_parent) = fs::canonicalize(parent)
    {
        return Some(canon_parent.join(file_name));
    }
    Some(abs)
}

fn pack_into_archive(
    src_path: &Path,
    is_dir: bool,
    dest_path: &Path,
    algorithm: Algorithm,
) -> Result<CompressionStats, String> {
    let mut archive = create_archive(dest_path)
        .map_err(|e| format!("failed to create archive {}: {e}", dest_path.display()))?;

    let result = if is_dir {
        let exclude = resolve_dest_exclusion(dest_path);
        compress_folder(src_path, &mut archive, algorithm, exclude.as_deref())
    } else {
        let file_name = src_path
            .file_name()
            .ok_or_else(|| format!("invalid source file path: {}", src_path.display()))?;
        let rel_path = normalize_rel_path(Path::new(file_name))?;
        let compressed = compress_file(src_path, algorithm)
            .map_err(|e| format!("failed to compress {}: {e}", src_path.display()))?;
        push_file(&mut archive, &rel_path, &compressed)
            .map_err(|e| format!("failed to write entry {rel_path:?}: {e}"))?;

        Ok(CompressionStats {
            files: 1,
            dirs: 0,
            original_bytes: compressed.original_size,
            compressed_bytes: compressed.data.len() as u64,
        })
    };

    match result {
        Ok(stats) => {
            if let Err(e) = archive.finish() {
                let _ = fs::remove_file(dest_path);
                return Err(format!(
                    "failed to finalize archive {}: {e}",
                    dest_path.display()
                ));
            }
            Ok(stats)
        }
        Err(err) => {
            drop(archive);
            let _ = fs::remove_file(dest_path);
            Err(err)
        }
    }
}

/// Compresses the file or directory at `path` into a single `.cmpr` archive at `dest`.
pub fn compress(path: &str, dest: &str, algorithm: Algorithm) -> Result<(), String> {
    let src_path = Path::new(path);
    let meta = fs::metadata(src_path)
        .map_err(|_| format!("file or directory not found: {}", src_path.display()))?;

    if !meta.is_dir() && !meta.is_file() {
        return Err(format!(
            "unsupported source type (expected regular file or directory): {}",
            src_path.display()
        ));
    }

    let dest_path = Path::new(dest);
    if dest_path.is_dir() {
        return Err(format!(
            "destination {} is an existing directory; expected an archive file path",
            dest_path.display()
        ));
    }

    if meta.is_file()
        && let (Ok(src_canon), Some(dest_excl)) =
            (fs::canonicalize(src_path), resolve_dest_exclusion(dest_path))
        && src_canon == dest_excl
    {
        return Err(format!(
            "destination archive {} cannot overwrite the input file",
            dest_path.display()
        ));
    }

    let stats = pack_into_archive(src_path, meta.is_dir(), dest_path, algorithm)?;
    let archive_size = fs::metadata(dest_path)
        .map(|m| m.len())
        .unwrap_or(stats.compressed_bytes);

    let savings = if stats.original_bytes > 0 {
        let pct = (1.0 - (archive_size as f64 / stats.original_bytes as f64)) * 100.0;
        format!("{pct:.1}% saved")
    } else {
        "0.0% saved".to_string()
    };

    println!(
        "{} {} -> {} ({} file(s), {} dir(s), {} -> {}, {})",
        "Compressed".green().bold(),
        src_path.display(),
        dest_path.display(),
        stats.files,
        stats.dirs,
        format_bytes(stats.original_bytes),
        format_bytes(archive_size),
        savings
    );

    Ok(())
}
