use std::fs;

pub fn create_archive(dest:String)-> String{ // restituisce la path dell'archivio
    fs::create_dir_all(&dest);
    let starter_content = "74 68 69 73 20 69 73 20 61 6E 20 61 72 63 68 69 76 65";
    fs::write(dest.clone() + "/" + ".__starter__",starter_content);
    return dest;
}