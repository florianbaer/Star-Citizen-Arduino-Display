# Integration Plan: Star Citizen gRPC Telemetry

Goal: replace the random-data sender with real game data from the Star Citizen
5.0 telemetry server (gRPC, `127.0.0.1:2942`), and optionally let the touch
screen send commands back to the game.

Reference: <https://github.com/NoNaNelson/StarCitizenTelemetryServerExample>
(PTU build 13238477, no `.proto` published — field numbers must be discovered).

```
 Star Citizen ──gRPC stream──▶ hud-sender (PC) ──COBS/serial──▶ ESP32 HUD
              ◀─TriggerGameAction──           ◀──action frames──  (touch)
```

---

## Phase 0 — Housekeeping (small, do first)

| # | Task | Files |
|---|------|-------|
| 0.1 | Fix GPIO 21 clash: `TFT_BL` should be `-1` (backlight is GPIO 27, driven manually) so it doesn't fight `TOUCH_INT`. | `docs/SETUP.md` |
| 0.2 | Delete unused `TelemetryParser.h` (old text protocol). | `lib/hud_widgets/`, `ship_hud/` |
| 0.3 | Add a `sync-headers` script (copy `lib/**` → `ship_hud/`) + CI/pre-commit check that they're identical. | `scripts/sync-headers.sh` |
| 0.4 | Add `platformio.ini` to the repo instead of only documenting it. | `platformio.ini` |
| 0.5 | Turn `proto/` + `sender-rs/` into a Cargo workspace. | `Cargo.toml` (root) |

**Done when:** the sketch builds via PlatformIO from a clean clone and `cargo test` passes at the root.

---

## Phase 1 — Schema discovery (blocker for everything after)

The stream only gives raw protobuf bytes. We know field *names* (`read_data.txt`)
but only one *path*: `response.6 → actor.1 → vitals.1 → health{1: cur f32, 2: max f32}`.

| # | Task | Notes |
|---|------|-------|
| 1.1 | Add `hud-sender probe` subcommand: connect, try **gRPC server reflection**; if available, dump all `FileDescriptorProto`s to `schema/`. | `tonic-reflection` client. Best case: full schema, skip 1.2–1.4. |
| 1.2 | Add `hud-sender dump --out frames.bin`: record raw `TelemetryStreamResponse` frames (length-prefixed) with timestamps. | No serial needed. |
| 1.3 | Add `hud-sender inspect frames.bin`: schemaless decode (field tree, wire types, float/varint interpretations) and a **diff mode** that highlights fields whose value changed between two timestamps. | Replaces `protoc --decode_raw`. |
| 1.4 | **In-game session (user):** record while doing scripted actions — sit in ship, power on, drop each shield face, burn H-fuel, spool QT, gear up/down, toggle decoupled, take damage, get out and walk. Note timestamps. | Needs PTU access. |
| 1.5 | Write the findings to `schema/fields.md` (+ hand-written `.proto` if possible): path, field number, type, unit/range. | Priority fields below. |
| 1.6 | Same for `TriggerGameAction` request (`action_name`, `hold_ms`): find method path + field numbers (reflection, or try `1`/`2`). | Only needed for Phase 5. |

Priority fields to map:

- `vehicle.state.shields.faces[]`, `shields.level`
- `vehicle.state.flight.instruments.hydrogenFuel`, `quantumFuel`
- `vehicle.state.annunciators.*` (pwr, wpn, thr, shld, cool, qdrv, radr, prox, hit, misl)
- `vehicle.state.flight.state.gear`, `isLanded`, `masterMode.mode`, `masterMode.spooled`
- `vehicle.state.name` (to detect "in a vehicle")
- `actor.vitals.health`, `oxygen`
- nice-to-have: `speed`, `gMeter.current`, `boost.reserves`, `vehicle.state.health`

**Done when:** `schema/fields.md` lists number + type + observed range for every priority field.

**Risk:** PTU-only, schema can change per patch → keep field numbers in one Rust module, log a warning when expected fields go missing.

---

## Phase 2 — Live SC source in the sender

| # | Task | Files |
|---|------|-------|
| 2.1 | Refactor sender around a `Source` trait (`next() -> Snapshot`) with `RandomSource` (existing logic) and `ScSource`. CLI: `hud-sender run COM6 --source sc\|random [--grpc 127.0.0.1:2942]`. | `sender-rs/src/source/{mod,random,sc}.rs` |
| 2.2 | `ScSource`: tonic channel + passthrough byte codec (or `prost` types if reflection worked), parse into an internal `Snapshot` struct (floats, Option per field). | `sender-rs/src/sc/{client,decode}.rs` |
| 2.3 | Normalisation layer `Snapshot → wire messages`: floats → 0–255, shield face mapping (see below), missing → "no data" flag. Unit-tested with recorded `frames.bin` fixtures. | `sender-rs/src/mapping.rs`, `sender-rs/tests/` |
| 2.4 | Reconnect loop: game not running / restarting → retry with backoff, report state to the HUD (Phase 3 `Link` message). | |
| 2.5 | Send policy: on change, capped at 30 Hz, plus 2 Hz keepalive. Move smoothing to the ESP32 (Phase 4) so it's done once. | |

Shield face mapping (ships have 1, 2, 4 or 6 faces):

| Faces | FWD | STB | AFT | PRT |
|-------|-----|-----|-----|-----|
| 1 | f0 | f0 | f0 | f0 |
| 2 | f0 | min(f0,f1) | f1 | min(f0,f1) |
| 4 | per game order (verify in 1.4) | | | |
| 6 | front | right | back | left (top/bottom folded into min of sides, or new widget) |

**Done when:** sitting in a ship on PTU, the HUD shows real shields/fuel, and killing the game makes the sender reconnect cleanly when it returns.

---

## Phase 3 — Protocol v2

Extend `proto/src/messages.rs` and `lib/hud_proto/messages.h` together (one change, one commit, round-trip tests in Rust).

| ID | Name | Dir | Payload |
|----|------|-----|---------|
| `0x01` | Telemetry | PC→HUD | unchanged (6 B) |
| `0x02` | Status | PC→HUD | `u16 annunciators` bitfield, `u8 flags` (gear, landed, decoupled, qt_spooled, in_vehicle), `u8 master_mode` |
| `0x03` | Vitals | PC→HUD | `u8 health`, `u8 oxygen` (0–255) |
| `0x04` | Link | PC→HUD | `u8 state` (0 = no game, 1 = connected/no vehicle, 2 = in vehicle) |
| `0x05` | Hello | both | `u8 proto_version` — HUD shows "update sender" on mismatch |
| `0x10` | Action | HUD→PC | `u8 action_id` (Phase 5) |

Rules: payloads only grow at the end; decoders accept longer payloads than they know (forward compatible).

**Done when:** Rust tests cover every message; ESP32 ignores unknown types without breaking.

---

## Phase 4 — HUD firmware updates

| # | Task |
|---|------|
| 4.1 | Handle `Status`: `AlertIndicator` driven by annunciators (shld/hit/misl = critical blink; others = steady amber) instead of the `shield < 85` heuristic. Small icon row for gear / QT / decoupled. |
| 4.2 | Link watchdog: no frame for 2 s → dim gauges + "NO SIGNAL" label; `Link=1` → "ON FOOT" screen. |
| 4.3 | On-foot screen (LVGL screen #2): health + O₂ bars from `Vitals`; auto-switch on `in_vehicle`. |
| 4.4 | Value smoothing on device (exponential, ~150 ms) — removes the need for 60 Hz sender traffic. |
| 4.5 | Send `Hello` at boot, show version mismatch. |

**Done when:** walking in/out of a ship switches screens automatically and alerts match the in-game annunciators.

---

## Phase 5 — Touch → game actions (optional)

| # | Task |
|---|------|
| 5.1 | Sender reads serial too (split read/write threads or async), decodes `Action` frames. |
| 5.2 | Action table: `action_id → action_name (+hold_ms)` in one shared place, e.g. `1 = v_power_toggle_shields`, `2 = v_toggle_landing_system`, `3 = v_toggle_qdrive_engagement`, `4 = v_ifcs_vector_decoupling_toggle`, `5 = v_invoke_ping`. |
| 5.3 | Calls `TriggerGameAction` (method + fields from 1.6). Blocklist dangerous actions (`v_self_destruct`, `v_eject`) — never mappable. |
| 5.4 | HUD: tap zones / buttons (e.g. tap shield gauge = toggle shields, tap ship = gear), with visual press feedback. Long-press for anything destructive-ish. |

**Done when:** tapping on the HUD toggles gear / shields in-game with < 200 ms perceived latency.

---

## Phase 6 — Docs & release

- README: replace "Not yet working" with the `user.cfg` setup and `hud-sender run COM6 --source sc`.
- `docs/PROTOCOL.md` (message table from Phase 3), `schema/fields.md` (Phase 1).
- Note PTU/patch version tested against.

---

## Order & dependencies

```
Phase 0 ──▶ Phase 1 ──▶ Phase 2 ──▶ Phase 4
                 │          ▲
                 └──▶ Phase 3 ┘
                 └──(1.6)──▶ Phase 5
```

Phase 3 can be designed in parallel with Phase 1; its field contents are finalised once the schema is known.

## Open questions

1. Is gRPC reflection enabled on the game server? (decides 1.2–1.4 effort)
2. Does `hydrogenFuel` / `quantumFuel` come as 0–1, 0–100, or absolute units?
3. Shield face order for 4-face ships — matches FWD/AFT/L/R or not?
4. Is the telemetry server available on LIVE, or PTU-only for now?
5. Rust (tonic) vs. a Python bridge — plan assumes Rust to keep one codebase with the shared `proto` crate.
