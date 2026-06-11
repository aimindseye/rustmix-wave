# Known issues and deferred work

## Weather provider reliability

Open-Meteo requests can fail transiently with transport, TLS, timeout, or HTTP service errors. The device already applies bounded retries, delayed backoff, and last-known-good in-memory retention. A cold boot with no successful fetch may still end in a readable `Weather unavailable` state.

## MCU deep sleep

Sleep-image mode suspends network services, sleeps the e-paper panel, and disables the panel rail, but the MCU event loop remains active. This preserves validated AXP2101 Power-key polling and GPIO45 RTC-alarm handling. Full MCU deep sleep remains deferred.

## EPUB scope

Reader supports bounded reflowable text extraction, TOC navigation, bookmarks, and resume. CSS layout, images, hyperlinks, footnotes, fixed-layout EPUB, DRM, ZIP64, and SD-backed EPUB anchor caches remain deferred.

## Calendar scope

Calendar personal events and U.S. holidays are active. U.S. holiday rows remain read-only. Calendar reminders do not automatically create RTC alarms. Non-U.S. calendar packs are intentionally excluded from the native Calendar route.

## Dictionary scope

Dictionary exact and prefix lookup is active through the complete X4 pack. Saved words, search history, and Reader word-selection lookup remain deferred.

## Merged factory-image release artifact

The supported release artifact is the ESP-IDF ELF flashed through `espflash flash`.
Raw-address flashing with `espflash write-bin` is intentionally unsupported. A
merged factory image remains deferred until the bootloader, partition-table, and
application offsets have been validated on physical hardware.

## EPUB bounds remain intentional

The Reader now inspects EPUB ZIP archives through a file-backed boundary and accepts larger Indic fixtures, but it remains bounded. Archives above 64 MiB, ZIP central directories above 2 MiB, more than 4096 ZIP / manifest / spine rows, or more than 7 MiB of flattened text are rejected with a readable Reader-loading error. Run `python3 scripts/audit-indic-epub-fixture.py <books...>` before copying a new large EPUB to SD.
## Successful build but older firmware still boots

If Cargo reports a new source version but the serial boot log still reports an older version or milestone, confirm that the flash path follows Cargo's active target directory. Run `./scripts/resolve-built-elf.sh` and use `./scripts/flash.sh monitor`. The scripts resolve the ELF through Cargo's `compiler-artifact.executable` JSON output and avoid stale repository-local artifacts or incorrect reconstructed paths.
## ESP-IDF App Descriptor missing during development flash

If `espflash` reports `ESP-IDF App Descriptor ... missing`, the selected file is a host executable rather than the Xtensa ESP-IDF firmware ELF. The supported scripts force `--target xtensa-esp32s3-espidf` and reject compiler artifacts outside that target directory. Use `./scripts/flash.sh monitor`; do not pass `target/release/waveshare-epd397-rust-app` to `espflash`.
## Xtensa `core` crate missing during explicit build

The `xtensa-esp32s3-espidf` target does not ship a precompiled Rust standard library. If Cargo reports `can't find crate for core`, use the repository scripts. They force `-Z build-std=std,panic_abort`, preserve the ESP-IDF target, and resolve Cargo's compiler-reported embedded ELF. The repository also retains `.cargo/config.toml` with the equivalent `[unstable] build-std` contract.


## EPUB parser worker contiguous internal heap

The file-backed EPUB parser runs on a short-lived worker so DEFLATE and XHTML work do not consume the 16 KiB main-task stack. After Wi-Fi and TLS activity, total free heap may remain healthy while the largest contiguous internal block shrinks. The parser prefers a 48 KiB stack, can fall back to 32 KiB for the file-backed path, reserves a 4 KiB preflight guard, and logs `before-worker-epub-parser` and `after-worker-epub-parser` telemetry. Library scans use FAT filenames first and defer OPF title metadata until open so repeated title-worker allocation does not fragment internal heap before parsing. If the readable preflight error still appears, capture the monitor log before increasing limits further.

## Reader Library row window

The portrait Library screen shows seven book rows at a time. Collections larger than seven rows are supported through a selection-following scrolling window. Move beyond the last visible row to reveal the next books; the full retained collection remains reachable.

## Indic pack regeneration after v1.1.0-r9

The r9 browser builder changes the default 1-bit alpha threshold from the earlier dark edge-preserving raster to `160`. Existing `.RWF` files remain readable, but regenerate and reinstall them to obtain the lighter Noto Sans rendering. Large EPUBs should use the r9 firmware because earlier builds attempted to retain complete script packs in RAM after parsing.

## Indic page-font degraded message

When one visible-page subset cannot be loaded, the Reader shows `Indic page font unavailable; check monitor`. Inspect the `reader-unicode-page-fonts` line for the exact SD pack or memory error, then regenerate and reinstall the ZIP when required.
