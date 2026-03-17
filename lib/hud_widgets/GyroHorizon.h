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
  GyroHorizon() : _canvas(nullptr), _label(nullptr), _modeLabel(nullptr), _cbuf(nullptr) {}

  ~GyroHorizon() { destroy(); }

  void destroy() {
    if (_cbuf) { delete[] _cbuf; _cbuf = nullptr; }
    // LVGL objects are freed when their parent screen is deleted
    _canvas = nullptr;
    _label = nullptr;
    _modeLabel = nullptr;
  }

  void create(lv_obj_t* parent, const GyroHorizonConfig& cfg) {
    if (cfg.radius < 10 || cfg.radius > 120) return;

    destroy(); // clean up any previous allocation
    _cfg = cfg;
    int d = cfg.radius * 2;

    _cbuf = new (std::nothrow) uint8_t[d * d * 2];
    if (!_cbuf) return;

    _canvas = lv_canvas_create(parent);
    if (!_canvas) { destroy(); return; }
    lv_canvas_set_buffer(_canvas, _cbuf, d, d, LV_COLOR_FORMAT_RGB565);
    lv_obj_set_pos(_canvas, cfg.cx - cfg.radius, cfg.cy - cfg.radius);
    lv_obj_remove_flag(_canvas, LV_OBJ_FLAG_CLICKABLE);

    _label = lv_label_create(parent);
    lv_obj_set_style_text_color(_label, lv_color_make(0, 200, 0), 0);
    lv_obj_set_style_text_font(_label, &lv_font_montserrat_14, 0);
    lv_label_set_text(_label, "HDG 000");
    lv_obj_set_pos(_label, cfg.cx - 28, cfg.cy + cfg.radius + 4);

    _modeLabel = lv_label_create(parent);
    lv_obj_set_style_text_color(_modeLabel, lv_color_make(100, 100, 100), 0);
    lv_obj_set_style_text_font(_modeLabel, &lv_font_montserrat_10, 0);
    lv_label_set_text(_modeLabel, "MSFS GYRO");
    lv_obj_set_pos(_modeLabel, cfg.cx - 25, cfg.cy - cfg.radius - 16);

    draw(0, 0, 0);
  }

  void setValue(int16_t pitch, int16_t roll, int16_t heading) {
    if (!_canvas || !_cbuf) return;

    // Throttle redraws: only if changed by > 5 tenths (0.5 deg)
    if (abs(pitch - _pitch) < 5 && abs(roll - _roll) < 5 && abs(heading - _heading) < 5) {
      return;
    }
    _pitch = pitch;
    _roll = roll;
    _heading = heading;
    draw(pitch, roll, heading);

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

  // Pixels per degree of pitch — at radius=90, full ±60° range fills the circle
  float pixPerDeg() const { return _cfg.radius / 60.0f; }

  void draw(int16_t pitch_t, int16_t roll_t, int16_t /*heading_t*/) {
    int d = _cfg.radius * 2;
    int r = _cfg.radius;
    float pitch_deg = pitch_t / 10.0f;
    float roll_rad = (roll_t / 10.0f) * (M_PI / 180.0f);

    float sin_r = sinf(roll_rad);
    float cos_r = cosf(roll_rad);
    // Positive pitch = nose up = horizon moves DOWN on screen
    float ppd = pixPerDeg();
    float pitch_px = pitch_deg * ppd;
    // Clamp so we don't waste cycles on fully sky/ground frames
    float pitch_clamped = fmaxf(-r * 1.5f, fminf(r * 1.5f, pitch_px));

    uint16_t sky = rgb565(0x20, 0x60, 0xC0);
    uint16_t gnd = rgb565(0x6B, 0x34, 0x10);
    uint16_t border_c = rgb565(0x40, 0x40, 0x40);

    uint16_t* px = (uint16_t*)_cbuf;
    int r2 = r * r;
    int r2border = (r - 2) * (r - 2);

    // For each pixel, rotate into aircraft frame and check against horizon.
    // In aircraft frame: horizon is at y = pitch_px (positive = below center).
    // Rotation from screen to aircraft frame (rotate by -roll):
    //   ax = dx * cos_r + dy * sin_r
    //   ay = -dx * sin_r + dy * cos_r
    // Pixel is sky if ay < pitch_px, ground otherwise.
    for (int y = 0; y < d; y++) {
      for (int x = 0; x < d; x++) {
        int dx = x - r;
        int dy = y - r;
        int dist2 = dx * dx + dy * dy;

        if (dist2 > r2) {
          px[y * d + x] = 0x0000; // outside circle: black
        } else if (dist2 > r2border) {
          px[y * d + x] = border_c;
        } else {
          // Rotate screen point into aircraft frame
          float ay = -dx * sin_r + dy * cos_r;
          px[y * d + x] = (ay < pitch_clamped) ? sky : gnd;
        }
      }
    }

    drawHorizonLine(px, d, r, pitch_clamped, sin_r, cos_r);
    drawReferenceMark(px, d, r);
    drawPitchLadder(px, d, r, pitch_clamped, sin_r, cos_r, ppd);

    lv_obj_invalidate(_canvas);
  }

  // Draw the horizon line: a horizontal line at y=offset in the aircraft frame,
  // transformed back to screen coordinates.
  // Aircraft frame point (t, offset) -> screen: sx = t*cos - offset*sin, sy = t*sin + offset*cos
  void drawHorizonLine(uint16_t* px, int d, int r, float offset, float sin_r, float cos_r) {
    uint16_t white = rgb565(255, 255, 255);
    int r2inner = (r - 3) * (r - 3);

    for (int t = -r; t < r; t++) {
      int sx = r + (int)(t * cos_r - offset * sin_r);
      int sy = r + (int)(t * sin_r + offset * cos_r);

      // Draw 3px wide line perpendicular to the line direction
      for (int w = -1; w <= 1; w++) {
        int py = sy + w;
        if (sx >= 0 && sx < d && py >= 0 && py < d) {
          int ddx = sx - r;
          int ddy = py - r;
          if (ddx * ddx + ddy * ddy < r2inner) {
            px[py * d + sx] = white;
          }
        }
      }
    }
  }

  void drawPitchLadder(uint16_t* px, int d, int r, float pitch_px,
                       float sin_r, float cos_r, float ppd) {
    uint16_t white = rgb565(200, 200, 200);
    int r2inner = (r - 4) * (r - 4);
    int tickHalf = r / 6; // scale tick width with radius

    for (int deg = -20; deg <= 20; deg += 10) {
      if (deg == 0) continue;
      // Offset of this pitch line in aircraft frame
      float lineOffset = pitch_px - deg * ppd;

      for (int t = -tickHalf; t <= tickHalf; t++) {
        int sx = r + (int)(t * cos_r - lineOffset * sin_r);
        int sy = r + (int)(t * sin_r + lineOffset * cos_r);

        if (sx >= 0 && sx < d && sy >= 0 && sy < d) {
          int ddx = sx - r;
          int ddy = sy - r;
          if (ddx * ddx + ddy * ddy < r2inner) {
            px[sy * d + sx] = white;
          }
        }
      }
    }
  }

  void drawReferenceMark(uint16_t* px, int d, int r) {
    uint16_t yellow = rgb565(255, 220, 0);
    // Scale wing bars with radius
    int wingOuter = r / 3;
    int wingInner = r / 9;

    // Left wing
    for (int x = -wingOuter; x <= -wingInner; x++) {
      setPixSafe(px, d, r, r + x, r, yellow);
      setPixSafe(px, d, r, r + x, r + 1, yellow);
    }
    // Right wing
    for (int x = wingInner; x <= wingOuter; x++) {
      setPixSafe(px, d, r, r + x, r, yellow);
      setPixSafe(px, d, r, r + x, r + 1, yellow);
    }
    // Center dot
    for (int dy = -1; dy <= 1; dy++) {
      for (int dx = -1; dx <= 1; dx++) {
        setPixSafe(px, d, r, r + dx, r + dy, yellow);
      }
    }
  }

  static void setPixSafe(uint16_t* px, int d, int r, int x, int y, uint16_t color) {
    if (x >= 0 && x < d && y >= 0 && y < d) {
      px[y * d + x] = color;
    }
  }

  static uint16_t rgb565(uint8_t r, uint8_t g, uint8_t b) {
    return ((r & 0xF8) << 8) | ((g & 0xFC) << 3) | (b >> 3);
  }
};
