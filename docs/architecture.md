# Rustmix Wave Architecture

Rustmix Wave is an embedded Rust application for the Waveshare ESP32-S3 3.97-inch e-paper device.

## Hardware target

- Board class: Waveshare ESP32-S3 3.97-inch e-paper device
- Display: 800 x 480 e-paper panel
- MCU: ESP32-S3
- Storage: SD card
- Memory: internal RAM plus PSRAM
- Wireless: Wi-Fi and Bluetooth LE through ESP32-S3 radio subsystem

## Main firmware architecture

```text
Boot / board init
        |
        v
SD card mount and config load
        |
        v
Services
  - display
  - buttons / power key
  - RTC / alarms
  - audio / voice notes
  - storage browser
  - reader state
  - optional Wi-Fi services
  - optional BLE Remote service
        |
        v
Main UI router
        |
        v
Apps and reader
  - TXT reader
  - EPUB reader
  - calendar
  - voice notes
  - dictionary
  - games
  - settings
```

## Release variants

Rustmix Wave publishes two firmware variants for the same hardware.

### Wi-Fi build

The Wi-Fi build is the normal daily-use firmware.

```text
cargo +esp build --release --target xtensa-esp32s3-espidf
```

Enabled behavior:

- Wi-Fi config
- Wi-Fi transfer
- weather/network fetch
- NTP/time sync
- normal reader/apps/settings behavior

BLE Remote is not enabled in this build.

### BLE Remote build

The BLE Remote build enables the Rustmix Remote BLE GATT service.

```text
cargo +esp build --release --target xtensa-esp32s3-espidf --features rustmix-remote-ble
```

Enabled behavior:

- BLE GATT advertising
- Rustmix Remote service UUID
- command characteristic
- RRBP packet parsing
- command queue
- TXT reader page turning
- EPUB reader page turning

Wi-Fi/network services are intentionally skipped in this build.

## Why Wi-Fi and BLE are separate

The ESP32-S3 supports Wi-Fi and Bluetooth LE, but both share the same 2.4 GHz RF subsystem. ESP-IDF uses coexistence/time-sharing to coordinate radio access. Rustmix Wave also has substantial memory and timing pressure from e-paper refresh, SD card access, EPUB/TXT pagination, audio, weather/network workers, and UI routing.

For the accepted Rustmix Remote BLE path, the BLE feature build gives BLE ownership of the modem and disables Wi-Fi/network features. This produces a reliable page-turning firmware for the Wear OS remote while preserving the normal Wi-Fi firmware as a separate release.

## Rustmix Remote BLE path

```text
Samsung Wear OS watch
        |
        | BLE GATT write
        v
Rustmix-Wave BLE GATT service
        |
        | RRBP parse/enqueue
        v
Main loop drains command queue
        |
        | reader event
        v
TXT / EPUB page navigation
```

## RRBP command packet

RRBP packets are 6 bytes:

```text
byte 0: version
byte 1: sequence
byte 2: command
byte 3: flags
byte 4: parameter
byte 5: reserved
```

Accepted command examples:

```text
01 00 01 00 00 00 = page next
01 02 02 00 00 00 = page previous
```

## BLE callback safety

The BLE callback must not mutate reader state or UI state directly.

Accepted boundary:

```text
BLE callback: parse and enqueue only
main loop: drain queue and mutate reader/UI state
```

This is required to keep page turning safe and deterministic.

## Documentation references

Espressif documents ESP32-S3 RF coexistence as a shared 2.4 GHz RF subsystem managed with time-division multiplexing. See:

- https://docs.espressif.com/projects/esp-idf/en/stable/esp32s3/api-guides/coexist.html
- https://docs.espressif.com/projects/esp-faq/en/latest/software-framework/coexistence.html
