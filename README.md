# Spaceship HUD

A spaceship shield/fuel HUD for the **ESP32-2432S024C** (Cheap Yellow Display), built with LVGL. Receives binary telemetry over USB serial and displays 4 shield arc gauges, 2 fuel bars, a ship silhouette, and a blinking alert indicator.

Supports two switchable display modes:
- **Star Citizen HUD** — Shield gauges, fuel bars, alerts
- **MSFS 2024 Gyroscope** — Artificial horizon with pitch, roll, and heading

Tap the touchscreen to toggle between modes.

## Hardware

- **Board**: ESP32-2432S024C (2.4" 320x240 ILI9341, capacitive touch CST816S)
- **Touch**: CST816S on I2C (SDA=33, SCL=32)
- **Display**: ILI9341 on HSPI

## Protocol

Binary frames over 115200 baud USB serial using COBS framing with CRC8 error checking.

### Wire format

```
[0x00] [COBS-encoded: msg_type | payload... | CRC8] [0x00]
```

### Message types

| ID | Name | Direction | Payload |
|----|------|-----------|---------|
| `0x01` | Telemetry | PC -> ESP32 | 6 bytes: SF, SB, SL, SR, HF, QF (all 0-255) |
| `0x02` | Attitude | PC -> ESP32 | 6 bytes: pitch, roll, heading (int16 LE, tenths of degrees) |

The protocol is shared between:
- **`proto/`** — Rust `no_std` crate (used by the senders)
- **`lib/hud_proto/`** — C headers (used by the ESP32 sketch)

## Project Structure

```
├── proto/                    # Shared Rust protocol crate (no_std)
│   └── src/
│       ├── cobs.rs           # COBS encode/decode
│       ├── crc8.rs           # CRC8/MAXIM checksum
│       ├── messages.rs       # Message types + packed structs
│       └── frame.rs          # Frame/deframe with delimiters
├── lib/
│   ├── hud_proto/            # C protocol headers (mirrors proto/)
│   │   ├── frame_decoder.h   # Stream-fed COBS frame decoder
│   │   ├── messages.h        # Packed C structs matching Rust layout
│   │   ├── cobs.h            # COBS decode
│   │   └── crc8.h            # CRC8/MAXIM
│   └── hud_widgets/          # C++ LVGL widget library
│       ├── ShieldGauge.h     # Concentric arc shield gauge
│       ├── FuelBar.h         # Horizontal fuel bar
│       ├── ShipSilhouette.h  # Line-drawn ship shape
│       ├── AlertIndicator.h  # Blinking warning + heartbeat LED
│       ├── GyroHorizon.h     # Artificial horizon (MSFS gyroscope)
│       └── ColorScale.h      # Threshold-based color mapping
├── sender-rs/                # Rust telemetry sender (Star Citizen)
│   └── src/
│       ├── main.rs           # CLI, 60Hz interpolated send loop
│       └── serial.rs         # Serial port + frame encoding
├── msfs-sender/              # Python MSFS 2024 attitude sender
│   ├── msfs_sender/
│   │   ├── __main__.py       # CLI entry point
│   │   ├── protocol.py       # COBS + CRC8 framing
│   │   └── simconnect_source.py  # SimConnect wrapper
│   └── tests/
│       └── test_protocol.py  # Protocol unit tests
└── ship_hud/
    └── ship_hud.ino          # Main ESP32 sketch (dual-mode)
```

## Quick Start

### ESP32

See [docs/SETUP.md](docs/SETUP.md) for full Arduino IDE and PlatformIO setup instructions.

### Sender — Star Citizen (Rust)

```sh
cd sender-rs
cargo run -- COM6                    # default: 5s interval, 115200 baud
cargo run -- COM6 --interval 2       # faster changes
cargo run -- /dev/ttyUSB0 --baud 921600
```

### Sender — MSFS 2024 (Python)

See [docs/MSFS_PLUGIN.md](docs/MSFS_PLUGIN.md) for full setup instructions.

```sh
cd msfs-sender
pip install -r requirements.txt
python -m msfs_sender COM6           # default: 20Hz, 115200 baud
python -m msfs_sender COM6 --hz 30   # faster updates
```

## License

GNU General Public License v3.0. See [LICENSE](LICENSE).
