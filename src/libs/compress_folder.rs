use std::path::Path;

use crate::libs::archive::Algorithm;
use crate::libs::compress_file::compress_file;
use crate::libs::create_archive::ArchiveWriter;
use crate::libs::push_to_archive::{push_directory, push_file};
use crate::libs::read_folder::read_folder;

/// Aggregate counters produced while packing a file or directory tree.
#[derive(Debug, Default, Clone, Copy)]
pub struct CompressionStats {
    pub files: usize,
    pub dirs: usize,
    pub original_bytes: u64,
    pub compressed_bytes: u64,
}

/// Recursively compresses `folder` and writes all directory and file entries into `archive`.
pub fn compress_folder(
    folder: &Path,
    archive: &mut ArchiveWriter,
    algorithm: Algorithm,
    exclude_path: Option<&Path>,
) -> Result<CompressionStats, String> {
    let entries = read_folder(folder, exclude_path)
        .map_err(|e| format!("failed to traverse folder {}: {e}", folder.display()))?;

    let mut stats = CompressionStats::default();
    for entry in entries {
        if entry.is_dir {
            push_directory(archive, &entry.rel_path, entry.mode)
                .map_err(|e| format!("failed to write directory {:?}: {e}", entry.rel_path))?;
            stats.dirs += 1;
        } else {
            let compressed = compress_file(&entry.abs_path, algorithm).map_err(|e| {
                format!("failed to compress {}: {e}", entry.abs_path.display())
            })?;
            push_file(archive, &entry.rel_path, &compressed)
                .map_err(|e| format!("failed to write file {:?}: {e}", entry.rel_path))?;
            stats.files += 1;
            stats.original_bytes += compressed.original_size;
            stats.compressed_bytes += compressed.data.len() as u64;
        }
    }

    Ok(stats)
}
