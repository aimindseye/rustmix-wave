# Rustmix Remote BLE Feature Build Boundary

The `rustmix-remote-ble` build is intentionally separate from the normal Wi-Fi build.

## Accepted BLE behavior

Validated on hardware:

- Samsung Wear OS Rustmix Remote app connects to Rustmix-Wave over BLE GATT.
- TXT reader Previous/Next page turning works.
- EPUB reader Previous/Next page turning works.
- RRBP packets are written to the command characteristic.
- Commands are queued and drained by the main loop.

## Why Wi-Fi is disabled in this build

The Waveshare 3.97-inch e-paper device uses ESP32-S3. Wi-Fi and Bluetooth LE share the ESP32-S3 RF subsystem. Rustmix-Wave also has e-paper refresh timing, SD card access, reader cache workers, and optional network/audio tasks.

The accepted BLE build gives BLE ownership of the modem to keep watch page turning reliable. Therefore, these features are intentionally skipped in the BLE build:

- Wi-Fi connection
- Wi-Fi transfer portal
- NTP/time sync
- weather/network fetch

Use the Wi-Fi release for those features.

## Build command

```bash
export PATH="$HOME/.cargo/bin:$PATH"
export RUSTFLAGS="${RUSTFLAGS:-} --cfg esp_idf_version_least_5_5_0"

cargo +esp build \
  --release \
  --target xtensa-esp32s3-espidf \
  --features rustmix-remote-ble
```

## Flash command

```bash
espflash flash --chip esp32s3 --monitor \
  target/xtensa-esp32s3-espidf/release/waveshare-epd397-rust-app
```
