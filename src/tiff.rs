use std::io::{Cursor, Read};

use super::utils::{vec_to_array, Endianness};

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum IFDEntryValue {
    BYTE(u8),
    ASCII(u8),
    SHORT(u16),
    LONG(u32),
    RATIONAL(u32, u32),
    SBYTE(i8),
    UNDEFINED(u8),
    SSHORT(i16),
    SLONG(i32),
    SRATIONAL(i32, i32),
    FLOAT(f32),
    DOUBLE(f64),
}

fn get_tiff_value_type_size(value_type: u16) -> Result<usize, TiffError> {
    match value_type {
        1 => Ok(1),
        2 => Ok(1),
        3 => Ok(2),
        4 => Ok(4),
        5 => Ok(8),
        6 => Ok(1),
        7 => Ok(1),
        8 => Ok(2),
        9 => Ok(4),
        10 => Ok(8),
        11 => Ok(4),
        12 => Ok(8),
        unknown => Err(TiffError(format!("Encountered unknown TIFF value type: {}", unknown))),
    }
}

#[derive(Debug)]
pub struct IFDEntry {
    pub tag: u16,
    pub values: Vec<IFDEntryValue>,
}

impl IFDEntry {
    pub fn get_single_value(&self) -> Result<IFDEntryValue, TiffError> {
        match self.values.len() {
            1 => Ok(self.values[0]),
            count => Err(TiffError(
                format!("[Tag {}] Expected exactly one value (got {})", self.tag, count),
            )),
        }
    }
}

impl TryInto<String> for IFDEntry {
    type Error = TiffError;

    fn try_into(self) -> Result<String, Self::Error> {
        let mut chars: Vec<u8> = vec![];
        for value in self.values {
            match value {
                IFDEntryValue::ASCII(0) => {}
                IFDEntryValue::ASCII(b) => chars.push(b),
                _ => {
                    return Err(TiffError(
                        format!("[Tag {}] Expected only ASCII values (got {:?})", self.tag, value),
                    ))
                }
            }
        }

        Ok(String::from_utf8_lossy(&chars).to_string())
    }
}

impl TryInto<u8> for IFDEntry {
    type Error = TiffError;

    fn try_into(self) -> Result<u8, Self::Error> {
        match self.get_single_value()? {
            IFDEntryValue::BYTE(v) => Ok(v),
            _ => Err(TiffError(
                format!("[Tag {}] Expected value to be BYTE (got {:?})", self.tag, self.values[0]),
            )),
        }
    }
}

impl TryInto<Vec<u8>> for IFDEntry {
    type Error = TiffError;

    fn try_into(self) -> Result<Vec<u8>, Self::Error> {
        let mut result = vec![];

        for v in self.values {
            match v {
                IFDEntryValue::BYTE(v) | IFDEntryValue::ASCII(v) | IFDEntryValue::UNDEFINED(v) => {
                    result.push(v)
                }
                _ => {
                    return Err(TiffError(
                        format!(
                            "[Tag {}] Expected all values to be BYTE/ASCII/UNDEFINED (got {:?})",
                            self.tag,
                            v,
                        ),
                    ))
                }
            }
        }

        Ok(result)
    }
}

impl TryInto<u16> for IFDEntry {
    type Error = TiffError;

    fn try_into(self) -> Result<u16, Self::Error> {
        match self.get_single_value()? {
            IFDEntryValue::SHORT(v) => Ok(v),
            _ => Err(TiffError(
                format!("[Tag {}] Expected value to be SHORT (got {:?})", self.tag, self.values[0]),
            )),
        }
    }
}

impl TryInto<u32> for IFDEntry {
    type Error = TiffError;

    fn try_into(self) -> Result<u32, Self::Error> {
        match self.get_single_value()? {
            IFDEntryValue::LONG(v) => Ok(v),
            _ => Err(TiffError(
                format!("[Tag {}] Expected value to be LONG (got {:?})", self.tag, self.values[0]),
            )),
        }
    }
}

impl TryInto<f64> for IFDEntry {
    type Error = TiffError;

    fn try_into(self) -> Result<f64, Self::Error> {
        match self.get_single_value()? {
            IFDEntryValue::RATIONAL(a, b) => Ok((a as f64) / (b as f64)),
            IFDEntryValue::SRATIONAL(a, b) => Ok((a as f64) / (b as f64)),
            IFDEntryValue::DOUBLE(v) => Ok(v),
            v => Err(TiffError(
                    format!("[Tag {}] Expected value to be RATIONAL/SRATIONAL/DOUBLE (got {:?})", self.tag, v),
            ))
        }
    }
}

impl TryInto<Vec<f64>> for IFDEntry {
    type Error = TiffError;

    fn try_into(self) -> Result<Vec<f64>, Self::Error> {
        let mut result: Vec<f64> = vec![];

        for v in self.values {
            match v {
                IFDEntryValue::RATIONAL(a, b) => result.push((a as f64) / (b as f64)),
                IFDEntryValue::SRATIONAL(a, b) => result.push((a as f64) / (b as f64)),
                IFDEntryValue::DOUBLE(v) => result.push(v),
                _ => {
                    return Err(TiffError(
                        format!("Expected all values to be RATIONAL/SRATIONAL/DOUBLE (got {:?})", v),
                    ))
                }
            }
        }

        Ok(result)
    }
}

// TODO: Some tags have overlapping IDs because the other IFDs (EXIF, GPS) can just put whatever tag IDs they want
#[derive(Debug)]
pub enum TiffTag {
    Unknown(IFDEntry),

    GPSVersionID([u8; 4]),
    GPSLatitudeRef(String),
    GPSLatitude([f64; 3]),
    GPSLongitudeRef(String),
    GPSLongitude([f64; 3]),
    GPSAltitudeRef(String),
    GPSAltitude(f64),
    GPSTimeStamp([f64; 3]),
    GPSSatellites(String),
    GPSStatus(String),
    GPSImgDirectionRef(String),
    GPSImgDirection(f64),
    GPSMapDatum(String),
    GPSDateStamp(String),
    Compression(String),
    ImageDescription(String),
    Make(String),
    Model(String),
    Orientation(u16),
    XResolution(f64),
    YResolution(f64),
    ResolutionUnit(String),
    Software(String),
    DateTime(String),
    Artist(String),

    Copyright(String),
    ExposureTime(String),
    FNumber(String),
    ExifIfdPointer(u32),
    ExposureProgram(String),
    GpsIfdPointer(u32),
    ExifVersion(String),
    DateTimeOriginal(String),
    DateTimeDigitized(String),
    CompressedBitsPerPixel(String),
    ShutterSpeedValue(String),
    ApertureValue(String),
    ExposureBiasValue(String),
    MaxApertureValue(String),
    MeteringMode(String),
    LightSource(String),
    Flash(String),
    FocalLength(String),
    MakerNote(Vec<u8>),
    UserComment(String),
    SubsecTime(String),
    SubsecTimeOriginal(String),
    SubsecTimeDigitized(String),

    FlashpixVersion(String),
    PixelXDimension(u32),
    PixelYDimension(u32),
    FocalPlaneXResolution(String),
    FocalPlaneYResolution(String),
    FocalPlaneResolutionUnit(String),
    SensingMethod(String),
    ExposureMode(String),
    WhiteBalance(String),
    DigitalZoomRatio(String),
    FocalLengthIn35mmFilm(u16),
    SceneCaptureType(String),
    GainControl(String),
    Contrast(String),
    Saturation(String),
    Sharpness(String),
    SubjectDistanceRange(String),
    
    // This enum is incomplete and includes tags I found interesting/encountered while testing
}

#[macro_export]
macro_rules! get_tag_value {
    ($tiff_tags:expr, $tag_variant:path) => {
        $tiff_tags.iter().find_map(|tag| match tag {
            $tag_variant(value) => Some(value),
            _ => None,
        })
    };
}

fn get_rational_repr_from_ifd_entry(entry: IFDEntry) -> Result<String, TiffError> {
    if entry.values.len() != 1 {
        return Err(TiffError(
            format!("Expected only one value (got {})", entry.values.len())
        ));
    }

    match entry.values[0] {
        IFDEntryValue::RATIONAL(a, b) => Ok(format!("{}/{}", a, b)),
        IFDEntryValue::SRATIONAL(a, b) => Ok(format!("{}/{}", a, b)),
        _ => Err(TiffError(format!(
            "Expected value to be RATIONAL/SRATIONAL (got {:?})",
            entry.values[0],
        ))),
    }
}

fn get_string_from_entry_with_undefined_values(entry: IFDEntry) -> Result<String, TiffError> {
    let mut string_data: Vec<u8> = vec![];

    for v in entry.values {
        match v {
            IFDEntryValue::UNDEFINED(b) => string_data.push(b),
            _ => return Err(TiffError(
                format!("Expected value to be UNDEFINED (got {:?})", v),
            )),
        }
    }

    Ok(String::from_utf8_lossy(&string_data).to_string())
}

fn get_ushort_or_ulong_from_entry(entry: IFDEntry) -> Result<u32, TiffError> {
    if entry.values.len() != 1 {
        return Err(TiffError(
            format!("Expected only one value (got {})", entry.values.len())
        ));
    }

    match entry.values[0] {
        IFDEntryValue::SHORT(v) => Ok(v as u32),
        IFDEntryValue::LONG(v) => Ok(v),
        _ => Err(TiffError(format!(
            "Expected value to be SHORT/LONG (got {:?})",
            entry.values[0],
        ))),
    }
}

impl TryFrom<IFDEntry> for TiffTag {
    type Error = TiffError;

    fn try_from(entry: IFDEntry) -> Result<TiffTag, TiffError> {
        match entry.tag {
            0 => match vec_to_array(entry.try_into()?) {
                Ok(arr) => Ok(TiffTag::GPSVersionID(arr)),
                Err(message) => Err(TiffError(message)),
            },
            1 => Ok(TiffTag::GPSLatitudeRef(entry.try_into()?)),
            2 => match vec_to_array(entry.try_into()?) {
                Ok(arr) => Ok(TiffTag::GPSLatitude(arr)),
                Err(message) => Err(TiffError(message)),
            },
            3 => Ok(TiffTag::GPSLongitudeRef(entry.try_into()?)),
            4 => match vec_to_array(entry.try_into()?) {
                Ok(arr) => Ok(TiffTag::GPSLongitude(arr)),
                Err(message) => Err(TiffError(message)),
            },
            5 => Ok(TiffTag::GPSAltitudeRef(match <IFDEntry as TryInto<u8>>::try_into(entry)? {
                0 => "Above sea level",
                1 => "Below sea level",
                _ => "Invalid",
            }.to_string())),
            6 => Ok(TiffTag::GPSAltitude(entry.try_into()?)),
            7 => match vec_to_array(entry.try_into()?) {
                Ok(arr) => Ok(TiffTag::GPSTimeStamp(arr)),
                Err(message) => Err(TiffError(message)),
            },
            8 => Ok(TiffTag::GPSSatellites(entry.try_into()?)),
            9 => Ok(TiffTag::GPSStatus(entry.try_into()?)),
            16 => Ok(TiffTag::GPSImgDirectionRef(entry.try_into()?)),
            17 => Ok(TiffTag::GPSImgDirection(entry.try_into()?)),
            18 => Ok(TiffTag::GPSMapDatum(entry.try_into()?)),
            29 => Ok(TiffTag::GPSDateStamp(entry.try_into()?)),
            259 => Ok(TiffTag::Compression(match <IFDEntry as TryInto<u16>>::try_into(entry)? {
                1 => "No compression",
                2 => "CCITT modified Huffman RLE",
                3 => "CCITT Group 3 fax encoding",
                4 => "CCITT Group 4 fax encoding",
                5 => "LZW",
                6 => "JPEG (old-style)",
                7 => "JPEG (new-style)",
                8 => "Deflate",
                32773 => "PackBits",
                _ => "Invalid/Unknown",
            }.to_string())),
            270 => Ok(TiffTag::ImageDescription(entry.try_into()?)),
            271 => Ok(TiffTag::Make(entry.try_into()?)),
            272 => Ok(TiffTag::Model(entry.try_into()?)),
            282 => Ok(TiffTag::XResolution(entry.try_into()?)),
            283 => Ok(TiffTag::YResolution(entry.try_into()?)),
            296 => Ok(TiffTag::ResolutionUnit(match <IFDEntry as TryInto<u16>>::try_into(entry)? {
                1 => "none",
                2 => "inch",
                3  => "centimeter",
                _ => "invalid",
            }.to_string())),
            274 => Ok(TiffTag::Orientation(entry.try_into()?)),
            305 => Ok(TiffTag::Software(entry.try_into()?)),
            306 => Ok(TiffTag::DateTime(entry.try_into()?)),
            315 => Ok(TiffTag::Artist(entry.try_into()?)),

            33432 => Ok(TiffTag::Copyright(entry.try_into()?)),
            33434 => Ok(TiffTag::ExposureTime(get_rational_repr_from_ifd_entry(entry)?)),
            33437 => Ok(TiffTag::FNumber(get_rational_repr_from_ifd_entry(entry)?)),
            34665 => Ok(TiffTag::ExifIfdPointer(entry.try_into()?)),
            34850 => Ok(TiffTag::ExposureProgram(match <IFDEntry as TryInto<u16>>::try_into(entry)? {
                0 => "Not defined",
                1 => "Manual",
                2 => "Normal program",
                3 => "Aperture priority",
                4 => "Shutter priority",
                5 => "Creative program (biased toward depth of field)",
                6 => "Action program (biased toward fast shutter speed)",
                7 => "Portrait mode (for closeup photos with the background out of focus)",
                8 => "Landscape mode (for landscape photos with the background in focus)",
                _ => "Invalid",
            }.to_string())),
            34853 => Ok(TiffTag::GpsIfdPointer(entry.try_into()?)),
            36864 => Ok(TiffTag::ExifVersion(get_string_from_entry_with_undefined_values(entry)?)),
            36867 => Ok(TiffTag::DateTimeOriginal(entry.try_into()?)),
            36868 => Ok(TiffTag::DateTimeDigitized(entry.try_into()?)),
            37122 => Ok(TiffTag::CompressedBitsPerPixel(get_rational_repr_from_ifd_entry(entry)?)),
            37377 => Ok(TiffTag::ShutterSpeedValue(get_rational_repr_from_ifd_entry(entry)?)),
            37378 => Ok(TiffTag::ApertureValue(get_rational_repr_from_ifd_entry(entry)?)),
            37380 => Ok(TiffTag::ExposureBiasValue(get_rational_repr_from_ifd_entry(entry)?)),
            37381 => Ok(TiffTag::MaxApertureValue(get_rational_repr_from_ifd_entry(entry)?)),
            37383 => Ok(TiffTag::MeteringMode(match <IFDEntry as TryInto<u16>>::try_into(entry)? {
                0 => "Unknown",
                1 => "Average",
                2 => "CenterWeightedAverage",
                3 => "Spot",
                4 => "MultiSpot",
                5 => "Pattern",
                6 => "Partial",
                255 => "Other",
                _ => "Invalid",
            }.to_string())),
            37384 => Ok(TiffTag::LightSource(match <IFDEntry as TryInto<u16>>::try_into(entry)? {
                0 => "Unknown",
                1 => "Daylight",
                2 => "Fluorescent",
                3 => "Tungsten (incandescent light)",
                4 => "Flash",
                9 => "Fine weather",
                10 => "Cloudy weather",
                11 => "Shade",
                12 => "Daylight fluorescent (D 5700 - 7100K)",
                13 => "Day white fluorescent (N 4600 - 5400K)",
                14 => "Cool white fluorescent (W 3900 - 4500K)",
                15 => "White fluorescent (WW 3200 - 3700K)",
                17 => "Standard light A",
                18 => "Standard light B",
                19 => "Standard light C",
                20 => "D55",
                21 => "D65",
                22 => "D75",
                23 => "D50",
                24 => "ISO studio tungsten",
                255 => "Other light source",
                _ => "Invalid",
            }.to_string())),
            37385 => Ok(TiffTag::Flash(match <IFDEntry as TryInto<u16>>::try_into(entry)? {
                0x0000 => "Flash did not fire",
                0x0001 => "Flash fired",
                0x0005 => "Strobe return light not detected",
                0x0007 => "Strobe return light detected",
                0x0009 => "Flash fired, compulsory flash mode",
                0x000D => "Flash fired, compulsory flash mode, return light not detected",
                0x000F => "Flash fired, compulsory flash mode, return light detected",
                0x0010 => "Flash did not fire, compulsory flash mode",
                0x0018 => "Flash did not fire, auto mode",
                0x0019 => "Flash fired, auto mode",
                0x001D => "Flash fired, auto mode, return light not detected",
                0x001F => "Flash fired, auto mode, return light detected",
                0x0020 => "No flash function",
                0x0041 => "Flash fired, red-eye reduction mode",
                0x0045 => "Flash fired, red-eye reduction mode, return light not detected",
                0x0047 => "Flash fired, red-eye reduction mode, return light detected",
                0x0049 => "Flash fired, compulsory flash mode, red-eye reduction mode",
                0x004D => "Flash fired, compulsory flash mode, red-eye reduction mode, return light not detected",
                0x004F => "Flash fired, compulsory flash mode, red-eye reduction mode, return light detected",
                0x0059 => "Flash fired, auto mode, red-eye reduction mode",
                0x005D => "Flash fired, auto mode, return light not detected, red-eye reduction mode",
                0x005F => "Flash fired, auto mode, return light detected, red-eye reduction mode",
                _ => "Invalid",
            }.to_string())),
            37386 => Ok(TiffTag::FocalLength(get_rational_repr_from_ifd_entry(entry)?)),
            37500 => Ok(TiffTag::MakerNote(entry.try_into()?)),
            37510 => {
                let data: Vec<u8> = entry.try_into()?;

                let encoding = match vec_to_array(data[0..8].to_vec()) {
                    Ok(arr) => Ok(match &arr {
                        b"ASCII\0\0\0" => "ascii",
                        b"JIS\0\0\0\0\0" => "jis",
                        b"UNICODE\0" => "unicode",
                        _ => "unknown",
                    }),
                    Err(message) => Err(TiffError(message)),
                }?;
                
                // TODO: Properly decode JIS/Unicode?
                let string_value = match encoding {
                    "ascii" | "unknown" | "jis" | "unicode" | &_ => String::from_utf8_lossy(&data[8..data.len()-1]),
                };

                Ok(TiffTag::UserComment(string_value.to_string()))
            },
            37520 => Ok(TiffTag::SubsecTime(entry.try_into()?)),
            37521 => Ok(TiffTag::SubsecTimeOriginal(entry.try_into()?)),
            37522 => Ok(TiffTag::SubsecTimeDigitized(entry.try_into()?)),

            40960 => Ok(TiffTag::FlashpixVersion(get_string_from_entry_with_undefined_values(entry)?)),
            40962 => Ok(TiffTag::PixelXDimension(get_ushort_or_ulong_from_entry(entry)?)),
            40963 => Ok(TiffTag::PixelYDimension(get_ushort_or_ulong_from_entry(entry)?)),
            41486 => Ok(TiffTag::FocalPlaneXResolution(get_rational_repr_from_ifd_entry(entry)?)),
            41487 => Ok(TiffTag::FocalPlaneYResolution(get_rational_repr_from_ifd_entry(entry)?)),
            41488 => Ok(TiffTag::FocalPlaneResolutionUnit(match <IFDEntry as TryInto<u16>>::try_into(entry)? {
                1 => "none",
                2 => "inch",
                3 => "centimeter",
                _ => "invalid",
            }.to_string())),
            41495 => Ok(TiffTag::SensingMethod(match <IFDEntry as TryInto<u16>>::try_into(entry)? {
                1 => "Not defined",
                2 => "One-chip color area sensor",
                3 => "Two-chip color area sensor",
                4 => "Three-chip color area sensor",
                5 => "Color sequential area sensor",
                7 => "Trilinear sensor",
                8 => "Color sequential linear sensor",
                _ => "Invalid",
            }.to_string())),
            41986 => Ok(TiffTag::ExposureMode(match <IFDEntry as TryInto<u16>>::try_into(entry)? {
                0 => "Auto exposure",
                1 => "Manual exposure",
                2 => "Auto bracket",
                _ => "Invalid",
            }.to_string())),
            41987 => Ok(TiffTag::WhiteBalance(match <IFDEntry as TryInto<u16>>::try_into(entry)? {
                0 => "Auto white balance",
                1 => "Manual white balance",
                _ => "Invalid",
            }.to_string())),
            41988 => Ok(TiffTag::DigitalZoomRatio(get_rational_repr_from_ifd_entry(entry)?)),
            41989 => Ok(TiffTag::FocalLengthIn35mmFilm(entry.try_into()?)),
            41990 => Ok(TiffTag::SceneCaptureType(match <IFDEntry as TryInto<u16>>::try_into(entry)? {
                0 => "Standard",
                1 => "Landscape",
                2 => "Portrait",
                3 => "Night scene",
                _ => "Invalid",
            }.to_string())),
            41991 => Ok(TiffTag::GainControl(match <IFDEntry as TryInto<u16>>::try_into(entry)? {
                0 => "None",
                1 => "Low gain up",
                2 => "High gain up",
                3 => "Low gain down",
                4 => "High gain down",
                _ => "Invalid",
            }.to_string())),
            41992 => Ok(TiffTag::Contrast(match <IFDEntry as TryInto<u16>>::try_into(entry)? {
                0 => "Normal",
                1 => "Soft",
                2 => "Hard",
                _ => "Invalid",
            }.to_string())),
            41993 => Ok(TiffTag::Saturation(match <IFDEntry as TryInto<u16>>::try_into(entry)? {
                0 => "Normal",
                1 => "Low saturation",
                2 => "High saturation",
                _ => "Invalid",
            }.to_string())),
            41994 => Ok(TiffTag::Sharpness(match <IFDEntry as TryInto<u16>>::try_into(entry)? {
                0 => "Normal",
                1 => "Soft",
                2 => "Hard",
                _ => "Invalid",
            }.to_string())),
            41996 => Ok(TiffTag::SubjectDistanceRange(match <IFDEntry as TryInto<u16>>::try_into(entry)? {
                0 => "Unknown",
                1 => "Macro",
                2 => "Close view",
                3 => "Distant view",
                _ => "Invalid",
            }.to_string())),

            _ => Ok(TiffTag::Unknown(entry)),
        }
    }
}

fn read_ifd_entry_values(
    value_type: u16,
    value_type_size: usize,
    value_count: usize,
    endianness: &Endianness,
    cursor: &mut Cursor<Vec<u8>>,
) -> Result<Vec<IFDEntryValue>, TiffError> {
    let mut values: Vec<IFDEntryValue> = vec![];

    while values.len() < value_count {
        let mut buf = vec![0_u8; value_type_size];
        cursor.read_exact(&mut buf).unwrap();

        values.push(match value_type {
            1 => IFDEntryValue::BYTE(buf[0]),
            2 => IFDEntryValue::ASCII(buf[0]),
            3 => {
                let data = [buf[0], buf[1]];
                IFDEntryValue::SHORT(match endianness {
                    Endianness::Big => u16::from_be_bytes(data),
                    Endianness::Little => u16::from_le_bytes(data),
                })
            }
            4 => {
                let data = [buf[0], buf[1], buf[2], buf[3]];
                IFDEntryValue::LONG(match endianness {
                    Endianness::Big => u32::from_be_bytes(data),
                    Endianness::Little => u32::from_le_bytes(data),
                })
            }
            5 => {
                let data = (
                    [buf[0], buf[1], buf[2], buf[3]],
                    [buf[4], buf[5], buf[6], buf[7]],
                );
                let (a, b) = match endianness {
                    Endianness::Big => (u32::from_be_bytes(data.0), u32::from_be_bytes(data.1)),
                    Endianness::Little => (u32::from_le_bytes(data.0), u32::from_le_bytes(data.1)),
                };

                IFDEntryValue::RATIONAL(a, b)
            }
            6 => IFDEntryValue::SBYTE(buf[0] as i8),
            7 => IFDEntryValue::UNDEFINED(buf[0]),
            8 => {
                let data = [buf[0], buf[1]];
                IFDEntryValue::SSHORT(match endianness {
                    Endianness::Big => i16::from_be_bytes(data),
                    Endianness::Little => i16::from_le_bytes(data),
                })
            }
            9 => {
                let data = [buf[0], buf[1], buf[2], buf[3]];
                IFDEntryValue::SLONG(match endianness {
                    Endianness::Big => i32::from_be_bytes(data),
                    Endianness::Little => i32::from_be_bytes(data),
                })
            }
            10 => {
                let data = (
                    [buf[0], buf[1], buf[2], buf[3]],
                    [buf[4], buf[5], buf[6], buf[7]],
                );
                let (a, b) = match endianness {
                    Endianness::Big => (i32::from_be_bytes(data.0), i32::from_be_bytes(data.1)),
                    Endianness::Little => (i32::from_le_bytes(data.0), i32::from_le_bytes(data.1)),
                };

                IFDEntryValue::SRATIONAL(a, b)
            }
            11 => {
                let data = [buf[0], buf[1], buf[2], buf[3]];
                IFDEntryValue::FLOAT(match endianness {
                    Endianness::Big => f32::from_be_bytes(data),
                    Endianness::Little => f32::from_le_bytes(data),
                })
            }
            12 => {
                let data = [
                    buf[0], buf[1], buf[2], buf[3], buf[4], buf[5], buf[6], buf[7],
                ];
                IFDEntryValue::DOUBLE(match endianness {
                    Endianness::Big => f64::from_be_bytes(data),
                    Endianness::Little => f64::from_le_bytes(data),
                })
            }
            unknown => return Err(TiffError(format!("Encountered unknown TIFF value type: {}", unknown))),
        })
    }

    Ok(values)
}

pub fn read_ifd_entry(cursor: &mut Cursor<Vec<u8>>, endianness: &Endianness) -> Result<IFDEntry, TiffError> {
    let tag = {
        let mut buf = [0_u8; 2];
        cursor.read_exact(&mut buf).unwrap();
        match endianness {
            Endianness::Big => u16::from_be_bytes(buf),
            Endianness::Little => u16::from_le_bytes(buf),
        }
    };

    let value_type = {
        let mut buf = [0_u8; 2];
        cursor.read_exact(&mut buf).unwrap();
        match endianness {
            Endianness::Big => u16::from_be_bytes(buf),
            Endianness::Little => u16::from_le_bytes(buf),
        }
    };

    let value_count = {
        let mut buf = [0_u8; 4];
        cursor.read_exact(&mut buf).unwrap();
        match endianness {
            Endianness::Big => u32::from_be_bytes(buf),
            Endianness::Little => u32::from_le_bytes(buf),
        }
    } as usize;

    let value_type_size = get_tiff_value_type_size(value_type)?;
    let size_of_all_values = value_count * value_type_size;

    let original_position = cursor.position();

    // If the size of all values is >4 then we need to seek to that position
    if size_of_all_values > 4 {
        let value_offset = {
            let mut buf = [0_u8; 4];
            cursor.read_exact(&mut buf).unwrap();
            match endianness {
                Endianness::Big => u32::from_be_bytes(buf),
                Endianness::Little => u32::from_le_bytes(buf),
            }
        };
        cursor.set_position(value_offset as u64)
    }

    let values =
        read_ifd_entry_values(value_type, value_type_size, value_count, endianness, cursor)?;

    cursor.set_position(original_position + 4);

    Ok(IFDEntry { tag, values })
}

pub fn read_ifd(cursor: &mut Cursor<Vec<u8>>, endianness: &Endianness) -> Result<Vec<IFDEntry>, TiffError> {
    let ifd_entry_count = {
        let mut buf = [0_u8; 2];
        cursor.read_exact(&mut buf).unwrap();
        match endianness {
            Endianness::Big => u16::from_be_bytes(buf),
            Endianness::Little => u16::from_le_bytes(buf),
        }
    };

    let mut entries: Vec<IFDEntry> = vec![];
    for _ in 0..ifd_entry_count {
        entries.push(read_ifd_entry(cursor, endianness)?);
    }

    Ok(entries)
}

#[derive(Debug)]
pub struct Tiff {
    pub tags: Vec<TiffTag>,
    pub endianness: Endianness,
}

#[derive(Debug)]
pub struct TiffError(pub String);

fn ifd_entries_to_tiff_tags(entries: Vec<IFDEntry>) -> Result<Vec<TiffTag>, TiffError> {
    let mut tags: Vec<TiffTag> = vec![];

    for entry in entries {
        tags.push(TiffTag::try_from(entry)?);
    }

    Ok(tags)
}

pub fn read_tiff(cursor: &mut Cursor<Vec<u8>>) -> Result<Tiff, TiffError> {
    let mut entries: Vec<IFDEntry> = Vec::new();

    let endianness = {
        let mut data = [0_u8; 2];
        cursor.read_exact(&mut data).unwrap();
        match data {
            [0x4D, 0x4D] => Endianness::Big,
            [0x49, 0x49] => Endianness::Little,
            unknown => return Err(TiffError(format!(
                "Expected MM or II but got {:?} instead",
                String::from_utf8_lossy(&unknown),
            ))),
        }
    };

    let magic_number = {
        let mut data = [0_u8; 2];
        cursor.read_exact(&mut data).unwrap();
        match endianness {
            Endianness::Big => u16::from_be_bytes(data),
            Endianness::Little => u16::from_le_bytes(data),
        }
    };
    if magic_number != 42 {
        return Err(TiffError(format!(
            "Expected magic number to be 42, but got {} instead",
            magic_number,
        )));
    }

    loop {
        let offset = {
            let mut buf = [0_u8; 4];
            cursor.read_exact(&mut buf).unwrap();
            match endianness {
                Endianness::Big => u32::from_be_bytes(buf),
                Endianness::Little => u32::from_le_bytes(buf),
            }
        };

        // Offset of zero means no more IFDs
        if offset == 0 {
            break;
        }

        cursor.set_position(offset as u64);

        entries.extend(read_ifd(cursor, &endianness)?);
    }

    let mut tags: Vec<TiffTag> = ifd_entries_to_tiff_tags(entries)?;

    let mut extra_found_entries: Vec<IFDEntry> = vec![];
    for tag in &tags {
        match tag {
            TiffTag::ExifIfdPointer(ifd_ptr) => {
                cursor.set_position(*ifd_ptr as u64);
                extra_found_entries.extend(read_ifd(cursor, &endianness)?);
            }
            TiffTag::GpsIfdPointer(ifd_ptr) => {
                cursor.set_position(*ifd_ptr as u64);
                extra_found_entries.extend(read_ifd(cursor, &endianness)?);
            }
            _ => {}
        }
    }
    tags.extend(ifd_entries_to_tiff_tags(extra_found_entries)?);

    Ok(Tiff { tags, endianness })
}
