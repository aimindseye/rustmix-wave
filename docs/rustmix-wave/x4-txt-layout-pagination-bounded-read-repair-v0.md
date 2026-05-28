# Rustmix-Wave X4 TXT Layout Pagination Bounded Read Repair v0

## Problem

The first layout-pagination port attempted to read the selected TXT book into a
single Vec before wrapping lines.

Large TXT books can exhaust memory on-device before the pagination code reaches
the wrap step.

Observed failure pattern:

- RAW-RUSTMIX-WAVE-X4-TXT-LAYOUT-PAGINATION-V0-START
- repeated RAW-RUSTMIX-WAVE-SD-TXT-READ-OK
- no RAW-RUSTMIX-WAVE-X4-TXT-LAYOUT-PAGINATION-READ-OK
- panic_abort and reset

## Repair

The reader now performs a bounded layout-pagination read.

Current cap:

- READER_LAYOUT_MAX_BOOK_BYTES = 65536

This allows selected books to open reliably while keeping the line/page
pagination path intact.

## Markers

- RAW-RUSTMIX-WAVE-X4-TXT-LAYOUT-PAGINATION-BOUNDED-READ-START
- RAW-RUSTMIX-WAVE-X4-TXT-LAYOUT-PAGINATION-READ-LIMIT-OK
- RAW-RUSTMIX-WAVE-X4-TXT-LAYOUT-PAGINATION-READ-EOF-OK
- RAW-RUSTMIX-WAVE-X4-TXT-LAYOUT-PAGINATION-BOUNDED-READ-OK

## Follow-up

This is a stability repair, not the final pagination architecture.

The better next slice is a streaming page index/cache that stores page starts and
renders one page without loading the whole TXT into RAM.
