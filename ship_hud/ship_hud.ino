// Spaceship HUD Display for ESP32-2432S024C (Capacitive touch)
// Receives COBS-framed binary telemetry (protocol v2, see docs/PROTOCOL.md) via USB serial
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

// ---- HUD widgets ----

lv_obj_t *vehPanel, *footPanel, *overlay;
lv_obj_t *nameLbl, *modeLbl;

ShieldGauge shields[4];
FuelBar hfuel, qfuel;
FuelBar footHealth, footO2;
ShipSilhouette ship;
AlertIndicator alert;
IndicatorLabel gearInd, qtInd, dcplInd;
FrameDecoder decoder;

// ---- HUD state ----

uint32_t g_lastFrameMs = 0;
bool g_anyFrame = false, g_linkSeen = false, g_statusValid = false;
uint8_t g_linkState = 0;
StatusMsg g_status;
uint8_t g_shieldVal[4] = {255, 255, 255, 255};
uint8_t g_shieldValid = 0;
int g_senderVersion = -1;
uint32_t g_lastHelloMs = 0;
HudMode g_mode = HUD_MODE_NO_SIGNAL;
bool g_chanValid[8];

// Smoothed channels: 0..3 shields (front, back, left, right as in the Telemetry message),
// 4 hydrogen, 5 quantum, 6 health, 7 oxygen.
Smoother sm[8];
// Message order (front, back, left, right) -> widget index (FWD, STB, AFT, PRT).
static const int SHIELD_WIDGET[4] = {0, 2, 3, 1};

// Last applied mode / overlay state (panel and overlay styles are only touched when these change).
int g_appliedMode = -1;
int g_appliedMismatch = -2;
int g_masterShown = -2;
char g_nameShown[40] = "\x01";

void sendHello() {
  uint8_t p[2] = {HUD_PROTO_VERSION, ROLE_HUD};
  uint8_t buf[HUD_MAX_FRAME];
  size_t n = hud_frame_encode(MSG_HELLO, p, sizeof(p), buf, sizeof(buf));
  if (n > 0) Serial.write(buf, n);
  g_lastHelloMs = millis();
}

lv_obj_t* makePanel(lv_obj_t* parent) {
  lv_obj_t* p = lv_obj_create(parent);
  lv_obj_set_size(p, TFT_HOR_RES, TFT_VER_RES);
  lv_obj_set_pos(p, 0, 0);
  lv_obj_set_style_bg_opa(p, LV_OPA_TRANSP, 0);
  lv_obj_set_style_border_width(p, 0, 0);
  lv_obj_set_style_pad_all(p, 0, 0);
  lv_obj_remove_flag(p, LV_OBJ_FLAG_SCROLLABLE);
  return p;
}

lv_obj_t* makeLabel(lv_obj_t* parent, const char* text, lv_align_t align, int x, int y, uint8_t r, uint8_t g, uint8_t b) {
  lv_obj_t* l = lv_label_create(parent);
  lv_obj_set_style_text_color(l, lv_color_make(r, g, b), 0);
  lv_obj_set_style_text_font(l, &lv_font_montserrat_14, 0);
  lv_label_set_text(l, text);
  lv_obj_align(l, align, x, y);
  return l;
}

// Channel update from a frame: valid -> smoother target (snap when it was invalid), invalid -> unknown on the widget.
void widgetSetUnknown(int ch) {
  if (ch < 4) shields[SHIELD_WIDGET[ch]].setUnknown();
  else if (ch == 4) hfuel.setUnknown();
  else if (ch == 5) qfuel.setUnknown();
  else if (ch == 6) footHealth.setUnknown();
  else if (ch == 7) footO2.setUnknown();
}

void widgetSetValue(int ch, uint8_t v) {
  if (ch < 4) shields[SHIELD_WIDGET[ch]].setValue(v);
  else if (ch == 4) hfuel.setValue(v);
  else if (ch == 5) qfuel.setValue(v);
  else if (ch == 6) footHealth.setValue(v);
  else if (ch == 7) footO2.setValue(v);
}

void setChannel(int ch, bool valid, uint8_t v) {
  if (valid) {
    if (!g_chanValid[ch]) sm[ch].snap(v);
    else sm[ch].setTarget(v);
    g_chanValid[ch] = true;
  } else {
    g_chanValid[ch] = false;
    widgetSetUnknown(ch);  // change-only inside the widget
  }
}

void handleTelemetry(const uint8_t* payload, int len) {
  TelemetryMsg t;
  if (!hud_decode_telemetry(payload, len, &t)) return;
  uint8_t vals[6] = {t.shield_front, t.shield_back, t.shield_left, t.shield_right, t.hydrogen_fuel, t.quantum_fuel};
  for (int i = 0; i < 6; i++) setChannel(i, (t.valid >> i) & 1, vals[i]);
  for (int i = 0; i < 4; i++) g_shieldVal[i] = vals[i];
  g_shieldValid = t.valid & 0x0F;
}

void handleVitals(const uint8_t* payload, int len) {
  VitalsMsg v;
  if (!hud_decode_vitals(payload, len, &v)) return;
  setChannel(6, v.valid & VV_HEALTH, v.health);
  setChannel(7, v.valid & VV_OXYGEN, v.oxygen);
}

void handleStatus(const uint8_t* payload, int len) {
  StatusMsg s;
  if (!hud_decode_status(payload, len, &s)) return;
  g_status = s;
  g_statusValid = true;

  switch (s.gear) {
    case 3: gearInd.setState(IndState::On); break;
    case 2:
    case 4: gearInd.setState(IndState::Transit); break;
    case 1: gearInd.setState(IndState::Off); break;
    default: gearInd.setState(IndState::Hidden); break;  // unknown / unspecified
  }
  qtInd.setState((s.flags_valid & SF_QT_SPOOLED) ? ((s.flags & SF_QT_SPOOLED) ? IndState::On : IndState::Off) : IndState::Hidden);
  dcplInd.setState((s.flags_valid & SF_DECOUPLED) ? ((s.flags & SF_DECOUPLED) ? IndState::On : IndState::Off) : IndState::Hidden);

  int mm = (s.master_mode >= 1 && s.master_mode <= 3) ? s.master_mode : 0;
  if (mm != g_masterShown) {
    g_masterShown = mm;
    if (mm == 0) {
      lv_obj_add_flag(modeLbl, LV_OBJ_FLAG_HIDDEN);
    } else {
      lv_label_set_text(modeLbl, mm == 1 ? "NAV" : (mm == 2 ? "SCM" : "STEALTH"));
      lv_obj_remove_flag(modeLbl, LV_OBJ_FLAG_HIDDEN);
    }
  }
}

void handleVehicleName(const uint8_t* payload, int len) {
  char name[40];
  hud_decode_vehicle_name(payload, len, name, sizeof(name));
  if (strcmp(name, g_nameShown) == 0) return;
  strncpy(g_nameShown, name, sizeof(g_nameShown) - 1);
  g_nameShown[sizeof(g_nameShown) - 1] = 0;
  lv_label_set_text(nameLbl, name);
}

// Panel visibility / dimming and overlay text. Only called when the mode or the version-mismatch text changes.
void applyMode(HudMode m, int mismatchVersion) {
  switch (m) {
    case HUD_MODE_VEHICLE:
      lv_obj_remove_flag(vehPanel, LV_OBJ_FLAG_HIDDEN);
      lv_obj_set_style_opa(vehPanel, LV_OPA_COVER, 0);
      lv_obj_add_flag(footPanel, LV_OBJ_FLAG_HIDDEN);
      break;
    case HUD_MODE_ON_FOOT:
      lv_obj_add_flag(vehPanel, LV_OBJ_FLAG_HIDDEN);
      lv_obj_remove_flag(footPanel, LV_OBJ_FLAG_HIDDEN);
      break;
    default:  // NO_GAME, NO_SIGNAL
      lv_obj_remove_flag(vehPanel, LV_OBJ_FLAG_HIDDEN);
      lv_obj_set_style_opa(vehPanel, LV_OPA_40, 0);
      lv_obj_add_flag(footPanel, LV_OBJ_FLAG_HIDDEN);
      break;
  }

  char text[56];
  text[0] = 0;
  if (m == HUD_MODE_NO_SIGNAL) snprintf(text, sizeof(text), "NO SIGNAL");
  else if (m == HUD_MODE_NO_GAME) snprintf(text, sizeof(text), "NO GAME");
  if (mismatchVersion >= 0) {
    size_t n = strlen(text);
    snprintf(text + n, sizeof(text) - n, "%sSENDER PROTO v%d", n ? "\n" : "", mismatchVersion);
  }
  if (text[0] == 0) {
    lv_obj_add_flag(overlay, LV_OBJ_FLAG_HIDDEN);
  } else {
    lv_label_set_text(overlay, text);
    lv_obj_align(overlay, LV_ALIGN_CENTER, 0, 0);
    lv_obj_remove_flag(overlay, LV_OBJ_FLAG_HIDDEN);
  }
}

void setup() {
  Serial.setRxBufferSize(1024);  // a full-screen redraw can keep loop() away from the UART for a while
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

  for (int i = 0; i < 8; i++) g_chanValid[i] = false;

  // ---- vehicle panel ----
  vehPanel = makePanel(scr);

  const ShieldDir dirs[] = {ShieldDir::FWD, ShieldDir::STB, ShieldDir::AFT, ShieldDir::PRT};
  for (int i = 0; i < 4; i++) {
    ShieldGaugeConfig sc;
    sc.direction = dirs[i];
    shields[i].create(vehPanel, sc);
    shields[i].setValue(255);
  }

  ShipSilhouetteConfig shipCfg;
  ship.create(vehPanel, shipCfg);

  AlertIndicatorConfig alertCfg;
  alert.create(vehPanel, alertCfg);

  FuelBarConfig hfCfg;
  hfCfg.barOffsetY = -36;
  hfCfg.labelOffsetY = -38;
  hfCfg.label = "H-FUEL";
  hfuel.create(vehPanel, hfCfg);
  hfuel.setValue(255);

  FuelBarConfig qfCfg;
  qfCfg.barOffsetY = -10;
  qfCfg.labelOffsetY = -12;
  qfCfg.r = 180; qfCfg.g = 0; qfCfg.b = 220;
  qfCfg.label = "Q-FUEL";
  qfuel.create(vehPanel, qfCfg);
  qfuel.setValue(255);

  IndicatorLabelConfig gearCfg;
  gearCfg.x = 4; gearCfg.y = 4; gearCfg.text = "GEAR";
  gearInd.create(vehPanel, gearCfg);
  IndicatorLabelConfig qtCfg;
  qtCfg.x = 4; qtCfg.y = 22; qtCfg.text = "QT";
  qtCfg.onR = 0; qtCfg.onG = 180; qtCfg.onB = 255;
  qtInd.create(vehPanel, qtCfg);
  IndicatorLabelConfig dcCfg;
  dcCfg.x = 4; dcCfg.y = 40; dcCfg.text = "DCPL";
  dcCfg.onR = 255; dcCfg.onG = 180; dcCfg.onB = 0;
  dcplInd.create(vehPanel, dcCfg);

  nameLbl = makeLabel(vehPanel, "", LV_ALIGN_TOP_RIGHT, -4, 4, 180, 180, 200);
  lv_label_set_long_mode(nameLbl, LV_LABEL_LONG_CLIP);
  lv_obj_set_width(nameLbl, 100);
  modeLbl = makeLabel(vehPanel, "", LV_ALIGN_TOP_RIGHT, -4, 22, 180, 180, 200);
  lv_obj_add_flag(modeLbl, LV_OBJ_FLAG_HIDDEN);

  // ---- on-foot panel ----
  footPanel = makePanel(scr);
  lv_obj_add_flag(footPanel, LV_OBJ_FLAG_HIDDEN);
  makeLabel(footPanel, "ON FOOT", LV_ALIGN_TOP_MID, 0, 20, 180, 180, 200);

  FuelBarConfig hpCfg;
  hpCfg.barAlign = LV_ALIGN_TOP_RIGHT; hpCfg.barOffsetX = -10; hpCfg.barOffsetY = 90;
  hpCfg.labelAlign = LV_ALIGN_TOP_LEFT; hpCfg.labelOffsetX = 10; hpCfg.labelOffsetY = 88;
  hpCfg.r = 0; hpCfg.g = 200; hpCfg.b = 80;
  hpCfg.label = "HEALTH";
  footHealth.create(footPanel, hpCfg);

  FuelBarConfig o2Cfg;
  o2Cfg.barAlign = LV_ALIGN_TOP_RIGHT; o2Cfg.barOffsetX = -10; o2Cfg.barOffsetY = 130;
  o2Cfg.labelAlign = LV_ALIGN_TOP_LEFT; o2Cfg.labelOffsetX = 10; o2Cfg.labelOffsetY = 128;
  o2Cfg.r = 0; o2Cfg.g = 180; o2Cfg.b = 220;
  o2Cfg.label = "O2";
  footO2.create(footPanel, o2Cfg);

  // ---- overlay ----
  overlay = lv_label_create(lv_layer_top());
  lv_obj_set_style_text_color(overlay, lv_color_make(255, 60, 60), 0);
  lv_obj_set_style_text_font(overlay, &lv_font_montserrat_14, 0);
  lv_label_set_text(overlay, "");
  lv_obj_add_flag(overlay, LV_OBJ_FLAG_HIDDEN);

  // Initial state: dimmed vehicle panel with "NO SIGNAL" until the first frame arrives.
  g_mode = HUD_MODE_NO_SIGNAL;
  applyMode(g_mode, -1);
  g_appliedMode = g_mode;
  g_appliedMismatch = -1;

  sendHello();
  Serial.println("HUD ready. Waiting for telemetry...");
}

uint32_t g_lastSmoothMs = 0;

void loop() {
  lv_tick_inc(millis() - lastTick);
  lastTick = millis();
  lv_timer_handler();

  // Feed serial bytes to the frame decoder
  while (Serial.available()) {
    decoder.feed(Serial.read());
    if (decoder.available()) {
      uint8_t payload[HUD_MAX_PAYLOAD];
      int len = decoder.payload(payload, sizeof(payload));
      g_lastFrameMs = millis();
      g_anyFrame = true;
      switch (decoder.msgType()) {
        case MSG_TELEMETRY: handleTelemetry(payload, len); break;
        case MSG_STATUS: handleStatus(payload, len); break;
        case MSG_VITALS: handleVitals(payload, len); break;
        case MSG_LINK: {
          LinkMsg l;
          if (hud_decode_link(payload, len, &l)) {
            g_linkSeen = true;
            g_linkState = l.state;
          }
          break;
        }
        case MSG_HELLO: {
          HelloMsg h;
          if (hud_decode_hello(payload, len, &h) && h.role == ROLE_SENDER) g_senderVersion = h.version;  // never reply
          break;
        }
        case MSG_VEHICLE_NAME: handleVehicleName(payload, len); break;
        default: break;  // unknown types and ActionAck: ignored
      }
      decoder.clear();
    }
  }

  uint32_t now = millis();

  // Mode
  HudMode m = hudComputeMode(now, g_lastFrameMs, g_anyFrame, g_linkSeen, g_linkState);
  if (m == HUD_MODE_NO_SIGNAL && g_mode != HUD_MODE_NO_SIGNAL) {
    g_linkSeen = false;     // stale alarms must not persist
    g_statusValid = false;
  }
  g_mode = m;
  int mismatch = (g_senderVersion >= 0 && g_senderVersion != HUD_PROTO_VERSION) ? g_senderVersion : -1;
  if ((int)g_mode != g_appliedMode || mismatch != g_appliedMismatch) {
    applyMode(g_mode, mismatch);
    g_appliedMode = g_mode;
    g_appliedMismatch = mismatch;
  }

  if (now - g_lastHelloMs >= 5000) sendHello();

  // Smoothing (>= 33 ms)
  uint32_t dt = now - g_lastSmoothMs;
  if (dt >= 33) {
    g_lastSmoothMs = now;
    for (int i = 0; i < 8; i++) {
      if (!g_chanValid[i]) continue;
      sm[i].tick(dt);
      widgetSetValue(i, sm[i].value());  // widgets cache and drop repeats
    }
  }

  // Alert
  uint8_t ann[10];
  for (int i = 0; i < 10; i++) ann[i] = g_statusValid ? hud_status_ann(&g_status, i) : ANN_UNKNOWN;
  AlertSeverity sev;
  bool blink;
  hudComputeAlert(ann, g_statusValid, g_shieldVal, g_shieldValid, &sev, &blink);
  if (g_mode != HUD_MODE_VEHICLE) sev = ALERT_NONE;
  alert.setSeverity(sev, blink);
  alert.tick(now);

  delay(5);
}
