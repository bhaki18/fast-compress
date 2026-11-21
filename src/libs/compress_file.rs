use std::fs;
use crate::rle::rle::rle;

pub fn compress_file(path:String) -> String{
    let content_not_compressed = fs::read_to_string(path);
    let content_compressed = rle(content_not_compressed.expect("REASON"));
    return content_compressed;
}