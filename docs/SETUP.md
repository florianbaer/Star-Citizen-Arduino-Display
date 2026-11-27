# Setup Guide

## Arduino IDE

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

| Library | Version |
|---------|---------|
| **lvgl** | 9.2.2 |
| **TFT_eSPI** | 2.5.43 |

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

Do not define `TFT_BL`: the backlight is GPIO 27, which the sketch drives itself, and GPIO 21 is `TOUCH_INT`. (Do not define `TFT_BACKLIGHT_ON` either.)

### 5. Configure LVGL

Copy `include/lv_conf.h` from this repo next to the lvgl library folder (e.g. `Arduino/libraries/lv_conf.h`, **not** inside `lvgl/`).

### 6. Upload

Open `ship_hud/ship_hud.ino` and upload.

> **Note:** The widget and protocol headers are duplicated in `ship_hud/` because Arduino IDE cannot resolve paths outside the sketch folder. The canonical sources are `lib/hud_widgets/` and `lib/hud_proto/`. PlatformIO does not have this limitation.

## PlatformIO

### 1. Configuration

The repo ships `platformio.ini`; run `pio run`.

### 2. Build & Upload

```sh
pio run -t upload
pio device monitor
```

PlatformIO compiles the sketch in `ship_hud/` (`src_dir = ship_hud`), so it uses the same header copies as the Arduino IDE.

## ESP32 core versions

The Arduino IDE instructions above say core **3.2.0**, while PlatformIO `espressif32@6.10.0` uses Arduino core **2.0.x**. `AlertIndicator.h` supports both (the `ledc` API differs). Only the 2.x path is compile-checked by PlatformIO.

## Header copies

After editing anything in `lib/`, run `python scripts/sync_headers.py` (use `--check` to verify). `cargo test --workspace` also enforces this.
