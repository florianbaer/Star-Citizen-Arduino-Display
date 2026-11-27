#pragma once
#include <lvgl.h>
#include <Arduino.h>
#include <esp_arduino_version.h>
#include "HudState.h"

struct AlertIndicatorConfig {
  int x = 153;  // centered for ~14px icon
  int y = 77;
  const char* symbol = LV_SYMBOL_WARNING;
  uint8_t r = 255, g = 30, b = 0;
  uint32_t blinkMs = 500;
  int ledPin = 4;        // red LED pin (-1 to disable)
  uint8_t ledChannel = 0;
};

// Heartbeat LED pattern (lub-dub), repeated 3 times with 1s gap, started when entering CRITICAL:
//   Step 0: 90% bright, 60ms  (lub)
//   Step 1: 10% dim,    40ms
//   Step 2: 70% bright, 60ms  (dub - slightly softer)
//   Step 3: fade 40%,   50ms  \
//   Step 4: fade 15%,   50ms   } fade out
//   Step 5: off,        50ms  /
//   ... 1s pause ...
//   (repeat 3x total)

class AlertIndicator {
public:
  AlertIndicator()
    : _lbl(nullptr), _sev(ALERT_NONE), _blink(false), _visible(false), _lastBlink(0),
      _wasCritical(false), _beatIndex(0), _stepIndex(0), _stepTime(0), _seqActive(false) {}

  void create(lv_obj_t* parent, const AlertIndicatorConfig& cfg) {
    _cfg = cfg;
    _lbl = lv_label_create(parent);
    lv_obj_set_style_text_color(_lbl, lv_color_make(cfg.r, cfg.g, cfg.b), 0);
    lv_obj_set_style_text_font(_lbl, &lv_font_montserrat_14, 0);
    lv_label_set_text(_lbl, cfg.symbol);
    lv_obj_set_pos(_lbl, cfg.x, cfg.y);
    lv_obj_add_flag(_lbl, LV_OBJ_FLAG_HIDDEN);

    if (cfg.ledPin >= 0) {
#if defined(ESP_ARDUINO_VERSION_MAJOR) && ESP_ARDUINO_VERSION_MAJOR >= 3
      ledcAttach(cfg.ledPin, 5000, 8);  // 3.x: channel picked automatically, keyed by pin
      ledcWrite(cfg.ledPin, 255);
#else
      ledcSetup(cfg.ledChannel, 5000, 8);
      ledcAttachPin(cfg.ledPin, cfg.ledChannel);
      ledcWrite(cfg.ledChannel, 255);
#endif
    }
  }

  /// Severity + blink. Called on every loop pass, so unchanged input does nothing (no LVGL calls).
  /// CRITICAL = configured red + heartbeat LED when entering; WARNING = amber, no LED; NONE hides the icon.
  /// blink == false -> icon steadily visible.
  void setSeverity(AlertSeverity s, bool blink) {
    if (s == ALERT_NONE) blink = false;
    if (s == _sev && blink == _blink) return;
    _sev = s;
    _blink = blink;
    if (s == ALERT_NONE) {
      lv_obj_add_flag(_lbl, LV_OBJ_FLAG_HIDDEN);
      _visible = false;
      return;
    }
    if (s == ALERT_CRITICAL)
      lv_obj_set_style_text_color(_lbl, lv_color_make(_cfg.r, _cfg.g, _cfg.b), 0);
    else
      lv_obj_set_style_text_color(_lbl, lv_color_make(255, 180, 0), 0);
    if (!blink) {
      lv_obj_remove_flag(_lbl, LV_OBJ_FLAG_HIDDEN);
      _visible = true;
    }
  }

  void setActive(bool active) { setSeverity(active ? ALERT_CRITICAL : ALERT_NONE, true); }

  void tick(uint32_t now) {
    // Trigger heartbeat on transition into CRITICAL
    bool critical = (_sev == ALERT_CRITICAL);
    if (critical && !_wasCritical) {
      _seqActive = true;
      _beatIndex = 0;
      _stepIndex = 0;
      _stepTime = now;
      ledSet(STEPS[0].duty);
    }
    _wasCritical = critical;

    // Run heartbeat sequence
    if (_seqActive && _cfg.ledPin >= 0) {
      uint32_t elapsed = now - _stepTime;

      if (_stepIndex < NUM_STEPS) {
        if (elapsed >= STEPS[_stepIndex].ms) {
          _stepTime = now;
          _stepIndex++;
          if (_stepIndex < NUM_STEPS)
            ledSet(STEPS[_stepIndex].duty);
          else
            ledSet(255);  // off after last step
        }
      } else {
        // Pause between beats
        if (elapsed >= 1000) {
          _beatIndex++;
          if (_beatIndex < 3) {
            _stepIndex = 0;
            _stepTime = now;
            ledSet(STEPS[0].duty);
          } else {
            _seqActive = false;
          }
        }
      }
    }

    // Blink warning icon
    if (_sev == ALERT_NONE || !_blink) return;
    if (now - _lastBlink >= _cfg.blinkMs) {
      _lastBlink = now;
      _visible = !_visible;
      if (_visible)
        lv_obj_remove_flag(_lbl, LV_OBJ_FLAG_HIDDEN);
      else
        lv_obj_add_flag(_lbl, LV_OBJ_FLAG_HIDDEN);
    }
  }

private:
  struct Step {
    uint8_t duty;   // active low: 0=full on, 255=off
    uint32_t ms;    // hold time
  };

  // Active low duty: brightness% -> 255 * (1 - pct)
  static constexpr Step STEPS[] = {
    { 25, 60},   // 90% - lub
    {230, 40},   // 10% - gap
    { 77, 60},   // 70% - dub (softer)
    {153, 50},   // 40% - fade
    {217, 50},   // 15% - fade
    {255, 50},   // off - fade out
  };
  static const int NUM_STEPS = 6;

  AlertIndicatorConfig _cfg;
  lv_obj_t* _lbl;
  AlertSeverity _sev;
  bool _blink;
  bool _visible;
  uint32_t _lastBlink;

  bool _wasCritical;
  bool _seqActive;
  int _beatIndex;    // 0..2 (3 heartbeats)
  int _stepIndex;    // 0..5 (steps within one beat)
  uint32_t _stepTime;

  void ledSet(uint8_t duty) {
    if (_cfg.ledPin < 0) return;
#if defined(ESP_ARDUINO_VERSION_MAJOR) && ESP_ARDUINO_VERSION_MAJOR >= 3
    ledcWrite(_cfg.ledPin, duty);
#else
    ledcWrite(_cfg.ledChannel, duty);
#endif
  }
};

constexpr AlertIndicator::Step AlertIndicator::STEPS[];
