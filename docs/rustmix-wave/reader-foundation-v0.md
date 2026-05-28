# Rustmix-Wave Reader Foundation v0

## Scope

This slice combines the reader display boundary, reader page renderer, reader
storage boundary, mock storage, and simulated page navigation.

It keeps:

- `DisplayBackendAdapter` as the display path.
- `ShellDisplayBridge + ReaderDisplaySurface` as the reader display path.
- GPIO3 reserved for EPD_BUSY.

It does not:

- enable real rotary input
- use GPIO3 for input
- port EPUB
- port the full Rustmix X4 reader manager
- mount or read real SD
- persist bookmarks or progress

## Runtime path

```text
MockReaderStorage
  -> ReaderStorage
  -> Reader page renderer
  -> ReaderDisplaySurface
  -> ShellDisplayBridge
  -> DisplayBackendAdapter
```

## Added pieces

- `ReaderStorage` trait
- `MockReaderStorage`
- `ReaderBook`
- `ReaderScreenState`
- wrapped mock text page renderer
- header/title
- footer/progress bar
- page number
- simulated first/next/previous page flow

## Acceptance markers

- `RAW-RUSTMIX-WAVE-READER-FOUNDATION-V0-START`
- `RAW-RUSTMIX-WAVE-READER-MOCK-STORAGE-OK`
- `RAW-RUSTMIX-WAVE-READER-PAGE-RENDER-START`
- `RAW-RUSTMIX-WAVE-READER-PAGE-RENDER-OK`
- `RAW-RUSTMIX-WAVE-READER-MOCK-FIRST-PAGE-OK`
- `RAW-RUSTMIX-WAVE-READER-MOCK-NAV-NEXT-OK`
- `RAW-RUSTMIX-WAVE-READER-MOCK-NAV-PREV-OK`
- `RAW-RUSTMIX-WAVE-READER-FOUNDATION-V0-OK`

## Next recommended slice

`Rustmix-Wave Reader SD TXT First Page v0`

That slice should replace `MockReaderStorage` with a real SD-backed
`ReaderStorage` implementation and render one TXT file from SD.
