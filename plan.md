# Implementation Plan: Star Citizen gRPC Telemetry → ESP32 HUD

Status: reviewed (see `## Review notes (Fable)` at the end); ready for implementation. Offline-testable only; nothing here has been verified against the live game.
Basis: `docs/PLAN.md` (high-level roadmap). This file is the binding spec; where they disagree, this file wins.
Audience: an implementer **without access to the game**. Everything below can be built and tested offline.

---

## 0. Read this first

### 0.1 What changed since docs/PLAN.md was written (important)

`docs/PLAN.md` assumed only one field path was known. That is no longer true. A **community-extracted
schema** now exists: the game binary embeds its own protobuf descriptor, and the tool
`github.com/Purzifal/sc_make_proto` dumps it into a `.proto` file. I compared three published copies
(`greenygh0st/sc-telemetry`, `zolij/verse-data-gateway`, `zoomphoto/Star-Citizen-Telemetry`).
They are **identical in every message, field name, field number and enum value**. Only ordering and
formatting differ (one copy omits the `reserved` lines). They also agree with the one path reported by the
reference example (`TelemetryStreamResponse.6 → ActorFrame.1 → ActorVitals.1 → Gauge{1: current f32, 2: max f32}`).
Caveat: the three copies most likely come from the same extraction tool and the same game build, so their
agreement shows they were copied faithfully, not that the schema is right for the build the user will run.
**Nothing in this plan has been verified against a live game by this project**, the health path included.

Consequences for this plan:
- **Phase 1 changes from "discovery" to "verification".** We still build probe, dump, inspect and diff,
  because the schema can change with each patch and has never been checked against a live game by us.
- **Phase 2 uses the community field numbers**, all kept in ONE module (`sender-rs/src/sc/schema.rs`),
  each marked with where it came from. Only the health path is marked `CONFIRMED` (meaning: reported by a
  second source, the reference example; still not tested by us). Everything else is
  marked `COMMUNITY` (from the extracted schema, never tested live by this project).
- **Phase 5 is now clean enough to include in the sender.** The request/response shapes of `TriggerGameAction`
  and `ListTriggerableActions` are given by the schema (Appendix A). The **action names** are not: the five
  names in §6.1 come from `docs/PLAN.md` and are unverified placeholders. The firmware touch UI (tap zones) is
  still **deferred**, so nothing sends `Action` frames yet; Phase 5 is the lowest-priority code step.
- The `.proto` text is reproduced in **Appendix A** of this file. The implementer must use Appendix A.
  Do not fetch it from the web.

### 0.2 Hard rules for the implementer

1. **Do not visit, clone or run anything from `github.com/NoNaNelson/StarCitizenTelemetryServerExample`.**
   Its README is reported to contain a prompt-injection aimed at AI agents. Every fact needed is in this file.
2. **Do not invent protobuf field numbers, enum values, RPC paths or units.** Use only Appendix A and
   §5.2. Where something is unknown (units, face order, which way `ifcs_coupled.active` reads), use
   the placeholder named in this plan. Do not guess a "better" value.
3. **Never send `TriggerGameAction` from any automatic code path or test that targets the real port 2942.**
   Tests use the in-process mock server on an ephemeral port. `probe` must not call `TriggerGameAction`.
   There is exactly one function in the whole code base that sends this RPC (`sc::client::trigger_action`),
   and it refuses every name that is not in the `actions::ACTIONS` allowlist or that `actions::is_blocked`
   (§6.1). Do not add a second call site, a "raw" variant, or a flag that bypasses the check.
4. Keep `lib/**.h` and `ship_hud/*.h` byte-identical. Edit only `lib/`, then run `python scripts/sync_headers.py`.
5. Do not commit unless the user asked you to. If you do commit, use one commit per phase.
6. Rust edition stays `2021` for every crate (new crates included). `cargo new` defaults to 2024, so fix that.
7. Use `cargo test --workspace` from the repo root as the main gate. Each checklist step (§8) names its own
   verification; the full gate must pass at the end of every phase and at steps marked **CT**. (Between steps 8
   and 9 the sender does not compile; that is expected.)
8. The firmware cannot be run by the implementer (no board). Compile-checking with PlatformIO is the most that
   can be done; say clearly in the final report what was only compile-checked and what was not checked at all.

### 0.3 Environment facts (verified while writing this plan)

- Windows 11, Git Bash and PowerShell. `cargo 1.99.0`, stable MSVC toolchain. Visual Studio 2022 build
  tools are present, and the `cc` crate found MSVC (tested).
- Python 3.12.10. `protoc` is **not** installed and is **not needed**: we hand-write the prost structs.
- PlatformIO is **not** installed (see §8.6 for install and fallback).
- I prototyped the following against the exact pinned versions in §1.3, and they compile and run:
  a raw-bytes tonic codec, a server-streaming client call, a hand-rolled tonic server (`NamedService` + `Service`),
  the `tonic-reflection` v1 client (built with `default-features = false`), hand-written `#[derive(prost::Message)]`
  structs decoding the health frame, and a C++ header test compiled through `cc` and called from a Rust test.

---

## 1. Target layout and dependencies

### 1.1 Final repository layout (new = ★, changed = ✎, deleted = ✗)

```
Cargo.toml                         ★ workspace root
Cargo.lock                         ★ (moved; proto/Cargo.lock and sender-rs/Cargo.lock ✗)
.gitignore                         ✎ add /target/ and /schema/probe-*/
platformio.ini                     ★
include/lv_conf.h                  ★
.github/workflows/ci.yml           ★ (optional, see step 0.8)
scripts/sync_headers.py            ★
schema/sc_telemetry_community.proto★ Appendix A, verbatim
schema/fields.md                   ★ field table with a "verified live" column
docs/PROTOCOL.md                   ★ wire protocol v2
docs/SETUP.md                      ✎
README.md                          ✎
proto/                             hud-proto (no_std)
  Cargo.toml                       ✎ (edition stays 2021, add rust-version)
  src/lib.rs                       ✎
  src/cobs.rs                      (unchanged)
  src/crc8.rs                      (unchanged, add golden test)
  src/frame.rs                     ✎ raw msg_type in DeframedMsg, stream FrameDecoder
  src/messages.rs                  ✎ protocol v2
  tests/golden.rs                  ★
lib/hud_proto/
  cobs.h                           ✎ add cobs_encode
  crc8.h                           (unchanged)
  frame_decoder.h                  ✎ overflow flag, HUD_MAX_PAYLOAD
  frame_encoder.h                  ★
  messages.h                       ✎ protocol v2, #pragma pack, decode helpers
  hud_proto.h                      ✎ include frame_encoder.h
lib/hud_widgets/
  AlertIndicator.h                 ✎ severity + ESP32 core 2.x/3.x ledc compat
  FuelBar.h                        ✎ setUnknown, change-only updates
  ShieldGauge.h                    ✎ setUnknown, isKnown
  IndicatorLabel.h                 ★
  Smoother.h                       ★ (no LVGL dependency)
  HudState.h                       ★ (no LVGL dependency)
  TelemetryParser.h                ✗
  hud_widgets.h                    ✎
ship_hud/                          copies of all lib/**/*.h (managed by script) + ship_hud.ino ✎
tools/proto-ctest/                 ★ hud-proto-ctest: C/C++ ↔ Rust parity tests + header sync test
  Cargo.toml, build.rs, shim/shim.cpp, src/lib.rs, tests/c_parity.rs, tests/headers_sync.rs
sender-rs/                         hud-sender (lib + bin)
  Cargo.toml                       ✎
  src/main.rs                      ✎ thin: parse CLI, call lib
  src/lib.rs                       ★
  src/cli.rs                       ★
  src/serial.rs                    ✎ writer thread + reader thread
  src/snapshot.rs                  ★ Snapshot types
  src/mapping.rs                   ★ Snapshot → protocol messages
  src/policy.rs                    ★ send policy (on-change, rate cap, keepalive)
  src/runner.rs                    ★ `run` subcommand main loop
  src/actions.rs                   ★ action allowlist, blocklist (step 18); gate, executor (step 24)
  src/dumpfile.rs                  ★ SCTD recording format
  src/source/mod.rs, random.rs     ★
  src/sc/mod.rs                    ★
  src/sc/schema.rs                 ★ THE ONLY place with SC field numbers / RPC paths / unverified constants
  src/sc/codec.rs                  ★ RawCodec (Bytes passthrough)
  src/sc/client.rs                 ★ connect, stream, trigger, list
  src/sc/decode.rs                 ★ Accumulator: responses → Snapshot
  src/sc/source.rs                 ★ ScSource reconnect loop
  src/sc/mock.rs                   ★ in-process fake GameTelemetryService
  src/sc/probe.rs                  ★ reflection + first-frame probe
  src/pb/mod.rs, wire.rs, encode.rs, inspect.rs  ★ schemaless protobuf tools
  tests/e2e_client.rs, tests/e2e_sc.rs, tests/e2e_actions.rs, tests/inspect_cli.rs  ★
```

### 1.2 Root `Cargo.toml`

```toml
[workspace]
resolver = "2"
members = ["proto", "sender-rs", "tools/proto-ctest"]

[workspace.package]
edition = "2021"
rust-version = "1.88"
license = "GPL-3.0-only"
```

Delete `proto/Cargo.lock` and `sender-rs/Cargo.lock`. Run `cargo generate-lockfile` at the root; the root
`Cargo.lock` is a tracked file (commit it only if the user asks for commits, rule 0.2.5). Add `/target/` and
`/schema/probe-*/` to `.gitignore`. Keep the old `sender-rs/target/` and `proto/target/` lines, since they are harmless.

**Ordering:** a workspace member must exist on disk or every cargo command fails. In checklist step 1 list only
`members = ["proto", "sender-rs"]`. Add `"tools/proto-ctest"` in step 5, when that crate is created.

### 1.3 Pinned dependencies (verified to resolve together on cargo 1.99; tonic 0.14 needs Rust ≥ 1.88)

`sender-rs/Cargo.toml`:
```toml
[package]
name = "hud-sender"
version = "0.2.0"
edition.workspace = true
rust-version.workspace = true

[lib]
name = "hud_sender"
path = "src/lib.rs"

[[bin]]
name = "hud-sender"
path = "src/main.rs"

[dependencies]
hud-proto        = { path = "../proto" }
serialport       = "=4.10.1"
clap             = { version = "=4.6.10", features = ["derive"] }
rand             = "=0.8.5"          # keep 0.8 API (thread_rng/gen); 0.9 renamed them
anyhow           = "=1.0.105"
tonic            = { version = "=0.14.9", default-features = false, features = ["transport", "codegen", "router"] }
tonic-reflection = { version = "=0.14.9", default-features = false }   # only the generated client is used
prost            = "=0.14.6"
prost-types      = "=0.14.6"
tokio            = { version = "=1.54.1", features = ["rt-multi-thread", "macros", "time", "sync", "net", "signal"] }
tokio-stream     = { version = "=0.1.19", features = ["net"] }
bytes            = "=1.12.2"

[dev-dependencies]
tempfile = "=3.27.0"
```
Notes:
- The `router` feature is **required** for `tonic::transport::Server::builder().add_service(..)`, which the mock server uses. Without it the method does not exist (verified).
- With `default-features = false`, `tonic-reflection` provides `tonic_reflection::pb::v1::server_reflection_client::ServerReflectionClient`
  (and the same under `pb::v1alpha`). `ServerReflectionClient::connect` is **not** available. Build a `Channel`
  yourself and use `ServerReflectionClient::new(channel)` (verified).
- `tonic-prost` is not needed directly. We use our own `RawCodec` and call `prost::Message::decode` ourselves.
- There is no direct `http` dependency. Use the re-export `tonic::codegen::http` wherever this plan writes `http::…`.
- `hud_proto::messages::Message` and `prost::Message` have the same name. In any file that needs both, import
  them as `use prost::Message as _;` and/or `use hud_proto::messages::Message as WireMessage;`.
- The reviewer re-checked this exact dependency set on cargo 1.99 (it resolves and compiles; a raw-codec client,
  the hand-rolled mock service and the reflection client ran). The compiled skeleton is in **Appendix D**; start from it.

`tools/proto-ctest/Cargo.toml`:
```toml
[package]
name = "hud-proto-ctest"
version = "0.1.0"
edition.workspace = true
publish = false

[dependencies]
hud-proto = { path = "../../proto" }

[build-dependencies]
cc = "=1.12.0"
```

`proto/Cargo.toml`: keep it as is, except **replace** the line `edition = "2021"` with `edition.workspace = true`
and add `rust-version.workspace = true` (having both `edition = …` and `edition.workspace = …` is a manifest error).

---

## 2. Phase 0 — Housekeeping

### 0.1 Workspace
As described in §1.2. `sender-rs` builds as before (still the old random sender until Phase 2).

### 0.2 `docs/SETUP.md` fixes
- In the Arduino `User_Setup.h` block, **delete** the line `#define TFT_BL   21` (the new `platformio.ini` has no
  `-DTFT_BL` flag either). The backlight is GPIO 27, which the sketch drives itself; GPIO 21 is `TOUCH_INT`.
  Add one sentence saying so. (Background: TFT_eSPI only drives `TFT_BL` when `TFT_BACKLIGHT_ON` is also defined,
  which this setup never does, so the old value was a latent trap rather than an active fault. Leaving the macro
  undefined removes the trap. Do not define `TFT_BACKLIGHT_ON`.)
- Replace the inline `lv_conf.h` block with: "Copy `include/lv_conf.h` from this repo next to the lvgl library folder."
- Replace the inline `platformio.ini` block with: "The repo ships `platformio.ini`; run `pio run`."
- Add a note on ESP32 core versions: Arduino IDE instructions say core **3.2.0**, while PlatformIO
  `espressif32@6.10.0` uses Arduino core **2.0.x**. `AlertIndicator.h` now supports both (step 0.4).
- Add a "Header copies" note: after editing anything in `lib/`, run `python scripts/sync_headers.py`.

### 0.3 Delete `TelemetryParser.h`
Delete it from both `lib/hud_widgets/` and `ship_hud/`. Nothing includes it. `hud_widgets.h` does not reference it; check with grep.

### 0.4 `AlertIndicator.h`: ledc API compatibility
`ledcSetup` and `ledcAttachPin` were removed in arduino-esp32 3.x. Wrap them:
```cpp
#if defined(ESP_ARDUINO_VERSION_MAJOR) && ESP_ARDUINO_VERSION_MAJOR >= 3
  ledcAttach(cfg.ledPin, 5000, 8);          // 3.x: channel picked automatically, keyed by pin
  // ledSet(): ledcWrite(_cfg.ledPin, duty);
#else
  ledcSetup(cfg.ledChannel, 5000, 8);
  ledcAttachPin(cfg.ledPin, cfg.ledChannel);
  // ledSet(): ledcWrite(_cfg.ledChannel, duty);
#endif
```
`ledSet()` must use the same `#if`. Add `#include <esp_arduino_version.h>` under `#include <Arduino.h>`.
(PlatformIO only checks the 2.x branch. Say so in the final report.) Phase 4 makes the severity changes to this same file.

### 0.5 `scripts/sync_headers.py`
Python 3, stdlib only, runs the same from Git Bash and PowerShell.
```
python scripts/sync_headers.py           # copy lib/hud_proto/*.h and lib/hud_widgets/*.h → ship_hud/
python scripts/sync_headers.py --check   # exit 1 if any copy differs or is missing, or a stale ship_hud/*.h exists
```
Behaviour:
- Sources: `lib/hud_proto/*.h` and `lib/hud_widgets/*.h`, non-recursive. Destination: `ship_hud/<basename>`.
- If the same basename appears in both lib dirs, fail with exit 2 and print both paths.
- Stale file: a `ship_hud/*.h` whose basename is not in either lib dir. In copy mode, delete it and print `removed`. In check mode, report it and exit 1.
- Compare and copy **bytes** (`shutil.copyfile`; compare with `filecmp.cmp(shallow=False)`). Do not touch line endings.
- Print one line per file: `ok`, `updated`, `missing`, `differs`, `stale`, `removed`. The last line is a summary.
- Resolve paths relative to the script's own location (`Path(__file__).resolve().parent.parent`), not the cwd.

### 0.6 PlatformIO files
`platformio.ini` (repo root):
```ini
[platformio]
src_dir = ship_hud
default_envs = esp32

[env:esp32]
platform = espressif32@6.10.0
board = esp32dev
framework = arduino
monitor_speed = 115200
upload_speed = 921600
lib_deps =
    lvgl/lvgl@9.2.2
    bodmer/TFT_eSPI@2.5.43
build_flags =
    -I include
    -DLV_CONF_INCLUDE_SIMPLE
    -DUSER_SETUP_LOADED
    -DILI9341_2_DRIVER
    -DTFT_WIDTH=240
    -DTFT_HEIGHT=320
    -DTFT_MISO=12
    -DTFT_MOSI=13
    -DTFT_SCLK=14
    -DTFT_CS=15
    -DTFT_DC=2
    -DTFT_RST=-1
    -DUSE_HSPI_PORT
    -DLOAD_GLCD
    -DLOAD_FONT2
    -DLOAD_FONT4
    -DLOAD_FONT6
    -DLOAD_FONT7
    -DLOAD_FONT8
    -DLOAD_GFXFF
    -DSMOOTH_FONT
    -DSPI_FREQUENCY=55000000
    -DSPI_READ_FREQUENCY=20000000
    -DSPI_TOUCH_FREQUENCY=2500000
```
`src_dir = ship_hud`: the sketch's quoted includes resolve to the `ship_hud/` copies first, so PlatformIO
and Arduino IDE compile the same headers. `lib/` is header-only, so PlatformIO picking it up as libraries
is harmless.

`include/lv_conf.h`: the existing SETUP.md content, with two changes: `#define LV_USE_TFT_ESPI 0`
(the sketch has its own flush callback, and the LVGL TFT_eSPI driver would need extra library discovery) and
`#define LV_MEM_SIZE (64U * 1024U)` (was 48 KiB; Phase 4 adds about 15 objects and nobody can measure heap use
without the board, so buy headroom; it is static RAM and fits easily). Change nothing else in that file.
Keep `LV_FONT_MONTSERRAT_14 1` as the only font. Phase 4 uses only that font.

### 0.7 `tools/proto-ctest` crate (skeleton now, filled in Phase 3)
- `build.rs`:
  ```rust
  fn main() {
      println!("cargo:rerun-if-changed=shim/shim.cpp");
      println!("cargo:rerun-if-changed=../../lib/hud_proto");
      println!("cargo:rerun-if-changed=../../lib/hud_widgets");
      cc::Build::new()
          .cpp(true)
          .std("c++17")
          .file("shim/shim.cpp")
          .include("../../lib/hud_proto")
          .include("../../lib/hud_widgets")
          .warnings(true)
          .compile("hudshim");
  }
  ```
- The shim may include only headers that do not depend on Arduino or LVGL: `hud_proto.h` (everything under `lib/hud_proto`), `HudState.h`, `Smoother.h`.
- `tests/headers_sync.rs`: the same rules as `sync_headers.py --check`, implemented in Rust (read files under
  `env!("CARGO_MANIFEST_DIR")/../../lib/...` and `/../../ship_hud/`). Assert byte equality, no missing files, and no
  stale files. The failure message must say: `run: python scripts/sync_headers.py`.
  This makes `cargo test --workspace` enforce the Arduino copy constraint, so no hook is required.

### 0.8 CI (skip unless the user asks; the implementer cannot verify that it runs)
`.github/workflows/ci.yml` with two jobs on `ubuntu-latest`:
(a) `cargo test --workspace` (ubuntu has g++, so the shim builds) plus `python3 scripts/sync_headers.py --check`;
(b) `pip install platformio && pio run -e esp32`.

**Phase 0 done when:** `cargo test --workspace` passes at the root, `python scripts/sync_headers.py --check` exits 0, and (if pio is available) `python -m platformio run -e esp32` succeeds with the **unchanged** sketch.

---

## 3. Phase 3 — Protocol v2 (implement before Phases 1 and 2; both depend on it)

### 3.1 Wire rules (unchanged framing)
- Frame: `0x00 | COBS( msg_type:u8 | payload | crc8 ) | 0x00`. CRC8 is poly 0x31, init 0x00, MSB-first, no reflection,
  computed over `msg_type | payload`. The existing name "CRC8/MAXIM" is inaccurate. Keep the code as is and fix the comments to say "CRC-8 poly 0x31 init 0x00 (non-reflected)".
- `MAX_PAYLOAD = 64`. Multi-byte integers are little-endian (none are used in v2).
- **Forward compatibility:** payloads only ever grow at the end. Decoders accept payloads longer than they know
  (extra bytes are ignored) and give documented defaults to missing trailing optional fields.
  Decoders ignore unknown `msg_type` values without error.
- `PROTO_VERSION = 2`.

### 3.2 Message table

| ID | Name | Dir | Size (min/max) | Layout (byte offsets) |
|----|------|-----|----------------|-----------------------|
| 0x01 | Telemetry | PC→HUD | 6 / 7 | 0 shield_front, 1 shield_back, 2 shield_left, 3 shield_right, 4 hydrogen_fuel, 5 quantum_fuel (all 0–255), 6 `valid` bitmask. If byte 6 is absent → `valid = 0x3F`. |
| 0x02 | Status | PC→HUD | 9 / 9 | 0–4 annunciator nibbles, 5 `flags`, 6 `flags_valid`, 7 `gear`, 8 `master_mode` |
| 0x03 | Vitals | PC→HUD | 3 / 3 | 0 health 0–255, 1 oxygen 0–255, 2 `valid` |
| 0x04 | Link | PC→HUD | 1 / 2 | 0 `state`, 1 `source` (absent → 0) |
| 0x05 | Hello | both | 1 / 2 | 0 `version`, 1 `role` (absent → 0) |
| 0x06 | VehicleName | PC→HUD | 0 / 32 | printable ASCII bytes, length = payload length |
| 0x10 | Action | HUD→PC | 2 / 2 | 0 `action_id`, 1 `seq` |
| 0x11 | ActionAck | PC→HUD | 3 / 4 | 0 `action_id`, 1 `seq`, 2 `result`, 3 `game_status` (absent → 0) |

Constants (identical names in Rust as `pub const` inside `messages.rs`, and in C as `#define`/`enum` inside `messages.h`):
- Telemetry `valid` bits: `TV_SHIELD_FRONT=0x01, TV_SHIELD_BACK=0x02, TV_SHIELD_LEFT=0x04, TV_SHIELD_RIGHT=0x08, TV_HYDROGEN=0x10, TV_QUANTUM=0x20, TV_ALL=0x3F`.
- Status annunciator index order (nibble i): `ANN_PWR=0, ANN_WPN=1, ANN_THR=2, ANN_SHLD=3, ANN_COOL=4, ANN_QDRV=5, ANN_RADR=6, ANN_PROX=7, ANN_HIT=8, ANN_MISL=9, ANN_COUNT=10`.
  Byte `k` (0..4) = `ann[2k] & 0x0F | (ann[2k+1] & 0x0F) << 4`.
  Nibble value = the game's `AnnunciatorState` enum (Appendix A: 0 UNSPECIFIED, 1 NOMINAL, 2 OFF, 3 MINOR_FLASHING, 4 MINOR_ON, 5 MAJOR_FLASHING, 6 MAJOR_ON).
  Raw values ≥ 15 are sent as 14. **`ANN_UNKNOWN = 15`** means "no annunciator data".
  Constants: `ANN_STATE_UNSPECIFIED=0, ANN_STATE_NOMINAL=1, ANN_STATE_OFF=2, ANN_STATE_MINOR_FLASHING=3, ANN_STATE_MINOR_ON=4, ANN_STATE_MAJOR_FLASHING=5, ANN_STATE_MAJOR_ON=6`.
- Status `flags` / `flags_valid` bits: `SF_LANDED=0x01, SF_QT_SPOOLED=0x02, SF_DECOUPLED=0x04, SF_IN_VEHICLE=0x08, SF_LIGHTS=0x10`.
- Status `gear`: the game's `VehicleComponentDeploymentState` (0 UNSPECIFIED, 1 RETRACTED, 2 DEPLOYING, 3 DEPLOYED, 4 RETRACTING), or **`GEAR_UNKNOWN = 0xFF`**.
- Status `master_mode`: the game's `VehicleMasterMode` (0 UNSPECIFIED, 1 NAVIGATION, 2 SCM, 3 STEALTH), or **`MASTER_MODE_UNKNOWN = 0xFF`**.
- Vitals `valid`: `VV_HEALTH=0x01, VV_OXYGEN=0x02`.
- Link `state`: `LINK_NO_GAME=0, LINK_CONNECTED=1` (game reachable, not in a vehicle), `LINK_IN_VEHICLE=2`. Link `source`: `SOURCE_SC=0, SOURCE_RANDOM=1`.
- Hello `role`: `ROLE_SENDER=0, ROLE_HUD=1`.
- ActionAck `result`: `ACK_OK=0, ACK_UNKNOWN_ID=1, ACK_DISABLED=2, ACK_BLOCKED=3, ACK_RATE_LIMITED=4, ACK_GAME_REJECTED=5, ACK_RPC_ERROR=6, ACK_NOT_CONNECTED=7`.
  `game_status` = the game's `TriggerGameActionStatus` raw value (Appendix A) when `result == ACK_GAME_REJECTED`, else 0.

### 3.3 Rust: `proto/src/messages.rs` (replace the file)
- Remove the `unsafe`/`#[repr(C, packed)]` approach. Use explicit byte encoding.
- API:
```rust
pub const PROTO_VERSION: u8 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MsgType { Telemetry = 0x01, Status = 0x02, Vitals = 0x03, Link = 0x04, Hello = 0x05,
                   VehicleName = 0x06, Action = 0x10, ActionAck = 0x11 }
impl MsgType { pub fn from_u8(v: u8) -> Option<Self>; pub fn name(self) -> &'static str; }

pub trait Message: Sized {
    const TYPE: MsgType;
    const MAX_SIZE: usize;   // bytes written by encode() (maximum, for VehicleName)
    const MIN_SIZE: usize;   // decode() returns None below this
    /// Writes the payload. Panics if out.len() < MAX_SIZE. Returns bytes written.
    fn encode(&self, out: &mut [u8]) -> usize;
    /// Ignores trailing extra bytes; fills documented defaults for missing optional trailing fields.
    fn decode(data: &[u8]) -> Option<Self>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TelemetryMsg { pub shield_front: u8, pub shield_back: u8, pub shield_left: u8,
                          pub shield_right: u8, pub hydrogen_fuel: u8, pub quantum_fuel: u8, pub valid: u8 }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatusMsg { pub annunciators: [u8; 10], pub flags: u8, pub flags_valid: u8, pub gear: u8, pub master_mode: u8 }
impl StatusMsg { pub const fn unknown() -> Self; } // all ANN_UNKNOWN, flags 0, flags_valid 0, gear/mm 0xFF
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VitalsMsg { pub health: u8, pub oxygen: u8, pub valid: u8 }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LinkMsg { pub state: u8, pub source: u8 }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HelloMsg { pub version: u8, pub role: u8 }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VehicleNameMsg { len: u8, bytes: [u8; 32] }
impl VehicleNameMsg {
    pub const MAX_LEN: usize = 32;
    /// Each char in 0x20..=0x7E is kept, any other char becomes b'?'; truncated to 32 bytes.
    pub fn from_str_lossy(s: &str) -> Self;
    pub fn as_str(&self) -> &str;   // always valid ASCII
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionMsg { pub action_id: u8, pub seq: u8 }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionAckMsg { pub action_id: u8, pub seq: u8, pub result: u8, pub game_status: u8 }
```
- `StatusMsg::encode` clamps each annunciator to `min(v, 15)` before packing (callers handle the 14/15 rule; see mapping).
- `VehicleNameMsg::decode`: takes `min(len, 32)` bytes and replaces any byte outside 0x20..=0x7E with `b'?'`.
- **Breaking change for callers:** `TelemetryMsg::as_bytes()` and `TelemetryMsg::SIZE` are removed. Update `frame.rs` tests.

### 3.4 Rust: `proto/src/frame.rs`
- Keep `frame(msg_type: MsgType, payload: &[u8], out: &mut [u8]) -> usize`. Add
  `debug_assert!(payload.len() <= MAX_PAYLOAD)` (it still truncates in release builds).
- Add `pub fn frame_message<M: Message>(msg: &M, out: &mut [u8]) -> usize` (encode into a `[u8; MAX_PAYLOAD]`, then `frame`).
- `DeframedMsg` becomes `{ pub msg_type: u8, pub payload: &'a [u8] }` plus `pub fn kind(&self) -> Option<MsgType>`.
  `deframe` no longer rejects unknown types (forward compatibility).
- Add a no_std stream decoder that mirrors the C one:
```rust
pub const MAX_ENCODED: usize = 72;
pub struct FrameDecoder { buf: [u8; MAX_ENCODED], pos: usize, synced: bool, overflow: bool, scratch: [u8; MAX_PAYLOAD + 2] }
impl FrameDecoder {
    pub const fn new() -> Self;
    /// Feed one byte. On 0x00: if synced && pos > 0 && !overflow, try deframe(buf[..pos]).
    /// Then reset pos/overflow and set synced = true. Bytes before the first 0x00 are discarded.
    /// A non-zero byte while synced: append if pos < MAX_ENCODED, else set overflow = true.
    pub fn feed(&mut self, byte: u8) -> Option<DeframedMsg<'_>>;
}
impl Default for FrameDecoder
```
Borrow-checker hint for `feed`: copy `pos`/`overflow` into locals, reset the fields, and only then
`return deframe(&self.buf[..n], &mut self.scratch)` (two different fields, so the borrows do not conflict).

### 3.5 Rust tests (`proto/tests/golden.rs` plus unit tests)
- One test per message: `frame_message(&msg)` equals the golden frame bytes in **Appendix B**, and `deframe` + `M::decode` round-trips.
- `TELEMETRY_V1` (6-byte payload) decodes with `valid == TV_ALL`.
- `decode` with extra trailing bytes (append `0xAA, 0xBB`) still succeeds and gives the same value.
- `decode` with fewer than `MIN_SIZE` bytes returns `None`, for every message.
- `deframe` of a frame with unknown type `0x7E` returns `Some` with `kind() == None`.
- `FrameDecoder`: feed the concatenation of all golden frames, with garbage text `b"Starting HUD...\r\n"` before
  the first frame and between two frames. Exactly the 10 golden messages come out, in order. Overflow test: feed one `0x00`
  (so the decoder is synced), then 100 non-zero bytes, then a valid golden frame: the overflowed junk is discarded and
  exactly one message (the valid one) comes out. (Without the leading `0x00` the junk is dropped by the "not yet synced"
  rule and the overflow flag is never exercised.)
- crc8: assert `crc8(&[0x01,0xFF,0x80,0x40,0x20,0xC8,0x64,0x3F]) == 0x3C` (from golden `TELEMETRY_V2`, computed independently in Python).
  Do not add check values from memory (for example the "123456789" check value).

### 3.6 C: `lib/hud_proto/messages.h` (replace)
- `#define HUD_PROTO_VERSION 2`, `#define HUD_MAX_PAYLOAD 64`, `enum MsgType : uint8_t { MSG_TELEMETRY=0x01, MSG_STATUS=0x02, MSG_VITALS=0x03, MSG_LINK=0x04, MSG_HELLO=0x05, MSG_VEHICLE_NAME=0x06, MSG_ACTION=0x10, MSG_ACTION_ACK=0x11 };`
- All constants from §3.2 as `#define` with the same names.
- Structs inside `#pragma pack(push, 1)` / `#pragma pack(pop)`. **Do not use `__attribute__((packed))`**, because MSVC (used by the parity tests) rejects it.
  ```cpp
  struct TelemetryMsg { uint8_t shield_front, shield_back, shield_left, shield_right, hydrogen_fuel, quantum_fuel, valid; };
  struct StatusMsg    { uint8_t ann[5]; uint8_t flags; uint8_t flags_valid; uint8_t gear; uint8_t master_mode; };
  struct VitalsMsg    { uint8_t health, oxygen, valid; };
  struct LinkMsg      { uint8_t state, source; };
  struct HelloMsg     { uint8_t version, role; };
  struct ActionMsg    { uint8_t action_id, seq; };
  struct ActionAckMsg { uint8_t action_id, seq, result, game_status; };
  ```
  plus `static_assert(sizeof(TelemetryMsg) == 7, "...")` for each struct (sizes 7, 9, 3, 2, 2, 2, 4).
- Decode helpers (`static inline`, return `bool`). Each one zero-fills `*out`, then fails if `len < MIN`, then copies `min(len, sizeof)` bytes, then applies the defaults:
  `hud_decode_telemetry` (len 6 → `valid = TV_ALL`), `hud_decode_status`, `hud_decode_vitals`, `hud_decode_link`, `hud_decode_hello`, `hud_decode_action`, `hud_decode_action_ack`.
  Also `static inline uint8_t hud_status_ann(const StatusMsg* s, int idx)` (nibble extract), and
  `static inline int hud_decode_vehicle_name(const uint8_t* p, int len, char* out, int cap)`, which copies at most `min(len, 32, cap-1)` bytes, replaces non-printable bytes with `'?'`, NUL-terminates, and returns the length.
- The existing field names of `TelemetryMsg` stay the same, so the sketch's member accesses keep compiling.

### 3.7 C: `cobs.h`, `frame_encoder.h`, `frame_decoder.h`
- `cobs.h`: add `static inline size_t cobs_encode(const uint8_t* src, size_t len, uint8_t* dst, size_t cap)`, using the same algorithm as `proto/src/cobs.rs::encode`.
  Return 0 if `cap < len + len/254 + 1`.
- `frame_encoder.h` (new): `static inline size_t hud_frame_encode(uint8_t type, const uint8_t* payload, size_t len, uint8_t* out, size_t cap)`.
  It builds `type|payload|crc` on the stack (fails with 0 if `len > HUD_MAX_PAYLOAD`), writes `out[0]=0`, COBS-encodes into `out+1`, and appends `0`.
  Returns the total length, or 0 on overflow. Add `#define HUD_MAX_FRAME (HUD_MAX_PAYLOAD + 2 + 1 + 2 + 1)`, which evaluates to 70.
  The true worst case is 69 bytes (66 raw bytes → 67 COBS bytes → + 2 delimiters), which is what the Rust `MAX_FRAME`
  evaluates to; 70 is a safe upper bound. Do not assert that the two constants are equal.
  `messages.h` needs `#include <string.h>` (memcpy/memset) and `#include <stddef.h>`.
- `frame_decoder.h`: add an `_overflow` flag with the same semantics as the Rust `FrameDecoder`. `BUF_SIZE` stays 72. Set `DECODE_SIZE = HUD_MAX_PAYLOAD + 2 + 2`.
- `hud_proto.h`: also include `frame_encoder.h`.

### 3.8 C ↔ Rust parity tests (`tools/proto-ctest`)
`shim/shim.cpp` exposes `extern "C"` functions. `src/lib.rs` declares them and wraps each in a safe Rust fn:
```c
int    hudc_decode_stream(const uint8_t* data, size_t len, uint8_t* types_out, uint8_t* payloads_out /* max_frames*64 */,
                          int* lens_out, int max_frames);          // fresh FrameDecoder; returns frames decoded
size_t hudc_frame_encode(uint8_t type, const uint8_t* p, size_t len, uint8_t* out, size_t cap);
size_t hudc_sizeof(uint8_t msg_type);                              // sizeof(struct) or 0 (VehicleName → 32)
int    hudc_decode_telemetry(const uint8_t* p, int len, uint8_t out7[7]);
int    hudc_decode_status(const uint8_t* p, int len, uint8_t ann10[10], uint8_t* flags, uint8_t* flags_valid, uint8_t* gear, uint8_t* mm);
int    hudc_decode_vitals(const uint8_t* p, int len, uint8_t out3[3]);
int    hudc_decode_link(const uint8_t* p, int len, uint8_t out2[2]);
int    hudc_decode_hello(const uint8_t* p, int len, uint8_t out2[2]);
int    hudc_decode_action_ack(const uint8_t* p, int len, uint8_t out4[4]);
int    hudc_decode_vehicle_name(const uint8_t* p, int len, char* out, int cap);
// Phase 4 additions (HudState.h / Smoother.h):
uint8_t hudc_compute_mode(uint32_t now, uint32_t last_frame, int any_frame, int link_seen, uint8_t link_state);
void    hudc_compute_alert(const uint8_t ann10[10], int status_valid, const uint8_t shields4[4], uint8_t shield_valid_mask,
                           uint8_t* severity, uint8_t* blink);
void*   hudc_smoother_new(float tau_ms);  void hudc_smoother_free(void*);
void    hudc_smoother_set(void*, uint8_t target);  int hudc_smoother_tick(void*, uint32_t dt_ms);  uint8_t hudc_smoother_value(void*);
```
`tests/c_parity.rs`:
- For each fixed-size message: `hudc_sizeof(t) == M::MAX_SIZE`. This checks struct layout parity.
- Rust `frame_message` of each golden message → `hudc_decode_stream` → same type, same payload, and the C `hudc_decode_*` gives the same field values as the Rust struct.
- `hudc_frame_encode` of `HELLO_HUD`, `ACTION` and `ACTION_ACK` gives byte-identical output to Appendix B, and Rust `FrameDecoder` decodes them.
- Telemetry 6-byte payload via C gives `valid == 0x3F`. Status nibble extraction via C equals the Rust `annunciators`.
- Stream with garbage text and an overflow, same as the Rust test → same frames.

**Phase 3 done when:** `cargo test --workspace` passes, including `c_parity` and `headers_sync`. The sketch still compiles
(it only uses `TelemetryMsg` field names; change its payload buffer to `uint8_t payload[HUD_MAX_PAYLOAD]` and
use `hud_decode_telemetry`).

---

## 4. Phase 1 — Verification tooling (offline-testable, used live by the user later)

All subcommands live in the single `hud-sender` binary. The `--grpc` default is `http://127.0.0.1:2942`.
A value without a scheme gets `http://` prepended (`client::normalize_endpoint`).

### 4.1 Schemaless protobuf parser: `sender-rs/src/pb/wire.rs`
```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WireValue<'a> { Varint(u64), Fixed64(u64), Bytes(&'a [u8]), Fixed32(u32) }
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Field<'a> { pub number: u32, pub value: WireValue<'a> }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireError { Truncated, VarintOverflow, InvalidWireType(u8), InvalidFieldNumber }
pub fn read_varint(buf: &[u8], pos: &mut usize) -> Result<u64, WireError>;   // max 10 bytes
pub fn parse_message(buf: &[u8]) -> Result<Vec<Field<'_>>, WireError>;
```
Wire types 0, 1, 2 and 5 are supported. Groups (3/4) and 6/7 return `InvalidWireType`. A field number of 0 or > 536_870_911 returns `InvalidFieldNumber`.

### 4.2 Encoder helpers: `sender-rs/src/pb/encode.rs` (pub; used by tests, mock and the action request)
`put_varint(&mut Vec<u8>, u64)`, `put_tag(buf, field: u32, wire_type: u8)`, `put_varint_field(buf, field, u64)`,
`put_f32_field(buf, field, f32)` (wire type 5), `put_bytes_field(buf, field, &[u8])`, `put_str_field(buf, field, &str)`,
`put_packed_f32_field(buf, field, &[f32])`.

### 4.3 Recording format: `sender-rs/src/dumpfile.rs` ("SCTD v1")
- Header (8 bytes): `b"SCTD"`, `version: u8 = 1`, then 3 zero bytes.
- Records, repeated: `kind: u8` (0 = frame, 1 = marker), `ts_us: u64 LE` (µs since recording start), `len: u32 LE`, then `len` bytes
  (a raw `TelemetryStreamResponse` message for kind 0, UTF-8 text for kind 1).
```rust
pub enum Record { Frame { ts_us: u64, data: Vec<u8> }, Marker { ts_us: u64, text: String } }
pub struct DumpWriter<W: std::io::Write> { .. }
impl<W: Write> DumpWriter<W> { pub fn new(w: W) -> io::Result<Self>; pub fn write_frame(&mut self, ts_us: u64, data: &[u8]) -> io::Result<()>;
    pub fn write_marker(&mut self, ts_us: u64, text: &str) -> io::Result<()>; pub fn flush(&mut self) -> io::Result<()>; }
pub fn read_all<R: std::io::Read>(r: R) -> anyhow::Result<Vec<Record>>;
pub fn load_frames_or_raw(path: &Path) -> anyhow::Result<Vec<Record>>;  // no SCTD magic → whole file = one Frame{ts_us:0}
```
Errors: wrong magic (only in `read_all`), version ≠ 1, unknown kind, truncated record (the error message includes the byte offset), `len > 16 MiB`.
Tests: round-trip of mixed records via `tempfile`, truncated file → error, raw fallback.

### 4.4 Inspect / diff / summary: `sender-rs/src/pb/inspect.rs`
- Path type: `Vec<PathSeg>`, where `PathSeg { num: u32, idx: Option<u32> }`. It displays as `3.2.10.1`. The `[i]` suffix is added
  **only when that field number occurs more than once in the same parent** (index = occurrence order).
  Paths sort by comparing segments numerically.
- Interpreting a length-delimited value, in this order:
  1. If len ≥ 1 and every byte is printable ASCII (0x20..=0x7E) → **string**.
  2. Else if `parse_message` succeeds, consumes everything, and yields ≥ 1 field → **message** (recurse; max depth 16, then bytes).
  3. Else if valid UTF-8 → string. Else → **bytes**.
  Document in the code comment that this is a heuristic: packed repeated values and some bytes may be shown as a message.
- Tree line format (indent = 2 spaces × depth). Tests check with `contains`, so keep these tokens exact:
  - `"{path} msg ({n} B)"`
  - `"{path} varint {u} (zz {zigzag})"`
  - `"{path} fixed32 f32={f} u32={u}"` (Rust `{}` Display of `f32::from_bits`)
  - `"{path} fixed64 f64={f} u64={u}"`
  - `"{path} str \"{escaped}\""` (escape with `escape_debug`)
  - `"{path} bytes ({n} B) {hex of first 32 bytes, space-separated, uppercase}"`. If n > 32, append ` …`. If n % 4 == 0, append ` f32[]=[a, b, …]` (first 8 values).
  - If the path, with indices dropped, matches an entry in `sc::schema::PATH_NAMES`, append `  # {name}`.
- `flatten(frame) -> BTreeMap<PathKey, Leaf>` with `Leaf { display: String, num: Option<f64> }` (varint → u64 as f64,
  fixed32 → f32, fixed64 → f64, string/bytes → None). Messages are not leaves.
- `diff(a, b, ignore: &[String]) -> Vec<String>`. Output lines: `~ {path}: {old} -> {new}` (for numeric leaves append ` (Δ {new-old})`),
  `+ {path}: {new}`, `- {path}: {old}`. A path whose display string starts with any ignore prefix is skipped.
- `summary(records) -> String`: frame count, duration, a marker list (`[t=12.345s] near frame #57: gear up`, where the nearest frame is the
  first frame with ts ≥ marker ts), then one line per leaf path:
  `{path}  n={count}  min={min}  max={max}  distinct={d}  first={first}  last={last}  # name?`
  (`distinct` is capped at 1000).

### 4.5 gRPC plumbing: `sender-rs/src/sc/codec.rs`, `client.rs`
`codec.rs`: `RawCodec` / `RawEncoder` / `RawDecoder` with `Encode = Decode = bytes::Bytes`, exactly as below (verified):
```rust
impl Encoder for RawEncoder { type Item = Bytes; type Error = Status;
    fn encode(&mut self, item: Bytes, dst: &mut EncodeBuf<'_>) -> Result<(), Status> { dst.put_slice(&item); Ok(()) } }
impl Decoder for RawDecoder { type Item = Bytes; type Error = Status;
    fn decode(&mut self, src: &mut DecodeBuf<'_>) -> Result<Option<Bytes>, Status> { let n = src.remaining(); Ok(Some(src.copy_to_bytes(n))) } }
```
(`use bytes::{Buf, BufMut, Bytes}; use tonic::codec::{Codec, DecodeBuf, Decoder, EncodeBuf, Encoder};`)

`client.rs`:
```rust
pub fn normalize_endpoint(s: &str) -> String;
pub async fn connect(endpoint: &str, timeout: Duration) -> anyhow::Result<tonic::transport::Channel>;
    // Endpoint::from_shared(..)?.connect_timeout(timeout).http2_keep_alive_interval(10s)
    //   .keep_alive_timeout(5s).keep_alive_while_idle(true).connect().await
pub async fn open_telemetry_stream(ch: Channel) -> Result<tonic::Streaming<Bytes>, tonic::Status>;
    // tonic::client::Grpc::new(ch); grpc.ready().await (map err to Status::unavailable);
    // grpc.server_streaming(Request::new(Bytes::new()), PathAndQuery::from_static(schema::PATH_TELEMETRY_STREAM), RawCodec)
pub async fn trigger_action(ch: Channel, name: &str, hold_ms: Option<u32>) -> Result<schema::TriggerGameActionResponse, tonic::Status>;
pub async fn list_actions(ch: Channel) -> Result<Vec<schema::TriggerableAction>, tonic::Status>;
```
`trigger_action` and `list_actions` use `grpc.unary(.., RawCodec)`. They encode the request with `prost::Message::encode_to_vec` and decode the response with `prost::Message::decode`.
**Safety choke point:** the first statement of `trigger_action` is
`if !actions::is_allowed(name) { return Err(tonic::Status::permission_denied("action not in allowlist")); }`
and it runs before any network I/O. This is the only function that may send `PATH_TRIGGER_GAME_ACTION`.
Call `grpc.ready().await` on a fresh `tonic::client::Grpc::new(ch)` before every call (as in `open_telemetry_stream`).
Map a decode failure to `Status::internal`. `PathAndQuery` is `tonic::codegen::http::uri::PathAndQuery`.

### 4.6 Mock server: `sender-rs/src/sc/mock.rs` (library code; used by tests and the `mock-server` subcommand)
```rust
#[derive(Clone)]
pub struct MockConfig {
    pub frames: Vec<bytes::Bytes>,     // each = one TelemetryStreamResponse
    pub interval: Duration,
    pub repeat: bool,                  // loop forever, or end the stream after the last frame
    pub action_log: Arc<Mutex<Vec<(String, Option<u32>)>>>,
    pub trigger_status: i32,           // status returned by every TriggerGameAction; use 1 (OK) unless a test says otherwise
}
pub async fn serve(listener: tokio::net::TcpListener, cfg: MockConfig) -> anyhow::Result<()>;
pub fn synthetic_frames() -> Vec<bytes::Bytes>;  // see below
```
Implementation (pattern verified in a prototype):
- `struct MockService(Arc<MockConfig>)` implements `tonic::server::NamedService` (`NAME = "sc.game.telemetry.v1.GameTelemetryService"`)
  and `tonic::codegen::Service<http::Request<tonic::body::Body>>` (`Response = http::Response<tonic::body::Body>`, `Error = Infallible`).
  `call` dispatches on `req.uri().path()`:
  - `PATH_TELEMETRY_STREAM` → `tonic::server::Grpc::new(RawCodec).server_streaming(StreamSvc, req)`. `StreamSvc::call` spawns a task that sends frames
    through a `tokio::sync::mpsc::channel(16)`, sleeping `interval` between frames, and returns `ReceiverStream` (Box::pin) as the response stream.
    The task stops when `send` fails (client gone) or after the last frame if `!repeat`.
  - `PATH_TRIGGER_GAME_ACTION` → unary. Decode `TriggerGameActionRequest` and push it to `action_log`. Respond with
    `status = cfg.trigger_status` and an empty message.
  - `PATH_LIST_TRIGGERABLE_ACTIONS` → unary. Return the five names from `actions::ACTIONS` with category `"mock"` and `holdable: false`.
  - Anything else → `Ok(tonic::Status::unimplemented("").into_http())`. Reflection is therefore unimplemented, which is the code path probe must handle.
  - `http` here means `tonic::codegen::http`. Appendix D has this service compiled and running; copy its trait impls.
  - `actions::ACTIONS` is needed here and `actions::is_allowed` in `client.rs`, so create `actions.rs` with the table,
    `BLOCKED_SUBSTRINGS`, `is_blocked`, `is_allowed` and `lookup` (§6.1) in this step. The gate and executor follow in step 24.
- `serve`: `tonic::transport::Server::builder().add_service(svc).serve_with_incoming(tokio_stream::wrappers::TcpListenerStream::new(listener))`.
- `synthetic_frames()`: 300 frames at the intended 10 Hz (30 s cycle), built with the `schema.rs` prost structs.
  Frames 0..199 are in a vehicle: name `"MOCK Gladius"`, shields level and 4 faces as sinusoids in 0..1, hydrogen/quantum fuel
  slowly decreasing in 0..1, gear `3`, `is_landed` true for the first 50 frames, annunciators all `1` except `shld = 5` during frames 120..140,
  master mode `2`, spooled true for 150..199, actor locomotion `3` (SEATED), health 100/100.
  Frames 200..299 are on foot: no vehicle field, actor locomotion `1`, health decreasing 100→60 of max 100, oxygen 80/100.

### 4.7 Probe: `sender-rs/src/sc/probe.rs`
`hud-sender probe [--grpc URL] [--out DIR] [--timeout-secs 5]`. The default `--out` is `schema/probe-<unix_secs>/`.
Steps (each prints one status line; continue on failure where possible):
1. `connect`. On failure print `cannot connect to {url}: {err}` and exit with code **2**.
2. Reflection v1: send `ListServices` over `server_reflection_info(tokio_stream::once(req))`. On `Code::Unimplemented`, retry with v1alpha
   (same shape, `tonic_reflection::pb::v1alpha`; duplicating the ~40 lines is acceptable). If both fail, write `reflection.txt` with the
   status and continue.
   On success, for each service except `grpc.reflection.*`, request `FileContainingSymbol(service)`. Collect `file_descriptor_proto` bytes,
   then request `FileByFilename` for every not-yet-seen dependency until closed. Write `descriptor_set.pb`
   (a `prost_types::FileDescriptorSet`, encoded) and `schema.txt` = `format!("{:#?}", descriptor_set)` (the prost `Debug` dump;
   it lists every message, field name, number and type, which is enough to compare with Appendix A by eye, or with `protoc` later).
   Do **not** write a custom `.proto` pretty-printer.
   **This success branch cannot be tested offline** (the mock has no reflection), so keep it short and linear, send one request
   per stream (`tokio_stream::once(req)`, read responses until the stream ends), and treat every error as "note it and continue".
   Request shape (compiles against the pinned version):
   `ServerReflectionRequest { host: String::new(), message_request: Some(MessageRequest::ListServices(String::new())) }` with
   `use tonic_reflection::pb::v1::{server_reflection_client::ServerReflectionClient, server_reflection_request::MessageRequest, ServerReflectionRequest};`.
3. `ListTriggerableActions` (read-only and safe) → `actions.txt`: one line per action, `{action_name}\t{category}\tholdable={b}`. Failure → note and continue.
4. Open the telemetry stream and wait up to `--timeout-secs` for the first message. Write `first_frame.bin` (raw) and
   `first_frame.txt` (inspect tree). Print the frame size and the top-level fields present.
5. **Never call TriggerGameAction.** Exit code 0 if step 4 received a frame, otherwise 3.

### 4.8 Dump
`hud-sender dump --out FILE [--grpc URL] [--duration SECS] [--max-frames N]`
- Connect and open the stream (failure → exit 2). Write every message as an SCTD frame record. Every stdin line typed meanwhile
  becomes a marker record. Read stdin on a plain detached `std::thread::spawn` thread and send lines over a
  `tokio::sync::mpsc` channel (`blocking_send`). Do **not** use `tokio::task::spawn_blocking` for this: the runtime waits for
  blocking tasks at shutdown, and a thread stuck in `read_line` on a terminal would hang the program after Ctrl-C.
  When stdin reaches EOF the thread just ends; the dump keeps running. Print `frames: N` once per second on stderr.
- Stop on stream end, Ctrl-C (`tokio::signal::ctrl_c`), `--duration` or `--max-frames`. Flush, then print a summary (frames, markers, bytes, duration).

### 4.9 Offline inspection subcommands
```
hud-sender inspect FILE                    # summary (§4.4)
hud-sender inspect FILE --frame N [--hex]  # tree of frame N (0-based over frame records); --hex also prints the raw hex
hud-sender diff FILE A B [--ignore PREFIX]...
hud-sender decode FILE [--every N]         # run decode::Accumulator over the frames (now = one fixed base Instant + ts_us); print one line per Nth frame:
                                           # "#{i} t={ms}ms link={state} name={..} faces=[..] lvl={..} hf={..} qf={..} ann=[..] gear={..} health={..} o2={..}"
```
A FILE without SCTD magic is treated as one raw frame (for example `first_frame.bin`).

### 4.10 Mock server subcommand
`hud-sender mock-server [--listen 127.0.0.1:50051] [--replay FILE] [--hz 10] [--once] [--duration SECS]`
- No `--replay` → `synthetic_frames()`. `--replay` → the frame records of the SCTD file. The interval is `1/hz` (replay ignores the original timestamps).
- `--once` → `repeat = false`. Prints received TriggerGameAction calls.

### 4.11 Phase 1 tests
- `pb::wire`: varint edge cases (0, 127, 128, u64::MAX, an 11-byte overflow → `VarintOverflow`), truncated fixed32 → `Truncated`, wire type 3 → `InvalidWireType(3)`, field 0 → `InvalidFieldNumber`.
- `pb::inspect` on `HEALTH_FRAME` (Appendix C): the tree contains `6 msg (14 B)`, `6.1.1.1 fixed32 f32=75`, `6.1.1.2 fixed32 f32=100`, and the name `actor.vitals.health.current`.
- `pb::inspect` on `VEHICLE_FRAME`: contains `3.2.1 str "Gladius"` and `3.2.10.2 bytes (16 B)` with `f32[]=[1, 0.5, 0.25, 0]`.
- `diff` of `HEALTH_FRAME` against the same frame with health 50.0: exactly one `~ 6.1.1.1:` line containing `(Δ -25)`.
- `summary` on a 3-frame SCTD file with one marker: the marker is placed near the correct frame.
- `tests/e2e_client.rs` (tokio test, multi_thread flavor; written in step 18, needs no `sc::source`): serve the mock on `127.0.0.1:0` with
  `frames = [HEALTH_FRAME]`; `connect` + `open_telemetry_stream` yields a first message whose bytes equal `HEALTH_FRAME`;
  `list_actions` returns 5 entries; `connect` to a closed port returns `Err` within the timeout.
- `tests/inspect_cli.rs`: write an SCTD file to a tempdir, run the built binary (`env!("CARGO_BIN_EXE_hud-sender")`) with `inspect`, `inspect --frame 0`, `diff` and `decode`, and assert exit 0 and key substrings.

**Phase 1 done when:** `cargo test --workspace` passes, and the manual smoke test in §8.5 works.

---

## 5. Phase 2 — Live SC source in the sender

### 5.1 Module map and data flow
```
source::random::run ─┐                                 ┌─ serial writer thread ─▶ COM port
                     ├─ watch<Snapshot> ─▶ runner (tick @ max_hz) ─ mapping ─ policy ─┤
sc::source::run ─────┘                                 └─ stdout hex (PORT == "-")
       ▲ RawCodec stream ─ prost decode (schema.rs) ─ decode::Accumulator
serial reader thread ─▶ mpsc<(u8, Vec<u8>)> ─▶ runner: Hello → reply/log, Action → actions::Gate → executor → ActionAck
```

### 5.2 `sender-rs/src/sc/schema.rs`: the single schema module
Everything that depends on the game's schema or on unverified game semantics lives here, and only here.
Every item carries one provenance tag in its doc comment:
`CONFIRMED` (reported by two sources: the reference example and the extracted schema; **not** verified live by this project),
`COMMUNITY` (from the extracted schema in Appendix A only, not verified live by this project),
or `ASSUMPTION` (our guess about meaning, units or names; must be verified live, see `schema/fields.md`).
None of the three means "we have seen it work".

Contents (write it exactly like this; only `CONFIRMED`/`COMMUNITY` numbers taken from Appendix A):
```rust
//! Star Citizen telemetry schema subset. SOURCE OF TRUTH for all field numbers.
//! Provenance: Appendix A of plan.md (sc_make_proto extraction; 3 identical public copies).
//! Health path 6.1.1.{1,2} CONFIRMED (two sources agree); everything else COMMUNITY unless noted.
//! Nothing here has been verified against a live game by this project.

pub const SERVICE: &str = "sc.game.telemetry.v1.GameTelemetryService";                 // CONFIRMED
pub const PATH_TELEMETRY_STREAM: &str = "/sc.game.telemetry.v1.GameTelemetryService/TelemetryStream"; // CONFIRMED
pub const PATH_TRIGGER_GAME_ACTION: &str = "/sc.game.telemetry.v1.GameTelemetryService/TriggerGameAction"; // COMMUNITY
pub const PATH_LIST_TRIGGERABLE_ACTIONS: &str = "/sc.game.telemetry.v1.GameTelemetryService/ListTriggerableActions"; // COMMUNITY
pub const DEFAULT_ENDPOINT: &str = "http://127.0.0.1:2942";                              // CONFIRMED

#[derive(Clone, PartialEq, prost::Message)]
pub struct TelemetryStreamResponse {
    #[prost(message, optional, tag = "3")] pub vehicle: Option<VehicleFrame>,
    #[prost(message, optional, tag = "5")] pub events: Option<EventsFrame>,
    #[prost(message, optional, tag = "6")] pub actor: Option<ActorFrame>,          // CONFIRMED (tag 6)
}
#[derive(Clone, PartialEq, prost::Message)]
pub struct VehicleFrame { #[prost(message, optional, tag = "2")] pub state: Option<VehicleState> }
#[derive(Clone, PartialEq, prost::Message)]
pub struct VehicleState {
    #[prost(string, tag = "1")] pub name: String,
    #[prost(string, tag = "2")] pub class_name: String,
    #[prost(string, tag = "3")] pub manufacturer: String,
    #[prost(int32, tag = "4")] pub vehicle_type: i32,
    #[prost(bool, tag = "5")] pub lights_enabled: bool,
    #[prost(message, optional, tag = "7")] pub annunciators: Option<VehicleAnnunciator>,
    #[prost(message, optional, tag = "8")] pub flight: Option<VehicleFlight>,
    #[prost(message, optional, tag = "10")] pub shields: Option<VehicleShields>,
    #[prost(float, tag = "11")] pub health: f32,
    #[prost(message, optional, tag = "12")] pub master_mode: Option<VehicleMasterModeState>,
}
#[derive(Clone, PartialEq, prost::Message)]
pub struct VehicleAnnunciator {   // each value: AnnunciatorState enum (ANN_STATE_*), encoded as int32 varint
    #[prost(int32, tag = "1")] pub pwr: i32,  #[prost(int32, tag = "2")] pub wpn: i32,
    #[prost(int32, tag = "3")] pub thr: i32,  #[prost(int32, tag = "4")] pub shld: i32,
    #[prost(int32, tag = "5")] pub cool: i32, #[prost(int32, tag = "6")] pub qdrv: i32,
    #[prost(int32, tag = "7")] pub radr: i32, #[prost(int32, tag = "8")] pub prox: i32,
    #[prost(int32, tag = "9")] pub hit: i32,  #[prost(int32, tag = "10")] pub misl: i32,
}
#[derive(Clone, PartialEq, prost::Message)]
pub struct VehicleFlight {
    #[prost(message, optional, tag = "1")] pub state: Option<VehicleFlightState>,
    #[prost(message, optional, tag = "2")] pub settings: Option<VehicleFlightSettings>,
    #[prost(message, optional, tag = "3")] pub instruments: Option<VehicleFlightInstruments>,
}
#[derive(Clone, PartialEq, prost::Message)]
pub struct VehicleFlightState {
    #[prost(bool, tag = "1")] pub is_landed: bool,
    #[prost(int32, optional, tag = "2")] pub gear: Option<i32>,   // VehicleComponentDeploymentState
}
#[derive(Clone, PartialEq, prost::Message)]
pub struct VehicleFlightSettings { #[prost(message, optional, tag = "1")] pub ifcs_coupled: Option<Availability> }
#[derive(Clone, PartialEq, prost::Message)]
pub struct Availability { #[prost(bool, tag = "1")] pub enabled: bool, #[prost(bool, tag = "2")] pub active: bool }
#[derive(Clone, PartialEq, prost::Message)]
pub struct VehicleFlightInstruments {
    #[prost(float, tag = "1")] pub speed: f32,
    #[prost(float, tag = "2")] pub hydrogen_fuel: f32,
    #[prost(float, tag = "3")] pub quantum_fuel: f32,
    #[prost(message, optional, tag = "4")] pub g_meter: Option<Gauge>,
}
#[derive(Clone, PartialEq, prost::Message)]
pub struct VehicleShields { #[prost(float, tag = "1")] pub level: f32, #[prost(float, repeated, tag = "2")] pub faces: Vec<f32> }
#[derive(Clone, PartialEq, prost::Message)]
pub struct VehicleMasterModeState { #[prost(int32, tag = "1")] pub mode: i32, #[prost(bool, tag = "2")] pub spooled: bool }
#[derive(Clone, PartialEq, prost::Message)]
pub struct EventsFrame { #[prost(message, repeated, tag = "1")] pub events: Vec<GameEvent> }
#[derive(Clone, PartialEq, prost::Message)]
pub struct GameEvent { #[prost(int32, tag = "1")] pub event_type: i32 }
#[derive(Clone, PartialEq, prost::Message)]
pub struct ActorFrame {
    #[prost(message, optional, tag = "1")] pub vitals: Option<ActorVitals>,     // CONFIRMED
    #[prost(message, optional, tag = "2")] pub state: Option<ActorState>,
}
#[derive(Clone, PartialEq, prost::Message)]
pub struct ActorVitals {
    #[prost(message, optional, tag = "1")] pub health: Option<Gauge>,           // CONFIRMED
    #[prost(message, optional, tag = "2")] pub oxygen: Option<Gauge>,
}
#[derive(Clone, PartialEq, prost::Message)]
pub struct Gauge { #[prost(float, tag = "1")] pub current: f32, #[prost(float, tag = "2")] pub max: f32 }  // CONFIRMED
#[derive(Clone, PartialEq, prost::Message)]
pub struct ActorState {
    #[prost(int32, tag = "1")] pub locomotion: i32,   // ActorLocomotionState
    #[prost(int32, tag = "2")] pub crew_role: i32,
    #[prost(string, tag = "3")] pub seat_name: String,
}
#[derive(Clone, PartialEq, prost::Message)]
pub struct TriggerGameActionRequest {
    #[prost(string, tag = "1")] pub action_name: String,
    #[prost(uint32, optional, tag = "2")] pub hold_ms: Option<u32>,
}
#[derive(Clone, PartialEq, prost::Message)]
pub struct TriggerGameActionResponse { #[prost(int32, tag = "1")] pub status: i32, #[prost(string, tag = "2")] pub message: String }
#[derive(Clone, PartialEq, prost::Message)]
pub struct TriggerableAction { #[prost(string, tag = "1")] pub action_name: String, #[prost(string, tag = "2")] pub category: String, #[prost(bool, tag = "3")] pub holdable: bool }
#[derive(Clone, PartialEq, prost::Message)]
pub struct ListTriggerableActionsResponse { #[prost(message, repeated, tag = "1")] pub actions: Vec<TriggerableAction> }

// Enum values (COMMUNITY)
pub const DEPLOY_UNSPECIFIED: i32 = 0; pub const DEPLOY_RETRACTED: i32 = 1; pub const DEPLOY_DEPLOYING: i32 = 2;
pub const DEPLOY_DEPLOYED: i32 = 3;    pub const DEPLOY_RETRACTING: i32 = 4;
pub const LOCOMOTION_UNSPECIFIED: i32 = 0; pub const LOCOMOTION_ON_FOOT: i32 = 1; pub const LOCOMOTION_EVA: i32 = 2; pub const LOCOMOTION_SEATED: i32 = 3;
pub const EVENT_ACTOR_SPAWN: i32 = 1; pub const EVENT_ACTOR_DEAD: i32 = 2; pub const EVENT_ACTOR_ENTER_SEAT: i32 = 3;
pub const EVENT_ACTOR_LEAVE_SEAT: i32 = 4; pub const EVENT_ACTOR_RESPAWN: i32 = 5; pub const EVENT_ACTOR_DOWNED: i32 = 6;
pub const TRIGGER_STATUS_OK: i32 = 1;

// ---- ASSUMPTIONS (verify live; see schema/fields.md) ----
/// ASSUMPTION: units of shields.level / shields.faces[] and hydrogen/quantum fuel. Overridable via CLI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Scale {
    /// value already 0..1
    Unit,
    /// value 0..100
    Percent,
}
pub const DEFAULT_SHIELD_SCALE: Scale = Scale::Unit;
pub const DEFAULT_FUEL_SCALE: Scale = Scale::Unit;
/// ASSUMPTION: indices into faces[] for [front, back, left, right] (4-face ships).
pub const FACE_ORDER_4: [usize; 4] = [0, 1, 2, 3];
/// ASSUMPTION: indices into faces[] for [front, back, left, right, top, bottom] (6-face ships).
pub const FACE_ORDER_6: [usize; 6] = [0, 1, 2, 3, 4, 5];
/// ASSUMPTION: for 2-face ships faces = [front, back].
pub const FACE_ORDER_2: [usize; 2] = [0, 1];
/// ASSUMPTION: ifcs_coupled.active == true means "coupled", so decoupled = !active.
pub const DECOUPLED_WHEN_COUPLED_ACTIVE_IS: bool = false;

/// Path → name table for `inspect` annotations (indices ignored).
pub const PATH_NAMES: &[(&[u32], &str)] = &[ /* see list below */ ];
```
`PATH_NAMES` entries (path → name):
`[3]` vehicle, `[3,2]` vehicle.state, `[3,2,1]` vehicle.state.name, `[3,2,2]` vehicle.state.class_name, `[3,2,3]` vehicle.state.manufacturer,
`[3,2,4]` vehicle.state.type, `[3,2,5]` vehicle.state.lights_enabled, `[3,2,7]` vehicle.state.annunciators, `[3,2,7,1]`…`[3,2,7,10]`
annunciators.pwr/wpn/thr/shld/cool/qdrv/radr/prox/hit/misl, `[3,2,8,1,1]` flight.state.is_landed, `[3,2,8,1,2]` flight.state.gear,
`[3,2,8,2,1]` flight.settings.ifcs_coupled, `[3,2,8,3,1]` instruments.speed, `[3,2,8,3,2]` instruments.hydrogen_fuel,
`[3,2,8,3,3]` instruments.quantum_fuel, `[3,2,8,3,4]` instruments.g_meter, `[3,2,10,1]` shields.level, `[3,2,10,2]` shields.faces,
`[3,2,11]` vehicle.state.health, `[3,2,12,1]` master_mode.mode, `[3,2,12,2]` master_mode.spooled, `[4]` weapons, `[5]` events,
`[5,1,1]` events.type, `[6]` actor, `[6,1,1,1]` actor.vitals.health.current, `[6,1,1,2]` actor.vitals.health.max,
`[6,1,2,1]` actor.vitals.oxygen.current, `[6,1,2,2]` actor.vitals.oxygen.max, `[6,2,1]` actor.state.locomotion, `[7]` force_feedback, `[8]` turret.
(Prefix the names with `vehicle.state.` where shortened above, e.g. `vehicle.state.flight.state.is_landed`.)

`clap::ValueEnum` on `Scale` makes schema.rs depend on clap. That is intended; define `Scale` only here.

### 5.3 `sender-rs/src/snapshot.rs`
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LinkState { #[default] NoGame, Connected, InVehicle }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SourceKind { #[default] Sc, Random }
#[derive(Debug, Clone, PartialEq, Default)]
pub struct VehicleSnap {
    pub name: String,
    pub shield_level: Option<f32>,      // raw (unscaled); None if the shields message is absent
    pub shield_faces: Vec<f32>,         // raw
    pub hydrogen_fuel: Option<f32>,     // raw; None if the instruments message is absent
    pub quantum_fuel: Option<f32>,
    pub annunciators: Option<[i32; 10]>,// raw enum values in ANN_* order; None if the annunciators message is absent
    pub is_landed: Option<bool>,        // None if flight.state is absent
    pub gear: Option<i32>,              // None if flight.state is absent OR gear is unset (proto3 optional)
    pub decoupled: Option<bool>,        // None if flight.settings.ifcs_coupled is absent
    pub master_mode: Option<i32>,       // None if master_mode is absent
    pub qt_spooled: Option<bool>,
    pub lights: Option<bool>,           // always Some (scalar on VehicleState)
}
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ActorSnap { pub health: Option<f32> /*0..1*/, pub oxygen: Option<f32> /*0..1*/, pub locomotion: i32 }
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Snapshot { pub link: LinkState, pub source: SourceKind, pub vehicle: Option<VehicleSnap>, pub actor: Option<ActorSnap> }
impl Snapshot { pub fn no_game(source: SourceKind) -> Self; }
```
**Presence semantics (proto3):** a scalar whose parent message is present but which is itself absent on the wire is its default (0, false, "").
`Option::None` above means "the parent message was absent". Gauges: `ratio = (current/max).clamp(0,1)` if `max > 0` and finite, else `None`.

### 5.4 `sender-rs/src/sc/decode.rs`: Accumulator
```rust
pub struct Accumulator { vehicle: Option<(Instant, VehicleState)>, actor: Option<(Instant, ActorFrame)>, stale: Option<Duration> }
impl Accumulator {
    pub fn new(stale: Option<Duration>) -> Self;
    pub fn ingest(&mut self, resp: TelemetryStreamResponse, now: Instant);
    pub fn snapshot(&self, now: Instant) -> Snapshot;   // link is Connected or InVehicle; source = Sc
}
pub fn vehicle_snap(v: &VehicleState) -> VehicleSnap;   // pure conversion, unit-testable
pub fn actor_snap(a: &ActorFrame) -> ActorSnap;
```
Rules (each needs a test):
1. `resp.vehicle == Some(VehicleFrame{state: Some(s)})` → replace the stored vehicle with `(now, s)`. `resp.vehicle == Some(VehicleFrame{state: None})` → clear the vehicle.
   `resp.vehicle == None` → keep the previous one. **ASSUMPTION:** a message may carry only some sub-frames, so we merge across messages per sub-frame
   (whole sub-frame replacement, not field-level merge).
2. Same for `actor` (no clear-on-empty; `Some(ActorFrame::default())` replaces).
   Rules 1–5 are all **ASSUMPTIONS** about how the game fills the stream (we do not know whether sub-frames are sent in every
   message or only on change, or what is sent after leaving a ship). Put that sentence in the module doc comment.
3. Events: any `GameEvent.event_type` in {`ACTOR_LEAVE_SEAT`, `ACTOR_DEAD`, `ACTOR_RESPAWN`} clears the stored vehicle (applied after rule 1 for the same message).
4. Staleness: in `snapshot(now)`, a stored sub-frame older than `stale` (when `Some`) is treated as absent.
5. In-vehicle decision: `InVehicle` iff the vehicle is present (not stale) **and** `name` is non-empty **and** NOT (actor present with locomotion ∈ {ON_FOOT, EVA}).
   Otherwise `Connected`. `Snapshot.vehicle` is populated only when the result is `InVehicle`.
6. `vehicle_snap` maps fields as described in §5.3. `decoupled = ifcs_coupled.map(|a| a.active == DECOUPLED_WHEN_COUPLED_ACTIVE_IS)`.

### 5.5 `sender-rs/src/sc/source.rs`: ScSource
```rust
pub struct ScConfig { pub endpoint: String, pub connect_timeout: Duration /*2s*/, pub stale: Option<Duration>, pub idle_timeout: Option<Duration> }
pub async fn run(cfg: ScConfig, tx: tokio::sync::watch::Sender<Snapshot>) -> anyhow::Result<()>;
```
Loop until `tx.is_closed()`:
1. Publish `Snapshot::no_game(Sc)` (only if it differs from the current value). Connect, then `open_telemetry_stream`. On error, log once per distinct error string
   (stderr, `[sc] ...`), sleep `backoff`, double it (start 500 ms, max 5 s) and retry.
2. On stream open: log `[sc] connected`, reset the backoff, and create a fresh `Accumulator`. Then `tokio::select!` over:
   (a) `stream.message()` → `Ok(Some(b))`: `TelemetryStreamResponse::decode(b)`. On error, increment a counter and log the 1st and every 100th error
   (`[sc] decode error #{n}: {e}; frame {len} B`) and continue. On success, `ingest(resp, Instant::now())` and publish `snapshot(now)`;
   `Ok(None)` or `Err` → log `[sc] stream ended: ..`, then go to step 1.
   (b) a 500 ms interval tick → re-publish `snapshot(now)` (applies staleness).
   (c) if `idle_timeout` is `Some(d)` and no message for `d` → log and go to step 1. **Default off** (we do not know whether the game streams continuously).
3. Use `tx.send_replace` or `send_if_modified`. Avoid re-sending identical snapshots, though correctness does not depend on it.

### 5.6 `sender-rs/src/source/random.rs`
`pub async fn run(interval: Duration, tx: watch::Sender<Snapshot>) -> anyhow::Result<()>`. This keeps the old behaviour: a new random target every
`interval`, linear interpolation published at 30 Hz. Snapshot: `link = InVehicle`, `source = Random`, vehicle `name = "RANDOM"`,
4 faces in 0..1 (raw = unit scale), `shield_level = Some(mean)`, fuel in 0..1, `annunciators: None`, all bool/gear/mode `None`,
actor health = a random walk in 0.3..1, oxygen `Some(1.0)`, locomotion `LOCOMOTION_SEATED`.

### 5.7 `sender-rs/src/mapping.rs`
```rust
pub struct Scales { pub shield: Scale, pub fuel: Scale }
pub struct WireSet { pub link: LinkMsg, pub telemetry: TelemetryMsg, pub status: StatusMsg, pub vitals: VitalsMsg, pub vehicle_name: VehicleNameMsg }
pub fn to_u8(v: f32) -> Option<u8>;               // None if !finite; else (v.clamp(0.0,1.0) * 255.0).round() as u8 (f32 math)
pub fn apply_scale(v: f32, s: Scale) -> f32;      // Unit → v, Percent → v / 100.0
pub fn map_faces(faces: &[f32], level: Option<f32>) -> Option<[f32; 4]>;   // input already scaled; output [front, back, left, right]
pub fn to_wire(s: &Snapshot, scales: Scales) -> WireSet;
impl WireSet { pub fn frames(&self) -> Vec<(MsgType, Vec<u8>)>; }  // order: Link, Telemetry, Status, Vitals, VehicleName
```
`map_faces` table (n = faces.len()):
- 0 → `level.map(|l| [l; 4])`
- 1 → all four = f0
- 2 → front = f[FACE_ORDER_2[0]], back = f[FACE_ORDER_2[1]], left = right = min(front, back)
- 4 → by `FACE_ORDER_4`
- 6 → front, back by `FACE_ORDER_6`; left = min(left, top, bottom); right = min(right, top, bottom)
- any other n → all four = min(faces)

`to_wire`:
- `link.state` from `LinkState`. `link.source` = `SOURCE_SC` or `SOURCE_RANDOM`.
- Telemetry: if a vehicle is present, the shields come from `map_faces(scaled faces, scaled level)`. Bits 0–3 are set if `Some` (and each `to_u8` is `Some`).
  Fuel uses `to_u8(apply_scale(..))` and sets bits 4/5. No vehicle → all zero with `valid = 0`.
- Status: no vehicle → `StatusMsg::unknown()`, except `flags_valid |= SF_IN_VEHICLE` with the in-vehicle flag clear when `link != NoGame`.
  With a vehicle: each annunciator is `raw` if `0..=14`, `14` if raw > 14 or < 0 (keep it simple: raw < 0 → 14), and all are `ANN_UNKNOWN` if `None`.
  Flags: landed, qt_spooled, decoupled, in_vehicle (always valid and set), lights. Each sets its `flags_valid` bit only if `Some`.
  `gear`: `Some(g)` with 0..=254 → g, else `GEAR_UNKNOWN`. `master_mode` the same with `MASTER_MODE_UNKNOWN`.
- Vitals from the actor. Health/oxygen `to_u8`, with the valid bits set when `Some`.
- VehicleName: `from_str_lossy(&vehicle.name)`, or empty if there is no vehicle.
- **Range warnings** (in the runner, not in the pure mapping): if any raw shield or fuel value > 1.0 while its scale is `Unit`, log one warning per field per run:
  `[map] hydrogen_fuel=57.3 > 1.0 with --fuel-scale unit; try --fuel-scale percent`.

### 5.8 `sender-rs/src/policy.rs`
```rust
pub struct SendPolicy { last: HashMap<u8, Vec<u8>>, last_full: Option<Instant>, last_hello: Option<Instant>, keepalive: Duration, hello_every: Duration /*5s*/ }
impl SendPolicy {
    pub fn new(keepalive: Duration) -> Self;
    /// Returns the frames to send now: everything whose payload differs from the last sent; everything if keepalive has elapsed
    /// since the last full send (or never sent); plus Hello{PROTO_VERSION, ROLE_SENDER} first if hello_every has elapsed (or never).
    pub fn plan(&mut self, now: Instant, frames: &[(MsgType, Vec<u8>)]) -> Vec<(MsgType, Vec<u8>)>;
}
```
Tests use synthetic `Instant`s (`let t0 = Instant::now(); t0 + Duration::from_millis(..)`):
the first call sends Hello + all 5; an identical second call 33 ms later sends nothing; a changed telemetry value sends only Telemetry;
after 500 ms everything is sent again; after 5 s Hello is included again.
**Hello is never sent in reply to a Hello** (on either side). Each side announces itself on its own 5 s timer; replying would
make the two ends ping-pong forever.

### 5.9 `sender-rs/src/serial.rs`
```rust
pub struct SerialLink { pub tx: std::sync::mpsc::Sender<Vec<u8>>, pub rx: tokio::sync::mpsc::Receiver<(u8, Vec<u8>)> }
pub fn open(port: &str, baud: u32) -> anyhow::Result<SerialLink>;  // opens the port, try_clone() for the reader; spawns 2 std threads
pub fn writer_loop<W: std::io::Write>(w: W, rx: std::sync::mpsc::Receiver<Vec<u8>>) -> std::io::Result<()>; // write_all + flush per frame
pub fn reader_loop<R: std::io::Read>(r: R, tx: tokio::sync::mpsc::Sender<(u8, Vec<u8>)>); // FrameDecoder; ignore TimedOut; exits on other errors or closed tx
pub fn stdout_link() -> SerialLink;  // PORT "-": the writer prints "TX {type name} [{payload hex}] {frame hex}" per frame; rx never yields
                                     // ("never yields" = keep the tokio Sender alive, e.g. move it into the writer thread; if it is
                                     //  dropped, rx.recv() returns None at once and a select! loop would spin)
```
Use `serialport::new(port, baud).timeout(Duration::from_millis(200)).open()`. The reader uses `blocking_send`. Unit tests drive
`writer_loop` with `Vec<u8>` and `reader_loop` with `std::io::Cursor` (golden frames plus garbage → the expected decoded tuples).

### 5.10 `sender-rs/src/runner.rs` and the CLI
`cli.rs` (clap derive):
```
hud-sender run <PORT> [--source sc|random] [--grpc URL] [--baud 115200] [--interval 5] [--max-hz 30] [--keepalive-ms 500]
                      [--frame-stale-ms 3000] [--idle-timeout-secs 0] [--shield-scale unit|percent] [--fuel-scale unit|percent]
                      [--allow-actions] [--duration SECS] [--verbose]
hud-sender probe | dump | inspect | diff | decode | mock-server   (Phase 1)
hud-sender actions list [--grpc URL]
hud-sender actions trigger <NAME> [--hold-ms N] [--grpc URL] [--yes]
```
- `--source` defaults to `sc`. PORT `-` means stdout (no serial). `--frame-stale-ms 0` and `--idle-timeout-secs 0` disable those features. `--duration 0` means forever.
- **Breaking change:** the old `hud-sender COM6` becomes `hud-sender run COM6 --source random`. Document this in the README.

`runner::run(args) -> anyhow::Result<()>`:
1. Create `watch::channel(Snapshot::no_game(kind))` and spawn the chosen source.
2. Open the `SerialLink` (or `stdout_link`).
3. Run `tokio::time::interval(1000 / max_hz ms)` (MissedTickBehavior::Skip) in a `select!` loop with: tick → `to_wire(&*rx.borrow(), scales).frames()` →
   `policy.plan(now, ..)` → `frame()` each → `link.tx.send(bytes)` (an error means the writer died: return Err "serial port closed");
   inbound `(t, payload)` → `Hello` with role `ROLE_HUD`: log `[hud] hello v{n}` the first time and whenever `n` changes (warn if ≠ PROTO_VERSION); send nothing in reply;
   `Action` → §6; others ignored. Also `ctrl_c` → Ok, and the duration deadline → Ok.
   If the inbound channel returns `None` (reader thread ended), stop polling it (e.g. `Some(m) = rx.recv(), if rx_open`) instead of
   spinning; the run continues until the writer fails.
4. Log link-state transitions: `[link] NoGame -> Connected`.

**Phase 2 done when:** `cargo test --workspace` passes, including the e2e tests in §5.11, and the manual smoke test §8.5 (c) shows `TX Vitals` lines with changing health against `mock-server`.

### 5.11 Phase 2 tests
- Decode tests use the **hand-encoded** vectors in Appendix C (independent of prost-derive, so they anchor the field numbers):
  - `HEALTH_FRAME` → `actor.health == Some(0.75)`, link `Connected`, `vehicle == None`.
  - `VEHICLE_FRAME` → `InVehicle`. Expected values: name "Gladius", level 0.8, faces [1,0.5,0.25,0], hydrogen 0.6, quantum 0.4, `is_landed` true, gear 3, `decoupled` None, master_mode 2, `qt_spooled` true, annunciators `[1,0,0,6,0,0,0,0,5,0]`.
  - `to_wire` of that snapshot (unit scales): Telemetry `[255,128,64,0,153,102]` valid 0x3F; Status annunciator bytes `[0x01,0x60,0x00,0x00,0x05]`,
    flags 0x0B, flags_valid 0x1B (LIGHTS is valid-and-false because VehicleState is present), gear 3, mm 2; VehicleName "Gladius"; Vitals valid 0.
  - `VEHICLE_FRAME` then `ACTOR_ON_FOOT` (actor locomotion 1) → `Connected`, vehicle None.
  - Staleness: ingest the vehicle at t0, snapshot at t0+3.1 s with stale 3 s → `Connected`. With `stale = None` → `InVehicle`.
  - Event clear: ingest the vehicle, then a frame with only events `[{1: 4}]` (LEAVE_SEAT) → `Connected`.
  - Empty `VehicleFrame` (`1A 00`) clears the vehicle.
  - Unknown extra fields (append field 99 varint and a nested unknown message to `HEALTH_FRAME`) → same result.
  - A truncated frame (drop the last byte of `HEALTH_FRAME`) → `decode` returns Err (the source keeps running; covered in e2e).
- Mapping: `to_u8` (0→0, 1→255, 0.5→128, 0.25→64, 1.5→255, −1→0, NaN→None); `apply_scale(57.0, Percent)=0.57`; every `map_faces` row above;
  annunciator clamping (raw 20 → 14, raw −1 → 14); name sanitising ("Ünïcode" → "?n?code"; a 40-char name → 32 bytes).
- `tests/e2e_sc.rs` (tokio test, multi_thread flavor):
  1. Bind `127.0.0.1:0` and serve the mock with `frames = [HEALTH_FRAME]`, interval 20 ms, repeat. Run `sc::source::run` against it. Within 5 s the watch shows
     `actor.health == Some(0.75)` and link `Connected`.
  2. Mock with `[HEALTH_FRAME, <truncated frame>, HEALTH_FRAME_50]`, interval 50 ms, `repeat = false`. Within 5 s the watch shows
     `actor.health == Some(0.5)`. That value only exists after the bad frame, so seeing it proves a decode error does not kill the stream.
     (The source reconnects after each stream end and replays the three frames; that is fine.)
     Then abort the mock's serve task (which drops the listener): within 5 s the snapshot is `NoGame` and stays `NoGame` for a further 1 s.
     Do **not** try to observe a brief `NoGame` between two reconnects: a `watch` channel keeps only the latest value, so that would be flaky.
     All e2e tests observe by polling `rx.borrow().clone()` every 5 ms with a deadline, never by counting `changed()` wake-ups.
  3. Endpoint on a port with nothing listening (bind then drop a listener to get a free port). The snapshot stays `NoGame` for 1.5 s, and the task is still running.
  4. Mock with `synthetic_frames()` at 100 Hz. Within 5 s `InVehicle` is observed, then `Connected` (on-foot part).

---

## 6. Phase 5 — Touch → game actions (sender side only; firmware UI deferred)

### 6.1 `sender-rs/src/actions.rs`
```rust
pub struct ActionDef { pub id: u8, pub name: &'static str, pub hold_ms: Option<u32> }
pub const ACTIONS: &[ActionDef] = &[
    ActionDef { id: 1, name: "v_power_toggle_shields",           hold_ms: None },
    ActionDef { id: 2, name: "v_toggle_landing_system",          hold_ms: None },
    ActionDef { id: 3, name: "v_toggle_qdrive_engagement",       hold_ms: None },
    ActionDef { id: 4, name: "v_ifcs_vector_decoupling_toggle",  hold_ms: None },
    ActionDef { id: 5, name: "v_invoke_ping",                    hold_ms: None },
];
pub const BLOCKED_SUBSTRINGS: &[&str] = &["destruct", "eject", "suicide"];
pub fn is_blocked(name: &str) -> bool;               // case-insensitive substring match
pub fn is_allowed(name: &str) -> bool;               // exact match with an ACTIONS entry AND !is_blocked(name)
pub fn lookup(id: u8) -> Option<&'static ActionDef>;
pub enum GateDecision { Execute(&'static ActionDef), Reject(u8 /*ACK_*/), Duplicate }
pub struct ActionGate { allow: bool, min_interval: Duration /*250ms*/, last_exec: Option<Instant>, last_seq: Option<(u8, Instant)> }
impl ActionGate { pub fn new(allow: bool) -> Self; pub fn decide(&mut self, m: &ActionMsg, now: Instant) -> GateDecision; }
pub async fn execute(endpoint: &str, def: &ActionDef) -> (u8 /*result*/, u8 /*game_status*/);
```
**The five action names are ASSUMPTIONS** (taken from `docs/PLAN.md`; they look like key-binding action names, and nobody has
checked that the game's `TriggerGameAction` accepts them). Say so in a comment above the table and in `schema/fields.md`. The user
checks them against `actions.txt` from `probe`. Do not add, rename or "correct" entries. A wrong name is harmless: the game is
expected to answer `UNKNOWN_ACTION`.

**Why self-destruct/eject cannot be triggered** (keep all three layers; each has a test):
1. Allowlist: `client::trigger_action` (the only sender of the RPC) rejects every name for which `is_allowed` is false, before any I/O.
   The HUD can only send a numeric `action_id`, which `lookup` maps to a table entry; free-form names never come from the wire.
2. Blocklist: `is_allowed` also requires `!is_blocked`, and a unit test asserts that no `ACTIONS` entry is blocked, so adding a
   dangerous name to the table later fails the test suite.
3. Opt-in: the serial path executes nothing without `--allow-actions`; the CLI path executes nothing without `--yes`.

`decide` order: same `seq` as the last one within 2 s → `Duplicate` (ignored, no ack); unknown id → `ACK_UNKNOWN_ID`; `is_blocked` → `ACK_BLOCKED`;
`!allow` → `ACK_DISABLED`; less than `min_interval` since the last execute → `ACK_RATE_LIMITED`; else `Execute` (and record the time and seq).
`execute`: `connect(endpoint, 1s)` fails → `ACK_NOT_CONNECTED`; RPC `Err` → `ACK_RPC_ERROR`; `status == TRIGGER_STATUS_OK` → `ACK_OK`; else
`(ACK_GAME_REJECTED, status.clamp(0,255) as u8)`.
Runner: on an inbound Action with `--source sc`, decide, then spawn `execute` for `Execute`. Send an `ActionAck` frame immediately for `Reject`, and when
`execute` finishes for `Execute`. With `--source random`, every action → `ACK_NOT_CONNECTED`.
`actions trigger` CLI: refuses with exit 1 unless `is_allowed(name)` (message `blocked` for blocked names, otherwise
`not in the built-in allowlist`). Without `--yes` it prints `would call TriggerGameAction(name, hold)` and exits 1. With `--yes` it calls
it and prints the status. There is no way to trigger an arbitrary name from the CLI; that is deliberate.

### 6.2 Tests
- Unit: no `ACTIONS` entry `is_blocked`; `is_blocked("V_SELF_DESTRUCT")`, `is_blocked("v_eject")`, `is_blocked("v_selfdestruct")`;
  `!is_allowed("v_self_destruct")`, `!is_allowed("anything_else")`, `is_allowed("v_invoke_ping")`; ids are unique; each branch of `decide`.
- Request encoding: `TriggerGameActionRequest{action_name:"v_invoke_ping", hold_ms:None}.encode_to_vec()` equals Appendix C `TRIGGER_REQ_NOHOLD`;
  with `Some(500)` it equals `TRIGGER_REQ_HOLD500`. `TriggerGameActionResponse::decode(TRIGGER_RESP_OK).status == 1`.
- `tests/e2e_actions.rs`: mock on an ephemeral port. `execute(ep, lookup(5))` → `(ACK_OK, 0)` and the mock `action_log` contains `("v_invoke_ping", None)`.
  `client::trigger_action(ch, "v_self_destruct", None)` and `client::trigger_action(ch, "v_not_in_table", None)` against the **mock** →
  `Err` with `Code::PermissionDenied`, and the mock's `action_log` is unchanged afterwards (nothing left the process).
  A second mock with `trigger_status = 7` → `execute(ep, lookup(5))` returns `(ACK_GAME_REJECTED, 7)`.
  `list_actions` returns 5 entries. None of these tests may ever point at port 2942.

### 6.3 Deferred (explicitly out of scope; write it as a TODO section in docs/PROTOCOL.md)
Firmware tap zones and long-press UI, press feedback, and ActionAck display on the HUD.

---

## 7. Phase 4 — HUD firmware

All new logic that does not need LVGL goes into `HudState.h` and `Smoother.h` so the shim can test it.

### 7.1 `lib/hud_widgets/Smoother.h`
```cpp
#pragma once
#include <math.h>
#include <stdint.h>
class Smoother {
public:
  explicit Smoother(float tau_ms = 150.0f) : _cur(0), _tgt(0), _tau(tau_ms), _init(false), _shown(-1) {}
  void setTarget(uint8_t t);          // first call (when !_init) snaps _cur = t and sets _init = true
  void snap(uint8_t v);               // _cur = _tgt = v, _init = true
  bool tick(uint32_t dt_ms);          // _cur += (_tgt - _cur) * (1 - expf(-dt/tau)); |diff| < 0.5 → _cur = _tgt;
                                      // returns true if value() differs from the last value returned by a tick that returned true
  uint8_t value() const;              // (uint8_t)lroundf(_cur) clamped 0..255
  bool initialized() const { return _init; }
private:
  float _cur, _tgt, _tau; bool _init; int _shown;
};
```
Shim tests: after setTarget(0) and setTarget(255), ticking 150 ms gives 255·(1−e⁻¹) ≈ 161 ± 2; after 2 s the value is exactly 255; tick returns false once stable.

### 7.2 `lib/hud_widgets/HudState.h`
```cpp
#pragma once
#include <stdint.h>
#define HUD_SIGNAL_TIMEOUT_MS 2000
enum HudMode : uint8_t { HUD_MODE_NO_SIGNAL = 0, HUD_MODE_NO_GAME = 1, HUD_MODE_ON_FOOT = 2, HUD_MODE_VEHICLE = 3 };
enum AlertSeverity : uint8_t { ALERT_NONE = 0, ALERT_WARNING = 1, ALERT_CRITICAL = 2 };
static inline HudMode hudComputeMode(uint32_t now, uint32_t lastFrame, bool anyFrame, bool linkSeen, uint8_t linkState);
static inline void hudComputeAlert(const uint8_t ann[10], bool statusValid, const uint8_t shields[4], uint8_t shieldValidMask,
                                   AlertSeverity* sev, bool* blink);
```
Mode rules: `!anyFrame || now - lastFrame > HUD_SIGNAL_TIMEOUT_MS` (unsigned subtraction) → NO_SIGNAL; else `!linkSeen` → VEHICLE
(a v1 sender never sends Link); else linkState 0 → NO_GAME, 1 → ON_FOOT, 2 → VEHICLE, any other value → VEHICLE.
Alert rules: if `statusValid` and any `ann[i] != 15` (known): for each known annunciator, value 5 or 6 means critical and 3 or 4 means warning.
`sev` = the highest found. `blink` = true if any annunciator at that highest severity is a FLASHING state (3 or 5).
Otherwise (no annunciator data) the legacy heuristic applies: any shield i with its valid bit set and `shields[i] < 85` → CRITICAL with blink; else NONE.
Shim tests: the mode table (including the 2000/2001 ms boundary and millis wraparound: `now = 5`, `lastFrame = 0xFFFFFFF0` → not timed out);
alert cases (all nominal → NONE; shld=6 → CRITICAL steady; hit=5 → CRITICAL blink; cool=3 → WARNING blink; unknown annunciators with shields [255,84,255,255] and mask 0x0F → CRITICAL; the same with mask 0x0D → NONE).

### 7.3 Widget changes
- **Rule for every widget setter in this section:** remember the last applied state and return immediately if nothing changes.
  The sketch calls setters on every frame and every loop pass; an LVGL style/flag/text call invalidates the object even when the
  value is the same, which would redraw the screen continuously.
- `ShieldGauge`: add `int _shown = -1` (−1 = never drawn, −2 = unknown, 0..255 = value). `setUnknown()`: if `_shown == -2` return; hide `_sh`
  with `LV_OBJ_FLAG_HIDDEN`; `_shown = -2`. `setValue(val)`: if `_shown == val` return; if `_shown == -2` remove the hidden flag; draw as today;
  `_shown = val`. Add `bool isKnown() const { return _shown != -2; }`. Keep `value()`.
- `FuelBar`: cache `int _last = -1`. `setValue(v)` returns early if `v == _last`. Use `LV_ANIM_OFF` (smoothing now happens upstream). `setUnknown()` →
  return if `_last == -2`; bar 0, label `"%s --"`, `_last = -2`. (Colours come from `FuelBarConfig`; no colour setter is needed.)
- `AlertIndicator`: `void setSeverity(AlertSeverity s, bool blink)`. CRITICAL uses the configured red (255,30,0) and starts the heartbeat LED sequence when entering CRITICAL;
  WARNING uses amber (255,180,0) and no LED; NONE hides the icon. `blink == false` → icon steadily visible. Keep `setActive(bool a)` as `setSeverity(a ? ALERT_CRITICAL : ALERT_NONE, true)`.
  `setSeverity` with unchanged `(s, blink)` does nothing (it is called on every loop pass). `ledSet` must do nothing when `ledPin < 0`.
  Include `HudState.h` for the enum.
- New `IndicatorLabel.h`:
  ```cpp
  enum class IndState : uint8_t { Hidden, Off, On, Transit };
  struct IndicatorLabelConfig { int x = 4, y = 4; const char* text = "GEAR"; uint8_t onR = 0, onG = 220, onB = 90; };
  class IndicatorLabel { public: void create(lv_obj_t* parent, const IndicatorLabelConfig& cfg); void setState(IndState s); };
  ```
  Colours: Off (90,90,100), On (cfg colour), Transit amber (255,180,0); Hidden sets `LV_OBJ_FLAG_HIDDEN`. Font montserrat_14.
  `setState` returns early when the state is unchanged.
- `hud_widgets.h` includes `Smoother.h`, `HudState.h` and `IndicatorLabel.h` in addition to the existing ones.

### 7.4 `ship_hud/ship_hud.ino` changes
Layout (320×240, rotation 1):
- `vehPanel = lv_obj_create(scr)`: size 320×240, pos 0,0, `bg_opa` TRANSP, border 0, pad 0, `LV_OBJ_FLAG_SCROLLABLE` removed. All existing vehicle widgets get `vehPanel` as parent (same coordinates as today).
  Add to `vehPanel`: `IndicatorLabel` GEAR (4,4), QT (4,22, on colour 0,180,255), DCPL (4,40, on colour 255,180,0); a vehicle name label (`LV_ALIGN_TOP_RIGHT`, −4, 4,
  colour 180,180,200, `LV_LABEL_LONG_CLIP`, width 100); a master-mode label (`LV_ALIGN_TOP_RIGHT`, −4, 22): "NAV"/"SCM"/"STEALTH", hidden for 0 or unknown.
- `footPanel`, created the same way, hidden at boot: title label "ON FOOT" (`LV_ALIGN_TOP_MID`, 0, 20); a `FuelBar` "HEALTH" (barAlign TOP_RIGHT −10/90,
  labelAlign TOP_LEFT 10/88, colour 0,200,80); a `FuelBar` "O2" (TOP_RIGHT −10/130, TOP_LEFT 10/128, colour 0,180,220).
- `overlay = lv_label_create(lv_layer_top())`: centered, red (255,60,60), hidden by default; text is set per mode.

State (globals): `uint32_t g_lastFrameMs; bool g_anyFrame, g_linkSeen, g_statusValid; uint8_t g_linkState; StatusMsg g_status;
uint8_t g_shieldVal[4], g_shieldValid; int g_senderVersion = -1; uint32_t g_lastHelloMs; HudMode g_mode; bool g_chanValid[8];`
`Smoother sm[8]`: shields 0..3, hydrogen, quantum, health, oxygen.

Frame handling (in `loop`, for each decoded frame; `uint8_t payload[HUD_MAX_PAYLOAD]`):
- Every valid frame: `g_lastFrameMs = millis(); g_anyFrame = true;`
- `MSG_TELEMETRY` → `hud_decode_telemetry`. Each shield: valid → `sm[i].setTarget(v)`; invalid → `shields[i].setUnknown()`. Same for fuel with `FuelBar::setUnknown`.
  Store `g_shieldVal` and `g_shieldValid` for the alert logic. Mapping is unchanged: front→`shields[0]`, back→`[2]`, left→`[3]`, right→`[1]`.
- `MSG_STATUS` → store `g_status` and `g_statusValid = true`. GEAR: unknown/0 → Hidden, 3 → On, 2/4 → Transit, 1 → Off. QT: `flags_valid & SF_QT_SPOOLED` ? (flag ? On : Off) : Hidden.
  DCPL is the same with `SF_DECOUPLED`. Master mode label from `master_mode`.
- `MSG_VITALS` → health/oxygen smoothers, or `setUnknown()` on the foot bars when the valid bit is clear.
- `MSG_LINK` → `g_linkSeen = true; g_linkState = state;`
- `MSG_HELLO` with role `ROLE_SENDER` → `g_senderVersion = version`. Send nothing in reply (see §5.8).
- `MSG_VEHICLE_NAME` → `hud_decode_vehicle_name` → label text.
- Unknown types and `MSG_ACTION_ACK` → ignore.

Every loop iteration:
- `g_mode = hudComputeMode(...)`. When entering NO_SIGNAL, clear `g_linkSeen` and `g_statusValid` (stale alarms must not persist).
- **Apply the panel and overlay changes below only when `g_mode` (or the version-mismatch text) differs from the previous loop pass**,
  plus once at the end of `setup()`. Never call `lv_obj_set_style_*`, `lv_obj_add_flag`/`lv_obj_remove_flag` or `lv_label_set_text`
  unconditionally in `loop`.
- Every 5 s (`millis() - g_lastHelloMs >= 5000`): send `Hello{HUD_PROTO_VERSION, ROLE_HUD}`.
- Panels: VEHICLE → vehPanel visible, opa COVER, footPanel hidden. ON_FOOT → vehPanel hidden, footPanel visible.
  NO_GAME / NO_SIGNAL → vehPanel visible with `lv_obj_set_style_opa(vehPanel, LV_OPA_40, 0)`, footPanel hidden.
- Overlay text: NO_SIGNAL → "NO SIGNAL"; NO_GAME → "NO GAME"; otherwise empty. If `g_senderVersion >= 0 && g_senderVersion != HUD_PROTO_VERSION`,
  append the line `"SENDER PROTO v%d"`. The overlay is hidden when its text is empty.
- Every ≥ 33 ms: tick all smoothers with the real dt; for each smoother whose channel is currently valid, call the widget's `setValue(sm[i].value())`
  (the widget's own cache drops repeats). `g_chanValid[i]` is set from the valid bits when a Telemetry/Vitals frame arrives. A channel
  that goes invalid gets `setUnknown()` on its widget; when it becomes valid again use `sm[i].snap(v)` instead of `setTarget(v)` so it does not sweep in from a stale value.
- Alert: `hudComputeAlert(ann unpacked via hud_status_ann, g_statusValid, g_shieldVal, g_shieldValid, &sev, &blink)` → `alert.setSeverity(sev, blink)`.
  In ON_FOOT/NO_GAME/NO_SIGNAL modes, force `ALERT_NONE`. `alert.tick(millis())`.
- Keep `delay(5)`.

Boot: call `Serial.setRxBufferSize(1024);` **before** `Serial.begin(115200)` (a full-screen redraw can keep `loop` away from the
UART for longer than the default 256-byte buffer covers). After creating the UI, send `Hello{HUD_PROTO_VERSION, ROLE_HUD}` with
`hud_frame_encode` + `Serial.write(buf, len)` and set `g_lastHelloMs`.
Dimming uses the plain `opa` style, which LVGL 9 applies by scaling each child's opacity while drawing; it needs no off-screen layer
buffer. Do not use `opa_layered`, transforms or blend modes (those allocate a layer the size of the panel).
Remove the `anyCritical()` function. The boot `Serial.println` lines may stay (the sender's decoder tolerates text); add no text output inside `loop`.

Initial state: shield gauges `setValue(255)` at boot as today, but the panel is dimmed with overlay "NO SIGNAL" until the first frame arrives.

**Phase 4 done when:** `python scripts/sync_headers.py --check` passes, `cargo test --workspace` passes (shim tests for HudState/Smoother), and
`python -m platformio run -e esp32` succeeds (or the fallback in §8.6 is reported).

---

## 8. Ordered checklist (each step: do it, then run its verification)

All commands run from the repo root in Git Bash unless noted. `cargo test --workspace` is abbreviated **CT**.

| # | Step | Verify |
|---|------|--------|
| 1 | Root workspace (§1.2) with members `proto` and `sender-rs` only, one root lockfile, `.gitignore` | `cargo build --workspace` and **CT** |
| 2 | SETUP.md fixes (0.2), delete TelemetryParser.h (0.3) | `grep -rn TelemetryParser lib ship_hud` → no output |
| 3 | AlertIndicator ledc compat (0.4) in `lib/` | covered by step 6 |
| 4 | `scripts/sync_headers.py` (0.5), then run it | `python scripts/sync_headers.py && python scripts/sync_headers.py --check` → exit 0; hand-edit a copy → `--check` exits 1 (then rerun the sync) |
| 5 | proto-ctest skeleton + `headers_sync` test (0.7); add it to the workspace `members`; `shim.cpp` for now only `#include "hud_proto.h"` | **CT** |
| 6 | `platformio.ini`, `include/lv_conf.h` (0.6) | `python -m platformio run -e esp32` (see §8.6) |
| 7 | CI workflow (0.8): **skip unless the user asked for it** | none locally (no runner) |
| 8 | Rust protocol v2: messages.rs, frame.rs, golden tests (§3.3–3.5) | `cargo test -p hud-proto` |
| 9 | Keep the old sender compiling against the new proto API (temporary: `frame_message(&msg)`, add `valid: TV_ALL`) | `cargo build -p hud-sender` |
| 10 | C protocol v2 headers (§3.6–3.7), sync | `python scripts/sync_headers.py --check` |
| 11 | Shim + `c_parity` tests (§3.8, protocol part) | `cargo test -p hud-proto-ctest` |
| 12 | Sketch: payload buffer + `hud_decode_telemetry` only | `python -m platformio run -e esp32` |
| 13 | sender-rs lib/bin split, `cli.rs` with all subcommands (stubs return `bail!("not implemented")`) | `cargo run -p hud-sender -- --help` lists run, probe, dump, inspect, diff, decode, mock-server, actions |
| 14 | `pb::wire`, `pb::encode`, tests (§4.1–4.2) | `cargo test -p hud-sender pb::` |
| 15 | `dumpfile.rs` + tests (§4.3) | `cargo test -p hud-sender dumpfile` |
| 16 | `sc/schema.rs` (§5.2), `schema/sc_telemetry_community.proto` (Appendix A verbatim), plus unit tests in `schema.rs` that decode the Appendix C frames with the prost structs (health 75/100; vehicle name "Gladius") and compare `encode_to_vec()` with the two `TRIGGER_REQ_*` vectors | `cargo test -p hud-sender schema` |
| 17 | `pb::inspect` + inspect/diff/decode subcommands (§4.4, §4.9) | `cargo test -p hud-sender inspect` |
| 18 | `codec.rs`, `client.rs`, `mock.rs`, static part of `actions.rs` (table, blocklist, `is_allowed`), `mock-server` subcommand (§4.5–4.6, §4.10, §6.1) | `cargo test -p hud-sender --test e2e_client` (§4.11) |
| 19 | probe + dump (§4.7–4.8) | manual: §8.5 (b) |
| 20 | `snapshot.rs`, `decode.rs` + tests (§5.3–5.4, §5.11) | `cargo test -p hud-sender decode` |
| 21 | `sc/source.rs` (§5.5) + remaining e2e_sc tests | `cargo test -p hud-sender --test e2e_sc` |
| 22 | `source/random.rs`, `mapping.rs`, `policy.rs` + tests (§5.6–5.8) | `cargo test -p hud-sender mapping` then `cargo test -p hud-sender policy` (cargo takes one name filter per run) |
| 23 | `serial.rs`, `runner.rs`, wire up `run` (§5.9–5.10) | §8.5 (a) and (c) |
| 24 | Rest of `actions.rs` (gate, executor), runner integration, `actions` subcommands + tests (§6) | `cargo test -p hud-sender actions` and `cargo test -p hud-sender --test e2e_actions` |
| 25 | Smoother.h, HudState.h + shim tests (§7.1–7.2) | `cargo test -p hud-proto-ctest` |
| 26 | Widget changes, IndicatorLabel (§7.3), sync | `python scripts/sync_headers.py --check` |
| 27 | Sketch rewrite (§7.4), sync | `python -m platformio run -e esp32` |
| 28 | Docs (§9) | read-through; `grep -n "Not yet working" README.md` → nothing; `grep -n "not yet verified against the live game" README.md` → one hit |
| 29 | Final gate | **CT**, `python scripts/sync_headers.py --check`, `python -m platformio run -e esp32`, `cargo build --release -p hud-sender` |

### 8.5 Manual offline smoke tests (Git Bash)
(a) Random source to stdout:
```
cargo run -p hud-sender -- run - --source random --duration 2
```
Expect a `TX Hello` line first, then `TX Link [02 01]`, `TX Telemetry ...` lines.

(b) Mock server + probe + dump + inspect:
```
cargo build -p hud-sender
./target/debug/hud-sender mock-server --listen 127.0.0.1:50051 --duration 20 &
sleep 1
./target/debug/hud-sender probe --grpc 127.0.0.1:50051 --out target/probe-test     # reflection unavailable; actions.txt has 5 lines; first frame OK; exit 0
./target/debug/hud-sender dump --grpc 127.0.0.1:50051 --out target/mock.sctd --duration 3 < /dev/null
./target/debug/hud-sender inspect target/mock.sctd
./target/debug/hud-sender decode target/mock.sctd --every 10
```
(c) SC source against the mock:
```
./target/debug/hud-sender mock-server --listen 127.0.0.1:50052 --hz 50 --duration 15 &
sleep 1
./target/debug/hud-sender run - --source sc --grpc 127.0.0.1:50052 --duration 10
```
(`--hz 50` shortens the synthetic cycle to 6 s: 4 s in the vehicle, 2 s on foot. At the default 10 Hz the on-foot part would only start after 20 s.)
Expect `[sc] connected`, `TX Link [02 00]`, Telemetry with changing shields, and after about 4 s `TX Link [01 00]` with `TX Vitals` payloads whose first byte falls (on-foot part).

### 8.6 PlatformIO availability
1. Check: `python -m platformio --version`.
2. If it is missing: `python -m pip install --user -U platformio`. Record the installed version in the final report. The first `run` downloads the toolchain (several hundred MB).
3. If installation or the toolchain download fails (no network or a proxy): do **not** block. Report "firmware NOT compile-verified". What still counts as checked is the parity shim
   (all of `lib/hud_proto`, `HudState.h`, `Smoother.h` compile under MSVC through `cc`). The unverified parts are LVGL-dependent widgets and the `.ino`.
   Do not install the Arduino IDE/CLI as a substitute unless the user asks.

---

## 9. Phase 6 — Docs

- `README.md`: replace "Not yet working…" with an honest status line: "Sender and firmware are implemented and tested offline against a
  mock server; **not yet verified against the live game**." Add a "Star Citizen setup" section with these lines for `user.cfg`
  (`grpc.client.server.enabled = 1`, `grpc.client.server.port = 2942`, `server.game.telemetry.enabled = 1`), introduced with
  "as reported by the community; unverified, and the server may be disabled in current builds", and the usage
  `hud-sender run COM6` (sc is the default), `--source random` for the demo, the verification tools, and the PTU/build note. Update the project tree and the message table (link docs/PROTOCOL.md).
- `docs/PROTOCOL.md`: §3.1–3.2 in full, the golden vectors (Appendix B), the forward-compatibility rules, and the deferred touch-UI TODO.
- `schema/fields.md`: a table with columns `path | name | type | provenance (CONFIRMED/COMMUNITY/ASSUMPTION) | used by HUD | verified live (date, build) | observed range/units`.
  Prefill it from §5.2. The "verified live" column is empty in **every** row, health included (CONFIRMED means two sources agree,
  not that this project saw it). Add rows for the five action names with provenance ASSUMPTION.
- Add a "Live verification checklist (needs the game)" section to `schema/fields.md` for the user:
  probe → compare `schema.txt` with Appendix A; dump with markers while doing these actions: sit in ship, power on, drop each shield face, burn H-fuel,
  spool QT, gear up/down, toggle decoupled, take damage, leave the seat and walk. Then `inspect` and `decode` to settle: fuel/shield units (set `--*-scale` defaults in
  schema.rs), face order (`FACE_ORDER_*`), the `ifcs_coupled.active` meaning, whether messages carry all sub-frames or only some (stale timeout),
  and whether the stream is continuous (idle timeout).
- `docs/PLAN.md`: leave it as is (it is untracked user material). Do not delete it.

---

## 10. Risks and things the implementer must NOT guess

| Risk | Impact | Mitigation in this plan |
|------|--------|-------------------------|
| **CIG reportedly disabled the telemetry server in a later PTU build** (Spectrum thread, community report) | The live feature may not work at all right now | Everything is offline-testable; the HUD shows NO GAME; the random source still works. Not something to fix in code. |
| Community schema is unofficial and per-build | Wrong numbers lead to wrong values or decode errors | Numbers live only in `schema.rs` with provenance tags; decode errors are logged and counted, never fatal; inspect/diff/probe exist to re-verify after each patch. |
| Units of fuel and shields unknown (0–1, 0–100, or absolute) | Bars wrong | `Scale` default `Unit` + CLI override + range warning. If values turn out to be absolute (needing a max), that is a follow-up and is **not** to be invented now. |
| Shield face order unknown | Wrong quadrant | `FACE_ORDER_*` constants marked ASSUMPTION. |
| `ifcs_coupled.active` semantics | DCPL indicator inverted | A single constant `DECOUPLED_WHEN_COUPLED_ACTIVE_IS`. |
| Messages may be partial or on-change only | Flicker between modes, or stale data | Per-sub-frame accumulator; `--frame-stale-ms` (0 disables); idle timeout off by default. |
| proto3 default omission (0/false are not on the wire) | "absent" misread as "unknown" | §5.3 presence rules; tested. |
| Serial line shares the ESP32 debug prints | Garbage bytes | CRC plus a resync-on-0x00 decoder on both sides; tested with garbage. |
| Opening the COM port may reset the ESP32 (DTR) | HUD reboots on sender start | Harmless: the HUD sends Hello at boot and the sender re-sends everything on its 500 ms keepalive. Do not change DTR behaviour. |
| ESP32 core 3.x ledc API | Arduino IDE build break | §0.4 `#if`. Only the 2.x path is verified by PlatformIO. |
| Dangerous actions | Ship destroyed or ejection | Allowlist enforced inside the only function that sends the RPC, plus substring blocklist, actions off unless `--allow-actions`/`--yes`, rate limit, no automatic or probe calls, tests only against the mock (§6.1). |
| Action names are unverified placeholders | Taps do nothing (`UNKNOWN_ACTION`) | Marked ASSUMPTION; the user compares them with `actions.txt` from `probe`. |
| The reflection branch of `probe` and all firmware behaviour cannot be exercised offline | Bugs surface only with the game or the board | Kept minimal; compile-checked only; called out in the final report. |
| NoNaNelson repo prompt injection | Agent compromise | Rule 0.2.1. |
| `cargo test --workspace` needs a C++ compiler (proto-ctest) | Fails on machines without MSVC/g++ | Documented; `--exclude hud-proto-ctest` fallback is allowed only if the compiler is truly missing, and must be reported. |

**Do not guess:** field numbers or enum values beyond Appendix A; the TriggerGameAction semantics of `hold_ms` (send `None` unless the table says otherwise);
fuel/shield units; face order; annunciator meaning beyond the enum names; LVGL APIs beyond those already used in the repo plus `lv_obj_set_style_opa`,
`lv_layer_top`, `lv_obj_create`, `lv_obj_remove_flag`, `lv_label_set_long_mode`. If one of these is needed and not covered, stop and report instead of improvising.

---

## Appendix A — Community schema (save verbatim as `schema/sc_telemetry_community.proto`)

Provenance header to put at the top of the file:
```
// Star Citizen gRPC telemetry schema, community-extracted from the game binary's embedded descriptor
// (tool: github.com/Purzifal/sc_make_proto). Cross-checked: greenygh0st/sc-telemetry, zolij/verse-data-gateway,
// zoomphoto/Star-Citizen-Telemetry, field-for-field identical (Nov 2026). Build: PTU 5.0 (exact build unknown).
// UNOFFICIAL. Field paths verified live by this project: see schema/fields.md.
```
```proto
syntax = "proto3";
package sc.game.telemetry.v1;

// ---------- api.proto ----------
enum TriggerGameActionStatus {
  TRIGGER_GAME_ACTION_STATUS_UNSPECIFIED = 0;
  TRIGGER_GAME_ACTION_STATUS_OK = 1;
  TRIGGER_GAME_ACTION_STATUS_UNKNOWN_ACTION = 2;
  TRIGGER_GAME_ACTION_STATUS_MAP_INACTIVE = 3;
  TRIGGER_GAME_ACTION_STATUS_RATE_LIMITED = 4;
  TRIGGER_GAME_ACTION_STATUS_FAILED = 5;
  TRIGGER_GAME_ACTION_STATUS_HOLD_NOT_ALLOWED = 6;
  TRIGGER_GAME_ACTION_STATUS_FILTERED = 7;
}
message TelemetryStreamRequest {}
message TelemetryStreamResponse {
  reserved 1, 2;
  VehicleFrame vehicle = 3;
  WeaponsFrame weapons = 4;
  EventsFrame events = 5;
  ActorFrame actor = 6;
  ForceFeedbackFrame force_feedback = 7;
  TurretFrame turret = 8;
}
message TriggerGameActionRequest {
  string action_name = 1;
  optional uint32 hold_ms = 2;
}
message TriggerGameActionResponse {
  TriggerGameActionStatus status = 1;
  string message = 2;
}
message ListTriggerableActionsRequest {}
message TriggerableAction {
  string action_name = 1;
  string category = 2;
  bool holdable = 3;
}
message ListTriggerableActionsResponse {
  repeated TriggerableAction actions = 1;
}
service GameTelemetryService {
  rpc TelemetryStream(TelemetryStreamRequest) returns (stream TelemetryStreamResponse);
  rpc TriggerGameAction(TriggerGameActionRequest) returns (TriggerGameActionResponse);
  rpc ListTriggerableActions(ListTriggerableActionsRequest) returns (ListTriggerableActionsResponse);
}

// ---------- types.proto ----------
enum VehicleComponentDeploymentState {
  VEHICLE_COMPONENT_DEPLOYMENT_STATE_UNSPECIFIED = 0;
  VEHICLE_COMPONENT_DEPLOYMENT_STATE_RETRACTED = 1;
  VEHICLE_COMPONENT_DEPLOYMENT_STATE_DEPLOYING = 2;
  VEHICLE_COMPONENT_DEPLOYMENT_STATE_DEPLOYED = 3;
  VEHICLE_COMPONENT_DEPLOYMENT_STATE_RETRACTING = 4;
}
enum VehicleType {
  VEHICLE_TYPE_UNSPECIFIED = 0;
  VEHICLE_TYPE_SPACESHIP = 1;
  VEHICLE_TYPE_GROUND = 2;
  VEHICLE_TYPE_GRAVLEV = 3;
  VEHICLE_TYPE_BOAT = 4;
}
enum VehicleMasterMode {
  VEHICLE_MASTER_MODE_UNSPECIFIED = 0;
  VEHICLE_MASTER_MODE_NAVIGATION = 1;
  VEHICLE_MASTER_MODE_SCM = 2;
  VEHICLE_MASTER_MODE_STEALTH = 3;
}
enum ActorLocomotionState {
  ACTOR_LOCOMOTION_STATE_UNSPECIFIED = 0;
  ACTOR_LOCOMOTION_STATE_ON_FOOT = 1;
  ACTOR_LOCOMOTION_STATE_EVA = 2;
  ACTOR_LOCOMOTION_STATE_SEATED = 3;
}
enum ActorCrewRole {
  ACTOR_CREW_ROLE_UNSPECIFIED = 0;
  ACTOR_CREW_ROLE_PILOT = 1;
  ACTOR_CREW_ROLE_GUNNER = 2;
  ACTOR_CREW_ROLE_CREW = 3;
}
message Availability { bool enabled = 1; bool active = 2; }
message Gauge { float current = 1; float max = 2; }
message VehicleDoorState {
  enum DoorOpenState { DOOR_OPEN_STATE_UNSPECIFIED = 0; DOOR_OPEN_STATE_OPENED = 1; DOOR_OPEN_STATE_CLOSED = 2; }
  enum DoorLockState { DOOR_LOCK_STATE_UNSPECIFIED = 0; DOOR_LOCK_STATE_UNLOCKED = 1; DOOR_LOCK_STATE_LOCKED = 2; }
  DoorOpenState open_state = 1;
  DoorLockState lock_state = 2;
}
message VehicleFlightSettings {
  Availability ifcs_coupled = 1;
  Availability ifcs_esp = 2;
  Availability ifcs_gravity_compensation = 3;
  Availability ifcs_g_safe = 4;
  Availability ifcs_cruise_mode = 5;
  bool ifcs_spacebreak_enables_boost = 6;
  Availability ifcs_speed_limiter = 7;
  Availability ifcs_proximity_assist = 8;
  Availability ifcs_proximity_assist_landing = 9;
  Availability ifcs_stability = 10;
  Availability ifcs_core = 11;
  bool advanced_hud_enabled_scm = 12;
  bool advanced_hud_enabled_nav = 13;
  bool advanced_hud_labels = 14;
  bool course_prediction = 15;
  Availability ifcs_wind_compensation = 16;
  bool ifcs_g_safe_disabled_by_boost = 17;
  Availability ifcs_command_behaviour = 18;
  bool flight_light_amplification = 19;
}
message VehicleFlightState {
  bool is_landed = 1;
  optional VehicleComponentDeploymentState gear = 2;
  optional VehicleComponentDeploymentState vtol = 3;
  optional VehicleComponentDeploymentState transformation = 4;
}
message VehicleFlight {
  VehicleFlightState state = 1;
  VehicleFlightSettings settings = 2;
  VehicleFlightInstruments instruments = 3;
}
message VehicleFlightInstruments {
  float speed = 1;
  float hydrogen_fuel = 2;
  float quantum_fuel = 3;
  optional Gauge g_meter = 4;
  VehicleThrusterBoost boost = 5;
  float altitude = 6;
  float radar_altitude = 7;
  float vertical_speed_indicator = 8;
  float atmospheric_density = 9;
  float heading = 10;
}
message VehicleThrusterBoost { bool engaged = 1; float output = 2; float reserves = 3; bool critical = 4; }
message VehiclePowerState {
  bool powerplants = 1; bool engines = 2; bool weapons = 3; bool coolers = 4; bool life_support = 5;
  bool gravity_generators = 6; bool sensors = 7; bool shield_generators = 8; bool shield_emitters = 9;
}
message VehicleAnnunciator {
  enum AnnunciatorState {
    ANNUNCIATOR_STATE_UNSPECIFIED = 0;
    ANNUNCIATOR_STATE_NOMINAL = 1;
    ANNUNCIATOR_STATE_OFF = 2;
    ANNUNCIATOR_STATE_MINOR_FLASHING = 3;
    ANNUNCIATOR_STATE_MINOR_ON = 4;
    ANNUNCIATOR_STATE_MAJOR_FLASHING = 5;
    ANNUNCIATOR_STATE_MAJOR_ON = 6;
  }
  AnnunciatorState pwr = 1;
  AnnunciatorState wpn = 2;
  AnnunciatorState thr = 3;
  AnnunciatorState shld = 4;
  AnnunciatorState cool = 5;
  AnnunciatorState qdrv = 6;
  AnnunciatorState radr = 7;
  AnnunciatorState prox = 8;
  AnnunciatorState hit = 9;
  AnnunciatorState misl = 10;
}
message VehicleShields { float level = 1; repeated float faces = 2; }
message VehicleMasterModeState { VehicleMasterMode mode = 1; bool spooled = 2; }
message VehicleState {
  string name = 1;
  string class_name = 2;
  string manufacturer = 3;
  VehicleType type = 4;
  bool lights_enabled = 5;
  optional VehicleDoorState doors = 6;
  optional VehicleAnnunciator annunciators = 7;
  optional VehicleFlight flight = 8;
  optional VehiclePowerState power = 9;
  optional VehicleShields shields = 10;
  float health = 11;
  optional VehicleMasterModeState master_mode = 12;
}
message VehicleFrame { reserved 1; VehicleState state = 2; }
message WeaponsSettings {
  bool fallback_convergence_distance = 1; bool magnified_ui = 2; bool precision_lines = 3; bool staggered_fire = 4;
  bool use_lag_pips = 5; bool use_single_pips = 6; bool use_fading_pips = 7; bool gimbals_locked = 8;
}
message WeaponState { string name = 1; bool is_powered = 2; bool ready_to_fire = 3; bool contained_in_preset = 4; }
message Countermeasures {
  enum CountermeasureType { COUNTERMEASURE_TYPE_UNSPECIFIED = 0; COUNTERMEASURE_TYPE_DECOY = 1; COUNTERMEASURE_TYPE_NOISE = 2; }
  CountermeasureType type = 1;
  uint32 amount = 2;
}
message WeaponsFrame {
  WeaponsSettings settings = 1;
  uint32 preset_index = 2;
  string preset_name = 3;
  repeated WeaponState weapons = 4;
  repeated Countermeasures countermeasures = 5;
}
message GameEvent {
  enum GameEventType {
    GAME_EVENT_TYPE_UNSPECIFIED = 0;
    GAME_EVENT_TYPE_ACTOR_SPAWN = 1;
    GAME_EVENT_TYPE_ACTOR_DEAD = 2;
    GAME_EVENT_TYPE_ACTOR_ENTER_SEAT = 3;
    GAME_EVENT_TYPE_ACTOR_LEAVE_SEAT = 4;
    GAME_EVENT_TYPE_ACTOR_RESPAWN = 5;
    GAME_EVENT_TYPE_ACTOR_DOWNED = 6;
  }
  GameEventType type = 1;
}
message EventsFrame { repeated GameEvent events = 1; }
message AxisFeedback { float resistance = 1; float damping = 2; }
message FeedbackAxes { AxisFeedback x = 1; AxisFeedback y = 2; AxisFeedback z = 3; }
message ForceFeedbackFrame {
  reserved 1;
  FeedbackAxes angular = 2;
  FeedbackAxes linear = 3;
  float gunfire = 4;
  float stick_shaker = 5;
}
message ActorVitals {
  reserved 6;
  Gauge health = 1;
  Gauge oxygen = 2;
  Gauge hunger = 3;
  Gauge thirst = 4;
  Gauge body_temperature = 5;
  float heart_rate = 7;
  float gravity = 8;
  float pressure = 9;
  int32 intoxication_level = 10;
}
message ActorEnvironment { float external_temperature = 1; float ambient_radiation_rate = 2; bool breathable_atmosphere = 3; }
message ActorState {
  ActorLocomotionState locomotion = 1;
  ActorCrewRole crew_role = 2;
  string seat_name = 3;
  int64 uec_balance = 4;
}
message ActorFrame { ActorVitals vitals = 1; ActorState state = 2; ActorEnvironment environment = 3; }
message TurretFrame { bool light_amplification = 1; }
```

## Appendix B — Protocol v2 golden vectors (computed independently in Python)

| Name | Type | Payload | Full frame on the wire |
|------|------|---------|------------------------|
| TELEMETRY_V2 | 0x01 | `FF 80 40 20 C8 64 3F` | `00 0A 01 FF 80 40 20 C8 64 3F 3C 00` |
| TELEMETRY_V1 | 0x01 | `FF 80 40 20 C8 64` | `00 09 01 FF 80 40 20 C8 64 4B 00` |
| STATUS | 0x02 | `11 61 42 11 05 0A 0F 03 02` | `00 0C 02 11 61 42 11 05 0A 0F 03 02 5D 00` |
| VITALS | 0x03 | `C8 00 01` | `00 03 03 C8 03 01 D8 00` |
| LINK | 0x04 | `02 00` | `00 03 04 02 02 F0 00` |
| HELLO_SENDER | 0x05 | `02 00` | `00 03 05 02 02 B6 00` |
| HELLO_HUD | 0x05 | `02 01` | `00 05 05 02 01 87 00` |
| VEHICLE_NAME | 0x06 | `"AEGS_Gladius"` = `41 45 47 53 5F 47 6C 61 64 69 75 73` | `00 0F 06 41 45 47 53 5F 47 6C 61 64 69 75 73 58 00` |
| ACTION | 0x10 | `01 07` | `00 05 10 01 07 C7 00` |
| ACTION_ACK | 0x11 | `01 07 05 07` | `00 07 11 01 07 05 07 37 00` |

Decoded meaning of STATUS: annunciators pwr=1, wpn=1, thr=1, shld=6, cool=2, qdrv=4, radr=1, prox=1, hit=5, misl=0; flags 0x0A (QT_SPOOLED, IN_VEHICLE);
flags_valid 0x0F; gear 3 (DEPLOYED); master_mode 2 (SCM). ACTION_ACK: result 5 (GAME_REJECTED), game_status 7 (FILTERED).

## Appendix C — Hand-encoded protobuf test vectors (independent of prost-derive)

```
HEALTH_FRAME (16 B)  actor{vitals{health{current 75.0, max 100.0}}}
32 0E 0A 0C 0A 0A 0D 00 00 96 42 15 00 00 C8 42

VEHICLE_FRAME (72 B) vehicle{state{name "Gladius", annunciators{pwr 1, shld 6, hit 5},
  flight{state{is_landed true, gear 3}, instruments{hydrogen 0.6, quantum 0.4}},
  shields{level 0.8, faces packed [1.0, 0.5, 0.25, 0.0]}, master_mode{mode 2, spooled true}}}
1A 46 12 44 0A 07 47 6C 61 64 69 75 73 3A 06 08 01 20 06 48 05 42 12 0A 04 08 01 10 03
1A 0A 15 9A 99 19 3F 1D CD CC CC 3E 52 17 0D CD CC 4C 3F 12 10 00 00 80 3F 00 00 00 3F
00 00 80 3E 00 00 00 00 62 04 08 02 10 01

ACTOR_ON_FOOT        actor{state{locomotion 1}}
32 04 12 02 08 01

EMPTY_VEHICLE_FRAME  vehicle{} (clears vehicle)
1A 00

LEAVE_SEAT_EVENT     events{events[{type 4}]}
2A 04 0A 02 08 04

TRIGGER_REQ_NOHOLD   {action_name "v_invoke_ping"}
0A 0D 76 5F 69 6E 76 6F 6B 65 5F 70 69 6E 67

TRIGGER_REQ_HOLD500  {action_name "v_invoke_ping", hold_ms 500}
0A 0D 76 5F 69 6E 76 6F 6B 65 5F 70 69 6E 67 10 F4 03

TRIGGER_RESP_OK      {status 1}
08 01

HEALTH_FRAME_50 (16 B) as HEALTH_FRAME, but current 50.0
32 0E 0A 0C 0A 0A 0D 00 00 48 42 15 00 00 C8 42
```
(`EMPTY_VEHICLE_FRAME` and `LEAVE_SEAT_EVENT` follow directly from the tag rule `(field << 3) | wire_type`: 3/len → `1A`, 5/len → `2A`,
1/len → `0A`, 1/varint → `08`. The other vectors were generated with a Python encoder while writing this plan.)

## Appendix D — tonic 0.14 skeleton (compiled and run by the reviewer against the §1.3 pins)

This is the part of the plan where API guessing is most likely, so here is code that is known to build and run on
cargo 1.99 with exactly the §1.3 dependency set. It served three frames over a raw-bytes server stream, answered a unary
call, and returned `Unimplemented` to a reflection request. Reuse the trait impls and call shapes; replace the bodies
(frame source, action log, paths from `schema.rs`) with what §4.5–4.6 specify.

```rust
use bytes::{Buf, BufMut, Bytes};
use std::{convert::Infallible, future::Future, pin::Pin, task::{Context, Poll}};
use tonic::codec::{Codec, DecodeBuf, Decoder, EncodeBuf, Encoder};
use tonic::codegen::http;                       // no direct `http` dependency
use tonic::codegen::http::uri::PathAndQuery;
use tonic::{Request, Response, Status};
use prost::Message as _;

// ---- codec.rs ----
#[derive(Default, Clone)] pub struct RawCodec;
pub struct RawEncoder; pub struct RawDecoder;
impl Codec for RawCodec {
    type Encode = Bytes; type Decode = Bytes; type Encoder = RawEncoder; type Decoder = RawDecoder;
    fn encoder(&mut self) -> RawEncoder { RawEncoder }
    fn decoder(&mut self) -> RawDecoder { RawDecoder }
}
impl Encoder for RawEncoder { type Item = Bytes; type Error = Status;
    fn encode(&mut self, item: Bytes, dst: &mut EncodeBuf<'_>) -> Result<(), Status> { dst.put_slice(&item); Ok(()) } }
impl Decoder for RawDecoder { type Item = Bytes; type Error = Status;
    fn decode(&mut self, src: &mut DecodeBuf<'_>) -> Result<Option<Bytes>, Status> { let n = src.remaining(); Ok(Some(src.copy_to_bytes(n))) } }

// ---- mock.rs ----
#[derive(Clone)] struct Mock;                    // real one holds Arc<MockConfig>
impl tonic::server::NamedService for Mock { const NAME: &'static str = "sc.game.telemetry.v1.GameTelemetryService"; }

type BoxStream = Pin<Box<dyn tokio_stream::Stream<Item = Result<Bytes, Status>> + Send>>;
struct StreamSvc;
impl tonic::server::ServerStreamingService<Bytes> for StreamSvc {
    type Response = Bytes; type ResponseStream = BoxStream;
    type Future = Pin<Box<dyn Future<Output = Result<Response<BoxStream>, Status>> + Send>>;
    fn call(&mut self, _r: Request<Bytes>) -> Self::Future {
        Box::pin(async move {
            let (tx, rx) = tokio::sync::mpsc::channel(16);
            tokio::spawn(async move {
                for frame in [Bytes::from_static(b"..")] {            // real one: cfg.frames, repeat, interval
                    if tx.send(Ok(frame)).await.is_err() { break; }   // client went away
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                }
            });
            Ok(Response::new(Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx)) as BoxStream))
        })
    }
}
struct UnarySvc;
impl tonic::server::UnaryService<Bytes> for UnarySvc {
    type Response = Bytes;
    type Future = Pin<Box<dyn Future<Output = Result<Response<Bytes>, Status>> + Send>>;
    fn call(&mut self, r: Request<Bytes>) -> Self::Future {
        Box::pin(async move {
            let _req_bytes: Bytes = r.into_inner();                   // real one: prost-decode, log, build the response
            Ok(Response::new(Bytes::from_static(&[0x08, 0x01])))
        })
    }
}
impl tonic::codegen::Service<http::Request<tonic::body::Body>> for Mock {
    type Response = http::Response<tonic::body::Body>;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Infallible>> + Send>>;
    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Infallible>> { Poll::Ready(Ok(())) }
    fn call(&mut self, req: http::Request<tonic::body::Body>) -> Self::Future {
        let path = req.uri().path().to_string();
        Box::pin(async move {
            match path.as_str() {
                P_STREAM => { let mut g = tonic::server::Grpc::new(RawCodec); Ok(g.server_streaming(StreamSvc, req).await) }
                P_TRIG   => { let mut g = tonic::server::Grpc::new(RawCodec); Ok(g.unary(UnarySvc, req).await) }
                _ => Ok(Status::unimplemented("").into_http()),
            }
        })
    }
}

// ---- serve + client calls ----
let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
let addr = listener.local_addr()?;
tokio::spawn(tonic::transport::Server::builder().add_service(Mock)
    .serve_with_incoming(tokio_stream::wrappers::TcpListenerStream::new(listener)));

let ch = tonic::transport::Endpoint::from_shared(format!("http://{addr}"))?
    .connect_timeout(std::time::Duration::from_secs(2))
    .http2_keep_alive_interval(std::time::Duration::from_secs(10))
    .keep_alive_timeout(std::time::Duration::from_secs(5))
    .keep_alive_while_idle(true)
    .connect().await?;

let mut g = tonic::client::Grpc::new(ch.clone());
g.ready().await.map_err(|e| Status::unavailable(e.to_string()))?;
let mut stream = g.server_streaming(Request::new(Bytes::new()), PathAndQuery::from_static(P_STREAM), RawCodec)
    .await?.into_inner();                                             // tonic::Streaming<Bytes>
while let Some(bytes) = stream.message().await? { /* prost decode */ }

let mut g = tonic::client::Grpc::new(ch.clone());
g.ready().await.map_err(|e| Status::unavailable(e.to_string()))?;
let resp: Bytes = g.unary(Request::new(Bytes::from(req_msg.encode_to_vec())), PathAndQuery::from_static(P_TRIG), RawCodec)
    .await?.into_inner();

use tonic_reflection::pb::v1::{server_reflection_client::ServerReflectionClient,
    server_reflection_request::MessageRequest, ServerReflectionRequest};
let mut rc = ServerReflectionClient::new(ch.clone());
let req = ServerReflectionRequest { host: String::new(), message_request: Some(MessageRequest::ListServices(String::new())) };
match rc.server_reflection_info(tokio_stream::once(req)).await {
    Ok(resp) => { let mut s = resp.into_inner(); while let Some(_m) = s.message().await? { /* … */ } }
    Err(e) if e.code() == tonic::Code::Unimplemented => { /* try v1alpha, then give up */ }
    Err(e) => { /* note and continue */ let _ = e; }
}
```
`P_STREAM` / `P_TRIG` stand for `schema::PATH_TELEMETRY_STREAM` / `schema::PATH_TRIGGER_GAME_ACTION`. Also checked with the same
pins: `#[prost(float, repeated, tag = "2")]` encodes packed (`12 08 …`), `#[prost(int32, optional, tag = "2")]` decodes to
`Some(3)`, and `TriggerGameActionRequest{"v_invoke_ping", Some(500)}` encodes to the `TRIGGER_REQ_HOLD500` bytes of Appendix C.

---

## Review notes (Fable)

Reviewed against the repo at `ff46d13` on 2026-11-27. What I checked myself: every Appendix B frame and every Appendix C
vector (recomputed with an independent Python COBS/CRC-8/protobuf encoder: all correct, including the `0x3C` CRC and the
`to_wire` expectations in §5.11); the §1.3 dependency set (resolves and compiles on cargo 1.99; the tonic/prost/reflection
calls named in the plan exist and run, see Appendix D); Appendix A against the three saved community `.proto` copies
(field-for-field identical apart from nested-type qualification and `reserved` lines); the current headers, sketch and
sender source. What I could **not** check: anything against the game, anything on the ESP32, and the PlatformIO build
(PlatformIO is not installed). I did not open the NoNaNelson repository or its clone.

### Changes made

Correctness
1. **§3.7 `HUD_MAX_FRAME`**: the plan said 70 equals the Rust `MAX_FRAME`. Rust's constant is 69 (the true worst case). Text
   corrected; the two must not be asserted equal.
2. **§3.5 overflow test**: 100 junk bytes before the first `0x00` are dropped by the "not synced" rule, so the test never
   reached the overflow flag. Now feeds a `0x00` first.
3. **§5.11 e2e test 2**: it tried to observe a momentary `NoGame` through a `watch` channel, which only keeps the latest
   value (flaky). Replaced with a deterministic version using a new vector `HEALTH_FRAME_50` (added to Appendix C, computed
   the same way) and an explicit "server gone → stays NoGame" check. All e2e tests now poll with a deadline.
4. **§8.5 (c) and the Phase 2 exit criterion**: at 10 Hz the synthetic on-foot section starts after 20 s, but the smoke test
   ran for 10 s, and health is constant while in the vehicle, so "TX Vitals with changing health" could never be seen.
   Now uses `--hz 50`. Added `sleep 1` after starting the mock in the background (probe raced the listener).
5. **Hello ping-pong (§5.8, §5.10, §7.4)**: the sender replied to a HUD Hello and the HUD replied to a sender Hello, so the two
   would exchange Hellos forever. Now neither side replies; each announces itself every 5 s. `force_hello` removed.
6. **§4.8 `dump`**: stdin was read with `spawn_blocking`; the tokio runtime waits for blocking tasks on shutdown, so the
   program would hang after Ctrl-C when stdin is a terminal (the smoke test hid this with `< /dev/null`). Now a detached
   `std::thread`.
7. **§7.3/§7.4 redraw storm**: the sketch was told to set panel opacity, overlay text, alert state and unknown-state on every
   loop pass. In LVGL every such call invalidates the object even if nothing changed, so the display would re-render
   continuously. Added an explicit change-only rule for all setters and for the mode-dependent styling; made the
   `ShieldGauge`/`FuelBar` caching rule unambiguous (the old "after the first call" wording was not implementable as written).
8. **§0.2 `proto/Cargo.toml`**: "keep as is plus `edition.workspace = true`" would leave two `edition` keys (manifest error).

Ordering and verification commands
9. **Step 1**: the root manifest listed `tools/proto-ctest` before the crate exists, which makes every cargo command fail.
   Members are now added when the crate is created (step 5).
10. **Step 18** verified itself with an e2e test that needs `sc::source::run` (step 21), and `mock.rs`/`client.rs` use
    `actions::…` (step 24). Added `tests/e2e_client.rs` for step 18 and moved the static part of `actions.rs` to step 18.
11. **Step 22**: `cargo test -p hud-sender mapping policy` is not valid (one filter per run). Split.
12. **Step 16** verified a schema module with `cargo build` only. Now it has unit tests that decode the Appendix C vectors.
13. **Rule 0.2.7** demanded a green workspace after every step, which steps 8→9 contradict. Reworded.

Safety (TriggerGameAction)
14. The only barrier between the CLI and a destructive action was a two-entry substring blocklist; `actions trigger <any name> --yes`
    would have sent anything else, e.g. a differently spelled self-destruct. Now `client::trigger_action`, the single
    function that sends the RPC, enforces the **allowlist** (exact table entry and not blocked) before any I/O, the CLI
    can only trigger table entries, the blocklist is widened to `destruct` / `eject` / `suicide` as a second layer, and the
    tests assert that a blocked or unknown name is rejected locally and never reaches the mock. The mock's "FILTERED"
    behaviour was replaced by a configurable `trigger_status` so rejection decoding is still tested.

Provenance and honesty
15. `CONFIRMED` was defined as "seen live". Nobody in this project has seen anything live; it now means "two sources agree".
    `schema/fields.md` no longer pre-fills health as verified. The claim of three "independently published" copies is toned
    down: they almost certainly share one extraction, so they are one source, copied three times.
16. The five action names were presented as facts while rule 0.2.2 forbids inventing names. They come from `docs/PLAN.md` and
    are now labelled ASSUMPTION everywhere. Same for the `user.cfg` lines in the README text and the accumulator rules in §5.4.
17. README status line now says "tested offline, not verified against the live game" instead of just deleting the warning.

Simplifications
18. `probe`: dropped the custom `.proto` pretty-printer and per-file descriptor dumps. That whole branch can never run
    offline (the mock has no reflection), so it is now a `Debug` dump of the `FileDescriptorSet`.
19. Removed `scripts/pre-commit`, `FuelBar::setColor`, the unused `Outbound` enum, and the "pick one" choice about `Scale`.
    CI workflow is now "skip unless asked".
20. `TFT_BL`: changed from "set to -1" to "remove the define". TFT_eSPI only touches that pin when `TFT_BACKLIGHT_ON` is also
    defined, which it never is here, so GPIO 21 was not actually being driven; removing the macro is the unambiguous fix.

Firmware hardening (cheap, cannot be measured here)
21. `LV_MEM_SIZE` 48 → 64 KiB, `Serial.setRxBufferSize(1024)`, and a note that dimming must use plain `opa` (no layer buffer).

Additions
22. Appendix D (compiled tonic skeleton), notes on the `Message` trait name clash and the missing `http` dependency,
    a borrow hint for `FrameDecoder::feed`, the `stdout_link` closed-channel spin, and `/schema/probe-*/` in `.gitignore`.

### Unresolved concerns for the implementer and the user

- **The feature may not work at all right now.** §10 notes a community report that CIG disabled the telemetry server in a later
  PTU build. Everything here is built on an unofficial schema from one extraction of one build. Treat the result as
  "ready for a first live test", not as working.
- **Stale-timeout default (`--frame-stale-ms 3000`) is a coin toss.** If the game only sends the vehicle sub-frame on change,
  the HUD will fall back to ON FOOT after 3 s of sitting still in a ship. If it sends nothing on exit and the timeout is off,
  the HUD stays in vehicle mode. Only a live dump settles this; leave the default and the flag as specified.
- **Units, face order, `ifcs_coupled.active` sense, action names**: all placeholders, as the plan says. Do not tune them.
- **Firmware is compile-checked at best.** Layout coordinates in §7.4, memory use, redraw cost and touch are unverified; the
  Arduino-core-3.x `ledc` branch is not compiled by PlatformIO (core 2.x) at all. If PlatformIO cannot be installed, the
  LVGL-dependent headers and the sketch are entirely unchecked; say so in the report rather than claiming success.
- **PlatformIO specifics I could not test**: `src_dir = ship_hud` with a `.ino`, `lib/` being picked up as two header-only
  libraries alongside the copies in `ship_hud/`, and LVGL 9.2.2 building under `espressif32@6.10.0`. If `pio run` fails for a
  reason unrelated to this plan's code, report the error verbatim and stop; do not start swapping versions.
- **Size.** This is about 29 steps and roughly 30 new files. If time runs short, the order of sacrifice is: step 7 (CI),
  step 24 (actions: nothing can send an `Action` frame until the deferred touch UI exists), then `diff`/`summary` polish.
  Protocol v2, the mock, `decode`/`mapping`, and the firmware state logic are the core.
- **Serial garbage vs. actions.** A corrupted or random byte run passes CRC-8 one time in 256. With `--allow-actions` on, such
  a frame with type `0x10` and a valid id would trigger a table action. The table only holds toggles, the rate limit applies,
  and the flag is off by default, so this is accepted; it is one more reason not to add consequential actions to the table.
- `docs/PLAN.md` still names the NoNaNelson repository as its reference. Rule 0.2.1 stands: do not open it.
