# Rustmix-Wave Shell Bridge UI Import v0

## Scope

This slice imports the shell-facing UI layer onto the accepted Waveshare display
backend.

It keeps:

- `DisplayBackendAdapter` as the display path.
- `ShellDisplayBridge` portrait 480x800 mapping.
- GPIO3 reserved for EPD_BUSY.

It adds:

- Rustmix-Wave rotary-first home dashboard.
- Vertical focus-first menu.
- Selected row highlight.
- Detail panel.
- Voice/status line.
- Simulated navigation only.

## Menu model

- Reader
- Network
- Productivity
- Voice
- Tools
- System

## What this slice does not do

- Does not enable real rotary input.
- Does not use GPIO3 for input.
- Does not port the reader.
- Does not enable audio/voice capture.

## Smoke markers

- `RAW-RUSTMIX-WAVE-SHELL-UI-V0-START`
- `RAW-RUSTMIX-WAVE-UI-SELECT-READER`
- `RAW-RUSTMIX-WAVE-UI-SELECT-NETWORK`
- `RAW-RUSTMIX-WAVE-UI-SELECT-PRODUCT`
- `RAW-RUSTMIX-WAVE-UI-SELECT-VOICE`
- `RAW-RUSTMIX-WAVE-UI-SELECT-TOOLS`
- `RAW-RUSTMIX-WAVE-UI-SELECT-SYSTEM`
- `RAW-RUSTMIX-WAVE-SHELL-UI-V0-OK`
