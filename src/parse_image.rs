use crate::chunks::ihdr;
use crate::chunks::plte;
use crate::helpers::*;

pub fn parse(mut image_data: Vec<u8>, ihdr_chunk: &ihdr::IHDRChunk) -> Image<RGBColor> {
    let mut image: Image<RGBColor> = Vec::new();

    // Make sure px_size isnt zero from truncation
    let px_size = ihdr_chunk.pixel_size().max(1) as usize;
    let bit_depth = ihdr_chunk.bit_depth();

    let scanline_length = ihdr_chunk.width() * px_size as u32 + 1;

    for i in 0..ihdr_chunk.height() {
        let s = (i * scanline_length) as usize;
        let filter_method = image_data[s];
        let s = s + 1;
        let e = s + (ihdr_chunk.width() as usize * px_size);
        match filter_method {
            0 => {}
            1 => {
                for x in s..e {
                    image_data[x] = image_data[x].wrapping_add(if x - s >= px_size {
                        image_data[x - px_size]
                    } else {
                        0
                    });
                }
            }
            2 => {
                for x in s..e {
                    image_data[x] = image_data[x].wrapping_add(if i > 0 {
                        image_data[x - scanline_length as usize]
                    } else {
                        0
                    });
                }
            }
            3 => {
                for x in s..e {
                    let top = if i > 0 {
                        image_data[x - scanline_length as usize]
                    } else {
                        0
                    } as u16;
                    let left = if x - s >= px_size {
                        image_data[x - px_size]
                    } else {
                        0
                    } as u16;

                    image_data[x] = image_data[x].wrapping_add(((top + left) / 2) as u8);
                }
            }
            4 => {
                for x in s..e {
                    let top = if i > 0 {
                        image_data[x - scanline_length as usize]
                    } else {
                        0
                    } as i32;

                    let left = if x - s >= px_size {
                        image_data[x - px_size]
                    } else {
                        0
                    } as i32;

                    let topleft = if x >= px_size && i > 0 {
                        image_data[x - px_size - scanline_length as usize]
                    } else {
                        0
                    } as i32;

                    image_data[x] =
                        image_data[x].wrapping_add(paeth_predictor(left, top, topleft, x));
                }
            }
            _ => panic!("Unrecognised filter method"),
        };
        let image_data = &image_data[s..e];
        let mut scanline = Vec::new();

        assert_eq!(image_data.len() % px_size, 0);

        let mut i = 0;
        while i < image_data.len() {
            match ihdr_chunk.color_type() {
                ihdr::ColorType::Palette => palette(
                    &image_data[i..i + px_size],
                    bit_depth,
                    &ihdr_chunk.palette(),
                    &mut scanline,
                ),
                ihdr::ColorType::RGBA => {
                    rgba(&image_data[i..i + px_size], bit_depth, &mut scanline)
                }
                ihdr::ColorType::RGB => rgb(&image_data[i..i + px_size], bit_depth, &mut scanline),
                ihdr::ColorType::Gray => {
                    gray(&image_data[i..i + px_size], bit_depth, &mut scanline)
                }
                ihdr::ColorType::GrayA => {
                    gray_a(&image_data[i..i + px_size], bit_depth, &mut scanline)
                }
            };
            i += px_size;
        }

        image.push(scanline);
    }

    image
}

fn paeth_predictor(a: i32, b: i32, c: i32, i: usize) -> u8 {
    let p = a + b - c; // initial estimate
    let pa = (p - a).abs(); // distances to a, b, c
    let pb = (p - b).abs();
    let pc = (p - c).abs();
    // return nearest of a,b,c,
    // breaking ties in order a,b,c.
    if pa <= pb && pa <= pc {
        a as u8
    } else if pb <= pc {
        b as u8
    } else {
        c as u8
    }
}

fn palette(
    image_data: &[u8],
    bit_depth: u8,
    pt: &Option<plte::PLTEChunk>,
    scanline: &mut Vec<RGBColor>,
) {
    let pt = match pt {
        Some(pt) => pt,
        None => panic!("Palette not found"),
    };
    scanline.push(pt.colors[image_data[0] as usize]);
}

fn rgba(image_data: &[u8], bit_depth: u8, scanline: &mut Vec<RGBColor>) {
    scanline.push(if image_data[3] >= 128 {
        (image_data[0], image_data[1], image_data[2])
    } else {
        (0, 0, 0)
    });
}

fn rgb(image_data: &[u8], bit_depth: u8, scanline: &mut Vec<RGBColor>) {
    scanline.push((image_data[0], image_data[1], image_data[2]));
}

fn gray(image_data: &[u8], bit_depth: u8, scanline: &mut Vec<RGBColor>) {
    scanline.push((image_data[0], image_data[0], image_data[0]));
}

fn gray_a(image_data: &[u8], bit_depth: u8, scanline: &mut Vec<RGBColor>) {
    scanline.push(if image_data[1] >= 128 {
        (image_data[0], image_data[0], image_data[0])
    } else {
        (0, 0, 0)
    });
}
