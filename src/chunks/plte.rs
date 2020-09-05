use crate::common::RGBColor;

pub struct PLTEChunk {
    pub length: usize,
    pub colors: Vec<RGBColor>,
}

impl PLTEChunk {
    pub fn parse(bytes: &[u8], size: usize) -> PLTEChunk {
        assert_eq!(size % 3, 0);

        let length = size / 3;
        let mut colors: Vec<RGBColor> = Vec::new();

        for i in 0..length {
            colors.push((bytes[i * 3], bytes[i * 3 + 1], bytes[i * 3 + 2]));
        }

        PLTEChunk { length, colors }
    }
}
