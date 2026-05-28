# target-waveshare-epd397

Target skeleton for Rustmix-Wave on the Waveshare ESP32-S3 e-Paper 3.97 board.

Repository Bootstrap v0 does not port display code yet and does not delete the
existing Rustmix X4 target/code.

Planned target flow:

1. Import accepted Focus Hub display backend into `hal-waveshare-epd397`.
2. Create a minimal Waveshare dashboard binary.
3. Port Rustmix reader/product model behind display/storage/input abstractions.
4. Add rotary-first navigation.
5. Add voice assistant states and audio bring-up.
