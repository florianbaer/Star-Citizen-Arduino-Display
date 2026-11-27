#pragma once
#include <stdint.h>
#include <stddef.h>
#include <string.h>

// Protocol v2. Must match proto/src/messages.rs (see docs/PROTOCOL.md).
#define HUD_PROTO_VERSION 2
#define HUD_MAX_PAYLOAD 64

enum MsgType : uint8_t {
  MSG_TELEMETRY = 0x01,
  MSG_STATUS = 0x02,
  MSG_VITALS = 0x03,
  MSG_LINK = 0x04,
  MSG_HELLO = 0x05,
  MSG_VEHICLE_NAME = 0x06,
  MSG_ACTION = 0x10,
  MSG_ACTION_ACK = 0x11,
};

// Telemetry valid bits
#define TV_SHIELD_FRONT 0x01
#define TV_SHIELD_BACK 0x02
#define TV_SHIELD_LEFT 0x04
#define TV_SHIELD_RIGHT 0x08
#define TV_HYDROGEN 0x10
#define TV_QUANTUM 0x20
#define TV_ALL 0x3F

// Annunciator indices
#define ANN_PWR 0
#define ANN_WPN 1
#define ANN_THR 2
#define ANN_SHLD 3
#define ANN_COOL 4
#define ANN_QDRV 5
#define ANN_RADR 6
#define ANN_PROX 7
#define ANN_HIT 8
#define ANN_MISL 9
#define ANN_COUNT 10
#define ANN_UNKNOWN 15

// AnnunciatorState (game enum)
#define ANN_STATE_UNSPECIFIED 0
#define ANN_STATE_NOMINAL 1
#define ANN_STATE_OFF 2
#define ANN_STATE_MINOR_FLASHING 3
#define ANN_STATE_MINOR_ON 4
#define ANN_STATE_MAJOR_FLASHING 5
#define ANN_STATE_MAJOR_ON 6

// Status flags
#define SF_LANDED 0x01
#define SF_QT_SPOOLED 0x02
#define SF_DECOUPLED 0x04
#define SF_IN_VEHICLE 0x08
#define SF_LIGHTS 0x10

#define GEAR_UNKNOWN 0xFF
#define MASTER_MODE_UNKNOWN 0xFF

// Vitals valid bits
#define VV_HEALTH 0x01
#define VV_OXYGEN 0x02

// Link state / source
#define LINK_NO_GAME 0
#define LINK_CONNECTED 1
#define LINK_IN_VEHICLE 2
#define SOURCE_SC 0
#define SOURCE_RANDOM 1

// Hello role
#define ROLE_SENDER 0
#define ROLE_HUD 1

// ActionAck result
#define ACK_OK 0
#define ACK_UNKNOWN_ID 1
#define ACK_DISABLED 2
#define ACK_BLOCKED 3
#define ACK_RATE_LIMITED 4
#define ACK_GAME_REJECTED 5
#define ACK_RPC_ERROR 6
#define ACK_NOT_CONNECTED 7

#pragma pack(push, 1)
struct TelemetryMsg { uint8_t shield_front, shield_back, shield_left, shield_right, hydrogen_fuel, quantum_fuel, valid; };
struct StatusMsg    { uint8_t ann[5]; uint8_t flags; uint8_t flags_valid; uint8_t gear; uint8_t master_mode; };
struct VitalsMsg    { uint8_t health, oxygen, valid; };
struct LinkMsg      { uint8_t state, source; };
struct HelloMsg     { uint8_t version, role; };
struct ActionMsg    { uint8_t action_id, seq; };
struct ActionAckMsg { uint8_t action_id, seq, result, game_status; };
#pragma pack(pop)

static_assert(sizeof(TelemetryMsg) == 7, "TelemetryMsg size");
static_assert(sizeof(StatusMsg) == 9, "StatusMsg size");
static_assert(sizeof(VitalsMsg) == 3, "VitalsMsg size");
static_assert(sizeof(LinkMsg) == 2, "LinkMsg size");
static_assert(sizeof(HelloMsg) == 2, "HelloMsg size");
static_assert(sizeof(ActionMsg) == 2, "ActionMsg size");
static_assert(sizeof(ActionAckMsg) == 4, "ActionAckMsg size");

// Decode helpers: zero-fill *out, fail if len < MIN, copy min(len, sizeof), apply defaults.
static inline bool hud_decode_generic(const uint8_t* p, int len, int min_len, void* out, size_t out_size) {
  memset(out, 0, out_size);
  if (len < min_len) return false;
  size_t n = (size_t)len < out_size ? (size_t)len : out_size;
  memcpy(out, p, n);
  return true;
}

static inline bool hud_decode_telemetry(const uint8_t* p, int len, TelemetryMsg* out) {
  if (!hud_decode_generic(p, len, 6, out, sizeof(*out))) return false;
  if (len < 7) out->valid = TV_ALL;
  return true;
}
static inline bool hud_decode_status(const uint8_t* p, int len, StatusMsg* out) {
  return hud_decode_generic(p, len, 9, out, sizeof(*out));
}
static inline bool hud_decode_vitals(const uint8_t* p, int len, VitalsMsg* out) {
  return hud_decode_generic(p, len, 3, out, sizeof(*out));
}
static inline bool hud_decode_link(const uint8_t* p, int len, LinkMsg* out) {
  return hud_decode_generic(p, len, 1, out, sizeof(*out));
}
static inline bool hud_decode_hello(const uint8_t* p, int len, HelloMsg* out) {
  return hud_decode_generic(p, len, 1, out, sizeof(*out));
}
static inline bool hud_decode_action(const uint8_t* p, int len, ActionMsg* out) {
  return hud_decode_generic(p, len, 2, out, sizeof(*out));
}
static inline bool hud_decode_action_ack(const uint8_t* p, int len, ActionAckMsg* out) {
  return hud_decode_generic(p, len, 3, out, sizeof(*out));
}

/// Annunciator nibble extract (idx 0..9).
static inline uint8_t hud_status_ann(const StatusMsg* s, int idx) {
  uint8_t b = s->ann[idx >> 1];
  return (idx & 1) ? (uint8_t)(b >> 4) : (uint8_t)(b & 0x0F);
}

/// Copies at most min(len, 32, cap-1) bytes, replaces non-printable bytes with '?',
/// NUL-terminates, returns the length.
static inline int hud_decode_vehicle_name(const uint8_t* p, int len, char* out, int cap) {
  if (cap <= 0) return 0;
  int n = len < 32 ? len : 32;
  if (n > cap - 1) n = cap - 1;
  if (n < 0) n = 0;
  for (int i = 0; i < n; i++) out[i] = (p[i] >= 0x20 && p[i] <= 0x7E) ? (char)p[i] : '?';
  out[n] = '\0';
  return n;
}
