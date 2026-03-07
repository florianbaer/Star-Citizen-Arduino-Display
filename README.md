# Spaceship HUD

A spaceship shield/fuel HUD for the **ESP32-2432S024C** (Cheap Yellow Display with capacitive touch), built with LVGL.

Receives telemetry over USB serial and displays:
- 4 concentric arc shield gauges (FWD/AFT/PRT/STB) that grow from center
- 2 horizontal fuel bars (H-FUEL / Q-FUEL)
- Ship silhouette with blinking warning indicator

## Hardware

- **Board**: ESP32-2432S024C (2.4" 320x240 ILI9341, capacitive touch CST816S)
- **Touch**: CST816S on I2C (SDA=33, SCL=32)
- **Display**: ILI9341 on HSPI

## Serial Protocol

CSV over 115200 baud USB serial:

```
SF:200,SB:100,SL:150,SR:255,HF:80,QF:40\n
```

| Key | Field | Range |
|-----|-------|-------|
| SF | Shield Front | 0-255 |
| SB | Shield Back | 0-255 |
| SL | Shield Left | 0-255 |
| SR | Shield Right | 0-255 |
| HF | Hydrogen Fuel | 0-255 |
| QF | Quantum Fuel | 0-255 |

## Widget Library API

The HUD is built from header-only C++ widgets in `lib/hud_widgets/`. Include everything via `hud_widgets.h` or pick individual headers.

### ColorScale

Threshold-based color mapping. Returns an `lv_color_t` for a given 0-255 value by matching against ordered thresholds.

```cpp
#include "ColorScale.h"

// Thresholds are checked top-down: first match where value > above wins
const ColorThreshold thresholds[] = {
  {170, 0, 180, 255},   // > 170 -> cyan
  {85,  255, 200, 0},   // > 85  -> yellow
  {0,   255, 30, 0}     // > 0   -> red
};

ColorScale cs(thresholds);          // construct from array
lv_color_t color = cs.get(200);     // -> cyan

ColorScale cs2;
cs2.set(thresholds, 3);             // or set dynamically (max 8 thresholds)
```

### ShieldGauge

Concentric arc gauge for one shield direction. The arc grows outward from center as the value increases. Color changes automatically based on a built-in `ColorScale` (cyan > 170, yellow > 85, red below).

```cpp
#include "ShieldGauge.h"

ShieldGaugeConfig cfg;
cfg.cx = 160;                        // arc center X (default: 160)
cfg.cy = 85;                         // arc center Y (default: 85)
cfg.direction = ShieldDir::FWD;      // FWD, STB, AFT, or PRT
cfg.radius = 70;                     // arc radius in px (default: 70)
cfg.width = 10;                      // arc stroke width (default: 10)
cfg.maxSpan = 78;                    // arc span in degrees (default: 78)

ShieldGauge gauge;
gauge.create(screen, cfg);
gauge.setValue(200);                  // 0-255, arc grows from center
gauge.setColorScale(myColorScale);   // override default colors
uint8_t v = gauge.value();           // read current value
```

### FuelBar

Horizontal bar with a percentage label. Animates on value change.

```cpp
#include "FuelBar.h"

FuelBarConfig cfg;
cfg.label = "H-FUEL";               // label text (default: "FUEL")
cfg.width = 200;                     // bar width in px (default: 200)
cfg.height = 16;                     // bar height in px (default: 16)
cfg.r = 0; cfg.g = 180; cfg.b = 220; // bar color (default: cyan)
cfg.barAlign = LV_ALIGN_BOTTOM_RIGHT;
cfg.barOffsetX = -10;
cfg.barOffsetY = -36;
cfg.labelAlign = LV_ALIGN_BOTTOM_LEFT;
cfg.labelOffsetX = 10;
cfg.labelOffsetY = -38;

FuelBar bar;
bar.create(screen, cfg);
bar.setValue(128);                    // 0-255, label shows "H-FUEL 50%"
```

### ShipSilhouette

Line-drawn ship shape rendered at screen center.

```cpp
#include "ShipSilhouette.h"

ShipSilhouetteConfig cfg;
cfg.cx = 160;                        // center X (default: 160)
cfg.cy = 85;                         // center Y (default: 85)
cfg.scale = 1.0f;                    // size multiplier (default: 1.0)
cfg.r = 0; cfg.g = 80; cfg.b = 80;  // line color (default: dark teal)
cfg.lineWidth = 2;                   // stroke width (default: 2)

ShipSilhouette ship;
ship.create(screen, cfg);
```

### AlertIndicator

Blinking warning icon with heartbeat LED pattern. Activates when shields are critical. The LED plays a lub-dub pattern (3 beats with 1s gaps) on transition to active.

```cpp
#include "AlertIndicator.h"

AlertIndicatorConfig cfg;
cfg.x = 153;                         // icon X position (default: 153)
cfg.y = 77;                          // icon Y position (default: 77)
cfg.symbol = LV_SYMBOL_WARNING;      // LVGL symbol (default: warning)
cfg.r = 255; cfg.g = 30; cfg.b = 0;  // icon color (default: red)
cfg.blinkMs = 500;                   // blink interval ms (default: 500)
cfg.ledPin = 4;                      // GPIO for LED (-1 to disable)
cfg.ledChannel = 0;                  // LEDC PWM channel (default: 0)

AlertIndicator alert;
alert.create(screen, cfg);
alert.setActive(true);               // show icon + trigger LED heartbeat
alert.tick(millis());                // call every loop iteration
```

### TelemetryParser

Callback-driven CSV parser for the serial protocol. Register field handlers by key, then feed a `Stream`.

```cpp
#include "TelemetryParser.h"

TelemetryParser parser;
parser.onField("SF", [](uint8_t v) { shields[0].setValue(v); });
parser.onField("HF", [](uint8_t v) { hfuel.setValue(v); });
// ... register up to 16 fields

// In loop():
parser.feed(Serial);                 // reads available bytes, parses complete lines
```

## Setup — Arduino IDE

### 1. Add ESP32 Board Support

In **File > Preferences**, add this URL to **Additional Board Manager URLs**:

```
https://raw.githubusercontent.com/espressif/arduino-esp32/gh-pages/package_esp32_index.json
```

Then go to **Tools > Board > Boards Manager**, search for **esp32** by Espressif Systems, and install version **3.2.0**.

### 2. Select Board

- **Tools > Board**: `ESP32 Dev Module`
- **Tools > Flash Size**: `4MB`
- **Tools > Partition Scheme**: `Default 4MB with spiffs`
- **Tools > Upload Speed**: `921600`
- **Tools > Port**: Select the COM port for your board

### 3. Install Libraries

Install via **Sketch > Include Library > Manage Libraries**:

| Library | Version | Notes |
|---------|---------|-------|
| **lvgl** | 9.2.2 | LVGL graphics library |
| **TFT_eSPI** | 2.5.43 | TFT display driver |

### 4. Configure TFT_eSPI

Replace `User_Setup.h` in your TFT_eSPI library folder (`Arduino/libraries/TFT_eSPI/User_Setup.h`) with:

```cpp
#define USER_SETUP_INFO "ESP32-2432S024"
#define ILI9341_2_DRIVER

#define TFT_WIDTH  240
#define TFT_HEIGHT 320

#define TFT_MISO 12
#define TFT_MOSI 13
#define TFT_SCLK 14
#define TFT_CS   15
#define TFT_DC    2
#define TFT_RST  -1
#define TFT_BL   21

#define USE_HSPI_PORT

#define LOAD_GLCD
#define LOAD_FONT2
#define LOAD_FONT4
#define LOAD_FONT6
#define LOAD_FONT7
#define LOAD_FONT8
#define LOAD_GFXFF
#define SMOOTH_FONT

#define SPI_FREQUENCY       55000000
#define SPI_READ_FREQUENCY  20000000
#define SPI_TOUCH_FREQUENCY  2500000
```

### 5. Configure LVGL

Copy `lv_conf.h` next to the lvgl library folder (e.g. `Arduino/libraries/lv_conf.h`, **not** inside `lvgl/`):

```cpp
#ifndef LV_CONF_H
#define LV_CONF_H

#ifndef __ASSEMBLER__
#include <stdint.h>
#endif

#define LV_USE_DEV_VERSION
#define LV_COLOR_DEPTH 16
#define LV_MEM_CUSTOM 0
#define LV_MEM_SIZE (48U * 1024U)
#define LV_DEF_REFR_PERIOD 33
#define LV_DEF_INDEV_READ_PERIOD 50
#define LV_USE_TFT_ESPI 1

#define LV_FONT_MONTSERRAT_14 1
#define LV_FONT_DEFAULT &lv_font_montserrat_14

#define LV_USE_SLIDER 1
#define LV_USE_LABEL  1
#define LV_USE_BTN    1
#define LV_USE_LOG 0

#endif
```

### 6. Upload

The widget headers (`.h` files) are included directly in the `ship_hud/` sketch folder. Open `ship_hud/ship_hud.ino` and upload.

> **Note:** The canonical source for the widget library is `lib/hud_widgets/`. The headers are duplicated into the sketch folder because Arduino IDE copies sketches to a temp build directory and cannot resolve paths outside it. If you modify a widget header, update both copies. PlatformIO does not have this limitation.

## Setup — PlatformIO

### 1. Create `platformio.ini`

```ini
[env:esp32]
platform = espressif32@6.10.0
board = esp32dev
framework = arduino

monitor_speed = 115200
upload_speed = 921600

lib_deps =
    lvgl/lvgl@^9.2.2
    bodmer/TFT_eSPI@^2.5.43

build_flags =
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
    -DTFT_BL=21
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

### 2. Project Structure

The `lib/hud_widgets/` folder is automatically picked up by PlatformIO. Move `ship_hud/ship_hud.ino` to `src/main.cpp` (or keep it as-is with Arduino IDE).

### 3. Build & Upload

```sh
pio run -t upload
pio device monitor
```

## Rust Sender

Replaces the previous Node.js sender. Sends randomized telemetry over serial with smooth 60Hz interpolation between random targets.

### Build

```sh
cd sender-rs
cargo build --release
```

### Run

```sh
# Default: new random target every 5 seconds
cargo run -- COM6

# Custom interval
cargo run -- COM6 --interval 2

# Custom baud rate
cargo run -- /dev/ttyUSB0 --baud 115200 --interval 3
```

### Sender Architecture

| Module | Description |
|--------|-------------|
| `main.rs` | CLI args (clap), main loop — picks random targets every N seconds, lerps at 60Hz |
| `protocol.rs` | `Telemetry` struct with `zero()`, `random()`, `lerp()`, and `to_csv()` |
| `serial.rs` | `HudSerial` wrapper around `serialport` crate for open/send |

## Known Limitations

### C++ standard (no designated initializers)

The ESP32 Arduino toolchain uses C++11/14 by default. C++20 designated initializers
(e.g. `{.direction = ShieldDir::FWD}`) are not supported, so widget configs must be
created as named variables with field assignments:

```cpp
ShieldGaugeConfig cfg;
cfg.direction = ShieldDir::FWD;
shield.create(scr, cfg);
```

<!-- TODO: Adding `-std=gnu++20` to build_flags in PlatformIO may enable designated
     initializers. Arduino IDE would need a platform.local.txt override. Worth testing
     once espressif32 platform updates to GCC 13+. -->

### Arduino IDE include path

Arduino IDE cannot include headers from outside the sketch folder. The widget `.h` files
are duplicated in `ship_hud/` (canonical source is `lib/hud_widgets/`). PlatformIO picks
up `lib/` automatically and does not need the copies.

## Project Structure

```
displayh/
├── lib/
│   └── hud_widgets/          # C++ header-only widget library
│       ├── hud_widgets.h     # umbrella include
│       ├── ShieldGauge.h     # concentric arc shield widget
│       ├── FuelBar.h         # horizontal fuel bar
│       ├── ShipSilhouette.h  # line-drawn ship shape
│       ├── AlertIndicator.h  # blinking warning icon
│       ├── TelemetryParser.h # CSV serial protocol parser
│       └── ColorScale.h      # threshold-based color mapping
├── sender-rs/                # Rust telemetry sender
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── protocol.rs
│       └── serial.rs
└── ship_hud/
    └── ship_hud.ino          # main sketch
```

## License

This project is licensed under the GNU General Public License v3.0. See [LICENSE](LICENSE) for the full text.
