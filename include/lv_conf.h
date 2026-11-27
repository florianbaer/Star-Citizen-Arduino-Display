#ifndef LV_CONF_H
#define LV_CONF_H

#ifndef __ASSEMBLER__
#include <stdint.h>
#endif

#define LV_USE_DEV_VERSION
#define LV_COLOR_DEPTH 16
#define LV_MEM_CUSTOM 0
#define LV_MEM_SIZE (64U * 1024U)
#define LV_DEF_REFR_PERIOD 33
#define LV_DEF_INDEV_READ_PERIOD 50
#define LV_USE_TFT_ESPI 0

#define LV_FONT_MONTSERRAT_14 1
#define LV_FONT_DEFAULT &lv_font_montserrat_14

#define LV_USE_SLIDER 1
#define LV_USE_LABEL  1
#define LV_USE_BTN    1
#define LV_USE_LOG 0

#endif
