# Rustmix-Wave Display Backend Import v0

## Scope

This slice imports the accepted Focus Hub Waveshare display backend into
`hal-waveshare-epd397` and adds a minimal `target-waveshare-epd397` display
smoke binary.

## Accepted pin map

- EPD_SCLK GPIO11
- EPD_MOSI GPIO12
- EPD_CS GPIO10
- EPD_DC GPIO9
- EPD_RST GPIO46
- EPD_BUSY GPIO3

GPIO3 is EPD_BUSY and must not be used for input.

## Imported backend pieces

- `init_display_free`
- `clear_display_free`
- `write_frame_free`
- `refresh_display_free`
- `sleep_display_free`
- `DisplayBackendAdapter`
- `ShellDisplayBridge`

## What this slice does not do

- Does not port the Rustmix reader.
- Does not enable rotary input.
- Does not add audio/voice.
- Does not delete X4 code.

## Smoke test

Build:

```bash
source "$HOME/export-esp.sh"
cargo +esp build -p target-waveshare-epd397 --release --target xtensa-esp32s3-espidf
```

Flash:

```bash
espflash flash \
  --chip esp32s3 \
  --port "$PORT" \
  --baud 921600 \
  --monitor \
  target/xtensa-esp32s3-espidf/release/target-waveshare-epd397 \
  | rg 'RAW-|RUSTMIX-WAVE|DISPLAY|BUSY|panic|assertion|boot-error|rst:|Saved PC'
```

Expected physical result: clean black refresh, pause, clean white refresh.
