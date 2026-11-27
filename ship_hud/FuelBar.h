#pragma once
#include <lvgl.h>

struct FuelBarConfig {
  lv_align_t barAlign = LV_ALIGN_BOTTOM_RIGHT;
  int barOffsetX = -10;
  int barOffsetY = -36;
  lv_align_t labelAlign = LV_ALIGN_BOTTOM_LEFT;
  int labelOffsetX = 10;
  int labelOffsetY = -38;
  int width = 200;
  int height = 16;
  uint8_t r = 0, g = 180, b = 220;
  const char* label = "FUEL";
};

class FuelBar {
public:
  FuelBar() : _bar(nullptr), _lbl(nullptr) {}

  void create(lv_obj_t* parent, const FuelBarConfig& cfg) {
    _cfg = cfg;

    // Bar background style
    _bar = lv_bar_create(parent);
    lv_obj_set_size(_bar, cfg.width, cfg.height);
    lv_obj_align(_bar, cfg.barAlign, cfg.barOffsetX, cfg.barOffsetY);
    lv_bar_set_range(_bar, 0, 255);
    lv_bar_set_value(_bar, 0, LV_ANIM_OFF);
    lv_obj_set_style_bg_color(_bar, lv_color_make(40, 40, 40), LV_PART_MAIN);
    lv_obj_set_style_bg_opa(_bar, LV_OPA_COVER, LV_PART_MAIN);
    lv_obj_set_style_bg_color(_bar, lv_color_make(cfg.r, cfg.g, cfg.b), LV_PART_INDICATOR);

    // Label
    _lbl = lv_label_create(parent);
    lv_obj_set_style_text_color(_lbl, lv_color_make(180, 180, 200), 0);
    lv_obj_set_style_text_font(_lbl, &lv_font_montserrat_14, 0);
    lv_obj_align(_lbl, cfg.labelAlign, cfg.labelOffsetX, cfg.labelOffsetY);

    char buf[24];
    snprintf(buf, sizeof(buf), "%s 0%%", cfg.label);
    lv_label_set_text(_lbl, buf);
  }

  /// No data: empty bar and "--" label. Change-only.
  void setUnknown() {
    if (_last == -2) return;
    lv_bar_set_value(_bar, 0, LV_ANIM_OFF);
    char buf[24];
    snprintf(buf, sizeof(buf), "%s --", _cfg.label);
    lv_label_set_text(_lbl, buf);
    _last = -2;
  }

  void setValue(uint8_t val) {
    if ((int)val == _last) return;  // change-only: LVGL invalidates on every setter call
    _last = val;
    lv_bar_set_value(_bar, val, LV_ANIM_OFF);  // smoothing happens upstream (Smoother)
    char buf[24];
    int pct = (val * 100) / 255;
    snprintf(buf, sizeof(buf), "%s %d%%", _cfg.label, pct);
    lv_label_set_text(_lbl, buf);
  }

private:
  FuelBarConfig _cfg;
  lv_obj_t* _bar;
  lv_obj_t* _lbl;
  int _last = -1;  // -1 never, -2 unknown, 0..255 value
};
