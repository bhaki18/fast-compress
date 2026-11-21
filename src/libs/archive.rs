use clap::ValueEnum;
use std::fmt;
use std::path::{Component, Path, PathBuf};

/// 8-byte magic header written at the beginning of every `.cmpr` archive.
pub const ARCHIVE_MAGIC: &[u8; 8] = b"FCMPR\x01\x00\x00";

/// Maximum permitted archive-relative path length in bytes.
pub const MAX_PATH_LEN: usize = 65_536;

/// Kind of entry stored inside an archive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum EntryKind {
    Directory = 0,
    File = 1,
}

impl TryFrom<u8> for EntryKind {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Directory),
            1 => Ok(Self::File),
            _ => Err(format!("unknown archive entry kind: {value}")),
        }
    }
}

/// Compression method recorded per file entry in the archive header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CompressionMethod {
    Stored = 0,
    Rle = 1,
    Lz77 = 2,
}

impl TryFrom<u8> for CompressionMethod {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Stored),
            1 => Ok(Self::Rle),
            2 => Ok(Self::Lz77),
            _ => Err(format!("unknown compression method: {value}")),
        }
    }
}

impl fmt::Display for CompressionMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Stored => "store",
            Self::Rle => "rle",
            Self::Lz77 => "lz77",
        };
        f.write_str(label)
    }
}

/// User-selectable compression algorithm from the CLI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Algorithm {
    Auto,
    Lz77,
    Rle,
    Store,
}

/// A single entry decoded from a `.cmpr` archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveEntry {
    pub kind: EntryKind,
    pub path: String,
    pub mode: u32,
    pub method: CompressionMethod,
    pub original_size: u64,
    pub compressed_size: u64,
    pub crc32: u32,
    pub data: Vec<u8>,
}

const fn make_crc32_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut i = 0usize;
    while i < 256 {
        let mut crc = i as u32;
        let mut bit = 0;
        while bit < 8 {
            crc = if (crc & 1) != 0 {
                0xEDB8_8320 ^ (crc >> 1)
            } else {
                crc >> 1
            };
            bit += 1;
        }
        table[i] = crc;
        i += 1;
    }
    table
}

const CRC32_TABLE: [u32; 256] = make_crc32_table();

/// Computes the standard IEEE 802.3 CRC-32 checksum of `data`.
pub fn crc32(data: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &byte in data {
        let idx = ((crc ^ u32::from(byte)) & 0xFF) as usize;
        crc = CRC32_TABLE[idx] ^ (crc >> 8);
    }
    !crc
}

/// Validates an archive-relative path and returns a sanitized relative `PathBuf`.
///
/// Rejects empty paths, NUL bytes, backslashes, leading slashes, `.` / `..`
/// components, or drive/root prefixes to prevent path traversal attacks.
pub fn validate_archive_rel_path(rel_path: &str) -> Result<PathBuf, String> {
    if rel_path.is_empty() {
        return Err("archive entry has an empty path".to_string());
    }
    if rel_path.contains('\0') {
        return Err("archive entry path contains NUL byte".to_string());
    }
    if rel_path.contains('\\') {
        return Err(format!(
            "archive entry path contains backslash: {rel_path:?}"
        ));
    }
    if rel_path.starts_with('/') {
        return Err(format!("archive entry path cannot be absolute: {rel_path:?}"));
    }

    for seg in rel_path.split('/') {
        if seg.is_empty() || seg == "." || seg == ".." {
            return Err(format!(
                "archive entry contains invalid path segment: {rel_path:?}"
            ));
        }
    }

    let mut clean = PathBuf::new();
    for component in Path::new(rel_path).components() {
        match component {
            Component::Normal(part) => clean.push(part),
            _ => {
                return Err(format!(
                    "archive entry contains unsafe path component: {rel_path:?}"
                ));
            }
        }
    }

    if clean.as_os_str().is_empty() {
        return Err(format!("archive entry path is invalid: {rel_path:?}"));
    }

    Ok(clean)
}

/// Converts a relative filesystem path into a normalized forward-slash UTF-8 string.
pub fn normalize_rel_path(path: &Path) -> Result<String, String> {
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(seg) => {
                let s = seg
                    .to_str()
                    .ok_or_else(|| format!("path contains non-UTF-8 segment: {path:?}"))?;
                parts.push(s);
            }
            Component::CurDir => {}
            _ => {
                return Err(format!("cannot normalize non-relative path: {path:?}"));
            }
        }
    }

    if parts.is_empty() {
        return Err(format!("relative path has no components: {path:?}"));
    }

    Ok(parts.join("/"))
}

/// Formats a byte count into a human-readable string using binary prefixes.
pub fn format_bytes(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * 1024.0;
    const GIB: f64 = MIB * 1024.0;

    if bytes < 1024 {
        format!("{bytes} B")
    } else if (bytes as f64) < MIB {
        format!("{:.2} KiB", bytes as f64 / KIB)
    } else if (bytes as f64) < GIB {
        format!("{:.2} MiB", bytes as f64 / MIB)
    } else {
        format!("{:.2} GiB", bytes as f64 / GIB)
    }
}
