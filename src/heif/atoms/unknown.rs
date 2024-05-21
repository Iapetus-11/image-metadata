use std::io::{Cursor, Read};

use crate::heif::atom::Atom;

#[derive(Debug)]
pub struct AtomUnknown {
    pub name: String,
    pub data: Vec<u8>,
}

impl Atom for AtomUnknown {
    fn read_from(name: String, size: u64, cursor: &mut Cursor<Vec<u8>>) -> Self {
        let mut data = vec![0_u8; (size - 8) as usize];
        cursor.read(&mut data).unwrap();
        AtomUnknown { name, data: vec![] } 
    }
}
