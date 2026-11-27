// Host stub of the Arduino API used by ship_hud.ino / hud_widgets (tools/hud-sim only).
#pragma once
#include <stdint.h>
#include <stddef.h>
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <vector>
#include <deque>

#define OUTPUT 1
#define INPUT 0
#define HIGH 1
#define LOW 0

extern uint32_t g_simMillis;
inline uint32_t millis() { return g_simMillis; }
inline void delay(uint32_t ms) { g_simMillis += ms; }
inline void pinMode(int, int) {}
inline void digitalWrite(int, int) {}
inline void ledcSetup(int, int, int) {}
inline void ledcAttachPin(int, int) {}
inline void ledcAttach(int, int, int) {}
inline void ledcWrite(int, int) {}

class SimSerial {
public:
  std::deque<uint8_t> rx;   // bytes "sent by the PC"
  void setRxBufferSize(size_t) {}
  void begin(unsigned long) {}
  void println(const char*) {}
  void print(const char*) {}
  int available() { return (int)rx.size(); }
  int read() { if (rx.empty()) return -1; uint8_t b = rx.front(); rx.pop_front(); return b; }
  size_t write(const uint8_t*, size_t n) { return n; }  // HUD -> PC: discarded
};
extern SimSerial Serial;
