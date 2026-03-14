#pragma once
#include <lvgl.h>
#include <math.h>

struct GyroHorizonConfig {
  int cx = 160;
  int cy = 105;
  int radius = 50;
};

class GyroHorizon {
public:
  GyroHorizon() : _canvas(nullptr), _label(nullptr), _cbuf(nullptr) {}

  void create(lv_obj_t* parent, const GyroHorizonConfig& cfg) {
    _cfg = cfg;
    int d = cfg.radius * 2;

    // Allocate canvas buffer (RGB565 = 2 bytes per pixel)
    _cbuf = new uint8_t[d * d * 2];

    _canvas = lv_canvas_create(parent);
    lv_canvas_set_buffer(_canvas, _cbuf, d, d, LV_COLOR_FORMAT_RGB565);
    lv_obj_set_pos(_canvas, cfg.cx - cfg.radius, cfg.cy - cfg.radius);
    lv_obj_remove_flag(_canvas, LV_OBJ_FLAG_CLICKABLE);

    // Heading label below the horizon circle
    _label = lv_label_create(parent);
    lv_obj_set_style_text_color(_label, lv_color_make(0, 200, 0), 0);
    lv_obj_set_style_text_font(_label, &lv_font_montserrat_14, 0);
    lv_label_set_text(_label, "HDG 000");
    lv_obj_set_pos(_label, cfg.cx - 28, cfg.cy + cfg.radius + 4);

    // Mode label above
    _modeLabel = lv_label_create(parent);
    lv_obj_set_style_text_color(_modeLabel, lv_color_make(100, 100, 100), 0);
    lv_obj_set_style_text_font(_modeLabel, &lv_font_montserrat_10, 0);
    lv_label_set_text(_modeLabel, "MSFS GYRO");
    lv_obj_set_pos(_modeLabel, cfg.cx - 25, cfg.cy - cfg.radius - 16);

    draw(0, 0, 0);
  }

  void setValue(int16_t pitch, int16_t roll, int16_t heading) {
    // Throttle redraws: only if changed by > 5 tenths (0.5 deg)
    if (abs(pitch - _pitch) < 5 && abs(roll - _roll) < 5 && abs(heading - _heading) < 5) {
      return;
    }
    _pitch = pitch;
    _roll = roll;
    _heading = heading;
    draw(pitch, roll, heading);

    // Update heading label
    int hdg = ((heading / 10) % 360 + 360) % 360;
    char buf[16];
    snprintf(buf, sizeof(buf), "HDG %03d", hdg);
    lv_label_set_text(_label, buf);
  }

private:
  GyroHorizonConfig _cfg;
  lv_obj_t* _canvas;
  lv_obj_t* _label;
  lv_obj_t* _modeLabel;
  uint8_t* _cbuf;
  int16_t _pitch = 0, _roll = 0, _heading = 0;

  static constexpr float DEG2RAD = 3.14159265f / 180.0f;
  static constexpr float PIX_PER_DEG = 1.5f; // pixels per degree of pitch

  void draw(int16_t pitch_t, int16_t roll_t, int16_t /*heading_t*/) {
    int d = _cfg.radius * 2;
    int r = _cfg.radius;
    float pitch_deg = pitch_t / 10.0f;
    float roll_rad = (roll_t / 10.0f) * DEG2RAD;

    float sin_r = sinf(roll_rad);
    float cos_r = cosf(roll_rad);
    float pitch_offset = pitch_deg * PIX_PER_DEG;

    // Sky and ground colors (RGB565)
    uint16_t sky = rgb565(0x20, 0x60, 0xC0);
    uint16_t gnd = rgb565(0x6B, 0x34, 0x10);
    uint16_t border = rgb565(0x40, 0x40, 0x40);

    uint16_t* px = (uint16_t*)_cbuf;
    int r2 = r * r;

    for (int y = 0; y < d; y++) {
      for (int x = 0; x < d; x++) {
        int dx = x - r;
        int dy = y - r;
        int dist2 = dx * dx + dy * dy;

        if (dist2 > r2) {
          // Outside circle: black
          px[y * d + x] = 0x0000;
        } else if (dist2 > (r - 2) * (r - 2)) {
          // Circle border
          px[y * d + x] = border;
        } else {
          // Rotate point by -roll, then check if above/below horizon
          // Horizon is at y_offset = pitch_offset from center
          float ry = -dx * sin_r + dy * cos_r;
          px[y * d + x] = (ry < pitch_offset) ? sky : gnd;
        }
      }
    }

    // Draw horizon line (white, through center offset by pitch, rotated by roll)
    drawHorizonLine(px, d, r, pitch_offset, sin_r, cos_r);

    // Draw aircraft reference mark (yellow crosshair at center)
    drawReferenceMark(px, d, r);

    // Draw pitch ladder lines (small tick marks at +/-10, +/-20 degrees)
    drawPitchLadder(px, d, r, pitch_offset, sin_r, cos_r);

    lv_obj_invalidate(_canvas);
  }

  void drawHorizonLine(uint16_t* px, int d, int r, float pitch_offset, float sin_r, float cos_r) {
    uint16_t white = rgb565(255, 255, 255);
    int r2inner = (r - 3) * (r - 3);

    for (int t = -r; t < r; t++) {
      // Line along roll-rotated x-axis, offset by pitch
      float fx = t * cos_r;
      float fy = t * sin_r + pitch_offset * cos_r;
      // Adjust for roll on pitch offset
      float px_x = fx - pitch_offset * sin_r * 0; // simplified
      // Actually: the horizon line in screen coords
      // Point on horizon in rotated frame: (t, pitch_offset)
      // Screen coords: x = t*cos_r - pitch_offset*sin_r, y = t*sin_r + pitch_offset*cos_r
      int sx = r + (int)(t * cos_r + pitch_offset * sin_r);
      int sy = r + (int)(t * sin_r - pitch_offset * cos_r);

      for (int w = -1; w <= 1; w++) {
        int py = sy + w;
        if (sx >= 0 && sx < d && py >= 0 && py < d) {
          int dx = sx - r;
          int dy = py - r;
          if (dx * dx + dy * dy < r2inner) {
            px[py * d + sx] = white;
          }
        }
      }
    }
  }

  void drawPitchLadder(uint16_t* px, int d, int r, float pitch_offset, float sin_r, float cos_r) {
    uint16_t white = rgb565(200, 200, 200);
    int r2inner = (r - 4) * (r - 4);
    int tickHalf = 8; // half-width of tick marks

    for (int deg = -20; deg <= 20; deg += 10) {
      if (deg == 0) continue;
      float lineY = pitch_offset - deg * PIX_PER_DEG;

      for (int t = -tickHalf; t <= tickHalf; t++) {
        int sx = r + (int)(t * cos_r + lineY * sin_r);
        int sy = r + (int)(t * sin_r - lineY * cos_r);

        if (sx >= 0 && sx < d && sy >= 0 && sy < d) {
          int dx = sx - r;
          int dy = sy - r;
          if (dx * dx + dy * dy < r2inner) {
            px[sy * d + sx] = white;
          }
        }
      }
    }
  }

  void drawReferenceMark(uint16_t* px, int d, int r) {
    uint16_t yellow = rgb565(255, 220, 0);

    // Horizontal bars (aircraft wings)
    for (int x = -15; x <= -5; x++) {
      px[r * d + (r + x)] = yellow;
      px[(r + 1) * d + (r + x)] = yellow;
    }
    for (int x = 5; x <= 15; x++) {
      px[r * d + (r + x)] = yellow;
      px[(r + 1) * d + (r + x)] = yellow;
    }
    // Center dot
    for (int dy = -1; dy <= 1; dy++) {
      for (int dx = -1; dx <= 1; dx++) {
        px[(r + dy) * d + (r + dx)] = yellow;
      }
    }
  }

  static uint16_t rgb565(uint8_t r, uint8_t g, uint8_t b) {
    return ((r & 0xF8) << 8) | ((g & 0xFC) << 3) | (b >> 3);
  }
};
