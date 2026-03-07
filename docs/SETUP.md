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

Open `ship_hud/ship_hud.ino` and upload.

> **Note:** The widget and protocol headers are duplicated in `ship_hud/` because Arduino IDE cannot resolve paths outside the sketch folder. The canonical sources are `lib/hud_widgets/` and `lib/hud_proto/`. PlatformIO does not have this limitation.

## PlatformIO

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

### 2. Build & Upload

```sh
pio run -t upload
pio device monitor
```

The `lib/hud_widgets/` and `lib/hud_proto/` folders are automatically picked up by PlatformIO.
