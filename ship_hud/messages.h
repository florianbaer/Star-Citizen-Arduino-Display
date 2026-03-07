#pragma once
#include <stdint.h>

enum MsgType : uint8_t {
  MSG_TELEMETRY = 0x01,
};

/// Telemetry message payload (PC -> ESP32).
/// Layout must match Rust `TelemetryMsg` in proto/src/messages.rs.
struct __attribute__((packed)) TelemetryMsg {
  uint8_t shield_front;
  uint8_t shield_back;
  uint8_t shield_left;
  uint8_t shield_right;
  uint8_t hydrogen_fuel;
  uint8_t quantum_fuel;
};
