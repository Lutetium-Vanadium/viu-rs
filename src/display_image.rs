use super::common::*;

fn replace_with_bg(col: &RGBColor, bkgd: &RGBColor) -> RGBColor {
    if is_transparent(col.0, col.1, col.2) {
        *bkgd
    } else {
        *col
    }
}

pub fn display_image(image: &Image<RGBColor>, bkgd: &RGBColor) {
    println!();

    let is_bg_transparent = is_transparent(bkgd.0, bkgd.1, bkgd.2);

    if HD {
        let w = image[0].len();
        let h = image.len();

        // 4 times the number of pixels instead of using ██ as one block, use ▀▄ as 4 blocks
        // Currently colours are messed up, due to bad downsizing algorithm
        let mut y = 0;
        while y < h {
            for x in 0..w {
                let (tr, tg, tb) = replace_with_bg(&image[y][x], bkgd);
                let (br, bg, bb) = if y + 1 == h {
                    (0, 0, 0)
                } else {
                    replace_with_bg(&image[y + 1][x], bkgd)
                };

                let s = if is_transparent(tr, tg, tb) && is_bg_transparent {
                    " "
                } else {
                    "▀"
                };

                print!(
                    "\x1B[38;2;{};{};{};48;2;{};{};{}m{}\x1B[0m",
                    tr, tg, tb, br, bg, bb, s
                );
            }
            println!();
            y += 2;
        }
    } else {
        let mut i = 0;
        for scanline in image {
            print!("{:02}: ", i);
            for (r, g, b) in scanline {
                let (r, g, b) = replace_with_bg(&(*r, *g, *b), bkgd);
                print!("\x1B[48;2;{};{};{}m  \x1B[0m", r, g, b);
            }
            println!();
            i += 1;
        }
    }
}
