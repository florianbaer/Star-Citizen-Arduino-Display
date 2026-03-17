#pragma once
#include <stdint.h>

enum MsgType : uint8_t {
  MSG_TELEMETRY   = 0x01,
  MSG_ATTITUDE    = 0x02,
  MSG_ENGINE      = 0x03,
  MSG_FLIGHT_DATA = 0x04,
  MSG_GFORCE      = 0x05,
};

/// Telemetry message payload (PC -> ESP32).
struct __attribute__((packed)) TelemetryMsg {
  uint8_t shield_front;
  uint8_t shield_back;
  uint8_t shield_left;
  uint8_t shield_right;
  uint8_t hydrogen_fuel;
  uint8_t quantum_fuel;
};

/// Attitude message payload (PC -> ESP32).
struct __attribute__((packed)) AttitudeMsg {
  int16_t pitch;    // tenths of degrees, -1800..+1800
  int16_t roll;     // tenths of degrees, -1800..+1800
  int16_t heading;  // tenths of degrees, 0..3599
};

/// Engine gauges message payload (PC -> ESP32).
struct __attribute__((packed)) EngineMsg {
  uint16_t rpm;
  uint8_t throttle;   // 0-100 %
  uint8_t fuel_flow;  // 0-255 mapped
  uint8_t oil_temp;   // 0-255 mapped
  uint8_t oil_press;  // 0-255 mapped
};

/// Flight data message payload (PC -> ESP32).
struct __attribute__((packed)) FlightDataMsg {
  uint16_t airspeed;     // tenths of knots
  int32_t altitude;      // feet
  int16_t vspeed;        // fpm
  uint16_t ground_speed; // tenths of knots
};

/// G-force message payload (PC -> ESP32).
struct __attribute__((packed)) GForceMsg {
  int16_t gforce_x;  // hundredths of G (longitudinal)
  int16_t gforce_y;  // hundredths of G (vertical, ~100 = 1G level)
  int16_t gforce_z;  // hundredths of G (lateral)
};
