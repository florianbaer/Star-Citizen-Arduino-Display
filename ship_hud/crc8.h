#pragma once
#include <stdint.h>
#include <stddef.h>

/// CRC8/MAXIM (polynomial 0x31, init 0x00).
/// Must match the Rust implementation in proto/src/crc8.rs.
static inline uint8_t crc8(const uint8_t* data, size_t len) {
  uint8_t crc = 0x00;
  for (size_t i = 0; i < len; i++) {
    uint8_t b = crc ^ data[i];
    // Bit-by-bit (no lookup table to save flash on ESP32)
    for (uint8_t bit = 0; bit < 8; bit++) {
      if (b & 0x80)
        b = (b << 1) ^ 0x31;
      else
        b <<= 1;
    }
    crc = b;
  }
  return crc;
}
