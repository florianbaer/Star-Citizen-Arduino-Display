#pragma once
#include <lvgl.h>
#include <stdint.h>

enum class IndState : uint8_t { Hidden, Off, On, Transit };

struct IndicatorLabelConfig {
  int x = 4, y = 4;
  const char* text = "GEAR";
  uint8_t onR = 0, onG = 220, onB = 90;
};

/// Small text indicator: hidden / off (grey) / on (configured colour) / in transit (amber).
class IndicatorLabel {
public:
  IndicatorLabel() : _lbl(nullptr), _state(IndState::Hidden), _first(true) {}

  void create(lv_obj_t* parent, const IndicatorLabelConfig& cfg) {
    _cfg = cfg;
    _lbl = lv_label_create(parent);
    lv_obj_set_style_text_font(_lbl, &lv_font_montserrat_14, 0);
    lv_label_set_text(_lbl, cfg.text);
    lv_obj_set_pos(_lbl, cfg.x, cfg.y);
    _first = true;
    setState(IndState::Hidden);
  }

  /// Change-only: returns immediately when the state is unchanged.
  void setState(IndState s) {
    if (!_first && s == _state) return;
    _first = false;
    _state = s;
    switch (s) {
      case IndState::Hidden:
        lv_obj_add_flag(_lbl, LV_OBJ_FLAG_HIDDEN);
        return;
      case IndState::Off:
        lv_obj_set_style_text_color(_lbl, lv_color_make(90, 90, 100), 0);
        break;
      case IndState::On:
        lv_obj_set_style_text_color(_lbl, lv_color_make(_cfg.onR, _cfg.onG, _cfg.onB), 0);
        break;
      case IndState::Transit:
        lv_obj_set_style_text_color(_lbl, lv_color_make(255, 180, 0), 0);
        break;
    }
    lv_obj_remove_flag(_lbl, LV_OBJ_FLAG_HIDDEN);
  }

private:
  IndicatorLabelConfig _cfg;
  lv_obj_t* _lbl;
  IndState _state;
  bool _first;
};
