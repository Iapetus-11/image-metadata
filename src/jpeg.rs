use std::io::{Cursor, Read, Seek, SeekFrom};

use crate::tiff::read_ifd;

use super::tiff;

#[derive(Debug)]
pub struct JpegError(pub String);

#[derive(Debug)]
pub struct Jpeg {
    pub comment: Option<String>,
    pub exif: Option<tiff::Tiff>,
}

// This enum is very incomplete and only contains some markers I encountered when testing
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum JpegMarker {
    UNKNOWN(u8),

    SOF0, // Start of Frame 0
    SOF1, // Start of Frame 1
    SOF2, // Start of Frame 2
    SOF3, // Start of Frame 3

    DHT, // Define Huffman Table

    RST0, // Restart Marker 0
    RST1, // Restart Marker 1
    RST2, // Restart Marker 2
    RST3, // Restart Marker 3

    APP0,
    APP1,
    APP2,

    SOS, // Start of scan

    DQT,

    DRI, // Define Restart Interval

    COM, // Comment
}

impl From<u8> for JpegMarker {
    fn from(value: u8) -> JpegMarker {
        match value {
            0xC0 => JpegMarker::SOF0,
            0xC1 => JpegMarker::SOF1,
            0xC2 => JpegMarker::SOF2,
            0xC3 => JpegMarker::SOF3,
            0xC4 => JpegMarker::DHT,
            0xD0 => JpegMarker::RST0,
            0xD1 => JpegMarker::RST1,
            0xD2 => JpegMarker::RST2,
            0xD3 => JpegMarker::RST3,
            0xE0 => JpegMarker::APP0,
            0xE1 => JpegMarker::APP1,
            0xE2 => JpegMarker::APP2,
            0xDA => JpegMarker::SOS,
            0xDB => JpegMarker::DQT,
            0xDD => JpegMarker::DRI,
            0xFE => JpegMarker::COM,
            _ => JpegMarker::UNKNOWN(value),
        }
    }
}

impl Into<u8> for JpegMarker {
    fn into(self) -> u8 {
        match self {
            JpegMarker::SOF0 => 0xC0,
            JpegMarker::SOF1 => 0xC1,
            JpegMarker::SOF2 => 0xC2,
            JpegMarker::SOF3 => 0xC3,
            JpegMarker::DHT => 0xC4,
            JpegMarker::RST0 => 0xD0,
            JpegMarker::RST1 => 0xD1,
            JpegMarker::RST2 => 0xD2,
            JpegMarker::RST3 => 0xD3,
            JpegMarker::APP0 => 0xE0,
            JpegMarker::APP1 => 0xE1,
            JpegMarker::APP2 => 0xE2,
            JpegMarker::SOS => 0xDA,
            JpegMarker::DQT => 0xDB,
            JpegMarker::DRI => 0xDD,
            JpegMarker::COM => 0xFE,
            JpegMarker::UNKNOWN(unknown) => unknown,
        }
    }
}

fn get_jpeg_sections(data: &Vec<u8>) -> Vec<(JpegMarker, Vec<u8>)> {
    let mut cursor = Cursor::new(data);
    cursor.seek(SeekFrom::Start(2)).unwrap();

    let data_len = data.len() as u64;
    let mut size: usize;

    let mut sections: Vec<(JpegMarker, Vec<u8>)> = Vec::new();

    loop {
        if cursor.position() >= data_len - 2 {
            break;
        }

        let marker = JpegMarker::from({
            let mut header = [0_u8; 2];
            cursor.read_exact(&mut header).unwrap();

            if header[0] != 0xFF {
                println!(
                    "D: {:?}",
                    sections
                        .iter()
                        .map(|(m, _)| *m)
                        .collect::<Vec<JpegMarker>>()
                );
                println!(
                    "D: {}",
                    sections
                        .iter()
                        .map(|(m, _)| format!("{:#04x}", <JpegMarker as Into<u8>>::into(*m)))
                        .collect::<Vec<String>>()
                        .join(", ")
                );
                let mut sus = [0_u8; 48];
                cursor.seek(SeekFrom::Current(-12)).unwrap();
                cursor.read_exact(&mut sus).unwrap();
                println!("{:?}", sus);
                panic!("Expected 0xFF but got {:#04x}", header[0]);
            }

            header[1]
        });

        size = {
            let mut buf = [0_u8; 2];
            cursor.read_exact(&mut buf).unwrap();
            u16::from_be_bytes(buf)
        } as usize
            - 2; // -2 because the size includes the size bytes

        let mut section_data: Vec<u8> = vec![0; size];
        cursor.read_exact(&mut section_data).unwrap();

        // The SOS marker's length is only for its "header", so we need to collect
        // the compressed data after until the next marker
        if [JpegMarker::SOS].contains(&marker) {
            println!("{:?} detected... scanning forward", marker);
            let mut buf = [0_u8; 2];

            loop {
                buf[0] = buf[1];
                if cursor.read(&mut buf[1..]).unwrap() == 0 {
                    break;
                }
                
                // Skip forward till we find a marker which isn't 0xFF or a restart marker (0xD0-0xD7)
                if buf[0] == 0xFF && ![0, 0xFF].contains(&buf[1]) && !(0xD0..0xD8).contains(&buf[1]) {
                    cursor.seek(SeekFrom::Current(-2)).unwrap();
                    break;
                }

                section_data.push(buf[0]);
            }
        }

        sections.push((marker, section_data));
    }

    sections
}

fn parse_exif_section(data: &Vec<u8>) -> Result<tiff::Tiff, JpegError> {
    if String::from_utf8_lossy(&data[0..4]) != "Exif" {
        return Err(JpegError(format!(
            "Expected 'Exif' but got {} instead",
            String::from_utf8_lossy(&data[0..4].to_vec())
        )));
    }

    if data[4..6] != [0, 0] {
        return Err(JpegError(format!(
            "Expected [0, 0] but got {:?} instead",
            data[4..6].to_vec()
        )));
    }

    let mut cursor = Cursor::new(data[6..].to_vec());

    match tiff::read_tiff(&mut cursor) {
        Ok(tiff) => Ok(tiff),
        Err(err) => Err(JpegError(err.0)),
    }
}

pub fn read_jpeg(data: Vec<u8>) -> Result<Jpeg, JpegError> {
    let sections = get_jpeg_sections(&data);

    let exif_section = sections
        .iter()
        .filter(|(m, _)| m == &JpegMarker::APP1)
        .map(|(_, d)| d)
        .next();
    let exif = match exif_section {
        Some(data) => Some(parse_exif_section(data)?),
        None => None,
    };

    let comment = sections
        .iter()
        .filter(|(m, _)| m == &JpegMarker::COM)
        .map(|(_, d)| String::from_utf8_lossy(d).to_string())
        .next();

    Ok(Jpeg { comment, exif })
}
