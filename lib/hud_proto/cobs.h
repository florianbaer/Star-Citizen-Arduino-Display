#pragma once
#include <stdint.h>
#include <stddef.h>

/// Decode COBS-encoded data. Returns decoded length, or -1 on error.
/// `src` is the COBS data (between 0x00 delimiters, NOT including them).
static inline int cobs_decode(const uint8_t* src, size_t src_len, uint8_t* dst, size_t dst_len) {
  size_t si = 0, di = 0;
  while (si < src_len) {
    uint8_t code = src[si++];
    if (code == 0) return -1;
    for (uint8_t i = 1; i < code; i++) {
      if (si >= src_len || di >= dst_len) return -1;
      dst[di++] = src[si++];
    }
    // Insert implicit zero between groups, but not after the last group
    if (code < 0xFF && si < src_len) {
      if (di >= dst_len) return -1;
      dst[di++] = 0x00;
    }
  }
  return (int)di;
}
