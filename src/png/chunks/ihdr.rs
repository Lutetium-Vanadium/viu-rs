use crate::common::*;
use std::io::{Error, ErrorKind, Result};

#[derive(Debug, PartialEq, Eq)]
pub enum InterlaceMethod {
    NoInterlace,
    Adam7,
}

pub struct IHDRChunk {
    width: u32,
    height: u32,
    bit_depth: u8,
    color_type: u8,
    compression_method: u8,
    filter_method: u8,
    interlace_method: u8,
}

impl std::default::Default for IHDRChunk {
    fn default() -> Self {
        IHDRChunk {
            width: 0,
            height: 0,
            bit_depth: 0,
            color_type: 0,
            compression_method: 0,
            filter_method: 0,
            interlace_method: 0,
        }
    }
}

impl IHDRChunk {
    pub fn parse(bytes: &[u8]) -> Result<IHDRChunk> {
        let ihdr = IHDRChunk {
            width: from_bytes_u32(&bytes[0..4]),
            height: from_bytes_u32(&bytes[4..8]),
            bit_depth: bytes[8],
            color_type: bytes[9],
            compression_method: bytes[10],
            filter_method: bytes[11],
            interlace_method: bytes[12],
        };

        // Check has a valid bit depth
        if ![0, 2, 3, 4, 6].contains(&ihdr.color_type) {
            return Err(Error::new(ErrorKind::InvalidData, "Malformed Color Type"));
        }

        match ihdr.color_type() {
            ColorType::Gray => {
                if ![1, 2, 4, 8, 16].contains(&ihdr.bit_depth) {
                    return Err(Error::new(ErrorKind::InvalidData, "Malformed Bit Depth"));
                }
            }
            ColorType::RGB => {
                if ![8, 16].contains(&ihdr.bit_depth) {
                    return Err(Error::new(ErrorKind::InvalidData, "Malformed Bit Depth"));
                }
            }
            ColorType::Palette => {
                if ![1, 2, 4, 8].contains(&ihdr.bit_depth) {
                    return Err(Error::new(ErrorKind::InvalidData, "Malformed Bit Depth"));
                }
            }
            ColorType::GrayA => {
                if ![8, 16].contains(&ihdr.bit_depth) {
                    return Err(Error::new(ErrorKind::InvalidData, "Malformed Bit Depth"));
                }
            }
            ColorType::RGBA => {
                if ![8, 16].contains(&ihdr.bit_depth) {
                    return Err(Error::new(ErrorKind::InvalidData, "Malformed Bit Depth"));
                }
            }
        };

        // At present, only compression method 0 (deflate/inflate compression with a
        // sliding window of at most 32768 bytes) is defined.
        if ihdr.compression_method != 0 {
            return Err(Error::new(
                ErrorKind::Other,
                format!("Unknown compression_method: {}", ihdr.compression_method),
            ));
        }

        // At present, only filter method 0 (adaptive filtering with five basic filter types) is defined
        if ihdr.filter_method != 0 {
            return Err(Error::new(
                ErrorKind::Other,
                format!("Unknown compression_method: {}", ihdr.compression_method),
            ));
        }

        // Two values are currently defined: 0 (no interlace) or 1 (Adam7 interlace)
        if ihdr.interlace_method >= 2 {
            return Err(Error::new(
                ErrorKind::Other,
                format!("Unknown compression_method: {}", ihdr.compression_method),
            ));
        }

        if ihdr.interlace_method() == InterlaceMethod::Adam7 {
            return Err(Error::new(
                ErrorKind::Other,
                "Interlacing is currently unsupported",
            ));
        }

        Ok(ihdr)
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    // panic used here as it provides a cleaner interface, and checks for valid color_type
    // and interlace_method are already performed in IHDRChunk::new
    pub fn color_type(&self) -> ColorType {
        match self.color_type {
            0 => ColorType::Gray,
            2 => ColorType::RGB,
            3 => ColorType::Palette,
            4 => ColorType::GrayA,
            6 => ColorType::RGBA,
            _ => panic!("Unknown color type: {}", self.color_type),
        }
    }

    pub fn bit_depth(&self) -> u8 {
        self.bit_depth
    }

    pub fn pixel_size(&self) -> u8 {
        ((self.bit_depth / 8) as f32
            * (match self.color_type() {
                ColorType::Gray | ColorType::Palette => 1f32,
                ColorType::GrayA => 2f32,
                ColorType::RGB => 3f32,
                ColorType::RGBA => 4f32,
            }))
        .ceil() as u8
    }

    pub fn interlace_method(&self) -> InterlaceMethod {
        match self.interlace_method {
            0 => InterlaceMethod::NoInterlace,
            1 => InterlaceMethod::Adam7,
            _ => panic!("Unknown interlace methods: {}", self.interlace_method),
        }
    }
}
