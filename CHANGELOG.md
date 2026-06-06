# Changelog

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
