#include <stdint.h>
#include <stddef.h>
#include <string.h>
#include "hud_proto.h"
#include "HudState.h"
#include "Smoother.h"

extern "C" {

int hudc_decode_stream(const uint8_t* data, size_t len, uint8_t* types_out, uint8_t* payloads_out,
                       int* lens_out, int max_frames) {
  FrameDecoder dec;
  int n = 0;
  for (size_t i = 0; i < len; i++) {
    dec.feed(data[i]);
    if (dec.available()) {
      if (n < max_frames) {
        types_out[n] = dec.msgType();
        lens_out[n] = dec.payload(payloads_out + (size_t)n * HUD_MAX_PAYLOAD, HUD_MAX_PAYLOAD);
        n++;
      }
      dec.clear();
    }
  }
  return n;
}

size_t hudc_frame_encode(uint8_t type, const uint8_t* p, size_t len, uint8_t* out, size_t cap) {
  return hud_frame_encode(type, p, len, out, cap);
}

size_t hudc_sizeof(uint8_t t) {
  switch (t) {
    case MSG_TELEMETRY: return sizeof(TelemetryMsg);
    case MSG_STATUS: return sizeof(StatusMsg);
    case MSG_VITALS: return sizeof(VitalsMsg);
    case MSG_LINK: return sizeof(LinkMsg);
    case MSG_HELLO: return sizeof(HelloMsg);
    case MSG_VEHICLE_NAME: return 32;
    case MSG_ACTION: return sizeof(ActionMsg);
    case MSG_ACTION_ACK: return sizeof(ActionAckMsg);
    default: return 0;
  }
}

int hudc_decode_telemetry(const uint8_t* p, int len, uint8_t out7[7]) {
  TelemetryMsg m;
  bool ok = hud_decode_telemetry(p, len, &m);
  memcpy(out7, &m, 7);
  return ok;
}

int hudc_decode_status(const uint8_t* p, int len, uint8_t ann10[10], uint8_t* flags, uint8_t* flags_valid,
                       uint8_t* gear, uint8_t* mm) {
  StatusMsg m;
  bool ok = hud_decode_status(p, len, &m);
  for (int i = 0; i < 10; i++) ann10[i] = hud_status_ann(&m, i);
  *flags = m.flags;
  *flags_valid = m.flags_valid;
  *gear = m.gear;
  *mm = m.master_mode;
  return ok;
}

int hudc_decode_vitals(const uint8_t* p, int len, uint8_t out3[3]) {
  VitalsMsg m;
  bool ok = hud_decode_vitals(p, len, &m);
  memcpy(out3, &m, 3);
  return ok;
}

int hudc_decode_link(const uint8_t* p, int len, uint8_t out2[2]) {
  LinkMsg m;
  bool ok = hud_decode_link(p, len, &m);
  memcpy(out2, &m, 2);
  return ok;
}

int hudc_decode_hello(const uint8_t* p, int len, uint8_t out2[2]) {
  HelloMsg m;
  bool ok = hud_decode_hello(p, len, &m);
  memcpy(out2, &m, 2);
  return ok;
}

int hudc_decode_action_ack(const uint8_t* p, int len, uint8_t out4[4]) {
  ActionAckMsg m;
  bool ok = hud_decode_action_ack(p, len, &m);
  memcpy(out4, &m, 4);
  return ok;
}

int hudc_decode_vehicle_name(const uint8_t* p, int len, char* out, int cap) {
  return hud_decode_vehicle_name(p, len, out, cap);
}

// ---- Phase 4 additions (HudState.h / Smoother.h) ----

uint8_t hudc_compute_mode(uint32_t now, uint32_t last_frame, int any_frame, int link_seen, uint8_t link_state) {
  return (uint8_t)hudComputeMode(now, last_frame, any_frame != 0, link_seen != 0, link_state);
}

void hudc_compute_alert(const uint8_t ann10[10], int status_valid, const uint8_t shields4[4], uint8_t shield_valid_mask,
                        uint8_t* severity, uint8_t* blink) {
  AlertSeverity s;
  bool b;
  hudComputeAlert(ann10, status_valid != 0, shields4, shield_valid_mask, &s, &b);
  *severity = (uint8_t)s;
  *blink = b ? 1 : 0;
}

void* hudc_smoother_new(float tau_ms) { return new Smoother(tau_ms); }
void hudc_smoother_free(void* p) { delete static_cast<Smoother*>(p); }
void hudc_smoother_set(void* p, uint8_t t) { static_cast<Smoother*>(p)->setTarget(t); }
void hudc_smoother_snap(void* p, uint8_t v) { static_cast<Smoother*>(p)->snap(v); }
int hudc_smoother_tick(void* p, uint32_t dt_ms) { return static_cast<Smoother*>(p)->tick(dt_ms) ? 1 : 0; }
uint8_t hudc_smoother_value(void* p) { return static_cast<Smoother*>(p)->value(); }

}  // extern "C"
