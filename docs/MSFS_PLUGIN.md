# MSFS 2024 Flight Data Sender

A companion app that reads flight data from Microsoft Flight Simulator 2024 via SimConnect and sends it to the ESP32 display over USB serial. Supports 4 MSFS display screens: gyroscope, engine gauges, flight data, and G-force meter.

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

## Auto-start with MSFS

MSFS can automatically launch companion apps on startup via an `exe.xml` file.

**File location:**
- **Steam**: `%APPDATA%\Microsoft Flight Simulator 2024\exe.xml`
- **MS Store**: `%LOCALAPPDATA%\Packages\Microsoft.FlightSimulator_8wekyb3d8bbwe\LocalCache\exe.xml`

If the file doesn't exist, create it. If it already exists, just add the `<Launch.Addon>` block inside the existing `<SimBase.Document>`.

```xml
<?xml version="1.0" encoding="windows-1252"?>
<SimBase.Document Type="Launch" version="1,0">
  <Descr>Launch</Descr>
  <Filename>exe.xml</Filename>
  <Disabled>False</Disabled>
  <Launch.ManualLoad>False</Launch.ManualLoad>
  <Launch.Addon>
    <Name>ESP32 Gyroscope Display</Name>
    <Disabled>False</Disabled>
    <ManualLoad>False</ManualLoad>
    <Path>C:\Your\Path\msfs-gyro-sender.exe</Path>
    <CommandLine>COM6</CommandLine>
  </Launch.Addon>
</SimBase.Document>
```

Replace `C:\Your\Path\` with the actual folder where you saved the exe, and `COM6` with your ESP32 serial port.

To disable auto-start later, change `<Disabled>False</Disabled>` to `<Disabled>True</Disabled>`.

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

## Display Screens

The ESP32 display has 5 screens cycled by touch tap:

| # | Screen | Data shown |
|---|--------|------------|
| 0 | **Star Citizen HUD** | Shield gauges, fuel bars, alerts |
| 1 | **MSFS Gyroscope** | Artificial horizon with pitch, roll, heading |
| 2 | **MSFS Engine Gauges** | RPM arc, throttle bar, oil temp/pressure, fuel flow |
| 3 | **MSFS Flight Data** | Airspeed, altitude, vertical speed, ground speed |
| 4 | **MSFS G-Force Meter** | Vertical/lateral/longitudinal G, peak tracking |

**Tap the touchscreen** to cycle to the next screen. All screens receive data simultaneously regardless of which is active.

## How It Works

1. The sender connects to MSFS via the SimConnect SDK
2. Each loop iteration reads attitude, engine, flight data, and G-force variables
3. Values are packed into protocol messages and framed with COBS encoding + CRC8 checksum
4. All 4 frames are sent over USB serial to the ESP32 at the configured Hz rate
5. The ESP32 dispatches each message to the appropriate widget

## Protocol

Four MSFS message types are sent every cycle:

**Attitude (0x02)** — 6 bytes

| Field | Type | Range | Description |
|-------|------|-------|-------------|
| pitch | int16 LE | -1800..+1800 | Tenths of degrees, nose up positive |
| roll | int16 LE | -1800..+1800 | Tenths of degrees, right wing down positive |
| heading | int16 LE | 0..3599 | Tenths of degrees, true north |

**Engine (0x03)** — 6 bytes

| Field | Type | Range | Description |
|-------|------|-------|-------------|
| rpm | uint16 LE | 0..65535 | Engine RPM |
| throttle | uint8 | 0..100 | Throttle percentage |
| fuel_flow | uint8 | 0..255 | Fuel flow (mapped) |
| oil_temp | uint8 | 0..255 | Oil temperature (mapped) |
| oil_press | uint8 | 0..255 | Oil pressure (mapped) |

**Flight Data (0x04)** — 10 bytes

| Field | Type | Range | Description |
|-------|------|-------|-------------|
| airspeed | uint16 LE | 0..65535 | Indicated airspeed in tenths of knots |
| altitude | int32 LE | signed | Altitude in feet |
| vspeed | int16 LE | signed | Vertical speed in fpm |
| ground_speed | uint16 LE | 0..65535 | Ground speed in tenths of knots |

**G-Force (0x05)** — 6 bytes

| Field | Type | Range | Description |
|-------|------|-------|-------------|
| gforce_x | int16 LE | signed | Longitudinal G in hundredths |
| gforce_y | int16 LE | signed | Vertical G in hundredths (~100 = 1G) |
| gforce_z | int16 LE | signed | Lateral G in hundredths |

## Troubleshooting

**"SimConnect connection failed"**
-> Make sure MSFS 2024 is running and you're in a flight (not the main menu).

**Serial port not found**
-> Check the port name in Device Manager. On Windows it's typically `COM3`-`COM9`.

**No data on display**
-> Verify baud rates match (default 115200). Make sure you're on one of the MSFS screens (tap to cycle through all 5).
