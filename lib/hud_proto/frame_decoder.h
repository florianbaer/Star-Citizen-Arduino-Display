#pragma once
#include <stdint.h>
#include <string.h>
#include "cobs.h"
#include "crc8.h"
#include "messages.h"

class FrameDecoder {
public:
  FrameDecoder() : _pos(0), _synced(false), _ready(false), _msgType(0), _payloadLen(0) {}

  /// Feed one byte from the serial stream.
  void feed(uint8_t byte) {
    if (byte == 0x00) {
      if (_synced && _pos > 0) {
        decode();
      }
      _pos = 0;
      _synced = true;
      return;
    }
    if (_synced && _pos < BUF_SIZE) {
      _buf[_pos++] = byte;
    }
  }

  /// Returns true when a valid, CRC-checked frame is available.
  bool available() const { return _ready; }

  /// Message type of the last decoded frame.
  uint8_t msgType() const { return _msgType; }

  /// Copy payload into `out`. Returns payload length, or 0 if no frame ready.
  int payload(uint8_t* out, int max_len) const {
    if (!_ready) return 0;
    int n = _payloadLen < max_len ? _payloadLen : max_len;
    memcpy(out, _payload, n);
    return n;
  }

  /// Clear the ready flag after consuming the frame.
  void clear() { _ready = false; }

private:
  static const int BUF_SIZE = 72;
  static const int DECODE_SIZE = 68;

  uint8_t _buf[BUF_SIZE];
  int _pos;
  bool _synced;

  bool _ready;
  uint8_t _msgType;
  uint8_t _payload[DECODE_SIZE];
  int _payloadLen;

  void decode() {
    uint8_t decoded[DECODE_SIZE];
    int len = cobs_decode(_buf, _pos, decoded, DECODE_SIZE);
    if (len < 2) return; // need msg_type + crc at minimum

    // Verify CRC8
    uint8_t expected_crc = decoded[len - 1];
    uint8_t actual_crc = crc8(decoded, len - 1);
    if (expected_crc != actual_crc) return;

    _msgType = decoded[0];
    _payloadLen = len - 2; // exclude msg_type and crc
    if (_payloadLen > 0) {
      memcpy(_payload, &decoded[1], _payloadLen);
    }
    _ready = true;
  }
};
