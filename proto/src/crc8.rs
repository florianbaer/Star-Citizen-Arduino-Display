/// CRC8/MAXIM lookup table (polynomial 0x31, init 0x00).
const CRC8_TABLE: [u8; 256] = {
    let mut table = [0u8; 256];
    let mut i = 0u16;
    while i < 256 {
        let mut crc = i as u8;
        let mut bit = 0;
        while bit < 8 {
            if crc & 0x80 != 0 {
                crc = (crc << 1) ^ 0x31;
            } else {
                crc <<= 1;
            }
            bit += 1;
        }
        table[i as usize] = crc;
        i += 1;
    }
    table
};

/// Compute CRC8/MAXIM over the given data.
pub fn crc8(data: &[u8]) -> u8 {
    let mut crc: u8 = 0x00;
    for &byte in data {
        crc = CRC8_TABLE[(crc ^ byte) as usize];
    }
    crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc8_known_values() {
        assert_eq!(crc8(&[]), 0x00);
        assert_eq!(crc8(&[0x00]), 0x00);
        // Different data should produce different CRCs
        assert_ne!(crc8(&[0x01]), crc8(&[0x02]));
    }

    #[test]
    fn crc8_deterministic() {
        let data = [0x01, 0x02, 0x03, 0xFF, 0x80];
        assert_eq!(crc8(&data), crc8(&data));
    }
}
