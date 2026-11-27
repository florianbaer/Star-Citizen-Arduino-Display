#pragma once
#include <stdint.h>

// Pure HUD state logic (no LVGL / Arduino dependency) so it can be tested off-target.
#define HUD_SIGNAL_TIMEOUT_MS 2000

enum HudMode : uint8_t { HUD_MODE_NO_SIGNAL = 0, HUD_MODE_NO_GAME = 1, HUD_MODE_ON_FOOT = 2, HUD_MODE_VEHICLE = 3 };
enum AlertSeverity : uint8_t { ALERT_NONE = 0, ALERT_WARNING = 1, ALERT_CRITICAL = 2 };

/// No frame for more than HUD_SIGNAL_TIMEOUT_MS (unsigned subtraction) -> NO_SIGNAL; no Link message
/// seen (a v1 sender never sends Link) -> VEHICLE; else by link state (0 NO_GAME, 1 ON_FOOT, 2 VEHICLE, other VEHICLE).
static inline HudMode hudComputeMode(uint32_t now, uint32_t lastFrame, bool anyFrame, bool linkSeen, uint8_t linkState) {
  if (!anyFrame || (uint32_t)(now - lastFrame) > HUD_SIGNAL_TIMEOUT_MS) return HUD_MODE_NO_SIGNAL;
  if (!linkSeen) return HUD_MODE_VEHICLE;
  switch (linkState) {
    case 0: return HUD_MODE_NO_GAME;
    case 1: return HUD_MODE_ON_FOOT;
    default: return HUD_MODE_VEHICLE;
  }
}

/// With annunciator data (statusValid and any ann != 15): 5/6 critical, 3/4 warning, highest wins,
/// blink if any annunciator at that severity is FLASHING (3 or 5).
/// Otherwise legacy heuristic: a shield with its valid bit set and value < 85 -> CRITICAL + blink.
static inline void hudComputeAlert(const uint8_t ann[10], bool statusValid, const uint8_t shields[4], uint8_t shieldValidMask,
                                   AlertSeverity* sev, bool* blink) {
  *sev = ALERT_NONE;
  *blink = false;
  bool anyKnown = false;
  if (statusValid) {
    for (int i = 0; i < 10; i++)
      if (ann[i] != 15) anyKnown = true;
  }
  if (anyKnown) {
    bool crit = false, warn = false, critBlink = false, warnBlink = false;
    for (int i = 0; i < 10; i++) {
      uint8_t a = ann[i];
      if (a == 5 || a == 6) {
        crit = true;
        if (a == 5) critBlink = true;
      } else if (a == 3 || a == 4) {
        warn = true;
        if (a == 3) warnBlink = true;
      }
    }
    if (crit) {
      *sev = ALERT_CRITICAL;
      *blink = critBlink;
    } else if (warn) {
      *sev = ALERT_WARNING;
      *blink = warnBlink;
    }
    return;
  }
  for (int i = 0; i < 4; i++) {
    if ((shieldValidMask & (1 << i)) && shields[i] < 85) {
      *sev = ALERT_CRITICAL;
      *blink = true;
      return;
    }
  }
}
