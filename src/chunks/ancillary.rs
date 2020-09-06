use super::ihdr;
use crate::common::*;
use std::fmt;

// IMPORTANT NOTE as per the png 1.2 spec [http://www.libpng.org/pub/png/spec/1.2/png-1.2-pdg.html#C.Anc-chunks]:
//      Note: when dealing with 16-bit grayscale or truecolor data, it is important to compare both bytes of the
//      sample values to determine whether a pixel is transparent. Although decoders may drop the low-order byte
//      of the samples for display, this must not occur until after the data has been tested for transparency.
//      For example, if the grayscale level 0x0001 is specified to be transparent, it would be incorrect to
//      compare only the high-order byte and decide that 0x0002 is also transparent.
//
// This is being ignored for simplicity, and all values are being scaled to a u8. This means for 16 bit colour depth,
// multiple values expected to be distinct will be all treated as transparent.
#[derive(Debug)]
pub enum TRNSChunk {
    RGB(u8, u8, u8),
    Gray(u8),
    Palette(Vec<u8>),
}

impl TRNSChunk {
    pub fn parse(bytes: &[u8], metadata: &Metadata) -> TRNSChunk {
        match metadata.ihdr_chunk.color_type() {
            ihdr::ColorType::Gray => {
                let val = from_bytes_u16(bytes);
                let bit_depth = metadata.ihdr_chunk.bit_depth();
                TRNSChunk::Gray(if bit_depth == 16 {
                    (val / 256) as u8
                } else {
                    (val as u8) * 8 / bit_depth
                })
            }
            ihdr::ColorType::RGB => {
                let r = from_bytes_u16(bytes);
                let g = from_bytes_u16(bytes);
                let b = from_bytes_u16(bytes);
                let bit_depth = metadata.ihdr_chunk.bit_depth();
                let (r, g, b) = if bit_depth == 16 {
                    ((r / 256) as u8, (g / 256) as u8, (b / 256) as u8)
                } else {
                    (
                        (r as u8) * 8 / bit_depth,
                        (g as u8) * 8 / bit_depth,
                        (b as u8) * 8 / bit_depth,
                    )
                };
                TRNSChunk::RGB(r, g, b)
            }
            ihdr::ColorType::Palette => {
                let len = match metadata.palette() {
                    Some(pt) => pt.colors.len(),
                    None => panic!("PLTE chunk must be present before tRNS"),
                };

                let mut alpha = Vec::with_capacity(len);

                for byte in bytes {
                    alpha.push(*byte);
                }

                for _ in bytes.len()..len {
                    alpha.push(255);
                }

                TRNSChunk::Palette(alpha)
            }
            // RGBA and GrayA already have alpha channels and tRNS chunks are unsupported for them
            color_type => panic!("tRNS chunk not allowed for color type: {:?}", color_type),
        }
    }
}

pub struct TIMEChunk {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

impl TIMEChunk {
    pub fn parse(bytes: &[u8]) -> TIMEChunk {
        TIMEChunk {
            year: from_bytes_u16(&bytes[0..2]),
            month: bytes[2],
            day: bytes[3],
            hour: bytes[4],
            minute: bytes[5],
            second: bytes[6],
        }
    }
}

impl fmt::Display for TIMEChunk {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}:{}:{}  {}/{}/{}",
            self.hour, self.minute, self.second, self.day, self.month, self.year
        )
    }
}

use std::str;

pub struct TextChunk {
    pub key: String,
    pub text: String,
}

type SplitChunk<'a> = (&'a [u8], &'a [u8]);

impl TextChunk {
    pub fn split(bytes: &[u8]) -> SplitChunk {
        let mut i = 0;
        while bytes[i] != 0 {
            i += 1;
        }
        i += 1;
        (&bytes[0..i], &bytes[i + 1..])
    }

    pub fn parse((keyword_bytes, text_bytes): SplitChunk) -> Result<TextChunk, str::Utf8Error> {
        let key = str::from_utf8(keyword_bytes)?.to_owned();
        let text = str::from_utf8(text_bytes)?.to_owned();

        Ok(TextChunk { key, text })
    }
}

pub fn parse_bkgd_chunk(bytes: &[u8], metadata: &Metadata) -> RGBColor {
    let (r, g, b) = match metadata.ihdr_chunk.color_type() {
        ihdr::ColorType::Palette => match metadata.palette() {
            Some(pt) => pt.colors[bytes[0] as usize],
            None => panic!("Palette not found"),
        },
        ihdr::ColorType::Gray | ihdr::ColorType::GrayA => {
            let val = from_bytes_u16(bytes);
            let bit_depth = metadata.ihdr_chunk.bit_depth();
            let val = if bit_depth == 16 {
                (val / 256) as u8
            } else {
                (val as u8) * 8 / bit_depth
            };
            (val, val, val)
        }
        ihdr::ColorType::RGBA | ihdr::ColorType::RGB => {
            let r = from_bytes_u16(bytes);
            let g = from_bytes_u16(bytes);
            let b = from_bytes_u16(bytes);
            let bit_depth = metadata.ihdr_chunk.bit_depth();
            let (r, g, b) = if bit_depth == 16 {
                ((r / 256) as u8, (g / 256) as u8, (b / 256) as u8)
            } else {
                (
                    (r as u8) * 8 / bit_depth,
                    (g as u8) * 8 / bit_depth,
                    (b as u8) * 8 / bit_depth,
                )
            };
            (r, g, b)
        }
    };
    if is_transparent(r, g, b) {
        (0, 0, 1)
    } else {
        (r, g, b)
    }
}
