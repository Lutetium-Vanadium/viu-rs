use super::common::*;

// General purpose
fn replace_with_bg(col: &RGBColor, bkgd: &RGBColor) -> RGBColor {
    if is_transparent(col.0, col.1, col.2) {
        *bkgd
    } else {
        *col
    }
}

// for blur
fn generate_kernel(n: i32) -> Vec<Vec<f32>> {
    println!("i: {} -> {} ; j: {} -> {}", 0, (n / 2 + 1), 1, (n / 2 + 1));
    let mut sum = 1.0; // For central tile
    for i in 0..(n / 2 + 1) {
        for j in 1..(n / 2 + 1) {
            // 4 times because we are going through only quater of the square
            sum += 4.0 / (i * i + j * j + 1) as f32;
        }
    }

    let sum_inverse = 1.0 / sum;

    let mut kernel = Vec::with_capacity(n as usize);
    for i in (-n / 2)..(n / 2 + 1) {
        let mut line = Vec::with_capacity(n as usize);
        for j in (-n / 2)..(n / 2 + 1) {
            line.push(sum_inverse / (i * i + j * j + 1) as f32);
        }
        kernel.push(line);
    }
    kernel
}

fn clamp<T: std::cmp::Ord>(n: T, min: T, max: T) -> T {
    std::cmp::min(max, std::cmp::max(n, min))
}

fn apply_blur(
    image: &Image<RGBColor>,
    x: i32,
    y: i32,
    intensity: i32,
    kernel: &Vec<Vec<f32>>,
    w: i32,
    h: i32,
    bkgd: &RGBColor,
) -> RGBColor {
    let mut r = 0.0;
    let mut g = 0.0;
    let mut b = 0.0;
    for i in 0..intensity {
        for j in 0..intensity {
            let px = replace_with_bg(
                &image[clamp(y + j - intensity / 2, 0, h - 1) as usize]
                    [clamp(x + i - intensity / 2, 0, w - 1) as usize],
                bkgd,
            );
            r += kernel[i as usize][j as usize] * px.0 as f32;
            g += kernel[i as usize][j as usize] * px.1 as f32;
            b += kernel[i as usize][j as usize] * px.2 as f32;
        }
    }

    // It can be directly to u8 which would floor it keeping it in the bounds
    // >    (r as u8, g as u8, b as u8)
    // but there are floating point errors, which means for the sum of all percentages
    // in the kernel may not be exactly 1. Suppose the value of the sum of the kernel is
    // 0.999, for a completely black background [(0, 0, 1) because of transparency],
    // the RGB value calculated will be (0, 0, 0.99) which should be taken as (0, 0, 1).
    //
    // So, the values are rounded and then clamped to help reduce floating point errors
    (
        clamp(r.round() as u8, 0, 255),
        clamp(g.round() as u8, 0, 255),
        clamp(b.round() as u8, 0, 255),
    )
}

// for ASCII
const CHARS: [char; 32] = [
    ' ', '.', '\'', '`', ',', ':', '-', '=', '~', '"', '+', '*', '>', '<', '}', '{', 'f', 'j', 'n',
    'v', 'z', 'u', 'k', 'U', 'O', '#', 'M', 'W', '&', '%', '$', '@',
];

pub fn display_image(image: &Image<RGBColor>, bkgd: &RGBColor, effect: Effect) {
    println!();

    let is_bg_transparent = is_transparent(bkgd.0, bkgd.1, bkgd.2);

    match effect {
        Effect::Blur(intensity) => {
            let w = image[0].len();
            let h = image.len();

            let kernel = generate_kernel(intensity as i32);

            // use ▀▄ as 4 pixels
            let mut y = 0;
            while y < h {
                for x in 0..w {
                    let (tr, tg, tb) = apply_blur(
                        image,
                        x as i32,
                        y as i32,
                        intensity as i32,
                        &kernel,
                        w as i32,
                        h as i32,
                        bkgd,
                    );

                    let (br, bg, bb) = if y + 1 == h {
                        (0, 0, 0)
                    } else {
                        apply_blur(
                            image,
                            x as i32,
                            y as i32 + 1,
                            intensity as i32,
                            &kernel,
                            w as i32,
                            h as i32,
                            bkgd,
                        )
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
        }
        Effect::ASCII => {
            // use @@ as one pixel
            for scaline in image {
                for col in scaline {
                    let (r, g, b) = replace_with_bg(col, bkgd);

                    let idx = (r as usize + g as usize + b as usize) / 24;
                    print!("{}{}", CHARS[idx], CHARS[idx]);
                }
                println!();
            }
        }
        Effect::NoEffect => {
            let w = image[0].len();
            let h = image.len();

            // use ▀▄ as 4 pixels
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
        }
    }
}
