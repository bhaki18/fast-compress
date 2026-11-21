use std::io::{self, BufWriter, Write};
use std::fs::File;

use crate::libs::archive::{EntryKind, MAX_PATH_LEN};
use crate::libs::compress_file::CompressedFile;
use crate::libs::create_archive::ArchiveWriter;

fn write_entry_header(
    writer: &mut BufWriter<File>,
    kind: EntryKind,
    mode: u32,
    rel_path: &str,
) -> io::Result<()> {
    let path_bytes = rel_path.as_bytes();
    if path_bytes.is_empty() || path_bytes.len() > MAX_PATH_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid archive path length: {}", path_bytes.len()),
        ));
    }

    writer.write_all(&[kind as u8])?;
    writer.write_all(&mode.to_le_bytes())?;
    writer.write_all(&(path_bytes.len() as u32).to_le_bytes())?;
    writer.write_all(path_bytes)?;
    Ok(())
}

/// Writes a directory entry header to the archive.
pub fn push_directory(archive: &mut ArchiveWriter, rel_path: &str, mode: u32) -> io::Result<()> {
    write_entry_header(archive.writer_mut(), EntryKind::Directory, mode, rel_path)
}

/// Writes a compressed file entry (header + payload) to the archive.
pub fn push_file(
    archive: &mut ArchiveWriter,
    rel_path: &str,
    compressed: &CompressedFile,
) -> io::Result<()> {
    let w = archive.writer_mut();
    write_entry_header(w, EntryKind::File, compressed.mode, rel_path)?;
    w.write_all(&[compressed.method as u8])?;
    w.write_all(&compressed.original_size.to_le_bytes())?;
    w.write_all(&(compressed.data.len() as u64).to_le_bytes())?;
    w.write_all(&compressed.crc32.to_le_bytes())?;
    w.write_all(&compressed.data)?;
    Ok(())
}
