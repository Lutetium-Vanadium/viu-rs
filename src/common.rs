use crate::chunks::{ancillary, ihdr, plte};
use std::io;

pub type RGBColor = (u8, u8, u8);
pub type Image<T> = Vec<Vec<T>>;

pub const HD: bool = true;

pub enum Effect {
    NoEffect,
    Blur,
    ASCII,
}

pub struct Metadata {
    pub ihdr_chunk: ihdr::IHDRChunk,
    palette: Option<plte::PLTEChunk>,
    alpha: Option<ancillary::TRNSChunk>,
    bkgd: RGBColor,
}

impl Metadata {
    pub fn new() -> Metadata {
        Metadata {
            alpha: None,
            ihdr_chunk: Default::default(),
            palette: None,
            bkgd: (0, 0, 0), // Default background is transparent
        }
    }

    pub fn palette(&self) -> &Option<plte::PLTEChunk> {
        &self.palette
    }

    pub fn set_palette(&mut self, palette: plte::PLTEChunk) {
        self.palette = Some(palette);
    }

    pub fn alpha(&self) -> &Option<ancillary::TRNSChunk> {
        &self.alpha
    }

    pub fn set_alpha(&mut self, alpha: ancillary::TRNSChunk) {
        self.alpha = Some(alpha);
    }

    pub fn bkgd(&self) -> &RGBColor {
        &self.bkgd
    }

    pub fn set_bkgd(&mut self, bkgd: RGBColor) {
        self.bkgd = bkgd;
    }
}

pub fn is_transparent(r: u8, g: u8, b: u8) -> bool {
    r == 0 && g == 0 && b == 0
}

pub fn from_bytes_u32(bytes: &[u8]) -> u32 {
    ((bytes[0] as u32) << 24)
        + ((bytes[1] as u32) << 16)
        + ((bytes[2] as u32) << 8)
        + (bytes[3] as u32)
}

pub fn from_bytes_u16(bytes: &[u8]) -> u16 {
    ((bytes[0] as u16) << 8) + (bytes[1] as u16)
}

pub fn auto_downsize_image(image: Image<RGBColor>) -> io::Result<Image<RGBColor>> {
    // Terminal dimensions
    let (tw, th) = if let Some((w, h)) = term_size::dimensions() {
        if HD {
            // Use fg + bg to make one character 2 pixels
            (w, h * 2)
        } else {
            // 2 characters for one pixel, otherwise it looks squished
            (w / 2, h)
        }
    } else {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Failed to get Terminal size",
        ));
    };

    // Raw image dimensions
    let iw = image[0].len();
    let ih = image.len();

    println!("t: {}x{}, i: {}x{}", tw, th, iw, ih);

    // The required image dimensions
    let (w, h, r) = if tw > iw && th > ih {
        (iw, ih, 1.0)
    } else if tw / th > iw / ih {
        let r = ih as f32 / th as f32;

        ((iw as f32 / r) as usize, th, r)
    } else {
        let r = iw as f32 / tw as f32;

        (tw, (ih as f32 / r) as usize, r)
    };

    println!("Print image height: {}x{} ratio: {}", w, h, r);

    let rstep = r * 0.98;
    let ir2 = 1.0 / (r * r);

    let mut downsized_image: Image<RGBColor> = Vec::new();

    for y in 0..h {
        let mut scanline = Vec::new();

        let mut sr = 0f32;
        let mut sg = 0f32;
        let mut sb = 0f32;

        for x in 0..w {
            let sx1 = r * x as f32;
            let sy1 = r * y as f32;
            let sx2 = sx1 + rstep;
            let sy2 = sy1 + rstep;

            let istart = sx1 as usize;
            let iend = sx2.ceil() as usize;
            let jstart = sy1 as usize;
            let jend = sy2.ceil() as usize;

            for i in istart..iend {
                for j in jstart..jend {
                    let dx = if i == istart {
                        (i + 1) as f32 - sx1
                    } else if i == iend - 1 {
                        sx2 - (i as f32)
                    } else {
                        1f32
                    };

                    let dy = if j == jstart {
                        (j + 1) as f32 - sy1
                    } else if j == jend - 1 {
                        sy2 - (j as f32)
                    } else {
                        1f32
                    };

                    let ar_ratio = dx * dy;

                    sr += (image[j][i].0 as f32) * ar_ratio;
                    sg += (image[j][i].1 as f32) * ar_ratio;
                    sb += (image[j][i].2 as f32) * ar_ratio;
                }
            }

            sr *= ir2;
            sg *= ir2;
            sb *= ir2;

            scanline.push((sr as u8, sg as u8, sb as u8));
        }
        downsized_image.push(scanline);
    }

    Ok(downsized_image)
}
