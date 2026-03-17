#pragma once
#include <lvgl.h>
#include <stdio.h>
#include "hud_proto.h"

/// Flight data (PFD-style) screen: airspeed, altitude, vertical speed bar, ground speed.
/// Uses only LVGL labels and bars — ~1KB memory.
class FlightData {
public:
  FlightData() = default;

  void create(lv_obj_t* parent) {
    _parent = parent;

    // Title
    lv_obj_t* title = lv_label_create(parent);
    lv_obj_set_style_text_color(title, lv_color_make(100, 100, 100), 0);
    lv_obj_set_style_text_font(title, &lv_font_montserrat_10, 0);
    lv_label_set_text(title, "MSFS FLIGHT DATA");
    lv_obj_align(title, LV_ALIGN_TOP_MID, 0, 2);

    // ---- Airspeed (left side) ----
    lv_obj_t* iasLabel = lv_label_create(parent);
    lv_obj_set_style_text_color(iasLabel, lv_color_make(100, 100, 100), 0);
    lv_obj_set_style_text_font(iasLabel, &lv_font_montserrat_10, 0);
    lv_label_set_text(iasLabel, "IAS");
    lv_obj_set_pos(iasLabel, 30, 30);

    _lblAirspeed = lv_label_create(parent);
    lv_obj_set_style_text_color(_lblAirspeed, lv_color_make(0, 220, 0), 0);
    lv_obj_set_style_text_font(_lblAirspeed, &lv_font_montserrat_28, 0);
    lv_label_set_text(_lblAirspeed, "0");
    lv_obj_set_pos(_lblAirspeed, 15, 45);

    lv_obj_t* ktsLabel = lv_label_create(parent);
    lv_obj_set_style_text_color(ktsLabel, lv_color_make(100, 100, 100), 0);
    lv_obj_set_style_text_font(ktsLabel, &lv_font_montserrat_10, 0);
    lv_label_set_text(ktsLabel, "KTS");
    lv_obj_set_pos(ktsLabel, 30, 80);

    // ---- Altitude (right side) ----
    lv_obj_t* altLabel = lv_label_create(parent);
    lv_obj_set_style_text_color(altLabel, lv_color_make(100, 100, 100), 0);
    lv_obj_set_style_text_font(altLabel, &lv_font_montserrat_10, 0);
    lv_label_set_text(altLabel, "ALT");
    lv_obj_set_pos(altLabel, 220, 30);

    _lblAltitude = lv_label_create(parent);
    lv_obj_set_style_text_color(_lblAltitude, lv_color_make(0, 220, 0), 0);
    lv_obj_set_style_text_font(_lblAltitude, &lv_font_montserrat_28, 0);
    lv_label_set_text(_lblAltitude, "0");
    lv_obj_set_pos(_lblAltitude, 200, 45);

    lv_obj_t* ftLabel = lv_label_create(parent);
    lv_obj_set_style_text_color(ftLabel, lv_color_make(100, 100, 100), 0);
    lv_obj_set_style_text_font(ftLabel, &lv_font_montserrat_10, 0);
    lv_label_set_text(ftLabel, "FT");
    lv_obj_set_pos(ftLabel, 228, 80);

    // ---- Vertical speed (center) ----
    lv_obj_t* vsLabel = lv_label_create(parent);
    lv_obj_set_style_text_color(vsLabel, lv_color_make(100, 100, 100), 0);
    lv_obj_set_style_text_font(vsLabel, &lv_font_montserrat_10, 0);
    lv_label_set_text(vsLabel, "VS FPM");
    lv_obj_set_pos(vsLabel, 137, 18);

    _barVspeed = lv_bar_create(parent);
    lv_obj_set_size(_barVspeed, 16, 140);
    lv_obj_set_pos(_barVspeed, 152, 32);
    lv_bar_set_range(_barVspeed, -3000, 3000);
    lv_bar_set_value(_barVspeed, 0, LV_ANIM_OFF);
    lv_obj_set_style_bg_color(_barVspeed, lv_color_make(40, 40, 40), LV_PART_MAIN);
    lv_obj_set_style_bg_color(_barVspeed, lv_color_make(0, 200, 0), LV_PART_INDICATOR);
    lv_obj_set_style_radius(_barVspeed, 3, LV_PART_MAIN);
    lv_obj_set_style_radius(_barVspeed, 3, LV_PART_INDICATOR);

    _lblVspeed = lv_label_create(parent);
    lv_obj_set_style_text_color(_lblVspeed, lv_color_make(0, 200, 0), 0);
    lv_obj_set_style_text_font(_lblVspeed, &lv_font_montserrat_14, 0);
    lv_label_set_text(_lblVspeed, "0");
    lv_obj_set_pos(_lblVspeed, 128, 175);

    // Zero reference line on vspeed bar
    lv_obj_t* zeroLine = lv_label_create(parent);
    lv_obj_set_style_text_color(zeroLine, lv_color_make(180, 180, 180), 0);
    lv_obj_set_style_text_font(zeroLine, &lv_font_montserrat_10, 0);
    lv_label_set_text(zeroLine, "--");
    lv_obj_set_pos(zeroLine, 170, 97);

    // ---- Ground speed (bottom center) ----
    lv_obj_t* gsTitle = lv_label_create(parent);
    lv_obj_set_style_text_color(gsTitle, lv_color_make(100, 100, 100), 0);
    lv_obj_set_style_text_font(gsTitle, &lv_font_montserrat_10, 0);
    lv_label_set_text(gsTitle, "GS");
    lv_obj_set_pos(gsTitle, 30, 120);

    _lblGroundSpeed = lv_label_create(parent);
    lv_obj_set_style_text_color(_lblGroundSpeed, lv_color_make(0, 180, 255), 0);
    lv_obj_set_style_text_font(_lblGroundSpeed, &lv_font_montserrat_18, 0);
    lv_label_set_text(_lblGroundSpeed, "0");
    lv_obj_set_pos(_lblGroundSpeed, 15, 135);

    lv_obj_t* gsUnit = lv_label_create(parent);
    lv_obj_set_style_text_color(gsUnit, lv_color_make(100, 100, 100), 0);
    lv_obj_set_style_text_font(gsUnit, &lv_font_montserrat_10, 0);
    lv_label_set_text(gsUnit, "KTS");
    lv_obj_set_pos(gsUnit, 30, 158);
  }

  void setValue(uint16_t airspeed, int32_t altitude, int16_t vspeed, uint16_t ground_speed) {
    if (!_lblAirspeed) return;

    char buf[16];

    // Airspeed: tenths of knots -> knots with 1 decimal
    snprintf(buf, sizeof(buf), "%u.%u", (unsigned)(airspeed / 10), (unsigned)(airspeed % 10));
    lv_label_set_text(_lblAirspeed, buf);

    // Altitude: feet
    snprintf(buf, sizeof(buf), "%ld", (long)altitude);
    lv_label_set_text(_lblAltitude, buf);

    // Vertical speed
    int16_t vs_clamped = vspeed;
    if (vs_clamped > 3000) vs_clamped = 3000;
    if (vs_clamped < -3000) vs_clamped = -3000;
    lv_bar_set_value(_barVspeed, vs_clamped, LV_ANIM_OFF);

    // Color: green for climb, red for descent
    lv_color_t vsColor = vspeed >= 0
        ? lv_color_make(0, 200, 0)
        : lv_color_make(255, 60, 60);
    lv_obj_set_style_bg_color(_barVspeed, vsColor, LV_PART_INDICATOR);

    snprintf(buf, sizeof(buf), "%d", (int)vspeed);
    lv_label_set_text(_lblVspeed, buf);
    lv_obj_set_style_text_color(_lblVspeed, vsColor, 0);

    // Ground speed: tenths of knots -> knots with 1 decimal
    snprintf(buf, sizeof(buf), "%u.%u", (unsigned)(ground_speed / 10), (unsigned)(ground_speed % 10));
    lv_label_set_text(_lblGroundSpeed, buf);
  }

private:
  lv_obj_t* _parent = nullptr;
  lv_obj_t* _lblAirspeed = nullptr;
  lv_obj_t* _lblAltitude = nullptr;
  lv_obj_t* _barVspeed = nullptr;
  lv_obj_t* _lblVspeed = nullptr;
  lv_obj_t* _lblGroundSpeed = nullptr;
};
