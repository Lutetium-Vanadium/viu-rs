pub mod chunks;
mod parse_image;

use crate::common::*;
use crate::crc::CRCHandler;
use chunks::*;
use libflate::zlib::Decoder;
use parse_image::parse_image;
use std::io;
use std::io::prelude::*;
use std::io::{Error, ErrorKind, Result};
use std::str;

pub fn parse(buffer: Vec<u8>, metadata: &mut Metadata) -> io::Result<Image<RGBColor>> {
    let mut i = 8;
    let crc_handler = CRCHandler::new();

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
                metadata.add_ihdr(ihdr::IHDRChunk::parse(chunk_data)?);

                println!("Image size: {}x{}", metadata.width(), metadata.height());
                println!("Image color type: {:?}", metadata.color_type());
                println!("Image pixel size: {}", metadata.pixel_size());
            } else if chunk_type == chunk_types::PLTE {
                let plte_chunk = plte::PLTEChunk::parse(chunk_data, chunk_length);

                println!("Palette length: {}", plte_chunk.length);
                println!("Palette Colors: {:?}", plte_chunk.colors);

                metadata.set_palette(plte_chunk.colors);
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
                match ancillary::parse_trns(chunk_data, &metadata) {
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

                // Ideally errors should be just printed here, instead of programme ending
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

    Ok(parse_image(image_data, &metadata)?)
}
