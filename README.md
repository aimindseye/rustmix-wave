# Rustmix Wave for Waveshare ESP32-S3 E-Paper 3.97

Rustmix Wave is a modular Rust / ESP-IDF firmware for the Waveshare ESP32-S3 3.97-inch e-paper board. The native panel is `800 × 480`; the product UI renders on a logical `480 × 800` portrait canvas.

Current release: **v1.1.0** (`sd-unicode-indic-epub-reader`; physically verified SD-loaded Noto Sans Devanagari and Noto Sans Gujarati EPUB support).

This repository is the cleaned source tree. Historical patch overlays, temporary ZIP archives, patch scripts, repair notes, and milestone-by-milestone smoke-test documents have been removed. Durable documentation is consolidated into this README and the small set of files under [`docs/`](docs/). A screenshot-driven operating guide is available at [`docs/USER_GUIDE.md`](docs/USER_GUIDE.md), with reference images stored under [`screenshots/`](screenshots/).

## Highlights

- Rotary-first product shell with Reader, Productivity, Games, Tools, and Settings categories.
- Physical **Power short press** opens a display-maintenance menu for manual ghost-clearing refresh.
- Physical **Power long press** enters the accepted random sleep-image mode with network suspension and route restoration after wake.
- Reader supports TXT and bounded reflowable EPUB files, TOC navigation, bookmarks, per-book resume, typography preferences, paragraph alignment, FAT 8.3-safe persistence, file-backed EPUB archive inspection, and physically verified SD-loaded Unicode cluster packs for Devanagari and Gujarati EPUB text.
- Voice Notes records PCM16 mono 16 kHz WAV files to SD, supports microphone gain, pause/resume, saved-note playback, titles, timestamps, delete confirmation, storage telemetry, and LAN export.
- Native Dictionary reuses the Rustmix X4 prefix-shard SD pack and uses BOOT-short `NAV H` / `NAV V` keyboard-axis switching.
- Native Calendar loads personal events and the U.S.-only 2026 pack, renders a daily agenda, and supports recovery-safe personal-event creation, editing, and deletion.
- Wi-Fi transfer portal provides explicit LAN-only SD access with protected configuration paths and atomic file replacement.
- RTC alarms, weather, unit conversion, file browsing, audio diagnostics, sensors, Lua apps, and native motion games remain available.

## Hardware target

| Component | Contract |
| --- | --- |
| MCU | ESP32-S3 |
| Display | Waveshare 3.97-inch SSD1677 e-paper, native `800 × 480` |
| Product orientation | Logical portrait `480 × 800` |
| Display SPI | SCLK GPIO11, MOSI GPIO12, CS GPIO10, DC GPIO9, RST GPIO46, BUSY GPIO3 |
| SD storage | FAT SD card mounted at `/sdcard` |
| BOOT button | GPIO0, short press contextual, long press hierarchical Back |
| Power key | AXP2101 PEK interrupts: short opens display menu, long enters sleep-image mode |
| RTC alarm interrupt | GPIO45, active low |
| Audio | ES8311 codec and native I2S ownership |
| Sensors | SHTC3 environment sensor, QMI8658 IMU |

See [`docs/BOARD_CONTRACT.md`](docs/BOARD_CONTRACT.md) for the stable board boundary.

## Application status

| Category | Application | Status |
| --- | --- | --- |
| Reader | Continue Reading, Library, Bookmarks | Ready: TXT, bounded reflowable EPUB, and optional SD Unicode Indic packs |
| Productivity | Calendar | Ready: U.S.-only agenda and personal-event editor |
| Productivity | Voice Notes | Ready: record, pause/resume, playback, title, delete, export |
| Games | SD Lua catalog | Ready: Hello Grid, Sudoku, Minesweeper, Tilt Maze, Motion 2048, Sokoban Tilt |
| Tools | Dictionary | Ready: native X4 prefix-shard lookup |
| Tools | File Browser | Ready: bounded read-only SD browser and text preview |
| Tools | Unit Converter | Ready: offline fixed-point conversions |
| Settings | Alarms | Ready: RTC schedules, snooze, dismiss |
| Settings | Audio | Ready: codec diagnostics and chime |
| Settings | Clock | Ready: RTC and power information |
| Settings | Display | Ready: UI font family and size persistence |
| Settings | Environment | Ready: temperature and humidity |
| Settings | Motion | Ready: IMU diagnostics |
| Settings | Network | Ready: Wi-Fi, SNTP, explicit LAN transfer portal |
| Settings | Weather | Ready with bounded retries and last-known-good cache |

## Sensor-driven utilities and motion games

Rustmix Wave uses the board peripherals as product features rather than treating them as diagnostics only.

| Hardware service | Firmware use |
| --- | --- |
| PCF85063 RTC | Localized clock, calendar date, persistent alarm schedules, and GPIO45 alarm wake |
| AXP2101 PMIC | Battery and USB/charge status, e-paper rail support, and Power-key short/long interrupt classification |
| SHTC3 environment sensor | Temperature and humidity cards, home status, and sensor details |
| QMI8658 accelerometer and gyroscope | Live Motion diagnostics, debounced `TILT`, `SHAKE`, `ROTATE`, and `LEVEL` events, Tilt Maze, Motion 2048, and Sokoban Tilt |
| ES8311 audio codec and I2S | Alarm chime, audio diagnostics, Voice Notes recording, and saved-WAV playback |
| SDMMC storage | Reader library, Voice Notes, Dictionary shards, Calendar events, sleep images, Wi-Fi transfer, and SD-loaded app packs |
| Wi-Fi and SNTP | Network status, RTC synchronization, weather fetch, and explicit LAN file transfer |

The native IMU event bridge keeps raw QMI8658 I2C samples inside Rust. It converts fixed-point accelerometer and gyroscope snapshots into debounced events with release latching and cooldowns. Motion games receive those bounded native events rather than raw I2C access:

- **Tilt Maze** maps planar tilt into maze movement.
- **Motion 2048** maps tilt into swipe directions for tile slides and merges.
- **Sokoban Tilt** maps tilt into player movement and crate pushes.

See [`docs/USER_GUIDE.md`](docs/USER_GUIDE.md) for the screen-by-screen controls and [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) for the sensor pipeline.

## Main-task safety and worker isolation

The ESP-IDF main task remains the narrow hardware-orchestration owner. It owns display refresh coordination, UI routing, and long-lived peripheral handles. Stack-heavy or blocking operations are isolated behind bounded workers or dedicated service tasks:

| Operation | Isolation policy |
| --- | --- |
| EPUB parse | Short-lived `epub-parser` worker with a preferred 48 KiB stack, a fragmentation-aware 32 KiB fallback, and a 4 KiB contiguous-internal-heap start guard |
| EPUB title lookup | Short-lived title worker with a 32 KiB stack |
| Weather HTTPS fetch | Short-lived `weather-fetch` worker with a 64 KiB stack and bounded response payload |
| Lua app open | Short-lived `lua-loader` worker with a 32 KiB stack |
| Wi-Fi transfer portal | Explicitly started ESP-IDF HTTP server task with a 24 KiB stack and 4 KiB stream chunks |
| Voice Notes capture and playback | Cooperative bounded I2S chunks while native `AudioRuntime` retains codec ownership |

`AppState` is heap-boxed, runtime memory snapshots report main-stack high-water margin and internal/PSRAM heap state, and workers return compact results before terminating. Lua apps never receive panel SPI, raw I2C, networking, or long-lived hardware handles.

## SD Unicode Indic EPUB fonts

Rustmix Wave v1.1 adds optional Reader font packs below `/RUSTMIX/FONTS`. Use the local browser tool at [`tools/font-builder/index.html`](tools/font-builder/index.html) with **Noto Sans Devanagari** and **Noto Sans Gujarati** font files selected from your computer. The builder renders shaped Unicode clusters in the browser and exports one `rustmix-indic-font-pack.zip` containing FAT-safe `.RWF` files plus `FONTS.TXT`; raw TTF and OTF files are not committed or copied to the device.

```bash
python3 scripts/extract-epub-font-corpus.py book1.epub book2.epub > indic-corpus.txt
./scripts/install-indic-font-pack.sh ~/Downloads/rustmix-indic-font-pack.zip /Volumes/YOUR_SD_CARD
./scripts/verify-indic-font-pack.sh /Volumes/YOUR_SD_CARD
```

The Reader preserves UTF-8 byte anchors for bookmarks and resume while paginating without splitting Indic clusters. Large EPUB archives are inspected through a file-backed ZIP boundary: only the bounded central directory and selected members are read into RAM. Before physical testing, audit local fixtures with:

```bash
python3 scripts/audit-indic-epub-fixture.py book1.epub book2.epub book3.epub
```

See [`docs/UNICODE_FONTS.md`](docs/UNICODE_FONTS.md).


## Physically verified Devanagari and Gujarati EPUB rendering

Rustmix Wave v1.1.0 has been physically verified on the Waveshare ESP32-S3 3.97-inch e-paper board with SD-generated **Noto Sans Devanagari Regular** and **Noto Sans Gujarati Regular** `.RWF` packs. The Reader opens large Indic EPUB archives first-page-first, grows page anchors lazily, and retains only the shaped glyph subset needed for the visible page. This protects PSRAM while preserving UTF-8 byte anchors for bookmarks and resume.

| Script | Verified example | Screenshot |
| --- | --- | --- |
| Devanagari | Hindi / Sanskrit Bhagavat text | [`epub-devanagari-bhagavat.jpg`](screenshots/epub-devanagari-bhagavat.jpg) |
| Devanagari | Hindi Garud Puran text | [`epub-devanagari-garud-puran.jpg`](screenshots/epub-devanagari-garud-puran.jpg) |
| Gujarati | Gujarati Valmiki Ramayan text | [`epub-gujarati-valmiki-ramayan.jpg`](screenshots/epub-gujarati-valmiki-ramayan.jpg) |

See [`Screenshots.md`](Screenshots.md) for the v1.1.0 physical-verification images and [`docs/UNICODE_FONTS.md`](docs/UNICODE_FONTS.md) for font-pack generation and SD installation.

## Repository layout

```text
.cargo/config.toml              ESP-IDF target, linker, runner, and environment
.github/workflows/ci.yml        GitHub Actions format, static-contract, and host-test workflow
src/                            Runtime, domain modules, app state, renderers, and host tests
examples/sd-card/RUSTMIX/       FAT SD-card examples and smoke packs
scripts/validate.sh             Formatting, source-contract, and native-target host tests
scripts/build.sh                Validated ESP-IDF release build
scripts/flash.sh                Build, flash, and monitor helper
scripts/build-release-firmware.sh  Build an ELF-only release bundle and checksums
scripts/flash-release.sh           Flash an existing release ELF safely
scripts/test-release-flash-workflow.sh  Verify the ELF-only release bundle with fake tools
scripts/package-release.sh      Create a GitHub-ready cleaned source archive
scripts/extract-epub-font-corpus.py  Extract local EPUB corpus text for browser font-pack generation
scripts/audit-indic-epub-fixture.py  Audit local EPUB structure against bounded Reader limits
scripts/install-indic-font-pack.sh   Install generated Indic font-pack directory or ZIP to SD
tools/font-builder/                  Local CrossPoint-inspired Unicode font-pack builder
docs/                           Consolidated durable documentation and screenshot-driven user guide
screenshots/                    Reference UI screenshots linked by docs/USER_GUIDE.md
```

The architecture and module ownership rules are documented in [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md).

## Prerequisites

Install the ESP Rust toolchain, ESP-IDF build dependencies, and `espflash`. The repository selects the `esp` toolchain through [`rust-toolchain.toml`](rust-toolchain.toml) and pins ESP-IDF settings in [`.cargo/config.toml`](.cargo/config.toml).

The stable Rust toolchain is also required for formatting and native host tests.

## Validate

```bash
./scripts/validate.sh
```

This runs:

```text
cargo +stable fmt --all -- --check
./scripts/validate_source_contract.sh
./scripts/test-host.sh
```

Host tests explicitly use the detected native target so they do not inherit the repository default `xtensa-esp32s3-espidf` target.

## Build

```bash
./scripts/build.sh
```

Equivalent embedded release build:

```bash
cargo +esp build -Z build-std=std,panic_abort --release --target xtensa-esp32s3-espidf
```

## Flash and monitor

```bash
./scripts/flash.sh
```

Pass an explicit serial port when needed:

```bash
./scripts/flash.sh /dev/cu.usbmodemXXXX
```

## Build an ELF-only firmware release

```bash
./scripts/build-release-firmware.sh
```

The script validates the source, builds the ESP-IDF release ELF, and creates an ELF-only release ZIP under `dist/`. Flash the supported ELF artifact with:

```bash
./scripts/flash-release.sh \
  dist/waveshare-epd397-rust-app-v1.1.0.elf
```

Do not use `espflash write-bin`: it is a raw-address operation. A merged factory-image workflow remains deferred until bootloader, partition-table, and application offsets have been validated on physical hardware.

See [`docs/RELEASE.md`](docs/RELEASE.md).

## Package a cleaned source archive

```bash
./scripts/package-release.sh
```

The output is written below `dist/` and excludes local build products, generated release artifacts, caches, temporary files, and extracted patch-overlay directories.

## SD-card setup

Install the bundled examples:

```bash
./scripts/install-sd-examples.sh /Volumes/YOUR_SD_CARD
```

Install the complete X4 Dictionary pack:

```bash
./scripts/install-dictionary-x4-pack.sh \
  --force \
  --x4-repo /Users/piyushdaiya/Documents/projects/rustmix-x4-firmware \
  /Volumes/YOUR_SD_CARD
```

Install the U.S.-only X4 Calendar pack:

```bash
./scripts/install-calendar-x4-pack.sh \
  --force \
  --x4-repo /Users/piyushdaiya/Documents/projects/rustmix-x4-firmware \
  /Volumes/YOUR_SD_CARD
```

See [`docs/SD_CARD_SETUP.md`](docs/SD_CARD_SETUP.md) for the complete storage contract.

## Input conventions

```text
ROTARY               Move the current selection
SELECT               Activate the current selection
BOOT short           Contextual action; keyboard/grid screens toggle NAV H / NAV V
BOOT long             Hierarchical Back
Power short          Open display-maintenance menu
Power long           Enter sleep-image mode
```

All new keyboard or grid-style text-entry screens should compose the shared `KeyboardGridNavigation` helper so BOOT-short H/V axis switching is consistent across apps.

## Durable documentation

- [`docs/USER_GUIDE.md`](docs/USER_GUIDE.md): screenshot-driven screen reference and UI navigation
- [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md): module boundaries and runtime ownership
- [`docs/BOARD_CONTRACT.md`](docs/BOARD_CONTRACT.md): board-level pin and hardware contract
- [`docs/SD_CARD_SETUP.md`](docs/SD_CARD_SETUP.md): removable-storage paths and installers
- [`docs/PHYSICAL_SMOKE_TEST.md`](docs/PHYSICAL_SMOKE_TEST.md): consolidated hardware verification
- [`docs/RELEASE.md`](docs/RELEASE.md): source and firmware release generation
- [`docs/KNOWN_ISSUES.md`](docs/KNOWN_ISSUES.md): intentionally deferred work
- [`CHANGELOG.md`](CHANGELOG.md): compact milestone history

## License

MIT. Embedded Reader font notices remain under [`docs/licenses/`](docs/licenses/).

## Indic EPUB memory and raster profile

For v1.1.0-r9, large Indic EPUB sessions are first-page-first: page anchors grow lazily after the first page appears. The Reader streams `.RWF` files from SD and retains only shaped glyphs referenced by the visible page, protecting PSRAM for large Hindi/Sanskrit volumes. The browser builder exposes an e-paper alpha threshold; use the default `160` with Noto Sans Devanagari Regular and Noto Sans Gujarati Regular for a lighter 1-bit page. Regenerate and reinstall font packs after changing the threshold.
