# Board contract

Target board: Waveshare ESP32-S3 e-paper 3.97-inch development board.

## Display

```text
Controller     SSD1677
Native frame   800 × 480 monochrome
Logical UI     480 × 800 portrait
SCLK           GPIO11
MOSI           GPIO12
CS             GPIO10
DC             GPIO9
RST            GPIO46
BUSY           GPIO3
```

## User inputs

```text
Rotary wheel / Select   Primary UI navigation
BOOT                    GPIO0, short contextual action, long hierarchical Back
Power key               AXP2101 PEK short / long interrupts
```

Power-key product behavior:

```text
Short Power press   Open display-maintenance menu
Long Power press    Enter random sleep-image mode
Wake Power press    Restore retained route after quiet guard
```

## Storage

```text
SD mount       /sdcard
Product root   /sdcard/RUSTMIX
Filesystem     FAT; generated writable names must remain FAT 8.3-safe
```

## Audio and sensors

```text
Audio codec        ES8311
RTC alarm input    GPIO45 active low
Environment        SHTC3
IMU                QMI8658
```

Hardware handles remain native-owned in `src/main.rs` and its focused runtime adapters. Lua apps do not receive raw peripheral access.
