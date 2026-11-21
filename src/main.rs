mod cli;
mod compressor;
mod decompressor;
mod libs;
mod lz77;
mod rle;

use owo_colors::OwoColorize;

fn main() {
    if let Err(err) = cli::cli::cli() {
        eprintln!("{} {err}", "error:".red().bold());
        std::process::exit(1);
    }
}
