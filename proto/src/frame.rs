use crate::cobs;
use crate::crc8::crc8;
use crate::messages::MsgType;

/// Maximum payload size (excluding msg_type and CRC).
pub const MAX_PAYLOAD: usize = 64;

/// Maximum frame size on the wire: 0x00 + COBS(msg_type + payload + crc8) + 0x00
/// COBS overhead is at most ceil(N/254) bytes.
pub const MAX_FRAME: usize = 2 + MAX_PAYLOAD + 1 + 1 + (MAX_PAYLOAD + 2) / 254 + 1;

/// Encode a message into a framed wire format.
/// Returns the number of bytes written to `out`.
///
/// Wire format: `[0x00] [COBS-encoded: msg_type | payload... | CRC8] [0x00]`
pub fn frame(msg_type: MsgType, payload: &[u8], out: &mut [u8]) -> usize {
    // Build: msg_type | payload | crc8
    let mut raw = [0u8; MAX_PAYLOAD + 2]; // +1 type, +1 crc
    raw[0] = msg_type as u8;
    let plen = payload.len().min(MAX_PAYLOAD);
    raw[1..1 + plen].copy_from_slice(&payload[..plen]);
    let raw_len = 1 + plen;
    let checksum = crc8(&raw[..raw_len]);
    raw[raw_len] = checksum;
    let raw_len = raw_len + 1;

    // COBS encode into out[1..]
    out[0] = 0x00;
    let cobs_len = cobs::encode(&raw[..raw_len], &mut out[1..]);
    out[1 + cobs_len] = 0x00;

    2 + cobs_len
}

/// Decoded frame result.
pub struct DeframedMsg<'a> {
    pub msg_type: MsgType,
    pub payload: &'a [u8],
}

/// Decode a COBS-encoded frame (without the 0x00 delimiters).
/// `data` should be the bytes between the two 0x00 delimiters.
/// `buf` is scratch space for decoding (at least MAX_PAYLOAD + 2 bytes).
/// Returns the message type and a slice into `buf` containing the payload.
pub fn deframe<'a>(data: &[u8], buf: &'a mut [u8]) -> Option<DeframedMsg<'a>> {
    let decoded_len = cobs::decode(data, buf)?;
    if decoded_len < 2 {
        return None; // need at least msg_type + crc
    }

    // Check CRC
    let payload_end = decoded_len - 1;
    let expected_crc = buf[payload_end];
    let actual_crc = crc8(&buf[..payload_end]);
    if expected_crc != actual_crc {
        return None;
    }

    let msg_type = MsgType::from_u8(buf[0])?;
    Some(DeframedMsg {
        msg_type,
        payload: &buf[1..payload_end],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::TelemetryMsg;

    #[test]
    fn roundtrip_telemetry() {
        let msg = TelemetryMsg {
            shield_front: 255,
            shield_back: 128,
            shield_left: 64,
            shield_right: 32,
            hydrogen_fuel: 200,
            quantum_fuel: 100,
        };

        let mut wire = [0u8; MAX_FRAME];
        let wire_len = frame(MsgType::Telemetry, msg.as_bytes(), &mut wire);

        // Wire should start and end with 0x00
        assert_eq!(wire[0], 0x00);
        assert_eq!(wire[wire_len - 1], 0x00);

        // Deframe the inner COBS data
        let mut buf = [0u8; MAX_PAYLOAD + 2];
        let result = deframe(&wire[1..wire_len - 1], &mut buf).unwrap();
        assert_eq!(result.msg_type, MsgType::Telemetry);

        let decoded = TelemetryMsg::from_bytes(result.payload).unwrap();
        assert_eq!(decoded, msg);
    }

    #[test]
    fn bad_crc_rejected() {
        let msg = TelemetryMsg {
            shield_front: 1,
            shield_back: 2,
            shield_left: 3,
            shield_right: 4,
            hydrogen_fuel: 5,
            quantum_fuel: 6,
        };

        let mut wire = [0u8; MAX_FRAME];
        let wire_len = frame(MsgType::Telemetry, msg.as_bytes(), &mut wire);

        // Corrupt a byte in the COBS data
        wire[3] ^= 0xFF;

        let mut buf = [0u8; MAX_PAYLOAD + 2];
        assert!(deframe(&wire[1..wire_len - 1], &mut buf).is_none());
    }
}
