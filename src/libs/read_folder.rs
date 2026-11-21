use owo_colors::OwoColorize;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::libs::archive::normalize_rel_path;
use crate::libs::read_file::file_mode;

/// A directory or regular file discovered during recursive directory traversal.
#[derive(Debug, Clone)]
pub struct FolderEntry {
    pub abs_path: PathBuf,
    pub rel_path: String,
    pub is_dir: bool,
    pub mode: u32,
}

fn is_excluded(candidate: &Path, exclude_canon: Option<&Path>) -> bool {
    let Some(excl) = exclude_canon else {
        return false;
    };
    fs::canonicalize(candidate).is_ok_and(|c| c == excl)
}

fn visit_dir(
    dir: &Path,
    rel_prefix: &Path,
    exclude_canon: Option<&Path>,
    out: &mut Vec<FolderEntry>,
) -> io::Result<()> {
    let mut children: Vec<(String, PathBuf)> = Vec::new();
    for entry_res in fs::read_dir(dir)? {
        let entry = entry_res?;
        let name = entry.file_name().to_string_lossy().into_owned();
        children.push((name, entry.path()));
    }
    children.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));

    for (name, child_abs) in children {
        if is_excluded(&child_abs, exclude_canon) {
            continue;
        }

        let meta = fs::symlink_metadata(&child_abs)?;
        if meta.file_type().is_symlink() {
            eprintln!(
                "{} skipping symlink: {}",
                "warning:".yellow(),
                child_abs.display()
            );
            continue;
        }

        let child_rel = rel_prefix.join(&name);
        let rel_str = normalize_rel_path(&child_rel)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let mode = file_mode(&child_abs);

        if meta.is_dir() {
            out.push(FolderEntry {
                abs_path: child_abs.clone(),
                rel_path: rel_str,
                is_dir: true,
                mode,
            });
            visit_dir(&child_abs, &child_rel, exclude_canon, out)?;
        } else if meta.is_file() {
            out.push(FolderEntry {
                abs_path: child_abs,
                rel_path: rel_str,
                is_dir: false,
                mode,
            });
        }
    }

    Ok(())
}

/// Recursively collects directories and files under `folder` in deterministic sorted order.
///
/// Symlinks are skipped to avoid cycles. If `exclude_path` matches any traversed path after
/// canonicalization, that entry is omitted (preventing self-inclusion when writing an archive
/// inside the source directory).
pub fn read_folder(folder: &Path, exclude_path: Option<&Path>) -> io::Result<Vec<FolderEntry>> {
    let mut entries = Vec::new();
    let exclude_canon = exclude_path.and_then(|p| fs::canonicalize(p).ok());

    let root_rel = match folder.file_name() {
        Some(name) => {
            let rel = PathBuf::from(name);
            let rel_str = normalize_rel_path(&rel)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            entries.push(FolderEntry {
                abs_path: folder.to_path_buf(),
                rel_path: rel_str,
                is_dir: true,
                mode: file_mode(folder),
            });
            rel
        }
        None => PathBuf::new(),
    };

    visit_dir(folder, &root_rel, exclude_canon.as_deref(), &mut entries)?;
    Ok(entries)
}

