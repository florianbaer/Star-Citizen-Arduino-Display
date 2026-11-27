//! Schemaless protobuf wire parser.

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WireValue<'a> {
    Varint(u64),
    Fixed64(u64),
    Bytes(&'a [u8]),
    Fixed32(u32),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Field<'a> {
    pub number: u32,
    pub value: WireValue<'a>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireError {
    Truncated,
    VarintOverflow,
    InvalidWireType(u8),
    InvalidFieldNumber,
}

impl std::fmt::Display for WireError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for WireError {}

/// Reads a varint (max 10 bytes), advancing `pos`.
pub fn read_varint(buf: &[u8], pos: &mut usize) -> Result<u64, WireError> {
    let mut result = 0u64;
    for i in 0..10 {
        let b = *buf.get(*pos).ok_or(WireError::Truncated)?;
        *pos += 1;
        let bits = (b & 0x7F) as u64;
        if i == 9 && bits > 1 {
            return Err(WireError::VarintOverflow);
        }
        result |= bits << (7 * i);
        if b & 0x80 == 0 {
            return Ok(result);
        }
    }
    Err(WireError::VarintOverflow)
}

pub fn parse_message(buf: &[u8]) -> Result<Vec<Field<'_>>, WireError> {
    let mut pos = 0;
    let mut out = Vec::new();
    while pos < buf.len() {
        let tag = read_varint(buf, &mut pos)?;
        let wt = (tag & 7) as u8;
        let number = tag >> 3;
        if number == 0 || number > 536_870_911 {
            return Err(WireError::InvalidFieldNumber);
        }
        let value = match wt {
            0 => WireValue::Varint(read_varint(buf, &mut pos)?),
            1 => {
                let end = pos.checked_add(8).filter(|&e| e <= buf.len()).ok_or(WireError::Truncated)?;
                let v = u64::from_le_bytes(buf[pos..end].try_into().unwrap());
                pos = end;
                WireValue::Fixed64(v)
            }
            2 => {
                let len = read_varint(buf, &mut pos)?;
                let end = usize::try_from(len)
                    .ok()
                    .and_then(|l| pos.checked_add(l))
                    .filter(|&e| e <= buf.len())
                    .ok_or(WireError::Truncated)?;
                let v = &buf[pos..end];
                pos = end;
                WireValue::Bytes(v)
            }
            5 => {
                let end = pos.checked_add(4).filter(|&e| e <= buf.len()).ok_or(WireError::Truncated)?;
                let v = u32::from_le_bytes(buf[pos..end].try_into().unwrap());
                pos = end;
                WireValue::Fixed32(v)
            }
            other => return Err(WireError::InvalidWireType(other)),
        };
        out.push(Field { number: number as u32, value });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rv(b: &[u8]) -> Result<u64, WireError> {
        let mut p = 0;
        read_varint(b, &mut p)
    }

    #[test]
    fn varint_edges() {
        assert_eq!(rv(&[0]), Ok(0));
        assert_eq!(rv(&[127]), Ok(127));
        assert_eq!(rv(&[0x80, 0x01]), Ok(128));
        let mut max = vec![0xFF; 9];
        max.push(0x01);
        assert_eq!(rv(&max), Ok(u64::MAX));
        assert_eq!(rv(&[0xFF; 11]), Err(WireError::VarintOverflow));
        assert_eq!(rv(&[0x80]), Err(WireError::Truncated));
    }

    #[test]
    fn truncated_fixed32() {
        assert_eq!(parse_message(&[0x0D, 0, 0]), Err(WireError::Truncated));
    }

    #[test]
    fn invalid_wire_type_and_field() {
        assert_eq!(parse_message(&[0x0B]), Err(WireError::InvalidWireType(3)));
        assert_eq!(parse_message(&[0x00, 0x01]), Err(WireError::InvalidFieldNumber));
    }

    #[test]
    fn parses_fields() {
        let f = parse_message(&[0x08, 0x96, 0x01, 0x12, 0x02, 0xAA, 0xBB, 0x1D, 0, 0, 0x80, 0x3F]).unwrap();
        assert_eq!(f.len(), 3);
        assert_eq!(f[0], Field { number: 1, value: WireValue::Varint(150) });
        assert_eq!(f[1], Field { number: 2, value: WireValue::Bytes(&[0xAA, 0xBB]) });
        assert_eq!(f[2], Field { number: 3, value: WireValue::Fixed32(0x3F80_0000) });
    }
}
