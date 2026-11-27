//! Minimal protobuf encoding helpers (tests, mock server, action request).

pub fn put_varint(buf: &mut Vec<u8>, mut v: u64) {
    while v >= 0x80 {
        buf.push((v as u8 & 0x7F) | 0x80);
        v >>= 7;
    }
    buf.push(v as u8);
}

pub fn put_tag(buf: &mut Vec<u8>, field: u32, wire_type: u8) {
    put_varint(buf, ((field as u64) << 3) | wire_type as u64);
}

pub fn put_varint_field(buf: &mut Vec<u8>, field: u32, v: u64) {
    put_tag(buf, field, 0);
    put_varint(buf, v);
}

pub fn put_f32_field(buf: &mut Vec<u8>, field: u32, v: f32) {
    put_tag(buf, field, 5);
    buf.extend_from_slice(&v.to_le_bytes());
}

pub fn put_bytes_field(buf: &mut Vec<u8>, field: u32, b: &[u8]) {
    put_tag(buf, field, 2);
    put_varint(buf, b.len() as u64);
    buf.extend_from_slice(b);
}

pub fn put_str_field(buf: &mut Vec<u8>, field: u32, s: &str) {
    put_bytes_field(buf, field, s.as_bytes());
}

pub fn put_packed_f32_field(buf: &mut Vec<u8>, field: u32, vals: &[f32]) {
    let mut payload = Vec::with_capacity(vals.len() * 4);
    for v in vals {
        payload.extend_from_slice(&v.to_le_bytes());
    }
    put_bytes_field(buf, field, &payload);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pb::wire::{parse_message, read_varint, WireValue};

    #[test]
    fn varint_roundtrip() {
        for v in [0u64, 1, 127, 128, 300, u32::MAX as u64, u64::MAX] {
            let mut b = Vec::new();
            put_varint(&mut b, v);
            let mut p = 0;
            assert_eq!(read_varint(&b, &mut p), Ok(v));
            assert_eq!(p, b.len());
        }
    }

    #[test]
    fn fields_roundtrip() {
        let mut b = Vec::new();
        put_varint_field(&mut b, 1, 5);
        put_f32_field(&mut b, 2, 1.5);
        put_str_field(&mut b, 3, "hi");
        put_packed_f32_field(&mut b, 4, &[1.0, 0.5]);
        let f = parse_message(&b).unwrap();
        assert_eq!(f[0].value, WireValue::Varint(5));
        assert_eq!(f[1].value, WireValue::Fixed32(1.5f32.to_bits()));
        assert_eq!(f[2].value, WireValue::Bytes(b"hi"));
        assert!(matches!(f[3].value, WireValue::Bytes(x) if x.len() == 8));
    }
}
