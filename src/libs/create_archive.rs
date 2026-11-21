use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::Path;

use crate::libs::archive::ARCHIVE_MAGIC;

/// Buffered writer for creating a `.cmpr` archive file.
pub struct ArchiveWriter {
    writer: BufWriter<File>,
}

impl ArchiveWriter {
    /// Returns mutable access to the underlying buffered file writer.
    pub fn writer_mut(&mut self) -> &mut BufWriter<File> {
        &mut self.writer
    }

    /// Flushes all buffered archive bytes to disk.
    pub fn finish(mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

/// Creates a new `.cmpr` archive at `dest` and writes the magic header.
pub fn create_archive(dest: &Path) -> io::Result<ArchiveWriter> {
    if let Some(parent) = dest.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }

    let file = File::create(dest)?;
    let mut writer = BufWriter::new(file);
    writer.write_all(ARCHIVE_MAGIC)?;

    Ok(ArchiveWriter { writer })
}
