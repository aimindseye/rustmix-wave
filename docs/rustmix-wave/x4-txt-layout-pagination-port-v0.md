# Rustmix-Wave X4 TXT Layout Pagination Port v0

## Scope

This slice replaces byte-stride TXT pagination with line/page pagination.

It keeps:

- TXT browser
- button navigation
- X4 compact reader layout
- SD-backed ReaderStorage
- ReaderDisplaySurface
- ShellDisplayBridge
- DisplayBackendAdapter
- GPIO3 reserved for EPD_BUSY

It adds:

- read full selected TXT through ReaderStorage
- normalize TXT to display-safe ASCII-ish text
- skip Project Gutenberg header/footer markers when present
- word wrapping
- lines-per-page pagination
- total page count from rendered lines
- clamped next/previous through existing ReaderScreenState
- layout-page renderer using current font

## Page model

- body max chars: 36
- lines per page: 32
- current font remains the temporary bitmap/debug-style font

## Runtime path

browser
  -> selected TXT copy
  -> build_txt_layout_pagination_v0
  -> TxtLayoutPagination
  -> ReaderScreenState::new_with_total_pages
  -> render_reader_layout_page_with_title_v0
  -> button Up/Down changes page_index
  -> same layout renderer

## Markers

- RAW-RUSTMIX-WAVE-X4-TXT-LAYOUT-PAGINATION-V0-START
- RAW-RUSTMIX-WAVE-X4-TXT-LAYOUT-PAGINATION-READ-OK
- RAW-RUSTMIX-WAVE-X4-TXT-LAYOUT-PAGINATION-WRAP-OK
- RAW-RUSTMIX-WAVE-X4-TXT-LAYOUT-PAGINATION-V0-OK
- RAW-RUSTMIX-WAVE-X4-TXT-LAYOUT-PAGINATION-TARGET-OK
- RAW-RUSTMIX-WAVE-X4-TXT-LAYOUT-PAGE-RENDER-START
- RAW-RUSTMIX-WAVE-X4-TXT-LAYOUT-PAGE-RENDER-OK

## Non-goals

- No font engine port yet.
- No EPUB.
- No bookmark persistence.
- No progress persistence.
