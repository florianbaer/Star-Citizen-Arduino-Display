/// COBS-encode `src` into `dst`. Returns the number of bytes written to `dst`.
/// `dst` must be at least `src.len() + src.len()/254 + 1` bytes.
pub fn encode(src: &[u8], dst: &mut [u8]) -> usize {
    let mut code_idx = 0;
    let mut code: u8 = 1;
    let mut write_idx = 1;

    for &byte in src {
        if byte == 0x00 {
            dst[code_idx] = code;
            code_idx = write_idx;
            write_idx += 1;
            code = 1;
        } else {
            dst[write_idx] = byte;
            write_idx += 1;
            code += 1;
            if code == 0xFF {
                dst[code_idx] = code;
                code_idx = write_idx;
                write_idx += 1;
                code = 1;
            }
        }
    }

    dst[code_idx] = code;
    write_idx
}

/// COBS-decode `src` into `dst`. Returns the number of decoded bytes, or `None` on error.
pub fn decode(src: &[u8], dst: &mut [u8]) -> Option<usize> {
    let mut src_idx = 0;
    let mut dst_idx = 0;

    while src_idx < src.len() {
        let code = src[src_idx] as usize;
        if code == 0 {
            return None;
        }
        src_idx += 1;

        for _ in 1..code {
            if src_idx >= src.len() || dst_idx >= dst.len() {
                return None;
            }
            dst[dst_idx] = src[src_idx];
            dst_idx += 1;
            src_idx += 1;
        }

        // Insert implicit zero between groups, but not after the last group
        if code < 0xFF && src_idx < src.len() {
            if dst_idx >= dst.len() {
                return None;
            }
            dst[dst_idx] = 0x00;
            dst_idx += 1;
        }
    }

    Some(dst_idx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_no_zeros() {
        let data = [0x01, 0x02, 0x03];
        let mut enc = [0u8; 16];
        let enc_len = encode(&data, &mut enc);
        let mut dec = [0u8; 16];
        let dec_len = decode(&enc[..enc_len], &mut dec).unwrap();
        assert_eq!(&dec[..dec_len], &data);
    }

    #[test]
    fn roundtrip_with_zeros() {
        let data = [0x00, 0x01, 0x00, 0x02];
        let mut enc = [0u8; 16];
        let enc_len = encode(&data, &mut enc);
        let mut dec = [0u8; 16];
        let dec_len = decode(&enc[..enc_len], &mut dec).unwrap();
        assert_eq!(&dec[..dec_len], &data);
    }

    #[test]
    fn roundtrip_all_zeros() {
        let data = [0x00, 0x00, 0x00];
        let mut enc = [0u8; 16];
        let enc_len = encode(&data, &mut enc);
        let mut dec = [0u8; 16];
        let dec_len = decode(&enc[..enc_len], &mut dec).unwrap();
        assert_eq!(&dec[..dec_len], &data);
    }

    #[test]
    fn roundtrip_empty() {
        let data = [];
        let mut enc = [0u8; 16];
        let enc_len = encode(&data, &mut enc);
        let mut dec = [0u8; 16];
        let dec_len = decode(&enc[..enc_len], &mut dec).unwrap();
        assert_eq!(dec_len, 0);
    }
}
