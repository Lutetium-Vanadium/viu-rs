use libflate::zlib::Decoder;
use std::env;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::str;

mod chunks;
mod common;
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
        panic!("Invalid Image given. The image is either not a png or has been corrupted.");
    }

    let mut metadata = Metadata::new();

    let mut parsed_first = false;
    let mut zlib_stream: Vec<u8> = Vec::new();

    loop {
        let chunk_length = from_bytes_u32(&buffer[i..i + 4]) as usize;
        i += 4;
        let chunk_type = &buffer[i..i + 4];
        i += 4;
        print!(
            "Found Chunk with size: {}.\tChunk Type: {}",
            chunk_length,
            str::from_utf8(chunk_type).unwrap()
        );
        let chunk_data = &buffer[i..i + chunk_length];
        i += chunk_length;
        let crc = &buffer[i..i + 4];
        i += 4;
        print!("\tCRC: {:x?}", crc);

        if !parsed_first && chunk_type != chunk_types::IHDR {
            panic!(
                "First chunk needs to be IHDR, got: {}",
                str::from_utf8(chunk_type).unwrap()
            );
        } else {
            parsed_first = true;
        }

        // Is Upper case => Important, cannot be ignored
        if chunk_type[0] & (1 << 5) == 0 {
            println!("\t\tIMP");
            if chunk_type == chunk_types::IHDR {
                metadata.ihdr_chunk = ihdr::IHDRChunk::parse(chunk_data);
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
                panic!(
                    "Unknown chunk type: {}",
                    str::from_utf8(chunk_type).unwrap()
                )
            }
        } else {
            if chunk_type == chunk_types::tRNS {
                let trns_chunk = ancillary::TRNSChunk::parse(chunk_data, &metadata);
                metadata.set_alpha(trns_chunk);
            } else if chunk_type == chunk_types::tIME {
                let time_chunk = ancillary::TIMEChunk::parse(chunk_data);
                print!("\nLast Changed: {}", time_chunk);
            } else if chunk_type == chunk_types::tEXt {
                let text_chunk =
                    ancillary::TextChunk::parse(ancillary::TextChunk::split(chunk_data)).unwrap();
                if text_chunk.key.len() > 0 {
                    print!("\n{}: {}", text_chunk.key, text_chunk.text);
                }
            } else if chunk_type == chunk_types::zTXt {
                let (keyword_chunk, text_chunk) = ancillary::TextChunk::split(chunk_data);
                let mut decoder = Decoder::new(&text_chunk[..])?;
                let mut text_chunk = Vec::new();
                decoder.read_to_end(&mut text_chunk)?;
                let text_chunk =
                    ancillary::TextChunk::parse((keyword_chunk, &text_chunk[..])).unwrap();
                if text_chunk.key.len() > 0 {
                    print!("\n{}: {}", text_chunk.key, text_chunk.text);
                }
            } else if chunk_type == chunk_types::bKGD {
                let bkgd = ancillary::parse_bkgd_chunk(chunk_data, &metadata);
                println!("Got backround: {:?}", bkgd);
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

    let image = parse_image::parse(image_data, &metadata);

    assert_eq!(image[0].len() as u32, metadata.ihdr_chunk.width());
    assert_eq!(image.len() as u32, metadata.ihdr_chunk.height());

    display_image(&image, metadata.bkgd());

    Ok(())
}
