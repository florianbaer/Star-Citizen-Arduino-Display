Not yet working (since no udp data accessible by star citizen)

# Spaceship HUD

A spaceship shield/fuel HUD for the **ESP32-2432S024C** (Cheap Yellow Display), built with LVGL. Receives binary telemetry over USB serial and displays 4 shield arc gauges, 2 fuel bars, a ship silhouette, and a blinking alert indicator.

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

The protocol is shared between:
- **`proto/`** — Rust `no_std` crate (used by the sender)
- **`lib/hud_proto/`** — C headers (used by the ESP32 sketch)

## Project Structure

```
displayh/
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
│       └── ColorScale.h      # Threshold-based color mapping
├── sender-rs/                # Rust telemetry sender
│   └── src/
│       ├── main.rs           # CLI, 60Hz interpolated send loop
│       └── serial.rs         # Serial port + frame encoding
└── ship_hud/
    └── ship_hud.ino          # Main ESP32 sketch
```

## Quick Start

### ESP32

See [docs/SETUP.md](docs/SETUP.md) for full Arduino IDE and PlatformIO setup instructions.

### Sender (Rust)

```sh
cd sender-rs
cargo run -- COM6                    # default: 5s interval, 115200 baud
cargo run -- COM6 --interval 2       # faster changes
cargo run -- /dev/ttyUSB0 --baud 921600
```

## License

GNU General Public License v3.0. See [LICENSE](LICENSE).
