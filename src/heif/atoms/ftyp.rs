use std::io::{Cursor, Read};

use crate::{heif::atom::Atom, read_unpack, utils::Endianness};

#[derive(Debug)]
pub struct AtomFtyp {
    pub major_brand: String,
    pub minor_version: i32,
    pub compatible_brands: Vec<String>,
}

impl Atom for AtomFtyp {
    fn read_from(_: String, size: u64, cursor: &mut Cursor<Vec<u8>>) -> Self {
        let major_brand = {
            let mut buf = [0_u8; 4];
            cursor.read_exact(&mut buf).unwrap();
            String::from_utf8_lossy(&buf).to_string()
        };

        let minor_version = read_unpack!(cursor, i32, Endianness::Big);

        let compatible_brands: Vec<String> = (0..(size / 4)-4).map(|_| {
            let mut buf = [0_u8; 4];
            cursor.read_exact(&mut buf).unwrap();
            String::from_utf8_lossy(&buf).to_string()
        }).collect();

        AtomFtyp { major_brand, minor_version, compatible_brands }
    }
}
