// Host stub: no touch controller present (endTransmission fails -> no touch).
#pragma once
#include <stdint.h>
class SimWire {
public:
  void begin(int, int) {}
  void beginTransmission(uint8_t) {}
  void write(uint8_t) {}
  int endTransmission() { return 1; }
  void requestFrom(uint8_t, uint8_t) {}
  int available() { return 0; }
  int read() { return 0; }
};
extern SimWire Wire;
