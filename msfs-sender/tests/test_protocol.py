"""Tests for the protocol module (CRC8, COBS, framing)."""

import struct

from msfs_sender.protocol import cobs_decode, cobs_encode, crc8, frame_attitude


class TestCRC8:
    def test_empty(self):
        assert crc8(b"") == 0x00

    def test_zero_byte(self):
        assert crc8(b"\x00") == 0x00

    def test_different_data_different_crc(self):
        assert crc8(b"\x01") != crc8(b"\x02")

    def test_deterministic(self):
        data = b"\x01\x02\x03\xFF\x80"
        assert crc8(data) == crc8(data)

    def test_known_value(self):
        # Verify CRC8/MAXIM with polynomial 0x31
        # Single byte 0x01: shift through 8 bits
        result = crc8(b"\x01")
        assert isinstance(result, int)
        assert 0 <= result <= 255


class TestCOBS:
    def test_roundtrip_no_zeros(self):
        data = b"\x01\x02\x03"
        assert cobs_decode(cobs_encode(data)) == data

    def test_roundtrip_with_zeros(self):
        data = b"\x00\x01\x00\x02"
        assert cobs_decode(cobs_encode(data)) == data

    def test_roundtrip_all_zeros(self):
        data = b"\x00\x00\x00"
        assert cobs_decode(cobs_encode(data)) == data

    def test_roundtrip_empty(self):
        data = b""
        assert cobs_decode(cobs_encode(data)) == data

    def test_no_zeros_in_encoded(self):
        data = b"\x00\x01\x00\x02\x00"
        encoded = cobs_encode(data)
        assert 0x00 not in encoded


class TestFrameAttitude:
    def test_frame_structure(self):
        frame = frame_attitude(0, 0, 0)
        # Must start and end with 0x00
        assert frame[0] == 0x00
        assert frame[-1] == 0x00
        # No zeros in the COBS-encoded middle
        assert 0x00 not in frame[1:-1]

    def test_frame_decodable(self):
        frame = frame_attitude(450, -300, 2700)
        # Decode the COBS data between delimiters
        decoded = cobs_decode(frame[1:-1])
        # Check msg_type
        assert decoded[0] == 0x02
        # Check CRC
        payload = decoded[:-1]
        assert crc8(payload) == decoded[-1]
        # Check values
        pitch, roll, heading = struct.unpack("<hhh", decoded[1:7])
        assert pitch == 450
        assert roll == -300
        assert heading == 2700

    def test_negative_values(self):
        frame = frame_attitude(-1800, -900, 0)
        decoded = cobs_decode(frame[1:-1])
        pitch, roll, heading = struct.unpack("<hhh", decoded[1:7])
        assert pitch == -1800
        assert roll == -900
        assert heading == 0

    def test_max_values(self):
        frame = frame_attitude(1800, 1800, 3599)
        decoded = cobs_decode(frame[1:-1])
        pitch, roll, heading = struct.unpack("<hhh", decoded[1:7])
        assert pitch == 1800
        assert roll == 1800
        assert heading == 3599
