# Rustmix-Wave Real TXT Pagination v0

## Scope

This slice replaces the placeholder three-page reader model with real TXT
pagination based on file length.

It keeps:

- direct TXT boot flow
- button reader navigation
- SD-backed ReaderStorage
- ReaderDisplaySurface
- ShellDisplayBridge
- DisplayBackendAdapter
- GPIO3 reserved for EPD_BUSY

It adds:

- TXT byte length lookup through filesystem metadata
- real total page count
- real page offsets
- next and previous clamping at file bounds
- real page number display
- real progress bar based on page number and total pages

## Page model

The current page stride is 720 bytes per page.

This is a simple first pagination model. It is byte-based, not word-layout-based.
A later slice can replace it with layout-aware pagination.

## Runtime path

boot
  -> SD TXT selected from /sdcard/BOOKS/*.txt
  -> txt metadata length
  -> ReaderScreenState::for_txt_len
  -> render_reader_page_v0
  -> button next/previous
  -> clamped page_index
  -> real page offset

## Markers

- RAW-RUSTMIX-WAVE-TXT-PAGINATION-V0-START
- RAW-RUSTMIX-WAVE-TXT-PAGINATION-LEN-OK
- RAW-RUSTMIX-WAVE-TXT-PAGINATION-V0-OK

## Non-goals

- No EPUB.
- No bookmark persistence.
- No progress persistence.
- No layout-aware pagination yet.
