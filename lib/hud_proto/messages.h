#pragma once
#include <stdint.h>

enum MsgType : uint8_t {
  MSG_TELEMETRY = 0x01,
  MSG_ATTITUDE  = 0x02,
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

/// Attitude message payload (PC -> ESP32).
/// Layout must match Rust `AttitudeMsg` in proto/src/messages.rs.
struct __attribute__((packed)) AttitudeMsg {
  int16_t pitch;    // tenths of degrees, -1800..+1800
  int16_t roll;     // tenths of degrees, -1800..+1800
  int16_t heading;  // tenths of degrees, 0..3599
};
