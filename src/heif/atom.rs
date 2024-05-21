use std::fmt;
use std::io::{Cursor, Read};

use crate::heif::atoms;
use crate::{read_unpack, utils::Endianness};

pub trait Atom: fmt::Debug {
    fn read_from(name: String, size: u64, cursor: &mut Cursor<Vec<u8>>) -> Self
    where
        Self: Sized;
}

#[derive(Debug)]
pub enum AtomVariant {
    Unknown(atoms::AtomUnknown),
    Ftyp(atoms::AtomFtyp),
    Meta(atoms::AtomMeta),
    MetaHdlr(atoms::AtomMetaHdlr),
    MetaDinf(atoms::AtomMetaDinf),
    MetaDinfDref(atoms::AtomMetaDinfDref),
    MetaDinfDrefEntry(atoms::AtomMetaDinfDrefEntry),
    MetaPitm(atoms::AtomMetaPitm),
    MetaIinf(atoms::AtomMetaIinf),
    MetaIinfInfe(atoms::AtomMetaIinfInfe),
    MetaIref(atoms::AtomMetaIref),
    MetaIloc(atoms::AtomMetaIloc),
}

#[macro_export]
macro_rules! get_atom_value {
    ($atom:expr, $variant:path) => {
        match $atom {
            $variant(value) => Some(value),
            _ => None,
        }
    };
}

#[macro_export]
macro_rules! find_atom_value {
    ($atoms:expr, $atom_variant:path) => {
        $atoms.iter().find_map(|atom: &AtomVariant| match atom {
            $atom_variant(value) => Some(value),
            _ => None,
        })
    };
}

pub fn read_atom_header(cursor: &mut Cursor<Vec<u8>>) -> (String, u64) {
    let mut size = read_unpack!(cursor, u32, Endianness::Big) as u64;

    let name = {
        let mut buf = [0_u8; 4];
        cursor.read_exact(&mut buf).unwrap();
        String::from_utf8_lossy(&buf).to_string()
    };

    // Atom size of 0 means last atom in file
    if size == 0 {
        size = cursor.get_ref().len() as u64 - cursor.position();
    } else if size == 1 {
        size = read_unpack!(cursor, u64, Endianness::Big);
    }

    (name, size)
}

pub fn read_sub_atom(parent: &str, cursor: &mut Cursor<Vec<u8>>) -> AtomVariant {
    let (name, size) = read_atom_header(cursor);

    match format!("{}.{}", parent, name).as_str() {
        "meta.hdlr" => AtomVariant::MetaHdlr(atoms::AtomMetaHdlr::read_from(name, size, cursor)),
        "meta.dinf" => AtomVariant::MetaDinf(atoms::AtomMetaDinf::read_from(name, size, cursor)),
        "meta.dinf.dref" => {
            AtomVariant::MetaDinfDref(atoms::AtomMetaDinfDref::read_from(name, size, cursor))
        }
        "meta.dinf.dref.alis" | "meta.dinf.dref.rsrc" | "meta.dinf.dref.url " => {
            AtomVariant::MetaDinfDrefEntry(atoms::AtomMetaDinfDrefEntry::read_from(
                name, size, cursor,
            ))
        }
        "meta.pitm" => AtomVariant::MetaPitm(atoms::AtomMetaPitm::read_from(name, size, cursor)),
        "meta.iinf" => AtomVariant::MetaIinf(atoms::AtomMetaIinf::read_from(name, size, cursor)),
        "meta.iinf.infe" => {
            AtomVariant::MetaIinfInfe(atoms::AtomMetaIinfInfe::read_from(name, size, cursor))
        }
        "meta.iref" => AtomVariant::MetaIref(atoms::AtomMetaIref::read_from(name, size, cursor)),
        "meta.iloc" => AtomVariant::MetaIloc(atoms::AtomMetaIloc::read_from(name, size, cursor)),
        _ => AtomVariant::Unknown(atoms::AtomUnknown::read_from(name, size, cursor)),
    }
}

pub fn read_top_atom(cursor: &mut Cursor<Vec<u8>>) -> AtomVariant {
    let (name, size) = read_atom_header(cursor);

    match name.as_str() {
        "ftyp" => AtomVariant::Ftyp(atoms::AtomFtyp::read_from(name, size, cursor)),
        "meta" => AtomVariant::Meta(atoms::AtomMeta::read_from(name, size, cursor)),
        _ => AtomVariant::Unknown(atoms::AtomUnknown::read_from(name, size, cursor)),
    }
}

// TODO: Use this everywhere
// Read the version (u8) and flags (technically a bit(24)) for an Atom
pub fn read_version_and_flags(cursor: &mut Cursor<Vec<u8>>) -> (u8, u32) {
    let version = read_unpack!(cursor, u8, Endianness::Big);
    let flags = {
        (read_unpack!(cursor, u8, Endianness::Big) as u32) << 16
            | (read_unpack!(cursor, u8, Endianness::Big) as u32) << 8
            | (read_unpack!(cursor, u8, Endianness::Big) as u32)
    };

    (version, flags)
}
