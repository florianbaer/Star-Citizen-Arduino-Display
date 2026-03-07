// Spaceship HUD Display for ESP32-2432S024C (Capacitive touch)
// Receives telemetry via USB serial: SF:200,SB:100,SL:150,SR:255,HF:80,QF:40\n
// Touch chip: CST816S on I2C

#include <lvgl.h>
#include <TFT_eSPI.h>
#include <Wire.h>
#include "hud_widgets.h"

#define TFT_HOR_RES   320
#define TFT_VER_RES   240
#define DRAW_BUF_SIZE (TFT_HOR_RES * TFT_VER_RES / 10 * (LV_COLOR_DEPTH / 8))

TFT_eSPI tft = TFT_eSPI();

void my_disp_flush(lv_display_t *disp, const lv_area_t *area, uint8_t *px_map) {
  uint32_t w = (area->x2 - area->x1 + 1);
  uint32_t h = (area->y2 - area->y1 + 1);
  tft.startWrite();
  tft.setAddrWindow(area->x1, area->y1, w, h);
  tft.pushColors((uint16_t *)px_map, w * h, true);
  tft.endWrite();
  lv_display_flush_ready(disp);
}

// CST816S I2C touch
#define TOUCH_SDA 33
#define TOUCH_SCL 32
#define TOUCH_INT 21
#define TOUCH_RST 25
#define CST816S_ADDR 0x15

uint32_t lastTick = 0;

void touchInit() {
  pinMode(TOUCH_RST, OUTPUT);
  digitalWrite(TOUCH_RST, LOW);
  delay(10);
  digitalWrite(TOUCH_RST, HIGH);
  delay(50);
  Wire.begin(TOUCH_SDA, TOUCH_SCL);
  pinMode(TOUCH_INT, INPUT);
}

bool touchRead(uint16_t *x, uint16_t *y) {
  Wire.beginTransmission(CST816S_ADDR);
  Wire.write(0x02);
  if (Wire.endTransmission() != 0) return false;

  Wire.requestFrom((uint8_t)CST816S_ADDR, (uint8_t)5);
  if (Wire.available() < 5) return false;

  uint8_t touchPoints = Wire.read();
  uint8_t xHigh = Wire.read();
  uint8_t xLow  = Wire.read();
  uint8_t yHigh = Wire.read();
  uint8_t yLow  = Wire.read();

  if (touchPoints == 0) return false;

  *x = ((xHigh & 0x0F) << 8) | xLow;
  *y = ((yHigh & 0x0F) << 8) | yLow;
  return true;
}

void my_touchpad_read(lv_indev_t * indev, lv_indev_data_t * data) {
  uint16_t tx, ty;
  if (touchRead(&tx, &ty)) {
    data->point.x = ty;
    data->point.y = TFT_VER_RES - tx;
    data->state = LV_INDEV_STATE_PRESSED;
  } else {
    data->state = LV_INDEV_STATE_RELEASED;
  }
}

// ---- HUD widgets ----

ShieldGauge shields[4];
FuelBar hfuel, qfuel;
ShipSilhouette ship;
AlertIndicator alert;
TelemetryParser telemetry;

bool anyCritical() {
  for (int i = 0; i < 4; i++)
    if (shields[i].value() < 85) return true;
  return false;
}

void setup() {
  Serial.begin(115200);
  Serial.println("Starting HUD...");

  pinMode(27, OUTPUT);
  digitalWrite(27, HIGH);
  touchInit();

  tft.init();
  tft.setRotation(1);
  tft.fillScreen(TFT_BLACK);

  lv_init();
  uint8_t* draw_buf = new uint8_t[DRAW_BUF_SIZE];
  lv_display_t * disp = lv_display_create(TFT_HOR_RES, TFT_VER_RES);
  lv_display_set_flush_cb(disp, my_disp_flush);
  lv_display_set_buffers(disp, draw_buf, NULL, DRAW_BUF_SIZE, LV_DISPLAY_RENDER_MODE_PARTIAL);

  lv_obj_t *scr = lv_scr_act();
  lv_obj_set_style_bg_color(scr, lv_color_black(), 0);

  lv_indev_t * indev = lv_indev_create();
  lv_indev_set_type(indev, LV_INDEV_TYPE_POINTER);
  lv_indev_set_read_cb(indev, my_touchpad_read);

  // Create HUD
  const ShieldDir dirs[] = {ShieldDir::FWD, ShieldDir::STB, ShieldDir::AFT, ShieldDir::PRT};
  for (int i = 0; i < 4; i++) {
    ShieldGaugeConfig sc;
    sc.direction = dirs[i];
    shields[i].create(scr, sc);
  }

  ShipSilhouetteConfig shipCfg;
  ship.create(scr, shipCfg);

  AlertIndicatorConfig alertCfg;
  alert.create(scr, alertCfg);

  FuelBarConfig hfCfg;
  hfCfg.barOffsetY = -36;
  hfCfg.labelOffsetY = -38;
  hfCfg.label = "H-FUEL";
  hfuel.create(scr, hfCfg);

  FuelBarConfig qfCfg;
  qfCfg.barOffsetY = -10;
  qfCfg.labelOffsetY = -12;
  qfCfg.r = 180; qfCfg.g = 0; qfCfg.b = 220;
  qfCfg.label = "Q-FUEL";
  qfuel.create(scr, qfCfg);

  telemetry.onField("SF", [](uint8_t v) { shields[0].setValue(v); });
  telemetry.onField("SB", [](uint8_t v) { shields[2].setValue(v); });
  telemetry.onField("SL", [](uint8_t v) { shields[3].setValue(v); });
  telemetry.onField("SR", [](uint8_t v) { shields[1].setValue(v); });
  telemetry.onField("HF", [](uint8_t v) { hfuel.setValue(v); });
  telemetry.onField("QF", [](uint8_t v) { qfuel.setValue(v); });

  Serial.println("HUD ready. Waiting for telemetry...");
}

void loop() {
  lv_tick_inc(millis() - lastTick);
  lastTick = millis();
  lv_timer_handler();

  alert.setActive(anyCritical());
  alert.tick(millis());
  telemetry.feed(Serial);

  delay(5);
}
