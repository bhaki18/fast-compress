use std::fs::File;
use std::io::{self, BufReader, Read, Seek};
use std::path::Path;

use crate::libs::archive::{
    ARCHIVE_MAGIC, ArchiveEntry, CompressionMethod, EntryKind, MAX_PATH_LEN,
};

/// Streaming reader for `.cmpr` archives.
pub struct ArchiveReader {
    reader: BufReader<File>,
    file_len: u64,
}

/// Opens a `.cmpr` archive at `path` and validates its 8-byte magic header.
pub fn open_archive(path: &Path) -> Result<ArchiveReader, String> {
    let file =
        File::open(path).map_err(|e| format!("failed to open archive {}: {e}", path.display()))?;
    let file_len = file
        .metadata()
        .map_err(|e| format!("failed to inspect archive {}: {e}", path.display()))?
        .len();

    if file_len < ARCHIVE_MAGIC.len() as u64 {
        return Err(format!(
            "archive {} is too small to be a valid .cmpr file",
            path.display()
        ));
    }

    let mut reader = BufReader::new(file);
    let mut magic = [0u8; 8];
    reader
        .read_exact(&mut magic)
        .map_err(|e| format!("failed to read archive header {}: {e}", path.display()))?;

    if &magic != ARCHIVE_MAGIC {
        return Err(format!(
            "invalid archive magic in {} (not a fast-compress .cmpr archive)",
            path.display()
        ));
    }

    Ok(ArchiveReader { reader, file_len })
}

impl ArchiveReader {
    fn remaining_bytes(&mut self) -> Result<u64, String> {
        let pos = self
            .reader
            .stream_position()
            .map_err(|e| format!("failed to query archive position: {e}"))?;
        Ok(self.file_len.saturating_sub(pos))
    }

    /// Reads the next entry from the archive, returning `Ok(None)` cleanly at EOF.
    pub fn next_entry(&mut self) -> Result<Option<ArchiveEntry>, String> {
        let mut kind_buf = [0u8; 1];
        loop {
            match self.reader.read(&mut kind_buf) {
                Ok(0) => return Ok(None),
                Ok(_) => break,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(format!("failed to read archive entry kind: {e}")),
            }
        }

        let kind = EntryKind::try_from(kind_buf[0])?;

        let mut mode_buf = [0u8; 4];
        self.reader
            .read_exact(&mut mode_buf)
            .map_err(|e| format!("truncated entry mode in archive: {e}"))?;
        let mode = u32::from_le_bytes(mode_buf);

        let mut path_len_buf = [0u8; 4];
        self.reader
            .read_exact(&mut path_len_buf)
            .map_err(|e| format!("truncated entry path length in archive: {e}"))?;
        let path_len = u32::from_le_bytes(path_len_buf) as usize;

        if path_len == 0 || path_len > MAX_PATH_LEN {
            return Err(format!(
                "invalid archive entry path length {path_len} (must be 1..={MAX_PATH_LEN})"
            ));
        }

        let remaining = self.remaining_bytes()?;
        if (path_len as u64) > remaining {
            return Err(format!(
                "truncated archive entry path: expected {path_len} bytes, only {remaining} remain"
            ));
        }

        let mut path_buf = vec![0u8; path_len];
        self.reader
            .read_exact(&mut path_buf)
            .map_err(|e| format!("failed to read archive entry path: {e}"))?;
        let path = String::from_utf8(path_buf)
            .map_err(|_| "archive entry path is not valid UTF-8".to_string())?;

        if kind == EntryKind::Directory {
            return Ok(Some(ArchiveEntry {
                kind,
                path,
                mode,
                method: CompressionMethod::Stored,
                original_size: 0,
                compressed_size: 0,
                crc32: 0,
                data: Vec::new(),
            }));
        }

        let mut file_hdr = [0u8; 21];
        self.reader
            .read_exact(&mut file_hdr)
            .map_err(|e| format!("truncated file metadata for {path:?}: {e}"))?;

        let method = CompressionMethod::try_from(file_hdr[0])?;
        let original_size = u64::from_le_bytes(file_hdr[1..9].try_into().unwrap());
        let compressed_size = u64::from_le_bytes(file_hdr[9..17].try_into().unwrap());
        let crc32 = u32::from_le_bytes(file_hdr[17..21].try_into().unwrap());

        let remaining = self.remaining_bytes()?;
        if compressed_size > remaining {
            return Err(format!(
                "truncated payload for {path:?}: expected {compressed_size} bytes, only {remaining} remain"
            ));
        }

        let comp_len = usize::try_from(compressed_size)
            .map_err(|_| format!("entry {path:?} exceeds addressable memory"))?;
        let mut data = vec![0u8; comp_len];
        self.reader
            .read_exact(&mut data)
            .map_err(|e| format!("failed to read compressed payload for {path:?}: {e}"))?;

        Ok(Some(ArchiveEntry {
            kind,
            path,
            mode,
            method,
            original_size,
            compressed_size,
            crc32,
            data,
        }))
    }
}

