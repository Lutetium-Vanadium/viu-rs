use libflate::zlib::Decoder;
use std::env;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::str;

mod chunks;
mod helpers;
mod parse_image;

use chunks::*;
use helpers::*;

// static FILE_NAME: &'static str = "Drishti.png";
// static FILE_NAME: &'static str = "code-oss.png";

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    let file_name = &args[1];
    let mut f = File::open(file_name)?;
    let mut buffer = Vec::new();

    f.read_to_end(&mut buffer)?;

    println!("Buffer length: {}", buffer.len());

    let mut i = 8;

    if buffer[..8] != [137, 80, 78, 71, 13, 10, 26, 10] {
        eprintln!("Invalid Image given. The image is either not a png or has been corrupted.");
        return Ok(());
    }

    let mut ihdr_chunk: ihdr::IHDRChunk = Default::default();
    let mut parsed_first = false;
    let mut zlib_stream: Vec<u8> = Vec::new();

    loop {
        let chunk_length = from_bytes(&buffer[i..i + 4]) as usize;
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
                ihdr_chunk = ihdr::IHDRChunk::parse(chunk_data);
                println!("Image size: {}x{}", ihdr_chunk.width(), ihdr_chunk.height());
                println!("Image color type: {:?}", ihdr_chunk.color_type());
                println!("Image pixel size: {}", ihdr_chunk.pixel_size());
                println!("Image interlace: {:?} ", ihdr_chunk.interlace_method())
            } else if chunk_type == chunk_types::PLTE {
                let plte_chunk = plte::PLTEChunk::parse(chunk_data, chunk_length);
                println!("Palette length: {}", plte_chunk.length);
                println!("Palette Colors: {:?}", plte_chunk.colors);
                ihdr_chunk.set_palette(plte_chunk);
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
            println!("");
        }
    }

    println!("got {} bytes of zlib data", zlib_stream.len());

    let mut decoder = Decoder::new(&zlib_stream[..])?;
    let mut image_data = Vec::new();
    decoder.read_to_end(&mut image_data)?;

    println!("got {} bytes of image data", image_data.len());

    let image = parse_image::parse(image_data, &ihdr_chunk);

    display_image(&image);

    Ok(())
}
