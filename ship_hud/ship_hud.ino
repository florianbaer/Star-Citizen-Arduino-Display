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

static const int NUM_SCREENS = 5;
lv_obj_t* screens[NUM_SCREENS] = {};
int currentScreen = 0;
bool touchWasPressed = false;
uint32_t lastToggleMs = 0;
static const uint32_t DEBOUNCE_MS = 300; // prevent accidental double-tap

// ---- HUD widgets (Star Citizen screen) ----

ShieldGauge shields[4];
FuelBar hfuel, qfuel;
ShipSilhouette ship;
AlertIndicator alert;

// ---- MSFS widgets ----

GyroHorizon gyro;
EngineGauges engine;
FlightData flightData;
GForceMeter gforce;

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

void handleEngine(const uint8_t* payload, int len) {
  if (len < (int)sizeof(EngineMsg)) return;
  EngineMsg msg;
  memcpy(&msg, payload, sizeof(EngineMsg));
  engine.setValue(msg.rpm, msg.throttle, msg.fuel_flow, msg.oil_temp, msg.oil_press);
}

void handleFlightData(const uint8_t* payload, int len) {
  if (len < (int)sizeof(FlightDataMsg)) return;
  FlightDataMsg msg;
  memcpy(&msg, payload, sizeof(FlightDataMsg));
  flightData.setValue(msg.airspeed, msg.altitude, msg.vspeed, msg.ground_speed);
}

void handleGForce(const uint8_t* payload, int len) {
  if (len < (int)sizeof(GForceMsg)) return;
  GForceMsg msg;
  memcpy(&msg, payload, sizeof(GForceMsg));
  gforce.setValue(msg.gforce_x, msg.gforce_y, msg.gforce_z);
}

void toggleMode() {
  currentScreen = (currentScreen + 1) % NUM_SCREENS;
  if (screens[currentScreen]) {
    lv_scr_load(screens[currentScreen]);
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

  // ---- Screen 0: Star Citizen HUD ----
  screens[0] = lv_obj_create(NULL);
  lv_obj_set_style_bg_color(screens[0], lv_color_black(), 0);

  const ShieldDir dirs[] = {ShieldDir::FWD, ShieldDir::STB, ShieldDir::AFT, ShieldDir::PRT};
  for (int i = 0; i < 4; i++) {
    ShieldGaugeConfig sc;
    sc.direction = dirs[i];
    shields[i].create(screens[0], sc);
    shields[i].setValue(255);
  }

  ShipSilhouetteConfig shipCfg;
  ship.create(screens[0], shipCfg);

  AlertIndicatorConfig alertCfg;
  alert.create(screens[0], alertCfg);

  FuelBarConfig hfCfg;
  hfCfg.barOffsetY = -36;
  hfCfg.labelOffsetY = -38;
  hfCfg.label = "H-FUEL";
  hfuel.create(screens[0], hfCfg);
  hfuel.setValue(255);

  FuelBarConfig qfCfg;
  qfCfg.barOffsetY = -10;
  qfCfg.labelOffsetY = -12;
  qfCfg.r = 180; qfCfg.g = 0; qfCfg.b = 220;
  qfCfg.label = "Q-FUEL";
  qfuel.create(screens[0], qfCfg);
  qfuel.setValue(255);

  // ---- Screen 1: MSFS Gyroscope ----
  screens[1] = lv_obj_create(NULL);
  lv_obj_set_style_bg_color(screens[1], lv_color_black(), 0);

  GyroHorizonConfig gyroCfg;
  gyroCfg.cx = 160;
  gyroCfg.cy = 105;
  gyroCfg.radius = 90;
  gyro.create(screens[1], gyroCfg);

  // ---- Screen 2: MSFS Engine Gauges ----
  screens[2] = lv_obj_create(NULL);
  lv_obj_set_style_bg_color(screens[2], lv_color_black(), 0);
  engine.create(screens[2]);

  // ---- Screen 3: MSFS Flight Data ----
  screens[3] = lv_obj_create(NULL);
  lv_obj_set_style_bg_color(screens[3], lv_color_black(), 0);
  flightData.create(screens[3]);

  // ---- Screen 4: MSFS G-Force Meter ----
  screens[4] = lv_obj_create(NULL);
  lv_obj_set_style_bg_color(screens[4], lv_color_black(), 0);
  gforce.create(screens[4]);

  // Start with Star Citizen HUD
  lv_scr_load(screens[0]);

  Serial.println("HUD ready. Touch to cycle screens (5 modes). Waiting for data...");
}

void loop() {
  lv_tick_inc(millis() - lastTick);
  lastTick = millis();
  lv_timer_handler();

  // Feed serial bytes to frame decoder
  while (Serial.available()) {
    decoder.feed(Serial.read());
    if (decoder.available()) {
      uint8_t payload[32]; // large enough for any message
      int len = decoder.payload(payload, sizeof(payload));
      switch (decoder.msgType()) {
        case MSG_TELEMETRY:
          handleTelemetry(payload, len);
          break;
        case MSG_ATTITUDE:
          handleAttitude(payload, len);
          break;
        case MSG_ENGINE:
          handleEngine(payload, len);
          break;
        case MSG_FLIGHT_DATA:
          handleFlightData(payload, len);
          break;
        case MSG_GFORCE:
          handleGForce(payload, len);
          break;
      }
      decoder.clear();
    }
  }

  // Touch to toggle mode (rising edge with debounce)
  uint16_t tx, ty;
  bool pressed = touchRead(&tx, &ty);
  if (pressed && !touchWasPressed && (millis() - lastToggleMs > DEBOUNCE_MS)) {
    toggleMode();
    lastToggleMs = millis();
  }
  touchWasPressed = pressed;

  if (currentScreen == 0) {
    alert.setActive(anyCritical());
    alert.tick(millis());
  }

  delay(5);
}
