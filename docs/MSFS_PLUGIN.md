# MSFS 2024 Gyroscope Plugin

Reads aircraft attitude data from Microsoft Flight Simulator 2024 via SimConnect and displays an artificial horizon on the ESP32 display.

## Prerequisites

- **Windows 10/11** (SimConnect is Windows-only)
- **MSFS 2024** installed and running
- **Python 3.10+** (64-bit required)
- **ESP32 display** connected via USB

## Installation

```sh
cd msfs-sender
pip install -r requirements.txt
```

Or install as a package:

```sh
cd msfs-sender
pip install .
```

## Usage

1. Flash the ESP32 with the updated `ship_hud` sketch
2. Start MSFS 2024 and load into a flight
3. Run the sender:

```sh
python -m msfs_sender COM6
```

### Options

| Flag | Default | Description |
|------|---------|-------------|
| `--baud` | 115200 | Serial baud rate |
| `--hz` | 20 | Send rate in Hz |

```sh
python -m msfs_sender COM6 --baud 115200 --hz 30
```

## Switching Display Modes

The ESP32 display supports two modes:

- **Star Citizen HUD** — Shield gauges, fuel bars, alerts
- **MSFS Gyroscope** — Artificial horizon with pitch, roll, and heading

**Tap the touchscreen** to toggle between modes. Both modes receive data simultaneously regardless of which screen is active.

## How It Works

1. The Python sender connects to MSFS via the SimConnect SDK
2. It reads `PLANE_PITCH_DEGREES`, `PLANE_BANK_DEGREES`, and `PLANE_HEADING_DEGREES_TRUE` (which return radians despite the names)
3. Values are converted to tenths of degrees and packed into an `AttitudeMsg` (6 bytes: 3x int16 LE)
4. The message is framed with COBS encoding + CRC8 checksum
5. The frame is sent over USB serial to the ESP32
6. The ESP32 renders the artificial horizon on an LVGL canvas

## Protocol

The attitude message uses message type `0x02`:

| Field | Type | Range | Description |
|-------|------|-------|-------------|
| pitch | int16 LE | -1800..+1800 | Tenths of degrees, nose up positive |
| roll | int16 LE | -1800..+1800 | Tenths of degrees, right wing down positive |
| heading | int16 LE | 0..3599 | Tenths of degrees, true north |

## Troubleshooting

**"OSError: [WinError 193] %1 is not a valid Win32 application"**
→ You're using 32-bit Python. Install 64-bit Python.

**"SimConnect connection failed"**
→ Make sure MSFS 2024 is running and you're in a flight (not the main menu).

**Serial port not found**
→ Check the port name in Device Manager. On Windows it's typically `COM3`-`COM9`.

**No data on display**
→ Verify baud rates match (default 115200). Make sure you're on the MSFS gyroscope screen (tap to toggle).
