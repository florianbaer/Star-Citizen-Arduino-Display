#pragma once
#include <math.h>
#include <stdint.h>

/// Exponential smoothing of a 0..255 value (no LVGL dependency).
class Smoother {
public:
  explicit Smoother(float tau_ms = 150.0f) : _cur(0), _tgt(0), _tau(tau_ms), _init(false), _shown(-1) {}

  /// First call (when not initialised) snaps the current value to `t`.
  void setTarget(uint8_t t) {
    if (!_init) {
      _cur = t;
      _init = true;
    }
    _tgt = t;
  }

  /// Jump straight to `v` (no sweep).
  void snap(uint8_t v) {
    _cur = _tgt = v;
    _init = true;
  }

  /// Advance by dt_ms. Returns true if value() differs from the last value returned by a tick that returned true.
  bool tick(uint32_t dt_ms) {
    float diff = _tgt - _cur;
    if (fabsf(diff) >= 0.5f) {
      _cur += diff * (1.0f - expf(-(float)dt_ms / _tau));
    }
    if (fabsf(_tgt - _cur) < 0.5f) _cur = _tgt;
    int v = value();
    if (v != _shown) {
      _shown = v;
      return true;
    }
    return false;
  }

  uint8_t value() const {
    long v = lroundf(_cur);
    if (v < 0) v = 0;
    if (v > 255) v = 255;
    return (uint8_t)v;
  }

  bool initialized() const { return _init; }

private:
  float _cur, _tgt, _tau;
  bool _init;
  int _shown;
};
