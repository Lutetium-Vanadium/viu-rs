pub type VerifyResult = Result<(), u32>;

pub struct CRCHandler {
    // Table of CRCs of all 8-bit messages.
    table: [u32; 256],
}

impl CRCHandler {
    // Make the table for a fast CRC.
    pub fn new() -> CRCHandler {
        let mut crc_table = [0; 256];
        for n in 0..256 {
            let mut c = n as u32;
            for _ in 0..8 {
                if c & 1 > 0 {
                    c = 0xEDB88320 ^ (c >> 1);
                } else {
                    c = c >> 1;
                }
            }
            crc_table[n] = c;
        }
        CRCHandler { table: crc_table }
    }

    // Update a running CRC with the bytes buf[0..len-1]--the CRC
    // should be initialized to all 1's, and the transmitted value
    // is the 1's complement of the final running CRC (see the
    // crc() routine below)).
    fn update_crc(&self, crc: u32, buf: &[u8], len: usize) -> u32 {
        let mut c = crc;
        for n in 0..len {
            c = self.table[((c ^ buf[n] as u32) & 0xFF) as usize] ^ (c >> 8);
        }
        return c;
    }

    // Return the CRC of the bytes buf[0..len-1].
    pub fn crc(&self, buf: &[u8]) -> u32 {
        self.update_crc(0xFFFFFFFF, buf, buf.len()) ^ 0xFFFFFFFF
    }

    pub fn verify(&self, crc: u32, buf: &[u8]) -> VerifyResult {
        let calc_crc = self.crc(buf);
        if calc_crc == crc {
            Ok(())
        } else {
            Err(calc_crc)
        }
    }
}
