#pragma once
#include <lvgl.h>
#include <stdio.h>
#include "hud_proto.h"

/// G-force meter screen: vertical G arc, lateral G bar, peak tracking.
/// Uses only LVGL primitives — ~2KB memory.
class GForceMeter {
public:
  GForceMeter() = default;

  void create(lv_obj_t* parent) {
    _parent = parent;
    _peakPosY = 100;  // 1.0G default
    _peakNegY = 100;

    // Title
    lv_obj_t* title = lv_label_create(parent);
    lv_obj_set_style_text_color(title, lv_color_make(100, 100, 100), 0);
    lv_obj_set_style_text_font(title, &lv_font_montserrat_10, 0);
    lv_label_set_text(title, "MSFS G-FORCE");
    lv_obj_align(title, LV_ALIGN_TOP_MID, 0, 2);

    // ---- Main vertical G arc (large, center-left) ----
    // Range: -200 to 600 (hundredths of G, -2G to +6G)
    _arcVertG = lv_arc_create(parent);
    lv_obj_set_size(_arcVertG, 150, 150);
    lv_obj_set_pos(_arcVertG, 10, 30);
    lv_arc_set_range(_arcVertG, -200, 600);
    lv_arc_set_value(_arcVertG, 100); // 1G default
    lv_arc_set_bg_angles(_arcVertG, 135, 45);
    lv_obj_remove_style(_arcVertG, NULL, LV_PART_KNOB);
    lv_obj_remove_flag(_arcVertG, LV_OBJ_FLAG_CLICKABLE);
    lv_obj_set_style_arc_color(_arcVertG, lv_color_make(40, 40, 40), LV_PART_MAIN);
    lv_obj_set_style_arc_color(_arcVertG, lv_color_make(0, 200, 0), LV_PART_INDICATOR);
    lv_obj_set_style_arc_width(_arcVertG, 10, LV_PART_MAIN);
    lv_obj_set_style_arc_width(_arcVertG, 10, LV_PART_INDICATOR);

    _lblVertG = lv_label_create(parent);
    lv_obj_set_style_text_color(_lblVertG, lv_color_make(0, 220, 0), 0);
    lv_obj_set_style_text_font(_lblVertG, &lv_font_montserrat_28, 0);
    lv_label_set_text(_lblVertG, "1.00");
    lv_obj_set_pos(_lblVertG, 45, 90);

    lv_obj_t* gLabel = lv_label_create(parent);
    lv_obj_set_style_text_color(gLabel, lv_color_make(100, 100, 100), 0);
    lv_obj_set_style_text_font(gLabel, &lv_font_montserrat_12, 0);
    lv_label_set_text(gLabel, "VERT G");
    lv_obj_set_pos(gLabel, 50, 122);

    // ---- Lateral G bar (bottom, horizontal) ----
    lv_obj_t* latTitle = lv_label_create(parent);
    lv_obj_set_style_text_color(latTitle, lv_color_make(100, 100, 100), 0);
    lv_obj_set_style_text_font(latTitle, &lv_font_montserrat_10, 0);
    lv_label_set_text(latTitle, "LATERAL G");
    lv_obj_set_pos(latTitle, 15, 190);

    _barLatG = lv_bar_create(parent);
    lv_obj_set_size(_barLatG, 200, 14);
    lv_obj_set_pos(_barLatG, 10, 205);
    lv_bar_set_range(_barLatG, -200, 200); // -2G to +2G
    lv_bar_set_value(_barLatG, 0, LV_ANIM_OFF);
    lv_obj_set_style_bg_color(_barLatG, lv_color_make(40, 40, 40), LV_PART_MAIN);
    lv_obj_set_style_bg_color(_barLatG, lv_color_make(0, 180, 255), LV_PART_INDICATOR);
    lv_obj_set_style_radius(_barLatG, 3, LV_PART_MAIN);
    lv_obj_set_style_radius(_barLatG, 3, LV_PART_INDICATOR);

    _lblLatG = lv_label_create(parent);
    lv_obj_set_style_text_color(_lblLatG, lv_color_make(0, 180, 255), 0);
    lv_obj_set_style_text_font(_lblLatG, &lv_font_montserrat_12, 0);
    lv_label_set_text(_lblLatG, "0.00");
    lv_obj_set_pos(_lblLatG, 215, 203);

    // ---- Longitudinal G (right side) ----
    lv_obj_t* longTitle = lv_label_create(parent);
    lv_obj_set_style_text_color(longTitle, lv_color_make(100, 100, 100), 0);
    lv_obj_set_style_text_font(longTitle, &lv_font_montserrat_10, 0);
    lv_label_set_text(longTitle, "LONG G");
    lv_obj_set_pos(longTitle, 200, 40);

    _lblLongG = lv_label_create(parent);
    lv_obj_set_style_text_color(_lblLongG, lv_color_make(200, 200, 0), 0);
    lv_obj_set_style_text_font(_lblLongG, &lv_font_montserrat_18, 0);
    lv_label_set_text(_lblLongG, "0.00");
    lv_obj_set_pos(_lblLongG, 195, 55);

    // ---- Peak G tracking labels (right side) ----
    lv_obj_t* peakTitle = lv_label_create(parent);
    lv_obj_set_style_text_color(peakTitle, lv_color_make(100, 100, 100), 0);
    lv_obj_set_style_text_font(peakTitle, &lv_font_montserrat_10, 0);
    lv_label_set_text(peakTitle, "PEAK");
    lv_obj_set_pos(peakTitle, 210, 95);

    _lblPeakPos = lv_label_create(parent);
    lv_obj_set_style_text_color(_lblPeakPos, lv_color_make(255, 100, 100), 0);
    lv_obj_set_style_text_font(_lblPeakPos, &lv_font_montserrat_14, 0);
    lv_label_set_text(_lblPeakPos, "+1.00G");
    lv_obj_set_pos(_lblPeakPos, 195, 108);

    _lblPeakNeg = lv_label_create(parent);
    lv_obj_set_style_text_color(_lblPeakNeg, lv_color_make(100, 100, 255), 0);
    lv_obj_set_style_text_font(_lblPeakNeg, &lv_font_montserrat_14, 0);
    lv_label_set_text(_lblPeakNeg, "+1.00G");
    lv_obj_set_pos(_lblPeakNeg, 195, 128);
  }

  void setValue(int16_t gx, int16_t gy, int16_t gz) {
    if (!_arcVertG) return;

    // Update peak tracking
    if (gy > _peakPosY) _peakPosY = gy;
    if (gy < _peakNegY) _peakNegY = gy;

    // Vertical G arc
    int16_t gy_clamped = gy;
    if (gy_clamped > 600) gy_clamped = 600;
    if (gy_clamped < -200) gy_clamped = -200;
    lv_arc_set_value(_arcVertG, gy_clamped);

    // Color: green normal, yellow >2G, red >4G or negative
    lv_color_t vColor;
    if (gy < 0 || gy > 400)       vColor = lv_color_make(255, 40, 40);
    else if (gy > 200)            vColor = lv_color_make(255, 200, 0);
    else                          vColor = lv_color_make(0, 200, 0);
    lv_obj_set_style_arc_color(_arcVertG, vColor, LV_PART_INDICATOR);

    char buf[16];
    formatG(buf, sizeof(buf), gy);
    lv_label_set_text(_lblVertG, buf);
    lv_obj_set_style_text_color(_lblVertG, vColor, 0);

    // Lateral G bar
    int16_t gz_clamped = gz;
    if (gz_clamped > 200) gz_clamped = 200;
    if (gz_clamped < -200) gz_clamped = -200;
    lv_bar_set_value(_barLatG, gz_clamped, LV_ANIM_OFF);
    formatG(buf, sizeof(buf), gz);
    lv_label_set_text(_lblLatG, buf);

    // Longitudinal G
    formatG(buf, sizeof(buf), gx);
    lv_label_set_text(_lblLongG, buf);

    // Peak labels
    formatGSigned(buf, sizeof(buf), _peakPosY);
    lv_label_set_text(_lblPeakPos, buf);
    formatGSigned(buf, sizeof(buf), _peakNegY);
    lv_label_set_text(_lblPeakNeg, buf);
  }

private:
  lv_obj_t* _parent = nullptr;
  lv_obj_t* _arcVertG = nullptr;
  lv_obj_t* _lblVertG = nullptr;
  lv_obj_t* _barLatG = nullptr;
  lv_obj_t* _lblLatG = nullptr;
  lv_obj_t* _lblLongG = nullptr;
  lv_obj_t* _lblPeakPos = nullptr;
  lv_obj_t* _lblPeakNeg = nullptr;

  int16_t _peakPosY = 100;
  int16_t _peakNegY = 100;

  /// Format hundredths of G as "X.XX" (e.g., 123 -> "1.23")
  static void formatG(char* buf, int buflen, int16_t hundredths) {
    int abs_val = hundredths < 0 ? -hundredths : hundredths;
    const char* sign = hundredths < 0 ? "-" : "";
    snprintf(buf, buflen, "%s%d.%02d", sign, abs_val / 100, abs_val % 100);
  }

  /// Format with sign and G suffix (e.g., "+1.23G")
  static void formatGSigned(char* buf, int buflen, int16_t hundredths) {
    int abs_val = hundredths < 0 ? -hundredths : hundredths;
    const char* sign = hundredths < 0 ? "-" : "+";
    snprintf(buf, buflen, "%s%d.%02dG", sign, abs_val / 100, abs_val % 100);
  }
};
