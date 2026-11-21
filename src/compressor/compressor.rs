
use owo_colors::OwoColorize;
use crate::libs::compress_file;
use crate::libs::compress_folder;
use crate::libs::create_archive;
use std::path;
use std::fs;

pub fn compress(path:&String,dest:&String){
    if fs::exists(path).is_err() || fs::exists(path).is_ok_and(|exists| exists == false){
        println!("{}","file or directory not found!".red());
        return;
    }

    let metadata = fs::metadata(path);
    let _ = create_archive::create_archive(dest.to_string());
    

    if metadata.expect("REASON").is_dir() {
        
        compress_folder::compress_folder(path.to_string(),dest.to_string());

    }else{
        let compressed = compress_file::compress_file(path.to_string());
        let filename =  path::Path::new(path).file_name().expect("REASON").to_str().expect("REASON").to_string();
        let write_in = dest.to_owned() + "/" + &filename;
        fs::write(write_in,compressed);
    }
}