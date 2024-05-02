use std::{env, fs};

use utils::{determine_file_kind, FileKind};

mod jpeg;
mod tiff;
mod utils;

fn main() {
    let file_path = env::args().nth(1).expect("Enter a file path");

    // For convienence of dev, I'm just loading all the data into memory
    let file_data = fs::read(file_path).unwrap();

    let file_type = determine_file_kind(&file_data).unwrap();

    if file_type == FileKind::JPEG {
        let jpeg = jpeg::read_jpeg(file_data).unwrap();
        println!("{:?}", jpeg);
    } else {
        panic!("Unknown or unsupported file type :/ ");
    }
}
