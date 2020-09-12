pub mod ancillary;
pub mod ihdr;
pub mod plte;

pub mod chunk_types {
    pub static IHDR: [u8; 4] = [73, 72, 68, 82];
    pub static PLTE: [u8; 4] = [80, 76, 84, 69];
    pub static IDAT: [u8; 4] = [73, 68, 65, 84];
    pub static IEND: [u8; 4] = [73, 69, 78, 68];
    #[allow(non_upper_case_globals)]
    pub static tRNS: [u8; 4] = [116, 82, 78, 83];
    #[allow(non_upper_case_globals)]
    pub static tIME: [u8; 4] = [116, 73, 77, 69];
    #[allow(non_upper_case_globals)]
    pub static tEXt: [u8; 4] = [116, 69, 88, 116];
    #[allow(non_upper_case_globals)]
    pub static zTXt: [u8; 4] = [122, 84, 88, 116];
    #[allow(non_upper_case_globals)]
    pub static bKGD: [u8; 4] = [98, 75, 71, 68];
}
