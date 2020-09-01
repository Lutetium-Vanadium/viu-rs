pub mod ihdr;
pub mod plte;

pub mod chunk_types {
    pub static IHDR: [u8; 4] = [73, 72, 68, 82];
    pub static PLTE: [u8; 4] = [80, 76, 84, 69];
    pub static IDAT: [u8; 4] = [73, 68, 65, 84];
    pub static IEND: [u8; 4] = [73, 69, 78, 68];
}
