
use owo_colors::OwoColorize;
use crate::libs::compress_file;
use crate::libs::compress_folder;
use std::fs;

pub fn compress(path:&String,dest:&String){
    if fs::exists(path).is_err() || fs::exists(path).is_ok_and(|exists| exists == false){
        println!("{}","file or directory not found!".red());
        return;
    }

    let metadata = fs::metadata(path);
    

    if metadata.expect("REASON").is_dir() {
        
        compress_folder::compress_folder(path.to_string());

    }else{
        compress_file::compress_file(path.to_string());
    }
}