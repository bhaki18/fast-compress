use clap::{ArgGroup, Parser};

use crate::compressor::compressor::compress;
use crate::decompressor::decompressor::{decompress, list_archive};
use crate::libs::archive::Algorithm;

/// Parses CLI arguments and executes the requested archive operation.
pub fn cli() -> Result<(), String> {
    let args = Args::parse();

    if args.compress {
        let dest = args.destination.as_deref().unwrap_or("archive.cmpr");
        return compress(&args.path, dest, args.algorithm);
    }

    if args.extract {
        let dest = args.destination.as_deref().unwrap_or("extracted");
        return decompress(&args.path, dest);
    }

    if args.list {
        return list_archive(&args.path);
    }

    Ok(())
}

#[derive(Parser, Debug)]
#[command(
    version,
    about = "A fast binary and directory file compressor",
    long_about = None,
    group(
        ArgGroup::new("mode")
            .required(true)
            .args(["compress", "extract", "list"])
    )
)]
struct Args {
    /// Path of the file/folder to compress, or archive to extract/list
    #[arg(short, long)]
    path: String,

    /// Compress the input file or directory into a .cmpr archive
    #[arg(short, long, default_value_t = false)]
    compress: bool,

    /// Extract entries from a .cmpr archive
    #[arg(short, long, alias = "exctract", default_value_t = false)]
    extract: bool,

    /// List entries inside a .cmpr archive
    #[arg(short, long, default_value_t = false)]
    list: bool,

    /// Destination archive path (for -c) or output directory (for -e)
    #[arg(short, long, alias = "destionation")]
    destination: Option<String>,

    /// Compression algorithm to use when compressing
    #[arg(short, long, value_enum, default_value_t = Algorithm::Auto)]
    algorithm: Algorithm,
}
