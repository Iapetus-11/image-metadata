use std::{
    fmt::Debug,
    io::{Cursor, Read},
};

use crate::heif;

#[derive(Debug, PartialEq)]
pub enum Endianness {
    Little,
    Big,
}

#[derive(Debug, PartialEq)]
pub enum FileKind {
    Jpeg,
    Png,
    Heif,
}

pub fn determine_file_kind(data: &[u8]) -> Option<FileKind> {
    if data.len() >= 2 && data[0..3] == *b"\xFF\xD8\xFF" {
        return Some(FileKind::Jpeg);
    }

    if data.len() >= 8 && data[0..8] == [137, 80, 78, 71, 13, 10, 26, 10] {
        return Some(FileKind::Png);
    }

    {
        let mut cursor = Cursor::new(data.to_vec());
        let (atom_name, _) = heif::read_atom_header(&mut cursor);

        if atom_name.as_str() == "ftyp" {
            return Some(FileKind::Heif);
        }
    }

    None
}

pub fn vec_to_array<T, const N: usize>(vec: Vec<T>) -> Result<[T; N], String> {
    match vec.try_into() {
        Ok(arr) => Ok(arr),
        Err(vec) => Err(format!(
            "Expected Vec of length {}, but got {}",
            N,
            vec.len()
        )),
    }
}

#[macro_export]
macro_rules! unpack {
    ($data:expr, $type:ty, $endianness:path) => {
        match $endianness {
            crate::utils::Endianness::Little => <$type>::from_le_bytes($data),
            crate::utils::Endianness::Big => <$type>::from_be_bytes($data),
        }
    };
}

#[macro_export]
macro_rules! read_unpack {
    ($cursor:expr, $type:ty, $endianness:path) => {{
        let mut buf = [0_u8; (<$type>::BITS / 8) as usize];
        $cursor.read_exact(&mut buf).unwrap();

        crate::unpack!(buf, $type, $endianness)
    }};
}

pub fn read_sized_string(cursor: &mut Cursor<Vec<u8>>, size: usize) -> String {
    let mut buf = vec![0_u8; size];
    cursor.read_exact(&mut buf).unwrap();

    let str_data: Vec<u8> = buf.into_iter().filter(|c| *c != 0).collect();

    String::from_utf8_lossy(&str_data).to_string()
}

pub fn read_c_string(cursor: &mut Cursor<Vec<u8>>) -> String {
    let mut str_data: Vec<u8> = vec![];

    let mut buf = [0_u8; 1];
    loop {
        let read_size = cursor.read(&mut buf).unwrap();

        if read_size == 0 || buf[0] == 0 {
            break;
        }

        str_data.push(buf[0]);
    }

    String::from_utf8_lossy(&str_data).to_string()
}

pub fn get_nibbles(byte: u8) -> (u8, u8) {
    let a = byte & 0x0F;
    let b = (byte >> 4) & 0x0F;

    (a, b)
}
