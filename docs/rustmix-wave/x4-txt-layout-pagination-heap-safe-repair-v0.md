# Rustmix-Wave X4 TXT Layout Pagination Heap-Safe Repair v0

## Problem

The bounded-read repair prevented unbounded SD reads, but large books still
panicked after `READ-OK` and before `WRAP-OK`.

That points to heap pressure in text conversion or wrapped-line construction.

## Repair

This repair reduces the temporary layout window and caps wrapped lines.

Current limits:

- READER_LAYOUT_MAX_BOOK_BYTES = 16384
- READER_LAYOUT_MAX_LINES = 192

This is intentionally conservative so selected books can open reliably.

## Added markers

- RAW-RUSTMIX-WAVE-X4-TXT-LAYOUT-PAGINATION-UTF8-OK
- RAW-RUSTMIX-WAVE-X4-TXT-LAYOUT-PAGINATION-WRAP-START
- RAW-RUSTMIX-WAVE-X4-TXT-LAYOUT-PAGINATION-WRAP-LIMIT-OK
- RAW-RUSTMIX-WAVE-X4-TXT-LAYOUT-PAGINATION-WRAP-DONE

## Follow-up

This is still not the final pagination architecture.

The correct long-term fix is streaming TXT pagination with page offsets or a
small page index/cache, so the device never allocates the full book or a large
Vec of Strings.
