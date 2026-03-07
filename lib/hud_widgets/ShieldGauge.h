#pragma once
#include <lvgl.h>
#include "ColorScale.h"

enum class ShieldDir { FWD = 0, STB = 1, AFT = 2, PRT = 3 };

struct ShieldGaugeConfig {
  int cx = 160;
  int cy = 85;
  ShieldDir direction = ShieldDir::FWD;
  int radius = 70;
  int width = 10;
  int maxSpan = 78;
  uint8_t trackR = 30, trackG = 30, trackB = 40;
};

class ShieldGauge {
public:
  ShieldGauge() : _parent(nullptr) {}

  void create(lv_obj_t* parent, const ShieldGaugeConfig& cfg) {
    _parent = parent;
    _cfg = cfg;
    _halfSpan = cfg.maxSpan / 2;

    int rotation = dirRotation(cfg.direction);
    int size = cfg.radius * 2;
    lv_color_t trackColor = lv_color_make(cfg.trackR, cfg.trackG, cfg.trackB);
    lv_color_t initColor = lv_color_make(0, 180, 255);

    _bg = createArc(size, cfg.width, rotation, 0, cfg.maxSpan, trackColor);
    _sh = createArc(size, cfg.width, rotation, _halfSpan, _halfSpan + 1, initColor);
  }

  void setValue(uint8_t val) {
    ensureColors();
    int half = ((int)val * _halfSpan) / 255;
    int start = _halfSpan - half;
    int end   = _halfSpan + half;
    if (end <= start) end = start + 1;

    lv_arc_set_bg_angles(_sh, start, end);

    lv_color_t c = _colors.get(val);
    lv_obj_set_style_arc_color(_sh, c, LV_PART_MAIN);

    _value = val;
  }

  uint8_t value() const { return _value; }

  void setColorScale(const ColorScale& cs) { _colors = cs; }

private:
  lv_obj_t* _parent;
  ShieldGaugeConfig _cfg;
  int _halfSpan;
  uint8_t _value = 0;

  lv_obj_t* _bg;
  lv_obj_t* _sh;

  ColorScale _colors;
  bool _colorsInit = false;

  void ensureColors() {
    if (!_colorsInit) {
      static const ColorThreshold t[] = {
        {170, 0, 180, 255},
        {85,  255, 200, 0},
        {0,   255, 30, 0}
      };
      _colors.set(t, 3);
      _colorsInit = true;
    }
  }

  static int dirRotation(ShieldDir d) {
    switch (d) {
      case ShieldDir::FWD: return 231;
      case ShieldDir::STB: return 321;
      case ShieldDir::AFT: return 51;
      case ShieldDir::PRT: return 141;
    }
    return 0;
  }

  lv_obj_t* createArc(int size, int width, int rotation, int startAng, int endAng, lv_color_t color) {
    lv_obj_t* arc = lv_arc_create(_parent);
    lv_obj_set_size(arc, size, size);
    lv_obj_set_pos(arc, _cfg.cx - size / 2, _cfg.cy - size / 2);
    lv_arc_set_rotation(arc, rotation);
    lv_arc_set_bg_angles(arc, startAng, endAng);
    lv_arc_set_range(arc, 0, 1);
    lv_arc_set_value(arc, 0);
    lv_obj_remove_flag(arc, LV_OBJ_FLAG_CLICKABLE);
    lv_obj_set_style_arc_color(arc, color, LV_PART_MAIN);
    lv_obj_set_style_arc_width(arc, width, LV_PART_MAIN);
    lv_obj_set_style_arc_rounded(arc, true, LV_PART_MAIN);
    lv_obj_set_style_arc_opa(arc, LV_OPA_TRANSP, LV_PART_INDICATOR);
    lv_obj_set_style_arc_width(arc, 0, LV_PART_INDICATOR);
    lv_obj_set_style_bg_opa(arc, LV_OPA_TRANSP, LV_PART_KNOB);
    lv_obj_set_style_pad_all(arc, 0, LV_PART_KNOB);
    return arc;
  }
};
