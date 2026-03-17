#pragma once
#include <lvgl.h>
#include <stdio.h>
#include "hud_proto.h"

/// Engine gauges screen: RPM arc, throttle bar, oil temp/press arcs, fuel flow label.
/// Uses only LVGL primitives (no canvas) — ~2KB memory.
class EngineGauges {
public:
  EngineGauges() = default;

  void create(lv_obj_t* parent) {
    _parent = parent;

    // Title
    lv_obj_t* title = lv_label_create(parent);
    lv_obj_set_style_text_color(title, lv_color_make(100, 100, 100), 0);
    lv_obj_set_style_text_font(title, &lv_font_montserrat_10, 0);
    lv_label_set_text(title, "MSFS ENGINE");
    lv_obj_align(title, LV_ALIGN_TOP_MID, 0, 2);

    // ---- RPM arc (large, top-left area) ----
    _arcRpm = lv_arc_create(parent);
    lv_obj_set_size(_arcRpm, 120, 120);
    lv_obj_set_pos(_arcRpm, 10, 20);
    lv_arc_set_range(_arcRpm, 0, 3000);
    lv_arc_set_value(_arcRpm, 0);
    lv_arc_set_bg_angles(_arcRpm, 135, 45);
    lv_arc_set_rotation(_arcRpm, 0);
    lv_obj_remove_style(_arcRpm, NULL, LV_PART_KNOB);
    lv_obj_remove_flag(_arcRpm, LV_OBJ_FLAG_CLICKABLE);
    lv_obj_set_style_arc_color(_arcRpm, lv_color_make(40, 40, 40), LV_PART_MAIN);
    lv_obj_set_style_arc_color(_arcRpm, lv_color_make(0, 200, 0), LV_PART_INDICATOR);
    lv_obj_set_style_arc_width(_arcRpm, 8, LV_PART_MAIN);
    lv_obj_set_style_arc_width(_arcRpm, 8, LV_PART_INDICATOR);

    _lblRpm = lv_label_create(parent);
    lv_obj_set_style_text_color(_lblRpm, lv_color_make(0, 220, 0), 0);
    lv_obj_set_style_text_font(_lblRpm, &lv_font_montserrat_18, 0);
    lv_label_set_text(_lblRpm, "0");
    lv_obj_set_pos(_lblRpm, 45, 72);

    lv_obj_t* rpmUnit = lv_label_create(parent);
    lv_obj_set_style_text_color(rpmUnit, lv_color_make(100, 100, 100), 0);
    lv_obj_set_style_text_font(rpmUnit, &lv_font_montserrat_10, 0);
    lv_label_set_text(rpmUnit, "RPM");
    lv_obj_set_pos(rpmUnit, 55, 94);

    // ---- Throttle vertical bar (right side) ----
    _barThrottle = lv_bar_create(parent);
    lv_obj_set_size(_barThrottle, 20, 160);
    lv_obj_set_pos(_barThrottle, 280, 30);
    lv_bar_set_range(_barThrottle, 0, 100);
    lv_bar_set_value(_barThrottle, 0, LV_ANIM_OFF);
    lv_obj_set_style_bg_color(_barThrottle, lv_color_make(40, 40, 40), LV_PART_MAIN);
    lv_obj_set_style_bg_color(_barThrottle, lv_color_make(0, 180, 255), LV_PART_INDICATOR);
    lv_obj_set_style_radius(_barThrottle, 4, LV_PART_MAIN);
    lv_obj_set_style_radius(_barThrottle, 4, LV_PART_INDICATOR);

    _lblThrottle = lv_label_create(parent);
    lv_obj_set_style_text_color(_lblThrottle, lv_color_make(0, 180, 255), 0);
    lv_obj_set_style_text_font(_lblThrottle, &lv_font_montserrat_12, 0);
    lv_label_set_text(_lblThrottle, "0%");
    lv_obj_set_pos(_lblThrottle, 273, 195);

    lv_obj_t* thrLabel = lv_label_create(parent);
    lv_obj_set_style_text_color(thrLabel, lv_color_make(100, 100, 100), 0);
    lv_obj_set_style_text_font(thrLabel, &lv_font_montserrat_10, 0);
    lv_label_set_text(thrLabel, "THR");
    lv_obj_set_pos(thrLabel, 278, 210);

    // ---- Oil temp arc (bottom-left) ----
    _arcOilTemp = lv_arc_create(parent);
    lv_obj_set_size(_arcOilTemp, 80, 80);
    lv_obj_set_pos(_arcOilTemp, 10, 150);
    lv_arc_set_range(_arcOilTemp, 0, 255);
    lv_arc_set_value(_arcOilTemp, 0);
    lv_arc_set_bg_angles(_arcOilTemp, 135, 45);
    lv_obj_remove_style(_arcOilTemp, NULL, LV_PART_KNOB);
    lv_obj_remove_flag(_arcOilTemp, LV_OBJ_FLAG_CLICKABLE);
    lv_obj_set_style_arc_color(_arcOilTemp, lv_color_make(40, 40, 40), LV_PART_MAIN);
    lv_obj_set_style_arc_color(_arcOilTemp, lv_color_make(255, 160, 0), LV_PART_INDICATOR);
    lv_obj_set_style_arc_width(_arcOilTemp, 6, LV_PART_MAIN);
    lv_obj_set_style_arc_width(_arcOilTemp, 6, LV_PART_INDICATOR);

    lv_obj_t* otLabel = lv_label_create(parent);
    lv_obj_set_style_text_color(otLabel, lv_color_make(255, 160, 0), 0);
    lv_obj_set_style_text_font(otLabel, &lv_font_montserrat_10, 0);
    lv_label_set_text(otLabel, "OIL T");
    lv_obj_set_pos(otLabel, 28, 185);

    // ---- Oil press arc (bottom-center) ----
    _arcOilPress = lv_arc_create(parent);
    lv_obj_set_size(_arcOilPress, 80, 80);
    lv_obj_set_pos(_arcOilPress, 100, 150);
    lv_arc_set_range(_arcOilPress, 0, 255);
    lv_arc_set_value(_arcOilPress, 0);
    lv_arc_set_bg_angles(_arcOilPress, 135, 45);
    lv_obj_remove_style(_arcOilPress, NULL, LV_PART_KNOB);
    lv_obj_remove_flag(_arcOilPress, LV_OBJ_FLAG_CLICKABLE);
    lv_obj_set_style_arc_color(_arcOilPress, lv_color_make(40, 40, 40), LV_PART_MAIN);
    lv_obj_set_style_arc_color(_arcOilPress, lv_color_make(0, 200, 100), LV_PART_INDICATOR);
    lv_obj_set_style_arc_width(_arcOilPress, 6, LV_PART_MAIN);
    lv_obj_set_style_arc_width(_arcOilPress, 6, LV_PART_INDICATOR);

    lv_obj_t* opLabel = lv_label_create(parent);
    lv_obj_set_style_text_color(opLabel, lv_color_make(0, 200, 100), 0);
    lv_obj_set_style_text_font(opLabel, &lv_font_montserrat_10, 0);
    lv_label_set_text(opLabel, "OIL P");
    lv_obj_set_pos(opLabel, 118, 185);

    // ---- Fuel flow display (right of oil gauges) ----
    _lblFuelFlow = lv_label_create(parent);
    lv_obj_set_style_text_color(_lblFuelFlow, lv_color_make(200, 200, 0), 0);
    lv_obj_set_style_text_font(_lblFuelFlow, &lv_font_montserrat_14, 0);
    lv_label_set_text(_lblFuelFlow, "FF: 0");
    lv_obj_set_pos(_lblFuelFlow, 195, 170);

    lv_obj_t* ffUnit = lv_label_create(parent);
    lv_obj_set_style_text_color(ffUnit, lv_color_make(100, 100, 100), 0);
    lv_obj_set_style_text_font(ffUnit, &lv_font_montserrat_10, 0);
    lv_label_set_text(ffUnit, "FUEL FLOW");
    lv_obj_set_pos(ffUnit, 195, 188);
  }

  void setValue(uint16_t rpm, uint8_t throttle, uint8_t fuel_flow,
                uint8_t oil_temp, uint8_t oil_press) {
    if (!_arcRpm) return;

    lv_arc_set_value(_arcRpm, rpm > 3000 ? 3000 : rpm);
    // Color RPM arc: green < 2200, yellow 2200-2700, red > 2700
    lv_color_t rpmColor;
    if (rpm > 2700)      rpmColor = lv_color_make(255, 40, 40);
    else if (rpm > 2200) rpmColor = lv_color_make(255, 200, 0);
    else                 rpmColor = lv_color_make(0, 200, 0);
    lv_obj_set_style_arc_color(_arcRpm, rpmColor, LV_PART_INDICATOR);

    char buf[16];
    snprintf(buf, sizeof(buf), "%u", (unsigned)rpm);
    lv_label_set_text(_lblRpm, buf);

    lv_bar_set_value(_barThrottle, throttle > 100 ? 100 : throttle, LV_ANIM_OFF);
    snprintf(buf, sizeof(buf), "%u%%", (unsigned)throttle);
    lv_label_set_text(_lblThrottle, buf);

    lv_arc_set_value(_arcOilTemp, oil_temp);
    lv_arc_set_value(_arcOilPress, oil_press);

    snprintf(buf, sizeof(buf), "FF: %u", (unsigned)fuel_flow);
    lv_label_set_text(_lblFuelFlow, buf);
  }

private:
  lv_obj_t* _parent = nullptr;
  lv_obj_t* _arcRpm = nullptr;
  lv_obj_t* _lblRpm = nullptr;
  lv_obj_t* _barThrottle = nullptr;
  lv_obj_t* _lblThrottle = nullptr;
  lv_obj_t* _arcOilTemp = nullptr;
  lv_obj_t* _arcOilPress = nullptr;
  lv_obj_t* _lblFuelFlow = nullptr;
};
