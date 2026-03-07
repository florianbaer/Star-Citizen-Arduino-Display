#pragma once
#include <lvgl.h>

struct ShipSilhouetteConfig {
  int cx = 160;
  int cy = 85;
  float scale = 1.0f;
  uint8_t r = 0, g = 80, b = 80;
  int lineWidth = 2;
};

class ShipSilhouette {
public:
  ShipSilhouette() : _line(nullptr) {}

  void create(lv_obj_t* parent, const ShipSilhouetteConfig& cfg) {
    int cx = cfg.cx;
    int cy = cfg.cy;
    float s = cfg.scale;

    _pts[0] = {(lv_value_precise_t)(cx),              (lv_value_precise_t)(cy - 20 * s)};
    _pts[1] = {(lv_value_precise_t)(cx + 12 * s),     (lv_value_precise_t)(cy + 6 * s)};
    _pts[2] = {(lv_value_precise_t)(cx + 5 * s),      (lv_value_precise_t)(cy + 18 * s)};
    _pts[3] = {(lv_value_precise_t)(cx - 5 * s),      (lv_value_precise_t)(cy + 18 * s)};
    _pts[4] = {(lv_value_precise_t)(cx - 12 * s),     (lv_value_precise_t)(cy + 6 * s)};
    _pts[5] = {(lv_value_precise_t)(cx),              (lv_value_precise_t)(cy - 20 * s)};

    _line = lv_line_create(parent);
    lv_line_set_points(_line, _pts, 6);
    lv_obj_set_style_line_color(_line, lv_color_make(cfg.r, cfg.g, cfg.b), 0);
    lv_obj_set_style_line_width(_line, cfg.lineWidth, 0);
    lv_obj_set_style_line_rounded(_line, true, 0);
  }

private:
  lv_obj_t* _line;
  lv_point_precise_t _pts[6];
};
