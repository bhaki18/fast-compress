use std::io;
use std::path::Path;

use crate::libs::archive::{Algorithm, CompressionMethod, crc32};
use crate::libs::read_file::{file_mode, read_file};
use crate::lz77::lz77::lz77_compress;
use crate::rle::rle::rle_compress;

/// In-memory compressed representation of a single file ready to be written to an archive.
#[derive(Debug, Clone)]
pub struct CompressedFile {
    pub method: CompressionMethod,
    pub original_size: u64,
    pub crc32: u32,
    pub data: Vec<u8>,
    pub mode: u32,
}

/// Compresses raw bytes according to `algorithm`, automatically falling back to
/// `Stored` in `Auto`/`Lz77` mode if compression does not reduce size.
pub fn compress_bytes(raw: &[u8], algorithm: Algorithm) -> (CompressionMethod, Vec<u8>) {
    match algorithm {
        Algorithm::Store => (CompressionMethod::Stored, raw.to_vec()),
        Algorithm::Rle => (CompressionMethod::Rle, rle_compress(raw)),
        Algorithm::Lz77 | Algorithm::Auto => {
            if raw.is_empty() {
                return (CompressionMethod::Stored, Vec::new());
            }
            let compressed = lz77_compress(raw);
            if compressed.len() < raw.len() {
                (CompressionMethod::Lz77, compressed)
            } else {
                (CompressionMethod::Stored, raw.to_vec())
            }
        }
    }
}

/// Reads and compresses the file at `path` using `algorithm`.
pub fn compress_file(path: &Path, algorithm: Algorithm) -> io::Result<CompressedFile> {
    let raw = read_file(path)?;
    let mode = file_mode(path);
    let original_size = raw.len() as u64;
    let checksum = crc32(&raw);
    let (method, data) = compress_bytes(&raw, algorithm);

    Ok(CompressedFile {
        method,
        original_size,
        crc32: checksum,
        data,
        mode,
    })
}
