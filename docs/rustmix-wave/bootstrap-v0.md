# Rustmix-Wave Repository Bootstrap v0

## Status

This repository is now the clean product home for Rustmix-Wave.

Rustmix-Wave means:

- Rustmix product model.
- Waveshare ESP32-S3 e-Paper 3.97 hardware target.
- Rotary-first non-touch UI.
- Future voice assistant layer.
- Reuse of working Rustmix X4 reader/app code where possible.
- Reuse of accepted Focus Hub Waveshare display backend where appropriate.

## What this slice does

- Keeps Rustmix X4 code as the upstream reference.
- Adds `hal-waveshare-epd397/` skeleton.
- Adds `target-waveshare-epd397/` skeleton.
- Adds Rustmix-Wave docs.
- Adds validation script.

## What this slice intentionally does not do

- Does not port the display backend yet.
- Does not delete X4 code.
- Does not wire a new target into the workspace.
- Does not enable rotary input.
- Does not add voice/audio code.
