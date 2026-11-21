use std::fs;
use std::io;
use std::path::Path;

/// Reads all bytes of the file at `path` (supports arbitrary binary and text files).
pub fn read_file(path: &Path) -> io::Result<Vec<u8>> {
    fs::read(path)
}

/// Returns Unix `rwxrwxrwx` mode bits (`0o777` mask), or `0` on non-Unix platforms.
#[cfg(unix)]
pub fn file_mode(path: &Path) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path)
        .map(|m| m.permissions().mode() & 0o777)
        .unwrap_or(0)
}

#[cfg(not(unix))]
pub fn file_mode(_path: &Path) -> u32 {
    0
}

