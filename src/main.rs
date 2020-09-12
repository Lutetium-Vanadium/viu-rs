use std::env;
use std::fs;
use std::io;
use std::io::prelude::*;
use std::io::{Error, ErrorKind};

mod common;
mod crc;
mod display_image;
mod png;

use common::*;
use display_image::display_image;

const HELP_STR: &'static str = "Usage: viu-rs [<option>] <image path>\n
Available Options:
    blur:
        Apply a blur of given intensity to the image
        Usage: viu-rs blur <intensity> <image path>
    ascii:
        Display a grayscale ascii version
        Usage: viu-rs ascii <image path>
    grayscale:
        Display a grayscale version of the image
        Usage: viu-rs grayscale <image path>
    show:
        Shows the image.
        Usage: viu-rs show <image path>
    <no-option-given>:
        Shows the image.
        Usage: viu-rs <image path>";

fn run() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        return Err(Error::new(
            ErrorKind::NotFound,
            format!("Invalid Arguments\n\n{}", HELP_STR),
        ));
    };

    let (file_name, effect) = match args[1].as_str() {
        "-h" | "--help" => return Ok(println!("{}", HELP_STR)),
        "blur" => {
            if args.len() < 4 {
                return Err(Error::new(
                    ErrorKind::NotFound,
                    format!("Invalid Arguments\n\n{}", HELP_STR),
                ));
            }
            (
                &args[3],
                Effect::Blur(match args[2].parse::<usize>() {
                    Ok(intensity) => intensity * 2 + 1,
                    Err(_) => {
                        return Err(Error::new(
                            ErrorKind::InvalidData,
                            "Blur size must be given",
                        ))
                    }
                }),
            )
        }
        "ascii" => {
            if args.len() < 3 {
                return Err(Error::new(
                    ErrorKind::NotFound,
                    format!("Invalid Arguments\n\n{}", HELP_STR),
                ));
            }
            (&args[2], Effect::ASCII)
        }
        "grayscale" => {
            if args.len() < 3 {
                return Err(Error::new(
                    ErrorKind::NotFound,
                    format!("Invalid Arguments\n\n{}", HELP_STR),
                ));
            }
            (&args[2], Effect::GrayScale)
        }
        "show" => {
            if args.len() < 3 {
                return Err(Error::new(
                    ErrorKind::NotFound,
                    format!("Invalid Arguments\n\n{}", HELP_STR),
                ));
            }
            (&args[2], Effect::NoEffect)
        }
        _ => (&args[1], Effect::NoEffect),
    };

    let mut f = fs::File::open(file_name)?;

    // Prevent Vector reallocation because size is too small
    let mut buffer = {
        let f_meta = fs::metadata(file_name)?;
        Vec::with_capacity(f_meta.len() as usize)
    };

    f.read_to_end(&mut buffer)?;
    println!("Buffer length: {}", buffer.len());

    let mut metadata = Metadata::new();

    // PNG file signature
    let image = if buffer[..8] == [137, 80, 78, 71, 13, 10, 26, 10] {
        png::parse(buffer, &mut metadata)?
    } else {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Invalid file signature: The image is either not a png or has been corrupted.",
        ));
    };

    assert_eq!(image[0].len() as u32, metadata.width());
    assert_eq!(image.len() as u32, metadata.height());

    let image = auto_downsize_image(image, &effect)?;

    display_image(&image, metadata.bkgd(), effect);

    Ok(())
}

fn main() {
    match run() {
        Err(e) => {
            eprintln!("{}", e);
        }
        _ => {}
    };
}
