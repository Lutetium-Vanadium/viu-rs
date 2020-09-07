use libflate::zlib::Decoder;
use std::env;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::io::{Error, ErrorKind};
use std::str;

mod chunks;
mod common;
mod crc;
mod parse_image;

use chunks::*;
use common::*;

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    let file_name = &args[1];
    let mut f = File::open(file_name)?;
    let mut buffer = Vec::new();

    f.read_to_end(&mut buffer)?;

    println!("Buffer length: {}", buffer.len());

    let mut i = 8;

    // PNG file signature
    if buffer[..8] != [137, 80, 78, 71, 13, 10, 26, 10] {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Invalid file signature: The image is either not a png or has been corrupted.",
        ));
    }

    let mut metadata = Metadata::new();
    let crc_handler = crc::CRCHandler::new();

    let mut parsed_first = false;
    let mut zlib_stream: Vec<u8> = Vec::new();

    loop {
        let chunk_length = from_bytes_u32(&buffer[i..i + 4]) as usize;
        i += 4;
        let crc_chunk_start = i;
        let chunk_type = &buffer[i..i + 4];
        i += 4;
        print!(
            "Found Chunk with size: {}.\tChunk Type: {}",
            chunk_length,
            str::from_utf8(chunk_type).unwrap()
        );
        let chunk_data = &buffer[i..i + chunk_length];
        i += chunk_length;
        let crc = from_bytes_u32(&buffer[i..i + 4]);
        match crc_handler.verify(crc, &buffer[crc_chunk_start..i]) {
            Err(calc_crc) => {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    format!(
                        "Invalid Chunk; CRC didnt match -> got: {}   calculated: {}",
                        crc, calc_crc,
                    ),
                ));
            }
            _ => {}
        };
        // i incremented after crc check because crc bytes shouldnt be included in the crc check
        i += 4;

        print!("\tCRC: {}", crc);

        if !parsed_first && chunk_type != chunk_types::IHDR {
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!(
                    "First chunk needs to be IHDR, got: {}",
                    str::from_utf8(chunk_type).unwrap()
                ),
            ));
        } else {
            parsed_first = true;
        }

        // Is Upper case => Important, cannot be ignored
        if chunk_type[0] & (1 << 5) == 0 {
            println!("\t\tIMP");
            if chunk_type == chunk_types::IHDR {
                metadata.ihdr_chunk = ihdr::IHDRChunk::parse(chunk_data)?;
                println!("DEBUG: {:0x?}", &buffer[crc_chunk_start..i]);
                println!(
                    "Image size: {}x{}",
                    metadata.ihdr_chunk.width(),
                    metadata.ihdr_chunk.height()
                );
                println!("Image color type: {:?}", metadata.ihdr_chunk.color_type());
                println!("Image pixel size: {}", metadata.ihdr_chunk.pixel_size());
                println!(
                    "Image interlace: {:?} ",
                    metadata.ihdr_chunk.interlace_method()
                )
            } else if chunk_type == chunk_types::PLTE {
                let plte_chunk = plte::PLTEChunk::parse(chunk_data, chunk_length);
                println!("Palette length: {}", plte_chunk.length);
                println!("Palette Colors: {:?}", plte_chunk.colors);
                metadata.set_palette(plte_chunk);
            } else if chunk_type == chunk_types::IDAT {
                zlib_stream.extend(chunk_data.iter());
            } else if chunk_type == chunk_types::IEND {
                break;
            } else {
                return Err(Error::new(
                    ErrorKind::Other,
                    format!(
                        "Unknown chunk type: {}",
                        str::from_utf8(chunk_type).unwrap()
                    ),
                ));
            }
        } else {
            if chunk_type == chunk_types::tRNS {
                match ancillary::TRNSChunk::parse(chunk_data, &metadata) {
                    Ok(trns_chunk) => metadata.set_alpha(trns_chunk),
                    Err(e) => eprintln!("{}", e),
                }
            } else if chunk_type == chunk_types::tIME {
                let time_chunk = ancillary::TIMEChunk::parse(chunk_data);
                print!("\nLast Changed: {}", time_chunk);
            } else if chunk_type == chunk_types::tEXt {
                match ancillary::TextChunk::parse(ancillary::TextChunk::split(chunk_data)) {
                    Ok(text_chunk) => {
                        if text_chunk.key.len() > 0 {
                            print!("\n{}: {}", text_chunk.key, text_chunk.text);
                        }
                    }
                    Err(e) => eprintln!("{}", e),
                };
            } else if chunk_type == chunk_types::zTXt {
                let (keyword_chunk, text_chunk) = ancillary::TextChunk::split(chunk_data);
                // Ideally errors should be just printed here, instead of programming ending
                let mut decoder = Decoder::new(&text_chunk[..])?;
                let mut text_chunk = Vec::new();
                decoder.read_to_end(&mut text_chunk)?;
                match ancillary::TextChunk::parse((keyword_chunk, &text_chunk[..])) {
                    Ok(text_chunk) => {
                        if text_chunk.key.len() > 0 {
                            print!("\n{}: {}", text_chunk.key, text_chunk.text);
                        }
                    }
                    Err(e) => eprintln!("{}", e),
                };
            } else if chunk_type == chunk_types::bKGD {
                let bkgd = match ancillary::parse_bkgd_chunk(chunk_data, &metadata) {
                    Ok(bkgd) => bkgd,
                    Err(e) => {
                        eprintln!("{}", e);
                        (0, 0, 0)
                    }
                };
                print!("\nGot backround: {:?}", bkgd);
                metadata.set_bkgd(bkgd);
            }
            println!("");
        }
    }

    println!("got {} bytes of zlib data", zlib_stream.len());

    let mut decoder = Decoder::new(&zlib_stream[..])?;
    let mut image_data = Vec::new();
    decoder.read_to_end(&mut image_data)?;

    println!("got {} bytes of image data", image_data.len());

    let image = parse_image::parse(image_data, &metadata)?;

    assert_eq!(image[0].len() as u32, metadata.ihdr_chunk.width());
    assert_eq!(image.len() as u32, metadata.ihdr_chunk.height());

    display_image(&image, metadata.bkgd());

    Ok(())
}
