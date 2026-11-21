use clap::Parser;
use crate::compressor::compressor::compress;
use crate::decompressor::decompressor::decompress;

pub fn cli(){
    let args = Args::parse();
    if args.exctract{
        decompress(&args.path,&args.destionation)
    }

    if args.compress{
        compress(&args.path,&args.destionation)
    }
}


#[derive(Parser, Debug)]
#[command(version = "0.0.1", about = "a simple file compressor", long_about = None)]
struct Args {
    /// path of the file 
    #[arg(short, long)]
    path: String,

    /// exctract file?
    #[arg(short, long, default_value_t = false)]
    exctract: bool,

    /// exctract file?
    #[arg(short, long, default_value_t = false)]
    compress: bool,

    // destionation
    #[arg(short,long,default_value_t = "archive.cmpr".to_string())]
    destionation: String
}
