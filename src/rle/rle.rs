pub fn rle(content:String)->String{
    if content.len() < 1{
        return "".to_string();
    }
    let mut counter: u32 = 0;
    let mut carattere: char = '\0';
    let mut stringa_compressa = String::new();
    for i in content.chars(){
        if carattere == '\0'{
            carattere = i;
            counter +=1;
            continue;
        }

        if carattere == i{
            counter+=1;
        }else{
            stringa_compressa += &counter.to_string();
            stringa_compressa += &carattere.to_string();
            counter = 1;
            carattere = i;
        }

    }
    /* `std::string::String` value */
    println!("{}",stringa_compressa);
    return stringa_compressa;
}