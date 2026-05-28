# hal-waveshare-epd397

Hardware abstraction skeleton for the Waveshare ESP32-S3 e-Paper 3.97 target.

This crate/folder is intentionally a skeleton in Repository Bootstrap v0.

Planned responsibilities:

- Display backend for Waveshare 3.97 e-paper.
- Rotary dial and safe button input mapping.
- Audio codec / microphone / speaker bring-up.
- Power, RTC, sensors, storage, Wi-Fi, and BLE board services.
- Board-specific pin ownership and hardware initialization.

Display backend source of truth for the next slice:

- Accepted Focus Hub free-function display backend.
- Accepted DisplayBackendAdapter.
- Accepted ShellDisplayBridge portrait mapping.
- Accepted Waveshare display pin map:
  - EPD_SCLK GPIO11
  - EPD_MOSI GPIO12
  - EPD_CS GPIO10
  - EPD_DC GPIO9
  - EPD_RST GPIO46
  - EPD_BUSY GPIO3

Do not port display code in this bootstrap slice.
