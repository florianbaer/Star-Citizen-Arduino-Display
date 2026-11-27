# Spaceship HUD

Status: sender and firmware are implemented and tested offline against a mock server; **not yet verified against the live game**.

A spaceship HUD for the **ESP32-2432S024C** (Cheap Yellow Display), built with LVGL. A PC-side sender reads Star Citizen's gRPC telemetry stream (or generates random demo data) and forwards compact binary frames over USB serial. The display shows 4 shield arc gauges, 2 fuel bars, gear / quantum / decoupled indicators, master mode, vehicle name, an annunciator-driven alert indicator, and an on-foot screen (health, oxygen).

## Hardware

- **Board**: ESP32-2432S024C (2.4" 320x240 ILI9341, capacitive touch CST816S)
- **Touch**: CST816S on I2C (SDA=33, SCL=32)
- **Display**: ILI9341 on HSPI

## Star Citizen setup

The telemetry server is configured through `user.cfg`. As reported by the community; unverified, and the server may be disabled in current builds:

```
grpc.client.server.enabled = 1
grpc.client.server.port = 2942
server.game.telemetry.enabled = 1
```

The schema used by the sender is community-extracted from the game binary (`schema/sc_telemetry_community.proto`, field table in `schema/fields.md`). It is unofficial, may change with every patch, and has never been verified live by this project. Only the health path is reported by two sources. Check the PTU / build you run against `schema/fields.md` before trusting any value.

## Usage (sender)

```sh
cargo build --release -p hud-sender

hud-sender run COM6                          # live Star Citizen source (the default), 115200 baud
hud-sender run COM6 --source random          # demo data, no game needed
hud-sender run - --source random --duration 2   # PORT "-" prints the frames to stdout instead of a serial port
hud-sender run COM6 --shield-scale percent --fuel-scale percent   # if the game reports 0..100
```

Breaking change: the old `hud-sender COM6` is now `hud-sender run COM6 --source random`.

Verification tools (all offline-testable against `hud-sender mock-server`):

```sh
hud-sender probe [--grpc URL] [--out DIR]           # reflection, action list, first frame -> schema/probe-<time>/
hud-sender dump --out FILE [--duration SECS]        # record the stream; lines typed on stdin become markers
hud-sender inspect FILE [--frame N [--hex]]         # summary of a recording, or the tree of one frame
hud-sender diff FILE A B [--ignore PREFIX]...       # field-level diff of two frames
hud-sender decode FILE [--every N]                  # run the snapshot decoder over a recording
hud-sender mock-server [--listen ADDR] [--replay FILE] [--hz N]
hud-sender actions list                             # ask the game which actions it offers
```

`--allow-actions` and `actions trigger` exist for a future touch UI. Only a fixed, built-in allowlist can ever be triggered (see `sender-rs/src/actions.rs`); the action names are unverified placeholders.

PTU / build note: the schema is per game build. After a patch, run `probe` and `dump`, then compare with `schema/fields.md` (see its live verification checklist).

## Protocol

Binary frames over 115200 baud USB serial using COBS framing with CRC-8. Protocol v2, documented in [docs/PROTOCOL.md](docs/PROTOCOL.md).

```
[0x00] [COBS-encoded: msg_type | payload... | CRC8] [0x00]
```

| ID | Name | Direction | Payload |
|----|------|-----------|---------|
| `0x01` | Telemetry | PC -> ESP32 | shield F/B/L/R, hydrogen, quantum (0-255), valid bitmask |
| `0x02` | Status | PC -> ESP32 | 10 annunciator nibbles, flags, flags_valid, gear, master mode |
| `0x03` | Vitals | PC -> ESP32 | health, oxygen, valid bitmask |
| `0x04` | Link | PC -> ESP32 | game link state, source |
| `0x05` | Hello | both | protocol version, role |
| `0x06` | VehicleName | PC -> ESP32 | printable ASCII, up to 32 bytes |
| `0x10` | Action | ESP32 -> PC | action id, sequence (touch UI not implemented yet) |
| `0x11` | ActionAck | PC -> ESP32 | action id, sequence, result, game status |

The protocol is shared between `proto/` (Rust `no_std` crate used by the sender) and `lib/hud_proto/` (C headers used by the ESP32 sketch). `tools/proto-ctest` compiles the C headers and checks them against the Rust implementation.

## Project Structure

```
Cargo.toml                    # workspace (proto, sender-rs, tools/proto-ctest)
platformio.ini, include/      # PlatformIO build (src_dir = ship_hud) and lv_conf.h
scripts/sync_headers.py       # copies lib/**/*.h into ship_hud/ (Arduino IDE needs them there)
schema/                       # community .proto, field table, probe output
docs/                         # SETUP.md, PROTOCOL.md
proto/                        # shared Rust protocol crate (no_std) + golden tests
lib/hud_proto/                # C protocol headers (mirror proto/)
lib/hud_widgets/              # LVGL widgets + HudState.h / Smoother.h (pure logic, tested off-target)
sender-rs/                    # hud-sender (lib + bin): sources, mapping, policy, serial, probe/dump/inspect, mock server
tools/proto-ctest/            # C++ <-> Rust parity tests and header-copy check
ship_hud/                     # Arduino sketch + generated header copies
```

After editing anything in `lib/`, run `python scripts/sync_headers.py`. `cargo test --workspace` fails if the copies differ. `cargo test --workspace` needs a C++ compiler (MSVC or g++).

## Quick Start

### ESP32

See [docs/SETUP.md](docs/SETUP.md) for Arduino IDE and PlatformIO setup. With PlatformIO: `pio run -t upload`.

### Tests

```sh
cargo test --workspace
python scripts/sync_headers.py --check
pio run -e esp32
```

## License

GNU General Public License v3.0. See [LICENSE](LICENSE).
