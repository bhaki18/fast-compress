# fast-compress

A lossless CLI compressor and extractor written in Rust for packing arbitrary files (binary and text) and nested directory trees into a single `.cmpr` archive.

## Key Features

- **Full binary and text support:** operates directly on raw bytes (`u8`), with no UTF-8 requirement.
- **Recursive directory archiving:** preserves subdirectory hierarchy, empty directories, and Unix permissions (`0o777`).
- **Lossless compression algorithms:**
  - `auto` *(default)*: uses **LZ77** and automatically falls back to `store` when data is incompressible.
  - `lz77`: LZ4-style hash-table encoding in $O(n)$ time with overlapping match support.
  - `rle`: PackBits-style byte-level Run-Length Encoding.
  - `store`: stores data uncompressed.
- **Integrity and safety:**
  - **CRC-32 (IEEE 802.3)** checksum verified for every extracted file.
  - Relative path sanitization during extraction to prevent *path traversal* (`../`, absolute paths, `\`, NUL bytes).
  - Self-inclusion prevention when the output archive resides inside the source directory (symlinks are skipped with a warning).

## Building

```bash
cargo build --release
```

The compiled binary will be available at `target/release/fast-compress`.

## Usage

```bash
# Compress a file or directory (default destination: archive.cmpr)
fast-compress -p ./path/to/input -c

# Compress with a custom destination and algorithm
fast-compress -p ./my_folder -c -d backup.cmpr -a lz77

# List archive contents
fast-compress -p backup.cmpr -l

# Extract an archive (default destination: extracted/)
fast-compress -p backup.cmpr -e -d ./output
```

### CLI Options

| Flag | Long form | Description | Default |
| :--- | :--- | :--- | :--- |
| `-p` | `--path <PATH>` | Source file/directory to compress or `.cmpr` archive to read | *(required)* |
| `-c` | `--compress` | Compress a file or directory into a `.cmpr` archive | `false` |
| `-e` | `--extract` | Extract entries from a `.cmpr` archive | `false` |
| `-l` | `--list` | List entries stored inside an archive | `false` |
| `-d` | `--destination <DEST>` | Output archive file (`-c`) or extraction directory (`-e`) | `archive.cmpr` / `extracted` |
| `-a` | `--algorithm <ALGO>` | Compression algorithm: `auto`, `lz77`, `rle`, `store` | `auto` |

*(Note: legacy typo flags `--exctract` and `--destionation` remain supported as aliases.)*

## `.cmpr` Archive Format

Every archive starts with an 8-byte magic header `FCMPR\x01\x00\x00`, followed by a sequence of entries:

- **Entry header:** `entry_kind (1B: 0=dir, 1=file)` | `mode (u32 LE)` | `path_len (u32 LE)` | `path (UTF-8 '/')`
- **File entries only:** `method (1B: 0=store, 1=rle, 2=lz77)` | `original_size (u64 LE)` | `compressed_size (u64 LE)` | `crc32 (u32 LE)` | `data`

## Project Structure

```text
src/
├── main.rs                    # Entry point and CLI error handling
├── cli/cli.rs                 # Argument parsing (clap) and command dispatch
├── compressor/compressor.rs   # Compression orchestration and summary output
├── decompressor/decompressor.rs # Archive extraction and listing (-e, -l)
├── lz77/lz77.rs               # Hash-table LZ77 codec
├── rle/rle.rs                 # Byte-level RLE codec (PackBits-style)
└── libs/
    ├── archive.rs             # .cmpr format constants, CRC-32, path validation
    ├── read_file.rs           # Binary file reader and permissions lookup
    ├── read_folder.rs         # Recursive directory traversal
    ├── compress_file.rs       # Single-file compression
    ├── compress_folder.rs     # Recursive folder compression
    ├── create_archive.rs      # Buffered .cmpr archive writer
    ├── push_to_archive.rs     # Archive entry serialization
    ├── open_archive.rs        # Streaming archive reader and header validation
    └── decompress_file.rs     # CRC-32 check, decompression, and extraction
```
