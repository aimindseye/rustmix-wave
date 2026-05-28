# Rustmix-Wave TXT Boot Flow Cleanup v0

## Scope

This slice removes old runtime verification flows and boots directly into the
real SD TXT reader path.

It keeps:

- DisplayBackendAdapter
- ShellDisplayBridge
- ReaderDisplaySurface
- SD-backed ReaderStorage
- GPIO3 reserved for EPD_BUSY

It removes from runtime:

- dashboard navigation smoke
- mock reader first/next/previous smoke
- old display verification/smoke path

The display backend remains intact. This cleanup only removes old verification
flows from the active boot path.

## Runtime path

boot
  -> DisplayBackendAdapter
  -> ShellDisplayBridge init
  -> SD mount
  -> prefer /sdcard/BOOKS/*.txt
  -> SdTxtReaderStorage
  -> render_reader_page_v0
  -> ReaderDisplaySurface

## New markers

- RAW-RUSTMIX-WAVE-TXT-BOOT-FLOW-V0-START
- RAW-RUSTMIX-WAVE-TXT-BOOT-DISPLAY-READY-OK
- RAW-RUSTMIX-WAVE-TXT-BOOT-SD-MOUNT-OK
- RAW-RUSTMIX-WAVE-SD-TXT-USER-BOOK-FOUND
- RAW-RUSTMIX-WAVE-SD-TXT-USER-BOOK-COPIED
- RAW-RUSTMIX-WAVE-SD-TXT-READ-OK
- RAW-RUSTMIX-WAVE-TXT-BOOT-FIRST-PAGE-OK
- RAW-RUSTMIX-WAVE-TXT-BOOT-FLOW-V0-OK

## Non-goals

- No EPUB.
- No bookmark persistence.
- No progress persistence.
- No real rotary input.
- No full Rustmix X4 reader manager port.
