# Rustmix-Wave UI Direction

Rustmix-Wave should feel like a Waveshare-native Rustmix device, not a direct
copy of the X4 UI.

## Home model

Use a vertical rotary-first menu:

- Reader
- Network
- Productivity
- Voice
- Tools
- System

Each selected row should update a detail panel with:

- Detail title.
- Short description.
- Current action hint.
- Voice/status strip.

## Footer

The footer should communicate physical controls:

- Rotate: Select
- Press: Open
- Hold: Talk

## E-paper rules

- Prefer strong black/white contrast.
- Avoid excessive full-screen redraws.
- Use partial redraw only after the full display pipeline is stable.
- Design around focus state, not touch targets.
