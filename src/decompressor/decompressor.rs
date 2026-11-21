use owo_colors::OwoColorize;
use std::fs;
use std::path::Path;

use crate::libs::archive::{EntryKind, format_bytes};
use crate::libs::decompress_file::decompress_file;
use crate::libs::open_archive::open_archive;

/// Extracts all entries from the `.cmpr` archive at `archive_path` into `dest`.
pub fn decompress(archive_path: &str, dest: &str) -> Result<(), String> {
    let archive = Path::new(archive_path);
    let dest_dir = Path::new(dest);

    if dest_dir.exists() && !dest_dir.is_dir() {
        return Err(format!(
            "destination {} exists and is not a directory",
            dest_dir.display()
        ));
    }

    let mut reader = open_archive(archive)?;
    fs::create_dir_all(dest_dir).map_err(|e| {
        format!(
            "failed to create destination directory {}: {e}",
            dest_dir.display()
        )
    })?;

    let mut files = 0usize;
    let mut dirs = 0usize;
    let mut total_bytes = 0u64;

    while let Some(entry) = reader.next_entry()? {
        decompress_file(&entry, dest_dir)?;
        match entry.kind {
            EntryKind::Directory => dirs += 1,
            EntryKind::File => {
                files += 1;
                total_bytes += entry.original_size;
            }
        }
    }

    println!(
        "{} {} -> {} ({} file(s), {} dir(s), {})",
        "Extracted".green().bold(),
        archive.display(),
        dest_dir.display(),
        files,
        dirs,
        format_bytes(total_bytes)
    );

    Ok(())
}

/// Lists all entries inside the `.cmpr` archive at `archive_path` without extracting them.
pub fn list_archive(archive_path: &str) -> Result<(), String> {
    let archive = Path::new(archive_path);
    let mut reader = open_archive(archive)?;

    println!(
        "{:<6} {:<7} {:>12} {:>12}  {}",
        "KIND".bold(),
        "METHOD".bold(),
        "ORIGINAL".bold(),
        "COMPRESSED".bold(),
        "PATH".bold()
    );

    let mut files = 0usize;
    let mut dirs = 0usize;
    let mut total_orig = 0u64;
    let mut total_comp = 0u64;

    while let Some(entry) = reader.next_entry()? {
        match entry.kind {
            EntryKind::Directory => {
                dirs += 1;
                println!(
                    "{:<6} {:<7} {:>12} {:>12}  {}/",
                    "dir".cyan(),
                    "-",
                    "-",
                    "-",
                    entry.path
                );
            }
            EntryKind::File => {
                files += 1;
                total_orig += entry.original_size;
                total_comp += entry.compressed_size;
                println!(
                    "{:<6} {:<7} {:>12} {:>12}  {}",
                    "file",
                    entry.method,
                    format_bytes(entry.original_size),
                    format_bytes(entry.compressed_size),
                    entry.path
                );
            }
        }
    }

    println!(
        "{} {} file(s), {} dir(s), {} -> {}",
        "Total:".bold(),
        files,
        dirs,
        format_bytes(total_orig),
        format_bytes(total_comp)
    );

    Ok(())
}
