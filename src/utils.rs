use std::fmt::Debug;

#[derive(Debug, PartialEq)]
pub enum Endianness {
    LITTLE,
    BIG,
}


#[derive(Debug, PartialEq)]
pub enum FileKind {
    JPEG,
    PNG,
}

pub fn determine_file_kind(data: &Vec<u8>) -> Option<FileKind> {
    if data.len() >= 2 && data[0..3] == *b"\xFF\xD8\xFF" {
        return Some(FileKind::JPEG);
    }

    if data.len() >= 8 && data[0..8] == [137, 80, 78, 71, 13, 10, 26, 10] {
        return Some(FileKind::PNG);
    }

    return None;
}

pub fn vec_to_array<T, const N: usize>(vec: Vec<T>) -> Result<[T; N], String> where T: Debug {
    match vec.try_into() {
        Ok(arr) => Ok(arr),
        Err(vec) => Err(format!("Expected Vec of length {}, but got {}", N, vec.len())),
    }
}
