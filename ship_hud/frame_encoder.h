#pragma once
#include <stdint.h>
#include <stddef.h>
#include <string.h>
#include "cobs.h"
#include "crc8.h"
#include "messages.h"

/// Safe upper bound for an encoded frame (the true worst case is 69; Rust MAX_FRAME is 69).
#define HUD_MAX_FRAME (HUD_MAX_PAYLOAD + 2 + 1 + 2 + 1)

/// Builds `0x00 | COBS(type | payload | crc8) | 0x00` into `out`.
/// Returns the total length, or 0 on overflow / oversize payload.
static inline size_t hud_frame_encode(uint8_t type, const uint8_t* payload, size_t len, uint8_t* out, size_t cap) {
  if (len > HUD_MAX_PAYLOAD) return 0;
  uint8_t raw[HUD_MAX_PAYLOAD + 2];
  raw[0] = type;
  if (len > 0) memcpy(&raw[1], payload, len);
  raw[1 + len] = crc8(raw, 1 + len);
  size_t raw_len = len + 2;
  if (cap < 2) return 0;
  out[0] = 0x00;
  size_t n = cobs_encode(raw, raw_len, out + 1, cap - 1);
  if (n == 0 || 1 + n + 1 > cap) return 0;
  out[1 + n] = 0x00;
  return n + 2;
}
