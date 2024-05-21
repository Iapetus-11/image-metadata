// http://fileformats.archiveteam.org/wiki/Boxes/atoms_format
// https://b.goeswhere.com/ISO_IEC_14496-12_2015.pdf
// https://developer.apple.com/documentation/quicktime-file-format/atoms
// https://xhelmboyx.tripod.com/formats/mp4-layout.txt

use std::io::{Cursor, Read, Seek, SeekFrom};

use crate::{
    find_atom_value,
    heif::{atom::read_top_atom, atoms::AtomMetaIinfInfeVariant},
    tiff::{self, read_exif_section},
};

use super::{atom::AtomVariant, atoms::AtomMetaIlocItem};

#[derive(Debug)]
pub struct Heif {
    pub atoms: Vec<AtomVariant>,
    pub exif: Option<tiff::Tiff>,
    pub xmp: Option<String>,
}

fn get_iloc_item_for_item_type<'a, 'b>(
    atoms: &'a Vec<AtomVariant>,
    item_type: &'b str,
) -> Option<&'a AtomMetaIlocItem> {
    let meta = find_atom_value!(atoms, AtomVariant::Meta)?;
    let iinf = find_atom_value!(meta.children, AtomVariant::MetaIinf)?;
    let iloc = find_atom_value!(meta.children, AtomVariant::MetaIloc)?;

    let exif_item_id = iinf.entries.iter().find_map(|item| match &item.value {
        AtomMetaIinfInfeVariant::V2Or3 {
            item_id: id,
            item_type: it,
            ..
        } => {
            if it.as_str() == item_type {
                Some(id)
            } else {
                None
            }
        }
        _ => None,
    })?;

    iloc.items.iter().find(|item| item.item_id == *exif_item_id)
}

fn get_exif(atoms: &Vec<AtomVariant>, cursor: &mut Cursor<Vec<u8>>) -> Option<tiff::Tiff> {
    let exif_iloc_item = get_iloc_item_for_item_type(atoms, "Exif")?;

    let start = exif_iloc_item.extents[0].extent_offset as usize;
    let length = exif_iloc_item.extents[0].extent_length as usize;

    let mut data = vec![0_u8; length - 4];
    cursor.seek(SeekFrom::Start(start as u64 + 4)).unwrap();
    cursor.read_exact(&mut data).unwrap();

    match read_exif_section(&data) {
        Ok(t) => Some(t),
        Err(_) => None,
    }
}

fn get_xmp(atoms: &Vec<AtomVariant>, cursor: &mut Cursor<Vec<u8>>) -> Option<String> {
    let item = get_iloc_item_for_item_type(&atoms, "mime")?;

    let start = item.extents[0].extent_offset as usize;
    let length = item.extents[0].extent_length as usize;

    let mut data = vec![0_u8; length];
    cursor.seek(SeekFrom::Start(start as u64)).unwrap();
    cursor.read_exact(&mut data).unwrap();

    Some(String::from_utf8_lossy(&data).to_string())
}

pub fn read_heif(data: Vec<u8>) -> Heif {
    let file_size = data.len() as u64;
    let mut cursor = Cursor::new(data);
    let mut atoms: Vec<AtomVariant> = vec![];

    while cursor.position() < file_size {
        atoms.push(read_top_atom(&mut cursor));
    }

    let exif = get_exif(&atoms, &mut cursor);
    let xmp = get_xmp(&atoms, &mut cursor);

    Heif { atoms, exif, xmp }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::{find_atom_value, get_tag_value, heif::AtomVariant, tiff::TiffTag, utils::Endianness};

    use super::read_heif;

    #[test]
    fn test_read_miata2_iphone() {
        let data = fs::read("test_images/Miata2.HEIC").unwrap();

        let heif = read_heif(data);

        assert_eq!(heif.atoms.len(), 3);
        
        let meta = find_atom_value!(heif.atoms, AtomVariant::Meta).unwrap();
        let iinf = find_atom_value!(meta.children, AtomVariant::MetaIinf).unwrap();
        let iloc = find_atom_value!(meta.children, AtomVariant::MetaIloc).unwrap();
        assert_eq!(iinf.entries.len(), 53);
        assert_eq!(iloc.items.len(), 53);

        let exif = heif.exif.unwrap();
        assert_eq!(exif.endianness, Endianness::Big);
        assert_eq!(get_tag_value!(exif.tags, TiffTag::GPSAltitude).unwrap(), &1074.3307593307593);
        assert_eq!(get_tag_value!(exif.tags, TiffTag::GPSLatitude).unwrap(), &[35.0, 39.0, 44.46]);
        assert_eq!(get_tag_value!(exif.tags, TiffTag::GPSLongitude).unwrap(), &[82.0, 30.0, 21.56]);

        assert_ne!(heif.xmp, None);
    }

    #[test]
    fn test_image1() {
        let data = fs::read("test_images/image1.heic").unwrap();

        let heif = read_heif(data);

        let exif = heif.exif.unwrap();
        assert_eq!(get_tag_value!(exif.tags, TiffTag::Orientation).unwrap(), &1);
        assert_eq!(get_tag_value!(exif.tags, TiffTag::XResolution).unwrap(), &72.0);
        assert_eq!(get_tag_value!(exif.tags, TiffTag::YResolution).unwrap(), &72.0);
        assert_eq!(get_tag_value!(exif.tags, TiffTag::ResolutionUnit).unwrap(), "inch");
    }
}
