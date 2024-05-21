use std::io::{Cursor, Read};

use crate::{
    get_atom_value,
    heif::{
        atom::{read_atom_header, read_sub_atom, Atom, AtomVariant},
        read_version_and_flags,
    },
    read_unpack,
    utils::{get_nibbles, read_c_string, read_sized_string, Endianness},
};

#[derive(Debug)]
pub struct AtomMeta {
    pub version: u8,
    pub flags: u32,
    pub children: Vec<AtomVariant>,
}

impl Atom for AtomMeta {
    fn read_from(_: String, size: u64, cursor: &mut Cursor<Vec<u8>>) -> Self {
        let (version, flags) = read_version_and_flags(cursor);

        let start_position = cursor.position();
        let size_minus_already_read = size - (4 * 3);
        let mut children: Vec<AtomVariant> = vec![];
        while cursor.position() - start_position < size_minus_already_read {
            children.push(read_sub_atom("meta", cursor));
        }

        AtomMeta {
            version,
            flags,
            children,
        }
    }
}

#[derive(Debug)]
pub struct AtomMetaHdlr {
    pub version: u8,
    pub flags: u32,
    pub predefined: u32,
    pub handler_type: String,
    pub reserved: [u32; 3],
    pub name: String,
}

impl Atom for AtomMetaHdlr {
    fn read_from(_: String, size: u64, cursor: &mut Cursor<Vec<u8>>) -> Self {
        let (version, flags) = read_version_and_flags(cursor);
        let predefined = read_unpack!(cursor, u32, Endianness::Big);
        let handler_type = read_sized_string(cursor, 4);
        let reserved = [
            read_unpack!(cursor, u32, Endianness::Big),
            read_unpack!(cursor, u32, Endianness::Big),
            read_unpack!(cursor, u32, Endianness::Big),
        ];
        let name = read_sized_string(cursor, (size - (4 * 8)) as usize);

        AtomMetaHdlr {
            version,
            flags,
            predefined,
            handler_type,
            reserved,
            name,
        }
    }
}

#[derive(Debug)]
pub struct AtomMetaDinf {
    pub data_references: AtomMetaDinfDref,
}

impl Atom for AtomMetaDinf {
    fn read_from(_: String, _: u64, cursor: &mut Cursor<Vec<u8>>) -> Self
    where
        Self: Sized,
    {
        let sub_atom = read_sub_atom("meta.dinf", cursor);
        let data_references = get_atom_value!(sub_atom, AtomVariant::MetaDinfDref).unwrap();

        AtomMetaDinf { data_references }
    }
}

#[derive(Debug)]
pub struct AtomMetaDinfDrefEntry {
    pub name: String,
    pub version: u8,
    pub flags: u32,
    pub string_value: String,
}

impl Atom for AtomMetaDinfDrefEntry {
    fn read_from(name: String, size: u64, cursor: &mut Cursor<Vec<u8>>) -> Self
    where
        Self: Sized,
    {
        let (version, flags) = read_version_and_flags(cursor);
        let string_value = read_sized_string(cursor, (size - 12) as usize);

        AtomMetaDinfDrefEntry {
            name,
            version,
            flags,
            string_value,
        }
    }
}

#[derive(Debug)]
pub struct AtomMetaDinfDref {
    pub version: u8,
    pub flags: u32,
    pub data_references: Vec<AtomMetaDinfDrefEntry>,
}

impl Atom for AtomMetaDinfDref {
    fn read_from(_: String, _: u64, cursor: &mut Cursor<Vec<u8>>) -> Self
    where
        Self: Sized,
    {
        let (version, flags) = read_version_and_flags(cursor);
        let number_of_entries = read_unpack!(cursor, u32, Endianness::Big);

        let mut entries: Vec<AtomMetaDinfDrefEntry> = vec![];
        for _ in 0..number_of_entries {
            match read_sub_atom("meta.dinf.dref", cursor) {
                AtomVariant::MetaDinfDrefEntry(entry) => entries.push(entry),
                atom => panic!(
                    "Encountered atom of unexpected type (expected alis|rsrc|url): {:?}",
                    atom
                ),
            }
        }

        AtomMetaDinfDref {
            version,
            flags,
            data_references: entries,
        }
    }
}

#[derive(Debug)]
pub struct AtomMetaPitm {
    pub version: u8,
    pub flags: u32,
    pub item_id: u32,
}

impl Atom for AtomMetaPitm {
    fn read_from(_: String, _: u64, cursor: &mut Cursor<Vec<u8>>) -> Self
    where
        Self: Sized,
    {
        let (version, flags) = read_version_and_flags(cursor);

        let item_id: u32 = match version {
            0 => read_unpack!(cursor, u16, Endianness::Big) as u32,
            _ => read_unpack!(cursor, u32, Endianness::Big),
        };

        AtomMetaPitm {
            version,
            flags,
            item_id,
        }
    }
}

#[derive(Debug)]
pub enum AtomMetaIinfInfeVariant {
    V0Or1 {
        item_id: u16,
        item_protection_index: u16,
        item_name: String,
        content_type: String,
        content_encoding: Option<String>,

        // Only if V1
        extension_type: Option<u32>,
        extension: Option<Vec<u8>>,
        // extension: , // TODO
    },
    V2Or3 {
        item_id: u32, // Converted from a u16 if not V3
        item_protection_index: u16,
        item_type: String,
        item_name: String,

        // If item_type == 'mime'
        content_type: Option<String>,
        content_encoding: Option<String>, // Optional

        // If item_type == 'uri '
        item_uri_type: Option<String>,
    },
    Unknown(String),
}

#[derive(Debug)]
pub struct AtomMetaIinfInfe {
    pub version: u8,
    pub flags: u32,
    pub value: AtomMetaIinfInfeVariant,
}

impl Atom for AtomMetaIinfInfe {
    fn read_from(_: String, size: u64, cursor: &mut Cursor<Vec<u8>>) -> Self
    where
        Self: Sized,
    {
        let atom_start = cursor.position();

        let (version, flags) = read_version_and_flags(cursor);

        let value: AtomMetaIinfInfeVariant = match version {
            // TODO: Properly handle V0 & V1
            2 | 3 => {
                let item_id: u32 = match version {
                    2 => read_unpack!(cursor, u16, Endianness::Big) as u32,
                    3 => read_unpack!(cursor, u32, Endianness::Big),
                    _ => panic!(
                        "Impossible value for AtomMetaIinfInfe version encountered: {}",
                        version
                    ),
                };
                let item_protection_index = read_unpack!(cursor, u16, Endianness::Big);
                let item_type = read_sized_string(cursor, 4);
                let item_name = read_c_string(cursor);

                let mut content_type = None;
                let mut content_encoding = None;
                let mut item_uri_type = None;
                match item_type.as_str() {
                    "mime" => {
                        content_type = Some(read_c_string(cursor));

                        // If we're at the end of the atom, this string doesn't exist
                        if cursor.position() - atom_start >= size {
                            content_encoding = Some(read_c_string(cursor));
                        }
                    }
                    "uri " => {
                        item_uri_type = Some(read_c_string(cursor));
                    }
                    _ => {}
                }

                AtomMetaIinfInfeVariant::V2Or3 {
                    item_id,
                    item_protection_index,
                    item_type,
                    item_name,
                    content_type,
                    content_encoding,
                    item_uri_type,
                }
            }
            _ => {
                let mut data = vec![0_u8; (size - 8) as usize];
                cursor.read_exact(&mut data).unwrap();
                AtomMetaIinfInfeVariant::Unknown(String::from_utf8_lossy(&data).to_string())
            }
        };

        AtomMetaIinfInfe {
            version,
            flags,
            value,
        }
    }
}

#[derive(Debug)]
pub struct AtomMetaIinf {
    pub version: u8,
    pub flags: u32,
    pub entries: Vec<AtomMetaIinfInfe>,
}

impl Atom for AtomMetaIinf {
    fn read_from(_: String, _: u64, cursor: &mut Cursor<Vec<u8>>) -> Self
    where
        Self: Sized,
    {
        let (version, flags) = read_version_and_flags(cursor);

        let number_of_entries: u32 = match version {
            0 => read_unpack!(cursor, u16, Endianness::Big) as u32,
            _ => read_unpack!(cursor, u32, Endianness::Big),
        };

        let mut entries: Vec<AtomMetaIinfInfe> = vec![];
        for _ in 0..number_of_entries {
            match read_sub_atom("meta.iinf", cursor) {
                AtomVariant::MetaIinfInfe(value) => entries.push(value),
                atom => panic!(
                    "Encountered atom of unexpected type (expected infe): {:?}",
                    atom
                ),
            }
        }

        AtomMetaIinf {
            version,
            flags,
            entries,
        }
    }
}

#[derive(Debug)]
pub struct AtomMetaIrefReference {
    pub name: String,
    pub from_item_id: u32,
    pub references: Vec<u32>,
}

#[derive(Debug)]
pub struct AtomMetaIref {
    pub version: u8,
    pub flags: u32,
    pub entries: Vec<AtomMetaIrefReference>,
}

impl Atom for AtomMetaIref {
    fn read_from(_: String, size: u64, cursor: &mut Cursor<Vec<u8>>) -> Self
    where
        Self: Sized,
    {
        let (version, flags) = read_version_and_flags(cursor);

        let mut entries: Vec<AtomMetaIrefReference> = vec![];
        let start_position = cursor.position();
        let size_minus_already_read = size - (4 * 3);
        while cursor.position() - start_position < size_minus_already_read {
            let (sub_name, _) = read_atom_header(cursor);

            let from_item_id: u32 = match version {
                0 => read_unpack!(cursor, u16, Endianness::Big) as u32,
                1 => read_unpack!(cursor, u32, Endianness::Big),
                _ => panic!(
                    "Impossible value for AtomMetaIref version encountered: {}",
                    version
                ),
            };

            let reference_count = read_unpack!(cursor, u16, Endianness::Big);
            let references: Vec<u32> = (0..reference_count)
                .map(|_| match version {
                    0 => read_unpack!(cursor, u16, Endianness::Big) as u32,
                    1 => read_unpack!(cursor, u32, Endianness::Big),
                    _ => panic!(
                        "Impossible value for AtomMetaIref version encountered: {}",
                        version
                    ),
                })
                .collect();

            entries.push(AtomMetaIrefReference {
                name: sub_name,
                from_item_id,
                references,
            });
        }

        AtomMetaIref {
            version,
            flags,
            entries,
        }
    }
}

#[derive(Debug)]
pub struct AtomMetaIlocItemExtent {
    pub extent_index: Option<u64>,
    pub extent_offset: u64,
    pub extent_length: u64,
}

#[derive(Debug)]
pub struct AtomMetaIlocItem {
    pub item_id: u32,
    pub reserved: Option<u16>,
    pub construction_method: Option<u8>,
    pub data_reference_index: u16,
    pub base_offset: u64,
    pub extents: Vec<AtomMetaIlocItemExtent>,
}

#[derive(Debug)]
pub struct AtomMetaIloc {
    pub version: u8,
    pub flags: u32,
    pub items: Vec<AtomMetaIlocItem>,
}

impl Atom for AtomMetaIloc {
    fn read_from(_: String, _: u64, cursor: &mut Cursor<Vec<u8>>) -> Self
    where
        Self: Sized,
    {
        let (version, flags) = read_version_and_flags(cursor);

        let (offset_size, length_size) = get_nibbles(read_unpack!(cursor, u8, Endianness::Big));
        let (base_offset_size, index_size_or_reserved) =
            get_nibbles(read_unpack!(cursor, u8, Endianness::Big));

        let item_count: u32 = match version {
            0 | 1 => read_unpack!(cursor, u16, Endianness::Big) as u32,
            2 => read_unpack!(cursor, u32, Endianness::Big),
            v => panic!(
                "Impossible value for AtomMetaIloc version encountered: {}",
                v
            ),
        };

        let mut items: Vec<AtomMetaIlocItem> = vec![];
        for _ in 0..item_count {
            let item_id = match version {
                0 | 1 => read_unpack!(cursor, u16, Endianness::Big) as u32,
                2 => read_unpack!(cursor, u32, Endianness::Big),
                v => panic!(
                    "Impossible value for AtomMetaIloc version encountered: {}",
                    v
                ),
            };

            let mut reserved: Option<u16> = None;
            let mut construction_method: Option<u8> = None;
            if version == 1 || version == 2 {
                let reserved_and_construction_method = read_unpack!(cursor, u16, Endianness::Big);

                reserved = Some(reserved_and_construction_method & 0xFFF);
                construction_method = Some((reserved_and_construction_method & 0b1111) as u8);
            }

            let data_reference_index = read_unpack!(cursor, u16, Endianness::Big);

            let base_offset: u64 = match base_offset_size {
                0 => 0,
                4 => read_unpack!(cursor, u32, Endianness::Big) as u64,
                8 => read_unpack!(cursor, u64, Endianness::Big),
                v => panic!(
                    "Impossible value for AtomMetaIloc.base_offset_size encountered: {}",
                    v
                ),
            };

            let extent_count = read_unpack!(cursor, u16, Endianness::Big);
            let mut extents: Vec<AtomMetaIlocItemExtent> = vec![];
            for _ in 0..extent_count {
                let mut extent_index: Option<u64> = None;
                if version == 1 || version == 2 {
                    extent_index = match index_size_or_reserved {
                        0 => Some(0),
                        4 => Some(read_unpack!(cursor, u32, Endianness::Big) as u64),
                        8 => Some(read_unpack!(cursor, u64, Endianness::Big)),
                        v => panic!(
                            "Impossible value for AtomMetaIloc.index_size encountered: {}",
                            v
                        ),
                    }
                }

                let extent_offset = match offset_size {
                    0 => 0,
                    4 => read_unpack!(cursor, u32, Endianness::Big) as u64,
                    8 => read_unpack!(cursor, u64, Endianness::Big),
                    v => panic!(
                        "Impossible value for AtomMetaIloc.offset_size encountered: {}",
                        v
                    ),
                };

                let extent_length = match length_size {
                    0 => 0,
                    4 => read_unpack!(cursor, u32, Endianness::Big) as u64,
                    8 => read_unpack!(cursor, u64, Endianness::Big),
                    v => panic!(
                        "Impossible value for AtomMetaIloc.length_size encountered: {}",
                        v
                    ),
                };

                extents.push(AtomMetaIlocItemExtent {
                    extent_index,
                    extent_offset,
                    extent_length,
                });
            }

            items.push(AtomMetaIlocItem {
                item_id,
                reserved,
                construction_method,
                data_reference_index,
                base_offset,
                extents,
            });
        }

        AtomMetaIloc {
            version,
            flags,
            items,
        }
    }
}
