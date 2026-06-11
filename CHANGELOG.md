## v1.1.0-r10 — Indic EPUB Physical Acceptance Documentation

- Record successful physical rendering of large Hindi / Sanskrit, Hindi, and Gujarati EPUB pages on the Waveshare ESP32-S3 3.97-inch e-paper board.
- Add `Screenshots.md` and three v1.1.0 physical-verification images under `screenshots/`.
- Update `README.md`, `docs/USER_GUIDE.md`, and `screenshots/README.md` with Noto Sans Devanagari Regular and Noto Sans Gujarati Regular Reader coverage.
- Preserve the accepted first-page-first EPUB flow, lazy page anchors, visible-page `.RWF` glyph subsets, Library scrolling, ELF-only flashing, and all non-Reader paths.

# Changelog

## v1.1.0-r9-r2 — Overlay Self-Test Cargo Fixture Repair

- Add a minimal `src/lib.rs` target to the overlay self-test fixture before invoking `cargo fmt`.
- Prevent the standalone repair validator from stopping with `no targets specified in the manifest`.
- Preserve the v1.1.0-r9 Reader runtime, Indic subset loading, lighter raster builder, and host-test import repair unchanged.

## v1.1.0-r9-r1 — Native Host-Test Missing Font Directory Import Repair

- Import `READER_FONTS_DIRECTORY` inside the `reader.rs` native host-test module.
- Preserve the v1.1.0-r9 first-page Indic subset loader, lighter raster builder, Library scrolling, and runtime behavior unchanged.
- Add a source-contract guard so later overlays cannot reintroduce the native host-test compile failure.


## v1.1.0-r9 — First-Page Indic Subset Loading and Lighter Raster Repair

- Open EPUB sessions first-page-first and grow page anchors lazily instead of indexing every EPUB page on the main task before display.
- Stream `.RWF` packs from SD and retain only shaped glyphs required for the visible page, preventing large Hindi/Sanskrit EPUBs from exhausting PSRAM after parsing.
- Reload the visible-page Indic subset after page navigation and TOC jumps while preserving UTF-8 byte anchors.
- Add Reader monitor telemetry for visible-page font loading and degraded pack errors.
- Add an adjustable browser-builder e-paper alpha threshold with a lighter Noto Sans Regular default of `160`; generated packs must be regenerated and reinstalled.
- Preserve Library scrolling, filename-first scans, parser worker boundaries, Reader persistence, and accepted non-Reader paths.

## v1.1.0-r8 — Reader Library Scrolling and EPUB Parser Heap-Preservation Repair

- Render the Reader Library through a selection-following seven-row scrolling window so all retained books remain reachable and visible.
- Use FAT filename titles during Library scans and defer OPF metadata titles until the selected EPUB opens, avoiding repeated 32 KiB title-worker churn immediately before the parser worker starts.
- Keep the preferred 48 KiB EPUB parser stack, add a 32 KiB fragmentation-aware fallback, and reserve a 4 KiB pthread bookkeeping guard.
- Preserve the 16 KiB main-task boundary, file-backed large-archive parser, Indic font packs, bookmarks, resume anchors, and accepted runtime paths.

## v1.1.0-r7 — EPUB Parser Fragmentation-Aware Stack Budget Repair

- Reduce the short-lived EPUB parser worker stack from 64 KiB to 48 KiB so it can start after Wi-Fi and TLS initialization fragment internal heap.
- Reserve an explicit 8 KiB contiguous internal-heap guard for pthread bookkeeping.
- Log parser-worker memory telemetry before spawn and after join, and return a readable preflight error when the largest internal block is insufficient.
- Preserve the 16 KiB main-task boundary, the 32 KiB EPUB-title worker, file-backed large-archive parsing, Indic font packs, and all accepted runtime paths.

## v1.1.0-r6 — Xtensa Build-Std ELF Resolver Repair

- Restore the ESP-IDF Cargo config contract under `.cargo/config.toml`.
- Build `std` and `panic_abort` from `rust-src` explicitly for `xtensa-esp32s3-espidf`.
- Preserve compiler-artifact ELF selection and reject native host artifacts before flashing.

## v1.1.0-r5 — Explicit Xtensa Target ELF Selection Repair

- Force `--target xtensa-esp32s3-espidf` in the firmware resolver and ordinary embedded build helper.
- Reject host `target/release/...` compiler artifacts even when Cargo reports them as executable binaries.
- Keep development flashing and ELF-only release packaging restricted to compiler-reported embedded Xtensa artifacts.
- Add a regression test that reports both a misleading host executable and a valid embedded executable, then proves only the embedded ELF is selected.

## v1.1.0-r4 — Cargo Compiler Artifact ELF Resolution Repair

- Resolve the ESP32-S3 firmware ELF from Cargo `compiler-artifact.executable` JSON instead of reconstructing a presumed output path.
- Keep development flashing and ELF-only release packaging aligned with the exact executable Cargo produced, including nonstandard or hashed artifact layouts.
- Add a regression test with a stale repository-local ELF, misleading metadata target directory, and a fresh compiler-reported executable in a nonstandard path.

## v1.1.0-r3 — Cargo Target Directory Aware ELF Flash Repair

- Resolve the ESP32-S3 release ELF from `cargo metadata` after each build instead of assuming a repository-local `target/` directory.
- Prevent `scripts/flash.sh` and the ELF-only release builder from flashing or packaging a stale local artifact when Cargo uses `CARGO_TARGET_DIR` or another configured target directory.
- Add a sandboxed regression test that creates both a stale local ELF and a fresh custom-target ELF and proves that the fresh Cargo artifact is selected.

## v1.1.0-r2 — Large EPUB Archive Compatibility Repair

- Read EPUB ZIP central-directory metadata and selected members from the SD file instead of retaining the complete archive in RAM.
- Raise bounded ZIP-entry, OPF manifest, spine, flattened-text, and chapter-page-anchor ceilings for the supplied Hindi, Sanskrit, and Gujarati EPUB fixtures.
- Stop silently truncating OPF manifest or spine rows at the old limits.
- Skip malformed missing spine references and EPUB3 navigation rows while preserving readable chapters.
- Add `scripts/audit-indic-epub-fixture.py` to check local EPUBs against the embedded Reader bounds before physical testing.

## v1.1.0-r1 — Indic Font Builder Single-ZIP Download Repair

- Replace nine browser-triggered font-pack downloads with one `rustmix-indic-font-pack.zip` archive.
- Keep `FONTS.TXT` and every generated `.RWF` pack together so browser multiple-download blocking cannot create an incomplete SD pack.
- Allow `scripts/install-indic-font-pack.sh` to install directly from the generated ZIP or from an extracted directory.
- Add a stored-ZIP writer with CRC validation and an automated ZIP install self-test.

## v1.1.0 — SD Unicode Indic EPUB Reader

- Add optional SD-loaded `.RWF` Reader packs for Devanagari and Gujarati EPUB text.
- Use local Noto Sans Devanagari and Noto Sans Gujarati font files through a browser-only pack builder; raw font files are not distributed.
- Preserve Indic Unicode during EPUB normalization and avoid line breaks inside virama-based clusters.
- Keep UTF-8 byte anchors authoritative for bookmarks and resume positions.
- Add local EPUB corpus extraction, SD pack installation, verification scripts, and documentation.

## v1.0.0-r3 — Screenshot User Guide and Architecture Documentation

- Add `screenshots/` with the physically verified UI screenshot set.
- Add `docs/USER_GUIDE.md` with screen-by-screen navigation for Home, Reader, Productivity, Games, Tools, Settings, Wi-Fi transfer, and sleep-image mode.
- Expand `README.md` with sensor-driven utility coverage, motion-game behavior, and main-task worker-isolation policy.
- Expand `docs/ARCHITECTURE.md` with board-service ownership, the native QMI8658 motion-event pipeline, Lua/native game boundaries, runtime memory telemetry, and named worker stack budgets.
- Preserve firmware runtime behavior and the ELF-only release workflow.

## v1.0.0-r2 — Text Editor Layout Alignment

- Move the Voice Notes friendly-title editor onto the shared grid keyboard with BOOT-short NAV H / NAV V toggling.
- Give the Voice Notes title editor its own header, width-safe status strip, keyboard SAVE/CANCEL actions, and long-BOOT cancel/back behavior.
- Compact the Calendar personal-event editor status date and footer so NAV H / NAV V and instructions remain readable on e-paper.
- Preserve the ELF-only release flash workflow and all accepted runtime paths.

## v1.0.0-r1 — Release Flash Workflow Safety Repair

- Remove the unsafe raw-address `espflash write-bin ... 0x0` release instructions and the unverified `*-flash.bin` artifact.
- Publish the ESP-IDF ELF as the supported firmware release artifact.
- Add `scripts/flash-release.sh` to flash an existing release ELF through `espflash flash --chip esp32s3 --monitor`.
- Preserve the ordinary `./scripts/flash.sh monitor` development path.
- Defer any merged factory-image workflow until bootloader, partition-table, and application offsets are validated on physical hardware.

## v1.0.0 — First Stable Rustmix Wave Release

- Promote the physically accepted Rustmix Wave firmware baseline to the first stable release.
- Preserve Reader, Voice Notes, Dictionary, Calendar, Wi-Fi transfer, RTC alarms, weather, audio, sensors, sleep-image mode, Power-key maintenance menu, Lua apps, and motion games.
- Preserve short Power press for manual ghost-clearing maintenance and long Power press for sleep-image mode.

## v0.20.5 — Repository Cleanup, Consolidated Documentation, CI, and Release Binary Builder

- Remove extracted patch-overlay folders, generated archives, patch scripts, repair documents, milestone smoke-test documents, and cache artifacts from the source tree.
- Consolidate durable project documentation into `README.md`, `docs/ARCHITECTURE.md`, `docs/BOARD_CONTRACT.md`, `docs/SD_CARD_SETUP.md`, `docs/PHYSICAL_SMOKE_TEST.md`, `docs/KNOWN_ISSUES.md`, and `docs/RELEASE.md`.
- Replace the stale GitHub workflow with `.github/workflows/ci.yml`, which checks stable formatting, the cleaned source contract, shell syntax, and native-target host tests.
- Add `scripts/build-release-firmware.sh` to produce release artifacts. The later v1.0.0-r1 safety repair restricts supported distribution to the ELF-aware flashing path.
- Tighten `scripts/package-release.sh` so generated source archives exclude overlay directories and local artifacts.
- Preserve the physically accepted runtime: Reader, Voice Notes, Dictionary, Calendar, Power-key maintenance menu and long-press sleep, Wi-Fi transfer, alarms, weather, audio, sensors, Lua apps, and motion games.

## Accepted runtime baseline before cleanup

- v0.20.4: short Power press display-maintenance menu and long Power press sleep-image mode.
- v0.20.3: Calendar personal-event editor with U.S. holidays read-only and atomic `EVENTS.TMP -> EVENTS.TXT` persistence with `EVENTS.BAK` rollback.
- v0.20.1: Calendar U.S. events, month markers, daily agenda, and details.
- v0.20.0: native X4-pack Dictionary with exact, prefix, wildcard, and BOOT-short `NAV H` / `NAV V` keyboard navigation.
- v0.19.x: Voice Notes recording, gain, pause/resume, playback, metadata, titles, storage telemetry, delete confirmation, and LAN export.
- v0.18.x: explicit Wi-Fi transfer portal, bounded Lua app foundation, native game bridges, and IMU motion games.
- v0.17.x: bounded reflowable EPUB Reader foundation and TOC navigation.
- v0.16.x: TXT Reader persistence, bookmarks, preferences, FAT 8.3 runtime names, and Library bookmark alignment.
