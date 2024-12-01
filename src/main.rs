use std::{env, fs};

use heif::read_heif;
use utils::{determine_file_kind, FileKind};

mod heif;
mod jpeg;
mod tiff;
mod utils;

fn main() {
    let file_path = env::args().nth(1).expect("Enter a file path");

    // For convienence of dev, I'm just loading all the data into memory
    let file_data = fs::read(file_path).unwrap();

    let file_type = determine_file_kind(&file_data).unwrap();

    if file_type == FileKind::Jpeg {
        let jpeg = jpeg::read_jpeg(&file_data).unwrap();
        println!("{:#?}", jpeg);
    } else if file_type == FileKind::Heif {
        let heif = read_heif(file_data.clone());
        println!("{:#?}", heif);
    } else if file_type == FileKind::Tiff {
        let tiff = tiff::read_tiff_file(&file_data);
        println!("{:#?}", tiff);
    } else {
        panic!("Unknown or unsupported file type :/ ");
    }
}
