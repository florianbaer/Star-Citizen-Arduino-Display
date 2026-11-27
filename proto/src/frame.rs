use crate::cobs;
use crate::crc8::crc8;
use crate::messages::{Message, MsgType};

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
    debug_assert!(payload.len() <= MAX_PAYLOAD);
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

/// Encode a typed message into a full wire frame. Returns bytes written.
pub fn frame_message<M: Message>(msg: &M, out: &mut [u8]) -> usize {
    let mut payload = [0u8; MAX_PAYLOAD];
    let n = msg.encode(&mut payload);
    frame(M::TYPE, &payload[..n], out)
}

/// Decoded frame result. Unknown message types are passed through (forward compatibility).
pub struct DeframedMsg<'a> {
    pub msg_type: u8,
    pub payload: &'a [u8],
}

impl DeframedMsg<'_> {
    pub fn kind(&self) -> Option<MsgType> {
        MsgType::from_u8(self.msg_type)
    }
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

    Some(DeframedMsg {
        msg_type: buf[0],
        payload: &buf[1..payload_end],
    })
}

/// Maximum encoded bytes (between the delimiters) the stream decoder buffers.
pub const MAX_ENCODED: usize = 72;

/// Byte-at-a-time stream decoder; mirrors `lib/hud_proto/frame_decoder.h`.
pub struct FrameDecoder {
    buf: [u8; MAX_ENCODED],
    pos: usize,
    synced: bool,
    overflow: bool,
    scratch: [u8; MAX_PAYLOAD + 2],
}

impl FrameDecoder {
    pub const fn new() -> Self {
        Self {
            buf: [0; MAX_ENCODED],
            pos: 0,
            synced: false,
            overflow: false,
            scratch: [0; MAX_PAYLOAD + 2],
        }
    }

    /// Feed one byte. On 0x00: if synced && pos > 0 && !overflow, try deframe(buf[..pos]).
    /// Then reset pos/overflow and set synced = true. Bytes before the first 0x00 are discarded.
    /// A non-zero byte while synced: append if pos < MAX_ENCODED, else set overflow = true.
    pub fn feed(&mut self, byte: u8) -> Option<DeframedMsg<'_>> {
        if byte == 0x00 {
            let n = self.pos;
            let try_decode = self.synced && n > 0 && !self.overflow;
            self.pos = 0;
            self.overflow = false;
            self.synced = true;
            if try_decode {
                return deframe(&self.buf[..n], &mut self.scratch);
            }
            return None;
        }
        if self.synced {
            if self.pos < MAX_ENCODED {
                self.buf[self.pos] = byte;
                self.pos += 1;
            } else {
                self.overflow = true;
            }
        }
        None
    }
}

impl Default for FrameDecoder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::{TelemetryMsg, TV_ALL};

    #[test]
    fn roundtrip_telemetry() {
        let msg = TelemetryMsg {
            shield_front: 255,
            shield_back: 128,
            shield_left: 64,
            shield_right: 32,
            hydrogen_fuel: 200,
            quantum_fuel: 100,
            valid: TV_ALL,
        };
        let mut wire = [0u8; MAX_FRAME];
        let wire_len = frame_message(&msg, &mut wire);
        assert_eq!(wire[0], 0x00);
        assert_eq!(wire[wire_len - 1], 0x00);
        let mut buf = [0u8; MAX_PAYLOAD + 2];
        let result = deframe(&wire[1..wire_len - 1], &mut buf).unwrap();
        assert_eq!(result.kind(), Some(MsgType::Telemetry));
        assert_eq!(TelemetryMsg::decode(result.payload).unwrap(), msg);
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
            valid: TV_ALL,
        };
        let mut wire = [0u8; MAX_FRAME];
        let wire_len = frame_message(&msg, &mut wire);
        wire[3] ^= 0xFF;
        let mut buf = [0u8; MAX_PAYLOAD + 2];
        assert!(deframe(&wire[1..wire_len - 1], &mut buf).is_none());
    }
}
