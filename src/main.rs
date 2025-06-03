use std::env;
use std::fs;
fn main() {
    let args: Vec<String> = env::args().collect();

    let query = &args[1];
    let file_name = &args[2];
    println!("Searching for: {}", query);
    println!("In file: {}", file_name);

    let contents = fs::read_to_string(file_name).expect("Something went panicky reading the file");

    println!("With text:\n{}", contents);
}
