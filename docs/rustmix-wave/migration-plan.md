# Rustmix-Wave Migration Plan

## Source repositories

- Product source base: Rustmix X4 firmware.
- Hardware bring-up source: Focus Hub Waveshare firmware experiments.

## Keep from Rustmix X4

- Reader/product model.
- Books/library state.
- Progress and bookmarks.
- Wi-Fi transfer.
- Dictionary shards.
- Flashcards.
- Lua app model.
- Fonts and asset conventions where portable.

## Replace for Waveshare

- X4 ESP32-C3 target setup.
- X4 e-paper driver and pin map.
- X4 input mapping.
- X4 power assumptions.
- X4 orientation assumptions.

## Import from Focus Hub bring-up later

- Accepted Waveshare display pin map.
- Free display backend.
- DisplayBackendAdapter.
- ShellDisplayBridge.
- Portrait mapping.
- Rotary-first UI experiments.

## Bootstrap rule

Do not delete the X4 code until the Rustmix-Wave target has its own display,
storage, reader, and UI path.
