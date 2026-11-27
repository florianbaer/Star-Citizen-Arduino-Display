// Host stub: captures pushed RGB565 pixels into a 320x240 framebuffer (rotation 1).
#pragma once
#include <stdint.h>
#define TFT_BLACK 0x0000
extern uint16_t g_fb[240][320];
class TFT_eSPI {
  int _x = 0, _y = 0, _w = 0, _h = 0;
public:
  void init() {}
  void setRotation(int) {}
  void fillScreen(uint16_t c) { for (int y = 0; y < 240; y++) for (int x = 0; x < 320; x++) g_fb[y][x] = c; }
  void startWrite() {}
  void endWrite() {}
  void setAddrWindow(int x, int y, int w, int h) { _x = x; _y = y; _w = w; _h = h; }
  void pushColors(uint16_t* px, uint32_t n, bool) {
    for (uint32_t i = 0; i < n; i++) {
      int x = _x + (int)(i % _w), y = _y + (int)(i / _w);
      if (x >= 0 && x < 320 && y >= 0 && y < 240) g_fb[y][x] = px[i];
    }
  }
};
