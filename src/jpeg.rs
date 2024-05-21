use std::io::{Cursor, Read, Seek, SeekFrom};

use crate::{
    read_unpack,
    tiff::{read_exif_section, Tiff},
    utils::Endianness,
};

use super::tiff;

#[derive(Debug)]
pub struct JpegError(pub String);

#[derive(Debug)]
pub struct Jpeg {
    pub comment: Option<String>,
    pub exif: Option<tiff::Tiff>,
    pub xmp: Option<String>,
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

fn get_jpeg_sections(data: &[u8]) -> Vec<(JpegMarker, Vec<u8>)> {
    let mut cursor = Cursor::new(data);
    cursor.seek(SeekFrom::Start(2)).unwrap();

    let data_len = data.len() as u64;

    let mut sections: Vec<(JpegMarker, Vec<u8>)> = Vec::new();

    loop {
        if cursor.position() >= data_len - 2 {
            break;
        }

        let marker = JpegMarker::from({
            let mut header = [0_u8; 2];
            cursor.read_exact(&mut header).unwrap();

            if header[0] != 0xFF {
                panic!("Expected 0xFF but got {:#04x}", header[0]);
            }

            header[1]
        });

        // -2 because the size includes the size bytes
        let size = read_unpack!(cursor, u16, Endianness::Big) as usize - 2;

        let mut section_data: Vec<u8> = vec![0; size];
        cursor.read_exact(&mut section_data).unwrap();

        // The SOS marker's length is only for its "header", so we need to collect
        // the compressed data after until the next marker
        if marker == JpegMarker::SOS {
            let mut buf = [0_u8; 2];

            loop {
                buf[0] = buf[1];
                if cursor.read(&mut buf[1..]).unwrap() == 0 {
                    break;
                }

                // Skip forward till we find a marker which isn't 0xFF or a restart marker (0xD0-0xD7)
                if buf[0] == 0xFF && ![0, 0xFF].contains(&buf[1]) && !(0xD0..0xD8).contains(&buf[1])
                {
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

pub fn read_jpeg(data: &[u8]) -> Result<Jpeg, JpegError> {
    let sections = get_jpeg_sections(data);

    let app1_section = sections
        .iter()
        .filter(|(m, _)| m == &JpegMarker::APP1)
        .map(|(_, d)| d)
        .next();

    let mut exif: Option<Tiff> = None;
    let mut xmp: Option<String> = None;
    match app1_section {
        Some(d) => {
            if d[0..4] == *b"Exif" {
                exif = match read_exif_section(d) {
                    Ok(t) => Some(t),
                    Err(m) => return Err(JpegError(m.0)),
                }
            }

            if d[0..4] == *b"http" {
                xmp = Some(String::from_utf8_lossy(d).to_string());
            }
        }
        _ => {}
    }

    let comment = sections
        .iter()
        .filter(|(m, _)| m == &JpegMarker::COM)
        .map(|(_, d)| String::from_utf8_lossy(d).to_string())
        .next();

    Ok(Jpeg { comment, exif, xmp })
}

#[cfg(test)]
mod tests {
    use super::read_jpeg;
    use crate::{get_tag_value, tiff::TiffTag, utils::Endianness};
    use std::fs;

    #[test]
    fn test_read_painttool_sample() {
        let data = fs::read("test_images/PaintTool_sample.jpeg").unwrap();

        let jpeg = read_jpeg(&data).unwrap();
        let exif_data = jpeg.exif.unwrap();

        assert!(jpeg.comment.is_none());

        assert_eq!(exif_data.endianness, Endianness::Little);

        assert_eq!(
            get_tag_value!(exif_data.tags, TiffTag::Software).unwrap(),
            "GIMP 2.4.5",
        );

        assert_eq!(
            *get_tag_value!(exif_data.tags, TiffTag::PixelXDimension).unwrap(),
            88,
        );

        assert_eq!(
            *get_tag_value!(exif_data.tags, TiffTag::PixelYDimension).unwrap(),
            100,
        );
    }

    #[test]
    fn test_read_test9() {
        let data = fs::read("test_images/test9.jpeg").unwrap();
        let jpeg = read_jpeg(&data).unwrap();
        let exif_data = jpeg.exif.unwrap();

        assert_eq!(exif_data.endianness, Endianness::Little);

        assert_eq!(
            get_tag_value!(exif_data.tags, TiffTag::Make).unwrap(),
            "NIKON"
        );

        assert_eq!(
            get_tag_value!(exif_data.tags, TiffTag::Model).unwrap(),
            "COOLPIX P510"
        );

        assert_eq!(
            get_tag_value!(exif_data.tags, TiffTag::Software).unwrap(),
            "COOLPIX P510   V1.0"
        );

        assert_eq!(
            get_tag_value!(exif_data.tags, TiffTag::ExposureMode).unwrap(),
            "Auto exposure"
        );

        assert_eq!(
            get_tag_value!(exif_data.tags, TiffTag::DigitalZoomRatio).unwrap(),
            "0/100"
        );
    }

    #[test]
    fn test_gps_data() {
        let data = fs::read("test_images/gps.jpeg").unwrap();
        let jpeg = read_jpeg(&data).unwrap();

        let exif_data = jpeg.exif.unwrap();

        assert_eq!(exif_data.endianness, Endianness::Little);

        assert_eq!(
            get_tag_value!(exif_data.tags, TiffTag::GPSLatitudeRef).unwrap(),
            "N",
        );

        assert_eq!(
            *get_tag_value!(exif_data.tags, TiffTag::GPSLatitude).unwrap(),
            [43.0, 28.0, 1.76399999],
        );

        assert_eq!(
            get_tag_value!(exif_data.tags, TiffTag::GPSLongitudeRef).unwrap(),
            "E",
        );

        assert_eq!(
            *get_tag_value!(exif_data.tags, TiffTag::GPSLongitude).unwrap(),
            [11.0, 53.0, 7.42199999],
        );

        assert_eq!(
            get_tag_value!(exif_data.tags, TiffTag::GPSAltitudeRef).unwrap(),
            "Above sea level",
        );

        assert_eq!(
            *get_tag_value!(exif_data.tags, TiffTag::GPSTimeStamp).unwrap(),
            [14.0, 28.0, 17.24],
        );

        assert_eq!(
            get_tag_value!(exif_data.tags, TiffTag::GPSSatellites).unwrap(),
            "06",
        );

        assert_eq!(
            get_tag_value!(exif_data.tags, TiffTag::GPSMapDatum).unwrap(),
            "WGS-84   ",
        );

        assert_eq!(
            get_tag_value!(exif_data.tags, TiffTag::GPSDateStamp).unwrap(),
            "2008:10:23",
        )
    }

    #[test]
    fn test_no_exif_only_comment() {
        let data = fs::read("test_images/only_comment.jpg").unwrap();
        let jpeg = read_jpeg(&data).unwrap();

        assert!(jpeg.exif.is_none());

        assert_eq!(
            jpeg.comment,
            Some("..and henceforth, shall he be named Frank, for he is a pumpkin.".to_string())
        );
    }

    #[test]
    fn test_no_exif_only_xmp() {
        let data = fs::read("test_images/no_exif_only_xmp.jpeg").unwrap();
        let jpeg = read_jpeg(&data).unwrap();

        assert!(jpeg.exif.is_none());

        let xmp = jpeg.xmp.unwrap();

        assert!(xmp.starts_with("http://ns.adobe.com/xap/1.0/\0<?xpacket"));
    }
}
