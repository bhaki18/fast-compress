use std::fs;

pub fn push_to_archive(name:String,dest:String,content:String,is_file:bool){
    let write_in = dest + "/" + name;
    if is_file{
        write(write_in,content);
    }else{
        create_dir_all(write_in);
    }
}