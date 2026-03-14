"""COBS framing and CRC8 protocol matching the Rust hud-proto crate."""

import struct


def _build_crc8_table() -> list[int]:
    """Build CRC8/MAXIM lookup table (polynomial 0x31, init 0x00)."""
    table = []
    for i in range(256):
        crc = i
        for _ in range(8):
            if crc & 0x80:
                crc = ((crc << 1) ^ 0x31) & 0xFF
            else:
                crc = (crc << 1) & 0xFF
        table.append(crc)
    return table


_CRC8_TABLE = _build_crc8_table()


def crc8(data: bytes) -> int:
    """Compute CRC8/MAXIM over data."""
    crc = 0x00
    for b in data:
        crc = _CRC8_TABLE[crc ^ b]
    return crc


def cobs_encode(data: bytes) -> bytes:
    """COBS-encode data (no zero bytes in output)."""
    out = bytearray(len(data) + len(data) // 254 + 2)
    code_idx = 0
    code = 1
    write_idx = 1

    for byte in data:
        if byte == 0x00:
            out[code_idx] = code
            code_idx = write_idx
            write_idx += 1
            code = 1
        else:
            out[write_idx] = byte
            write_idx += 1
            code += 1
            if code == 0xFF:
                out[code_idx] = code
                code_idx = write_idx
                write_idx += 1
                code = 1

    out[code_idx] = code
    return bytes(out[:write_idx])


def cobs_decode(data: bytes) -> bytes:
    """COBS-decode data. Raises ValueError on invalid input."""
    out = bytearray()
    idx = 0
    while idx < len(data):
        code = data[idx]
        if code == 0:
            raise ValueError("Zero byte in COBS data")
        idx += 1
        for _ in range(1, code):
            if idx >= len(data):
                raise ValueError("Truncated COBS data")
            out.append(data[idx])
            idx += 1
        if code < 0xFF and idx < len(data):
            out.append(0x00)
    return bytes(out)


MSG_ATTITUDE = 0x02


def frame_attitude(pitch_tenths: int, roll_tenths: int, heading_tenths: int) -> bytes:
    """Build a framed attitude message ready for serial transmission.

    Args:
        pitch_tenths: Pitch in tenths of degrees (-1800 to +1800)
        roll_tenths: Roll in tenths of degrees (-1800 to +1800)
        heading_tenths: Heading in tenths of degrees (0 to 3599)

    Returns:
        Complete wire frame: [0x00] [COBS-encoded: msg_type | payload | CRC8] [0x00]
    """
    # msg_type + payload (3x int16 LE)
    raw = struct.pack("<Bhhh", MSG_ATTITUDE, pitch_tenths, roll_tenths, heading_tenths)
    checksum = crc8(raw)
    raw_with_crc = raw + bytes([checksum])
    encoded = cobs_encode(raw_with_crc)
    return b"\x00" + encoded + b"\x00"
