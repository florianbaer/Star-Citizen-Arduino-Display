// Host simulator for ship_hud.ino: compiles the real sketch against stub Arduino/TFT/Wire headers,
// feeds it protocol v2 frames over a fake Serial, and writes the framebuffer as a PPM image.
//
// Usage: hud_sim <scenario> <out.ppm>
// Scenarios: no_signal, vehicle, damaged, on_foot, no_game

#include "stubs/Arduino.h"
#include "stubs/TFT_eSPI.h"
#include "stubs/Wire.h"

uint32_t g_simMillis = 0;
uint16_t g_fb[240][320];
SimSerial Serial;
SimWire Wire;

#include "../../ship_hud/ship_hud.ino"

#include <string>

static void sendFrame(uint8_t type, const void* payload, size_t len) {
  uint8_t buf[HUD_MAX_FRAME];
  size_t n = hud_frame_encode(type, (const uint8_t*)payload, len, buf, sizeof(buf));
  for (size_t i = 0; i < n; i++) Serial.rx.push_back(buf[i]);
}

struct Scenario {
  bool link = false;
  LinkMsg linkMsg{};
  bool telemetry = false;
  TelemetryMsg tel{};
  bool status = false;
  StatusMsg st{};
  bool vitals = false;
  VitalsMsg vit{};
  const char* name = nullptr;
};

static void annAll(StatusMsg& s, uint8_t state) {
  for (int i = 0; i < 5; i++) s.ann[i] = (uint8_t)(state | (state << 4));
}
static void annSet(StatusMsg& s, int idx, uint8_t state) {
  uint8_t& b = s.ann[idx >> 1];
  b = (idx & 1) ? (uint8_t)((b & 0x0F) | (state << 4)) : (uint8_t)((b & 0xF0) | state);
}

static void writePpm(const char* path) {
  FILE* f = fopen(path, "wb");
  if (!f) { perror(path); exit(1); }
  fprintf(f, "P6\n320 240\n255\n");
  for (int y = 0; y < 240; y++)
    for (int x = 0; x < 320; x++) {
      uint16_t c = g_fb[y][x];
      uint8_t rgb[3] = {(uint8_t)(((c >> 11) & 0x1F) * 255 / 31), (uint8_t)(((c >> 5) & 0x3F) * 255 / 63),
                        (uint8_t)((c & 0x1F) * 255 / 31)};
      fwrite(rgb, 1, 3, f);
    }
  fclose(f);
}

int main(int argc, char** argv) {
  if (argc < 3) { fprintf(stderr, "usage: hud_sim <scenario> <out.ppm> [capture_ms]\n"); return 2; }
  std::string sc = argv[1];
  uint32_t captureMs = argc > 3 ? (uint32_t)atoi(argv[3]) : 3000;

  Scenario s;
  if (sc == "vehicle" || sc == "damaged") {
    s.link = true; s.linkMsg = {LINK_IN_VEHICLE, SOURCE_SC};
    s.telemetry = true;
    s.status = true;
    annAll(s.st, ANN_STATE_NOMINAL);
    s.st.flags_valid = SF_QT_SPOOLED | SF_DECOUPLED | SF_IN_VEHICLE | SF_LANDED;
    s.name = "Aegis Gladius";
    if (sc == "vehicle") {
      s.tel = {235, 220, 228, 231, 190, 160, TV_ALL};
      s.st.flags = SF_IN_VEHICLE | SF_QT_SPOOLED;
      s.st.gear = 1;          // retracted
      s.st.master_mode = 1;   // NAV
    } else {
      s.tel = {40, 150, 12, 95, 70, 30, TV_ALL};
      s.st.flags = SF_IN_VEHICLE | SF_DECOUPLED;
      s.st.gear = 1;
      s.st.master_mode = 2;   // SCM
      annSet(s.st, ANN_SHLD, ANN_STATE_MAJOR_ON);
      annSet(s.st, ANN_HIT, ANN_STATE_MAJOR_ON);
    }
  } else if (sc == "on_foot") {
    s.link = true; s.linkMsg = {LINK_CONNECTED, SOURCE_SC};
    s.vitals = true; s.vit = {200, 140, VV_HEALTH | VV_OXYGEN};
  } else if (sc == "no_game") {
    s.link = true; s.linkMsg = {LINK_NO_GAME, SOURCE_SC};
  } else if (sc != "no_signal") {
    fprintf(stderr, "unknown scenario %s\n", sc.c_str());
    return 2;
  }

  setup();
  HelloMsg hello{HUD_PROTO_VERSION, ROLE_SENDER};
  if (s.link) sendFrame(MSG_HELLO, &hello, sizeof(hello));

  uint32_t lastSend = 0;
  bool first = true;
  while (g_simMillis < captureMs) {
    if (s.link && (first || g_simMillis - lastSend >= 100)) {
      first = false;
      lastSend = g_simMillis;
      sendFrame(MSG_LINK, &s.linkMsg, sizeof(s.linkMsg));
      if (s.telemetry) sendFrame(MSG_TELEMETRY, &s.tel, sizeof(s.tel));
      if (s.status) sendFrame(MSG_STATUS, &s.st, sizeof(s.st));
      if (s.vitals) sendFrame(MSG_VITALS, &s.vit, sizeof(s.vit));
      if (s.name) sendFrame(MSG_VEHICLE_NAME, s.name, strlen(s.name));
    }
    loop();
  }
  // Force a full refresh so the framebuffer reflects the final state.
  lv_obj_invalidate(lv_screen_active());
  lv_refr_now(NULL);
  writePpm(argv[2]);
  return 0;
}
