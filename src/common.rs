use crate::png::chunks::ihdr;
use std::io;

pub type RGBColor = (u8, u8, u8);
pub type Image<T> = Vec<Vec<T>>;

pub enum Effect {
    NoEffect,
    Blur(usize),
    ASCII,
    GrayScale,
}

#[derive(Debug)]
pub enum ColorType {
    Gray,
    RGB,
    Palette,
    GrayA,
    RGBA,
}

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
pub enum AlphaValue {
    RGB(u8, u8, u8),
    Gray(u8),
    Palette(Vec<u8>),
}

pub struct Metadata {
    width: u32,
    height: u32,
    bit_depth: u8,
    color_type: ColorType,
    palette: Option<Vec<RGBColor>>,
    alpha: Option<AlphaValue>,
    bkgd: RGBColor,
}

impl Metadata {
    pub fn new() -> Metadata {
        Metadata {
            alpha: None,
            palette: None,
            bkgd: (0, 0, 0), // Default background is transparent
            width: 0,
            height: 0,
            bit_depth: 0,
            color_type: ColorType::RGB,
        }
    }

    pub fn add_ihdr(&mut self, ihdr_chunk: ihdr::IHDRChunk) {
        self.width = ihdr_chunk.width();
        self.height = ihdr_chunk.height();
        self.bit_depth = ihdr_chunk.bit_depth();
        self.color_type = ihdr_chunk.color_type();
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn color_type(&self) -> &ColorType {
        &self.color_type
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

    pub fn palette(&self) -> &Option<Vec<RGBColor>> {
        &self.palette
    }

    pub fn set_palette(&mut self, palette: Vec<RGBColor>) {
        self.palette = Some(palette);
    }

    pub fn alpha(&self) -> &Option<AlphaValue> {
        &self.alpha
    }

    pub fn set_alpha(&mut self, alpha: AlphaValue) {
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

pub fn auto_downsize_image(image: Image<RGBColor>, effect: &Effect) -> io::Result<Image<RGBColor>> {
    // Terminal dimensions
    let (tw, th) = if let Some((w, h)) = term_size::dimensions() {
        match effect {
            // Double characters for one square pixel
            // Eg:
            //      ::  <-- One pixel
            //  not :
            Effect::ASCII => (w / 2, h),
            _ => {
                // Use fg + bg to make one character 2 pixels
                (w, h * 2)
            }
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

    if r == 1.0 {
        return Ok(image);
    }

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
