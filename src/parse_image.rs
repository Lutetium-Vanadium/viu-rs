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

                    let topleft = if x - s >= px_size && i > 0 {
                        image_data[x - px_size - scanline_length as usize]
                    } else {
                        0
                    } as i32;

                    image_data[x] = image_data[x].wrapping_add(paeth_predictor(left, top, topleft));
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

fn paeth_predictor(a: i32, b: i32, c: i32) -> u8 {
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

    // for all color.2 = 1: Ansi displays completely transparent if colour is set to (0, 0, 0) [at least for my terminal]
    //      with rgb colour codes. This make sure that opaque black pizels will be put as black instead
    //      of transparent
    //      Blue is increased since its least receptive for the human eye
    match bit_depth {
        1 => {
            for i in 0..8 {
                let mut color = pt.colors[(image_data[0] >> (7 - i) & 0b1) as usize];
                if color.0 == 0 && color.1 == 0 && color.2 == 0 {
                    color.2 = 1;
                }
                scanline.push(color);
            }
        }
        2 => {
            for i in 0..4 {
                let mut color = pt.colors[(image_data[0] >> (6 - i * 2) & 0b11) as usize];
                if color.0 == 0 && color.1 == 0 && color.2 == 0 {
                    color.2 = 1;
                }
                scanline.push(color);
            }
        }
        4 => {
            let mut color = pt.colors[(image_data[0] >> 4 & 0b1111) as usize];
            if color.0 == 0 && color.1 == 0 && color.2 == 0 {
                color.2 = 1;
            }
            scanline.push(color);
            color = pt.colors[(image_data[0] & 0b1111) as usize];
            if color.0 == 0 && color.1 == 0 && color.2 == 0 {
                color.2 = 1;
            }
            scanline.push(color);
        }
        8 => {
            let mut color = pt.colors[image_data[0] as usize];
            if color.0 == 0 && color.1 == 0 && color.2 == 0 {
                color.2 = 1;
            }
            scanline.push(color);
        }
        _ => panic!("invalid bit depth"),
    }
}

fn rgba(image_data: &[u8], bit_depth: u8, scanline: &mut Vec<RGBColor>) {
    let (r, g, mut b, a) = match bit_depth {
        8 => (image_data[0], image_data[1], image_data[2], image_data[3]),
        16 => (
            (from_bytes_u16(&image_data[..2]) / 256) as u8,
            (from_bytes_u16(&image_data[2..4]) / 256) as u8,
            (from_bytes_u16(&image_data[4..6]) / 256) as u8,
            (from_bytes_u16(&image_data[6..8]) / 256) as u8,
        ),
        _ => panic!("invalid bit depth"),
    };

    // Ansi displays completely transparent if colour is set to (0, 0, 0) [at least for my terminal]
    // with rgb colour codes. This make sure that opaque black pizels will be put as black instead
    // of transparent, but if opacity is 0, a transparent pixel will be shown
    // Blue is increased since its least receptive for the human eye
    if r == 0 && g == 0 && b == 0 {
        b = 1;
    }

    let opacity = (a as f32) / 256.0;

    scanline.push((
        (r as f32 * opacity) as u8,
        (g as f32 * opacity) as u8,
        (b as f32 * opacity) as u8,
    ));
}

fn rgb(image_data: &[u8], bit_depth: u8, scanline: &mut Vec<RGBColor>) {
    let (r, g, mut b) = match bit_depth {
        8 => (image_data[0], image_data[1], image_data[2]),
        16 => (
            (from_bytes_u16(&image_data[..2]) / 256) as u8,
            (from_bytes_u16(&image_data[2..4]) / 256) as u8,
            (from_bytes_u16(&image_data[4..6]) / 256) as u8,
        ),
        _ => panic!("invalid bit depth"),
    };

    // Ansi displays completely transparent if colour is set to (0, 0, 0) [at least for my terminal]
    // with rgb colour codes. This make sure that opaque black pizels will be put as black instead
    // of transparent
    // Blue is increased since its least receptive for the human eye
    if r == 0 && g == 0 && b == 0 {
        b = 1;
    }
    scanline.push((r, g, b));
}

fn gray(image_data: &[u8], bit_depth: u8, scanline: &mut Vec<RGBColor>) {
    // for all val = 1: Ansi displays completely transparent if colour is set to (0, 0, 0) [at least for my terminal]
    //      with rgb colour codes. This make sure that opaque black pizels will be put as black instead
    //      of transparent
    match bit_depth {
        1 => {
            for i in 0..8 {
                let mut val = (image_data[0] >> (7 - i) & 0b1) * 255;
                if val == 0 {
                    val = 1;
                }
                scanline.push((val, val, val));
            }
        }
        2 => {
            for i in 0..4 {
                let mut val = (image_data[0] >> (6 - i * 2) & 0b11) * 85;
                if val == 0 {
                    val = 1;
                }
                scanline.push((val, val, val));
            }
        }
        4 => {
            let mut val = (image_data[0] >> 4 & 0b1111) * 17;
            if val == 0 {
                val = 1;
            }
            scanline.push((val, val, val));
            val = (image_data[0] & 0b1111) * 17;
            if val == 0 {
                val = 1;
            }
            scanline.push((val, val, val));
        }
        8 => {
            let mut val = image_data[0];
            if val == 0 {
                val = 1;
            }
            scanline.push((val, val, val))
        }
        16 => {
            let mut val = (from_bytes_u16(image_data) / 256) as u8;
            if val == 0 {
                val = 1;
            }
            scanline.push((val, val, val));
        }
        _ => panic!("Invalid bit depth"),
    }
}

fn gray_a(image_data: &[u8], bit_depth: u8, scanline: &mut Vec<RGBColor>) {
    let (mut val, alpha) = match bit_depth {
        8 => (image_data[0], image_data[1]),
        16 => (
            (from_bytes_u16(&image_data[2..4]) / 256) as u8,
            (from_bytes_u16(&image_data[..2]) / 256) as u8,
        ),
        _ => panic!("Invalid bit depth"),
    };

    // Ansi displays completely transparent if colour is set to (0, 0, 0) [at least for my terminal]
    // with rgb colour codes. This make sure that opaque black pizels will be put as black instead
    // of transparent, but if opacity is 0, a transparent pixel will be shown
    if val == 0 {
        val = 1;
    }

    let val = ((val as f32) * (alpha as f32) / 256.0) as u8;

    scanline.push((val, val, val));
}
