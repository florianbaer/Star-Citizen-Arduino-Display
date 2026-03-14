# MSFS 2024 Gyroscope Sender

A companion app that reads aircraft attitude data from Microsoft Flight Simulator 2024 via SimConnect and sends it to the ESP32 display over USB serial.

## Installation (Pre-built .exe)

1. Download `msfs-gyro-sender.exe` from the latest [GitHub Actions build](../../actions)
2. Connect your ESP32 display via USB
3. Start MSFS 2024 and load into a flight
4. Run:

```
msfs-gyro-sender.exe COM6
```

That's it. The exe is a standalone Windows executable — no Python or other dependencies needed.

### Options

```
msfs-gyro-sender.exe COM6 --baud 115200 --hz 30
```

| Flag | Default | Description |
|------|---------|-------------|
| `--baud` | 115200 | Serial baud rate |
| `--hz` | 20 | Send rate in Hz |

## Installation (From source)

Requires **Windows 10/11**, **Python 3.10+ (64-bit)**.

```sh
cd msfs-sender
pip install -r requirements.txt
python -m msfs_sender COM6
```

### Building the .exe locally

```sh
cd msfs-sender
pip install pyinstaller pyserial SimConnect
pyinstaller msfs_sender.spec
# Output: dist/msfs-gyro-sender.exe
```

## Switching Display Modes

The ESP32 display supports two modes:

- **Star Citizen HUD** — Shield gauges, fuel bars, alerts
- **MSFS Gyroscope** — Artificial horizon with pitch, roll, and heading

**Tap the touchscreen** to toggle between modes. Both modes receive data simultaneously regardless of which screen is active.

## How It Works

1. The sender connects to MSFS via the SimConnect SDK
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

**"SimConnect connection failed"**
-> Make sure MSFS 2024 is running and you're in a flight (not the main menu).

**Serial port not found**
-> Check the port name in Device Manager. On Windows it's typically `COM3`-`COM9`.

**No data on display**
-> Verify baud rates match (default 115200). Make sure you're on the MSFS gyroscope screen (tap to toggle).
