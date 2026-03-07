#pragma once
#include <lvgl.h>

struct ColorThreshold {
  uint8_t above;
  uint8_t r, g, b;
};

class ColorScale {
public:
  ColorScale() : _count(0) {}

  template <int N>
  ColorScale(const ColorThreshold (&thresholds)[N]) : _count(N) {
    for (int i = 0; i < N && i < MAX_THRESHOLDS; i++)
      _thresholds[i] = thresholds[i];
  }

  void set(const ColorThreshold* thresholds, int count) {
    _count = count < MAX_THRESHOLDS ? count : MAX_THRESHOLDS;
    for (int i = 0; i < _count; i++)
      _thresholds[i] = thresholds[i];
  }

  lv_color_t get(uint8_t value) const {
    for (int i = 0; i < _count; i++) {
      if (value > _thresholds[i].above)
        return lv_color_make(_thresholds[i].r, _thresholds[i].g, _thresholds[i].b);
    }
    if (_count > 0) {
      const auto& last = _thresholds[_count - 1];
      return lv_color_make(last.r, last.g, last.b);
    }
    return lv_color_white();
  }

private:
  static const int MAX_THRESHOLDS = 8;
  ColorThreshold _thresholds[MAX_THRESHOLDS];
  int _count;
};
