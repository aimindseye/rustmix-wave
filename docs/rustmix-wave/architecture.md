# Rustmix-Wave Architecture Direction

## Product base

Rustmix-Wave should reuse the Rustmix product model from the Xteink X4 firmware:

- Reader.
- Library/recent books.
- Progress/bookmarks/settings.
- Wi-Fi transfer model.
- Dictionary shards.
- Flashcards.
- Lua app structure.
- Custom fonts and prepared assets where portable.

## Hardware target

The new board target is Waveshare ESP32-S3 e-Paper 3.97.

This target should be isolated in:

- `hal-waveshare-epd397/`
- `target-waveshare-epd397/`

## Display/backend source

The display backend source for the first real Waveshare slice is the accepted
Focus Hub bring-up path:

- Free-function display backend.
- DisplayBackendAdapter.
- ShellDisplayBridge.
- Portrait 480x800 logical mapping over native 800x480 RAM.

The old Focus Hub `EpaperDisplay::new` wrapper should not be reused because it
had a constructor/return-path hang during bring-up.

## Rotary-first UI

The UI should be non-touch and focus-first:

- Rotary turn changes selected row.
- Press opens selected item.
- Hold-to-talk activates voice.
- Selected row should be visually obvious with a focus bar, border, or inverse pill.
- Avoid touch-style grids as the primary home navigation.

## Future voice layer

Voice should be a system layer:

- UI-only voice states first.
- Audio codec record/playback second.
- Network assistant request third.
- Assistant workflows later.
