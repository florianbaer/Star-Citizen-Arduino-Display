// Spaceship HUD Display for ESP32-2432S024C (Capacitive touch)
// Supports two modes: Star Citizen HUD and MSFS 2024 Gyroscope
// Receives COBS-framed binary telemetry via USB serial
// Touch chip: CST816S on I2C

#include <lvgl.h>
#include <TFT_eSPI.h>
#include <Wire.h>
#include "hud_widgets.h"
#include "hud_proto.h"

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

// ---- Mode switching ----

lv_obj_t* scrHud = nullptr;   // Star Citizen HUD screen
lv_obj_t* scrGyro = nullptr;  // MSFS Gyroscope screen
bool msfsMode = false;
bool touchWasPressed = false;

// ---- HUD widgets (Star Citizen screen) ----

ShieldGauge shields[4];
FuelBar hfuel, qfuel;
ShipSilhouette ship;
AlertIndicator alert;

// ---- Gyro widget (MSFS screen) ----

GyroHorizon gyro;

// ---- Shared ----

FrameDecoder decoder;

bool anyCritical() {
  for (int i = 0; i < 4; i++)
    if (shields[i].value() < 85) return true;
  return false;
}

void handleTelemetry(const uint8_t* payload, int len) {
  if (len < (int)sizeof(TelemetryMsg)) return;
  TelemetryMsg msg;
  memcpy(&msg, payload, sizeof(TelemetryMsg));
  shields[0].setValue(msg.shield_front);
  shields[2].setValue(msg.shield_back);
  shields[3].setValue(msg.shield_left);
  shields[1].setValue(msg.shield_right);
  hfuel.setValue(msg.hydrogen_fuel);
  qfuel.setValue(msg.quantum_fuel);
}

void handleAttitude(const uint8_t* payload, int len) {
  if (len < (int)sizeof(AttitudeMsg)) return;
  AttitudeMsg msg;
  memcpy(&msg, payload, sizeof(AttitudeMsg));
  gyro.setValue(msg.pitch, msg.roll, msg.heading);
}

void toggleMode() {
  msfsMode = !msfsMode;
  if (msfsMode) {
    lv_scr_load(scrGyro);
  } else {
    lv_scr_load(scrHud);
  }
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

  lv_indev_t * indev = lv_indev_create();
  lv_indev_set_type(indev, LV_INDEV_TYPE_POINTER);
  lv_indev_set_read_cb(indev, my_touchpad_read);

  // ---- Screen 1: Star Citizen HUD ----
  scrHud = lv_obj_create(NULL);
  lv_obj_set_style_bg_color(scrHud, lv_color_black(), 0);

  const ShieldDir dirs[] = {ShieldDir::FWD, ShieldDir::STB, ShieldDir::AFT, ShieldDir::PRT};
  for (int i = 0; i < 4; i++) {
    ShieldGaugeConfig sc;
    sc.direction = dirs[i];
    shields[i].create(scrHud, sc);
    shields[i].setValue(255);
  }

  ShipSilhouetteConfig shipCfg;
  ship.create(scrHud, shipCfg);

  AlertIndicatorConfig alertCfg;
  alert.create(scrHud, alertCfg);

  FuelBarConfig hfCfg;
  hfCfg.barOffsetY = -36;
  hfCfg.labelOffsetY = -38;
  hfCfg.label = "H-FUEL";
  hfuel.create(scrHud, hfCfg);
  hfuel.setValue(255);

  FuelBarConfig qfCfg;
  qfCfg.barOffsetY = -10;
  qfCfg.labelOffsetY = -12;
  qfCfg.r = 180; qfCfg.g = 0; qfCfg.b = 220;
  qfCfg.label = "Q-FUEL";
  qfuel.create(scrHud, qfCfg);
  qfuel.setValue(255);

  // ---- Screen 2: MSFS Gyroscope ----
  scrGyro = lv_obj_create(NULL);
  lv_obj_set_style_bg_color(scrGyro, lv_color_black(), 0);

  GyroHorizonConfig gyroCfg;
  gyroCfg.cx = 160;
  gyroCfg.cy = 105;
  gyroCfg.radius = 90;
  gyro.create(scrGyro, gyroCfg);

  // Start with Star Citizen HUD
  lv_scr_load(scrHud);

  Serial.println("HUD ready. Touch to switch modes. Waiting for data...");
}

void loop() {
  lv_tick_inc(millis() - lastTick);
  lastTick = millis();
  lv_timer_handler();

  // Feed serial bytes to frame decoder
  while (Serial.available()) {
    decoder.feed(Serial.read());
    if (decoder.available()) {
      if (decoder.msgType() == MSG_TELEMETRY) {
        uint8_t payload[sizeof(TelemetryMsg)];
        int len = decoder.payload(payload, sizeof(payload));
        handleTelemetry(payload, len);
      } else if (decoder.msgType() == MSG_ATTITUDE) {
        uint8_t payload[sizeof(AttitudeMsg)];
        int len = decoder.payload(payload, sizeof(payload));
        handleAttitude(payload, len);
      }
      decoder.clear();
    }
  }

  // Touch to toggle mode (detect rising edge)
  uint16_t tx, ty;
  bool pressed = touchRead(&tx, &ty);
  if (pressed && !touchWasPressed) {
    toggleMode();
  }
  touchWasPressed = pressed;

  if (!msfsMode) {
    alert.setActive(anyCritical());
    alert.tick(millis());
  }

  delay(5);
}
