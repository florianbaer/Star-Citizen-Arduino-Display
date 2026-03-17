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
MSG_ENGINE = 0x03
MSG_FLIGHT_DATA = 0x04
MSG_GFORCE = 0x05


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


def frame_engine(rpm: int, throttle: int, fuel_flow: int, oil_temp: int, oil_press: int) -> bytes:
    """Build a framed engine message ready for serial transmission.

    Args:
        rpm: Engine RPM (0-65535)
        throttle: Throttle percentage (0-100)
        fuel_flow: Fuel flow mapped 0-255
        oil_temp: Oil temperature mapped 0-255
        oil_press: Oil pressure mapped 0-255

    Returns:
        Complete wire frame: [0x00] [COBS-encoded: msg_type | payload | CRC8] [0x00]
    """
    raw = struct.pack("<BHBBBB", MSG_ENGINE, rpm, throttle, fuel_flow, oil_temp, oil_press)
    checksum = crc8(raw)
    encoded = cobs_encode(raw + bytes([checksum]))
    return b"\x00" + encoded + b"\x00"


def frame_flight_data(airspeed: int, altitude: int, vspeed: int, ground_speed: int) -> bytes:
    """Build a framed flight data message ready for serial transmission.

    Args:
        airspeed: Indicated airspeed in tenths of knots (0-65535)
        altitude: Altitude in feet (signed int32)
        vspeed: Vertical speed in fpm (signed int16)
        ground_speed: Ground speed in tenths of knots (0-65535)

    Returns:
        Complete wire frame: [0x00] [COBS-encoded: msg_type | payload | CRC8] [0x00]
    """
    raw = struct.pack("<BHihH", MSG_FLIGHT_DATA, airspeed, altitude, vspeed, ground_speed)
    checksum = crc8(raw)
    encoded = cobs_encode(raw + bytes([checksum]))
    return b"\x00" + encoded + b"\x00"


def frame_gforce(gx: int, gy: int, gz: int) -> bytes:
    """Build a framed G-force message ready for serial transmission.

    Args:
        gx: Longitudinal G in hundredths (-32768 to +32767)
        gy: Vertical G in hundredths (~100 = 1G level flight)
        gz: Lateral G in hundredths (-32768 to +32767)

    Returns:
        Complete wire frame: [0x00] [COBS-encoded: msg_type | payload | CRC8] [0x00]
    """
    raw = struct.pack("<Bhhh", MSG_GFORCE, gx, gy, gz)
    checksum = crc8(raw)
    encoded = cobs_encode(raw + bytes([checksum]))
    return b"\x00" + encoded + b"\x00"
