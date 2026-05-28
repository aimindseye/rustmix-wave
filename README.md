# Rustmix X4 Firmware

Rustmix is a neutral Rust reference firmware for the Xteink X4 e-ink reader. It is intended for people who want to use the X4 as an open reader and for developers who want a practical Rust starting point for ESP32-C3 e-ink firmware.

Rustmix includes a reader, Wi-Fi transfer, first-boot SD provisioning, custom fonts, dictionary data, flashcards, random sleep images, and an SD-loaded Lua app model.

## Included capabilities

- ESP32-C3 Rust firmware target for Xteink X4.
- SSD1677 display path for the 800 × 480 e-paper panel.
- X4/CrossPoint-compatible partition table.
- Reader with recent books, bookmarks, TXT/EPUB support, cache progress, and reader settings.
- Wi-Fi Transfer for SD-card access without removing the card.
- First-boot SD setup under `/RUSTMIX`.
- SD-loaded Lua apps under `/RUSTMIX/APPS`.
- Custom VFN fonts under `/RUSTMIX/FONTS`.
- Dictionary prefix shards under `/RUSTMIX/APPS/DICT`.
- Flashcards with text and image topics under `/RUSTMIX/APPS/FLASHCRD`.
- Random sleep images under `/RUSTMIX/SLEEP`.

## Repository documentation policy

Release documentation is consolidated into these root files:

```text
README.md
ARCHITECTURE.md
USERGUIDE.md
SCOPE.md
ROADMAP.md
SCREENSHOTS.md
```

The repository intentionally keeps first-release documentation in root files only, so CI hygiene can keep release guidance in one visible place. Screenshot references live in [`SCREENSHOTS.md`](SCREENSHOTS.md), with image assets in `screenshots/`.

## Screenshots

See [`SCREENSHOTS.md`](SCREENSHOTS.md) for reference screenshots of the home dashboard, reader, dictionary, calendar, network setup, system settings, and sleep-image screens. Screenshot images are stored in `screenshots/`.

## SD-card layout

Rustmix creates and uses this SD-card root:

```text
/RUSTMIX
/RUSTMIX/APPS
/RUSTMIX/FONTS
/RUSTMIX/SLEEP
/RUSTMIX/CACHE
/RUSTMIX/STATE
/RUSTMIX/SETTINGS.TXT
/RUSTMIX/TIME.TXT
```

First boot seeds missing default files only. User files are preserved.

## Build latest firmware

Install Rust and the X4 target, then build:

```bash
rustup target add riscv32imc-unknown-none-elf
cargo install espflash --locked
cargo fmt --all
cargo build -p target-xteink-x4 --release --target riscv32imc-unknown-none-elf
```

Create the release firmware file:

```bash
scripts/create_rustfirmware_bin.sh
```

The output is:

```text
dist/rustmix-x4/rustfirmware.bin
```

## Flashing

Find the USB port:

```bash
ls /dev/ttyACM* 2>/dev/null || true
```

First install / full deploy:

```bash
scripts/flash_x4_release_bin.sh dist/rustmix-x4/rustfirmware.bin /dev/ttyACM0
```

Normal later app-only updates:

```bash
scripts/flash_x4_rustmix_app0.sh /dev/ttyACM0
```

## Release bundle

Create a compact release folder and zip with firmware, flashing notes, partition files, and the SD-card sleep-image pack:

```bash
scripts/create_rustmix_release_package.sh
```

## Developer references and inspirations

Rustmix is inspired by or informed by:

- pulp-os: minimal Xteink X4 Rust firmware lineage.
- Biscuit: simple e-reader product flow and dashboard ideas.
- CrossInk: compact e-ink UI patterns for headers, rows, tabs, and footer-safe screens.
- CrossPoint Reader: Xteink-compatible firmware and partition-layout awareness.
- Lua-reader experiments such as CrossLuaReader-style SD apps, bounded host APIs, and data-driven app folders.

Rustmix is not official Xteink firmware. Flashing custom firmware can fail on unsupported devices. Keep recovery options available.

<!-- BEGIN RUSTMIX_WAVE_REPOSITORY_BOOTSTRAP_V0 -->
## Rustmix-Wave

Rustmix-Wave is the Waveshare ESP32-S3 e-Paper 3.97 version of Rustmix.

This repository keeps the Rustmix X4 code as the upstream reference while adding
a new Waveshare target direction.

Repository Bootstrap v0 adds:

- `hal-waveshare-epd397/` skeleton.
- `target-waveshare-epd397/` skeleton.
- Rustmix-Wave architecture docs.
- Validation script for repository bootstrap.

Product direction:

- Reuse the Rustmix product model and reader/app logic where possible.
- Use the accepted Focus Hub Waveshare display/backend work as the hardware display source.
- Build a rotary-first UI for a non-touch e-paper device.
- Add a future voice assistant layer inspired by the Focus Hub / Durobo direction.

The first real hardware slice after this bootstrap should import the accepted
Waveshare 3.97 display backend. This bootstrap intentionally does not port
display code and does not delete the existing X4 code.
<!-- END RUSTMIX_WAVE_REPOSITORY_BOOTSTRAP_V0 -->

<!-- BEGIN RUSTMIX_WAVE_DISPLAY_BACKEND_IMPORT_V0 -->
## Rustmix-Wave Display Backend Import v0

Rustmix-Wave now includes a Waveshare ESP32-S3 e-Paper 3.97 display backend
imported into `hal-waveshare-epd397`.

Accepted display pin map:

- EPD_SCLK GPIO11
- EPD_MOSI GPIO12
- EPD_CS GPIO10
- EPD_DC GPIO9
- EPD_RST GPIO46
- EPD_BUSY GPIO3

This slice adds:

- Free-function Waveshare display backend.
- `DisplayBackendAdapter`.
- `ShellDisplayBridge`.
- Minimal `target-waveshare-epd397` black/white display smoke.

This slice intentionally does not port the reader yet.
<!-- END RUSTMIX_WAVE_DISPLAY_BACKEND_IMPORT_V0 -->

<!-- BEGIN RUSTMIX_WAVE_SHELL_BRIDGE_UI_IMPORT_V0 -->
## Rustmix-Wave Shell Bridge UI Import v0

Rustmix-Wave now renders a rotary-first home dashboard through the accepted
Waveshare display path:

`DisplayBackendAdapter -> ShellDisplayBridge -> Rustmix-Wave home UI`

This slice adds:

- Portrait 480x800 shell UI rendering.
- Rotary-first vertical home menu.
- Selected row highlight.
- Detail panel and voice/status line.
- Simulated navigation only.

This slice intentionally does not enable real rotary input and does not port the
reader yet. GPIO3 remains reserved for EPD_BUSY.
<!-- END RUSTMIX_WAVE_SHELL_BRIDGE_UI_IMPORT_V0 -->

<!-- BEGIN RUSTMIX_WAVE_READER_PORT_RECON_V0 -->
## Rustmix-Wave Reader Port Recon v0

Reader migration is currently in reconnaissance only.

This slice adds:

- Static audit script for Rustmix X4 reader/content/storage candidates.
- Reader port recon document.
- Reusable reader/core module categories.
- X4-only hardware/display/input assumptions to isolate.
- Proposed `ReaderDisplaySurface` adapter path to `ShellDisplayBridge`.
- Proposed `ReaderStorage` compatibility path.
- Concrete staged port plan before any reader code moves.

This slice intentionally does not port reader code, does not enable real rotary
input, and does not use GPIO3 for input. GPIO3 remains reserved for EPD_BUSY.
<!-- END RUSTMIX_WAVE_READER_PORT_RECON_V0 -->

<!-- BEGIN RUSTMIX_WAVE_READER_DISPLAY_SURFACE_BOUNDARY_V0 -->
## Rustmix-Wave Reader Display Surface Boundary v0

Rustmix-Wave now has a reader-facing display boundary:

`ReaderDisplaySurface -> ShellDisplayBridge -> DisplayBackendAdapter`

This slice adds:

- `ReaderDisplaySurface` trait.
- `ShellDisplayBridge` implementation.
- Static reader placeholder page rendered through the boundary.
- Explicit reader boundary smoke markers.

This slice intentionally does not port reader parsing, SD/storage, bookmarks,
progress, EPUB/TXT loading, or real rotary input. GPIO3 remains reserved for
EPD_BUSY.
<!-- END RUSTMIX_WAVE_READER_DISPLAY_SURFACE_BOUNDARY_V0 -->

<!-- BEGIN RUSTMIX_WAVE_READER_FOUNDATION_V0 -->
## Rustmix-Wave Reader Foundation v0

Rustmix-Wave now has a mock reader foundation on top of the accepted display
stack:

`MockReaderStorage -> ReaderStorage -> ReaderDisplaySurface -> ShellDisplayBridge -> DisplayBackendAdapter`

This slice adds:

- `ReaderStorage` trait.
- `MockReaderStorage`.
- `ReaderBook`.
- `ReaderScreenState`.
- wrapped mock text page renderer.
- title/header, body text, footer/progress bar, and page number.
- simulated reader flow: first page, next page, previous page.

This slice intentionally does not enable real rotary input, real SD reading,
bookmarks/progress persistence, EPUB, or the full Rustmix X4 reader manager.
GPIO3 remains reserved for EPD_BUSY.
<!-- END RUSTMIX_WAVE_READER_FOUNDATION_V0 -->

<!-- BEGIN RUSTMIX_WAVE_SD_TXT_FIRST_PAGE_V0 -->
## Rustmix-Wave SD TXT First Page v0

Rustmix-Wave now reads one TXT file from the microSD card through the existing
`ReaderStorage` trait and renders its first page through the existing reader
renderer.

Runtime path:

`SD TXT -> SdTxtReaderStorage -> ReaderStorage -> ReaderDisplaySurface -> ShellDisplayBridge -> DisplayBackendAdapter`

This slice adds:

- Waveshare SDMMC mount.
- SD-backed `ReaderStorage`.
- `/sdcard/RUSTMIX/BOOKS/WAVE.TXT` sample book creation if missing.
- first-page render from real SD bytes.

This slice intentionally does not enable EPUB, bookmarks/progress persistence,
real rotary input, or the full Rustmix X4 reader manager. GPIO3 remains reserved
for EPD_BUSY.
<!-- END RUSTMIX_WAVE_SD_TXT_FIRST_PAGE_V0 -->

<!-- BEGIN RUSTMIX_WAVE_TXT_BOOT_FLOW_CLEANUP_V0 -->
## Rustmix-Wave TXT Boot Flow Cleanup v0

Rustmix-Wave now boots directly into the SD TXT reader path instead of running
old display/dashboard/mock-reader verification flows.

Runtime path:

boot -> ShellDisplayBridge -> SD mount -> SdTxtReaderStorage -> ReaderDisplaySurface

This slice removes runtime calls for:

- dashboard navigation smoke
- mock reader first/next/previous smoke
- old display verification/smoke path

It keeps the accepted display backend and SD TXT reader path. GPIO3 remains
reserved for EPD_BUSY. EPUB, bookmarks/progress persistence, and real rotary
input remain out of scope.
<!-- END RUSTMIX_WAVE_TXT_BOOT_FLOW_CLEANUP_V0 -->

<!-- BEGIN RUSTMIX_WAVE_BUTTON_READER_NAVIGATION_V0 -->
## Rustmix-Wave Button Reader Navigation v0

Rustmix-Wave now polls the vendor app buttons for reader navigation.

Button mapping:

- GPIO4 Button_Up: previous page
- GPIO5 Button_Function: refresh/select current page
- GPIO6 Button_Down: next page

The buttons are pull-up inputs and active-low. GPIO0 Boot is documented only and
is not used by this slice. GPIO3 remains reserved for EPD_BUSY.

This slice does not use interrupts, does not enable EPUB, and does not add
bookmark/progress persistence.
<!-- END RUSTMIX_WAVE_BUTTON_READER_NAVIGATION_V0 -->

<!-- BEGIN RUSTMIX_WAVE_REAL_TXT_PAGINATION_V0 -->
## Rustmix-Wave Real TXT Pagination v0

Rustmix-Wave now calculates TXT page count from the selected SD TXT file length
instead of using the old placeholder three-page model.

This slice adds:

- TXT metadata length lookup.
- ReaderScreenState initialized from real TXT length.
- real page offsets.
- clamped next/previous page movement.
- real page number display.
- progress bar based on current page and total pages.

This is still simple byte-stride pagination. EPUB, bookmarks/progress
persistence, and layout-aware pagination remain out of scope.
<!-- END RUSTMIX_WAVE_REAL_TXT_PAGINATION_V0 -->

<!-- BEGIN RUSTMIX_WAVE_TXT_BOOK_BROWSER_V0 -->
## Rustmix-Wave TXT Book Browser v0

Rustmix-Wave now boots into a TXT book browser instead of opening the first TXT
file immediately.

This slice adds:

- scan of `/sdcard/BOOKS/*.txt`
- small in-memory book list
- browser screen through `ReaderDisplaySurface`
- GPIO6 Down selection movement
- GPIO4 Up selection movement
- GPIO5 Function opens selected TXT

After a book opens, the existing reader navigation remains active. EPUB,
bookmark persistence, and progress persistence remain out of scope.
<!-- END RUSTMIX_WAVE_TXT_BOOK_BROWSER_V0 -->

<!-- BEGIN RUSTMIX_WAVE_X4_READER_REUSE_MAP_V0 -->
## Rustmix-Wave X4 Reader Reuse Map v0

Rustmix-Wave now has a documented reuse map for existing Rustmix X4 reader/UI
code.

The rule is:

Reuse reader/UI/product logic. Do not reuse X4 hardware ownership.

The map classifies X4 reader/UI files into:

- reuse as-is
- reuse with adapter
- copy concepts only
- do not reuse

This slice is docs and validation only. It does not change runtime behavior.
<!-- END RUSTMIX_WAVE_X4_READER_REUSE_MAP_V0 -->

<!-- BEGIN RUSTMIX_WAVE_X4_READER_LAYOUT_PORT_V0 -->
## Rustmix-Wave X4 Reader Layout Port v0

Rustmix-Wave now uses a compact reader layout inspired by the X4 reader direction.

This slice keeps the accepted Wave runtime path and changes presentation only:

- compact header and footer
- selected book title in the reader header
- page number in the header
- thin separators instead of a large content border
- wider and taller body text area
- footer hints and progress bar

This slice does not port the X4 font engine or layout-aware pagination yet.
<!-- END RUSTMIX_WAVE_X4_READER_LAYOUT_PORT_V0 -->

<!-- BEGIN RUSTMIX_WAVE_X4_TXT_LAYOUT_PAGINATION_PORT_V0 -->
## Rustmix-Wave X4 TXT Layout Pagination Port v0

Rustmix-Wave now builds TXT pages from rendered wrapped lines instead of raw
byte offsets.

This slice keeps the TXT browser, button navigation, and compact reader layout,
then adds:

- full selected TXT read through ReaderStorage
- Project Gutenberg boilerplate skip when markers are present
- word wrapping
- line/page pagination
- real total pages based on rendered line count
- clamped next/previous through the existing ReaderScreenState

The current bitmap/debug-style font remains in place. EPUB and bookmark/progress
persistence remain out of scope.
<!-- END RUSTMIX_WAVE_X4_TXT_LAYOUT_PAGINATION_PORT_V0 -->

<!-- BEGIN RUSTMIX_WAVE_X4_TXT_LAYOUT_PAGINATION_BOUNDED_READ_REPAIR_V0 -->
## Rustmix-Wave X4 TXT Layout Pagination Bounded Read Repair v0

The first layout-pagination port tried to read the selected TXT book into RAM
before wrapping. Large TXT files could panic/reset before the book opened.

This repair adds a bounded read for layout pagination:

- `READER_LAYOUT_MAX_BOOK_BYTES = 65536`
- bounded TXT read before wrapping
- explicit bounded-read markers
- no display/input/storage ownership changes

This is a stability repair. A future streaming page index/cache should replace
the bounded full-read approach.
<!-- END RUSTMIX_WAVE_X4_TXT_LAYOUT_PAGINATION_BOUNDED_READ_REPAIR_V0 -->

<!-- BEGIN RUSTMIX_WAVE_X4_TXT_LAYOUT_PAGINATION_HEAP_SAFE_REPAIR_V0 -->
## Rustmix-Wave X4 TXT Layout Pagination Heap-Safe Repair v0

Large TXT books still panicked after the bounded read completed, likely during
UTF-8 conversion or wrapped-line construction.

This repair makes the temporary layout-pagination path more conservative:

- `READER_LAYOUT_MAX_BOOK_BYTES = 16384`
- `READER_LAYOUT_MAX_LINES = 192`
- extra markers around UTF-8 and wrapping

This is a stability patch. A streaming TXT page index/cache remains the correct
long-term architecture.
<!-- END RUSTMIX_WAVE_X4_TXT_LAYOUT_PAGINATION_HEAP_SAFE_REPAIR_V0 -->
