# Rustmix Wave architecture

Rustmix Wave is organized as a host-testable Rust library plus a narrow ESP-IDF firmware integration layer. New functionality should remain module-based: keep domain rules out of `main.rs`, keep hardware handles in native owners, and expose small explicit state transitions to the UI.

## Design rules

1. `src/main.rs` owns ESP-IDF integration, peripheral handles, the event loop, logging, and cross-domain coordination.
2. `src/lib.rs` exports host-testable modules.
3. `src/app/state.rs` owns UI state transitions and routes requests to the firmware loop.
4. `src/app/screens/` renders screens without owning hardware.
5. SD-backed features use bounded reads, FAT 8.3-safe writable names, and `.TMP` / `.BAK` recovery where writes are allowed.
6. E-paper refreshes flow through the shared refresh coordinator rather than feature-specific panel writes.
7. Lua scripts declare bounded app behavior; sensitive peripherals, networking, raw panel access, and raw I2C remain Rust-owned.
8. Keyboard and grid-style text entry compose `KeyboardGridNavigation` and use BOOT short press for `NAV H` / `NAV V` switching.

## Runtime layers

```text
ESP-IDF event loop and native hardware ownership
  src/main.rs
        |
        +-- AppState requests and snapshots
        |     src/app/state.rs
        |     src/app/router.rs
        |
        +-- Screen rendering
        |     src/app/screens/*
        |     src/app/widgets/*
        |     src/app/typography/*
        |
        +-- Domain modules
              reader.rs / epub.rs
              voice_notes.rs / voice_note_metadata.rs
              dictionary.rs / keyboard_navigation.rs
              calendar.rs
              wifi_transfer.rs
              alarm.rs / rtc.rs / rtc_alarm_interrupt.rs
              power_key.rs / power_key_menu.rs
              sleep_mode.rs / sleep_images.rs / sleep_network.rs
              games/* / lua_runtime/* / imu_events.rs
              network.rs / weather.rs / storage.rs / unit_converter.rs
```

## Board-service and sensor ownership

`src/board_services.rs` owns focused protocol drivers on one cloneable Rust-owned I2C adapter. Each optional service initializes independently so a missing sensor cannot prevent the verified e-paper shell from booting.

```text
Shared Rust-owned I2C bus
  +-- PCF85063 RTC
  +-- AXP2101 PMIC and Power-key status
  +-- SHTC3 temperature / humidity sensor
  +-- QMI8658 accelerometer / gyroscope
```

The UI consumes compact snapshots rather than moving I2C handles into screens or scripts:

| Service | Snapshot use | Product features |
| --- | --- | --- |
| PCF85063 RTC | local date/time, integrity state | Home clock, Calendar, Clock details, RTC alarm schedules, GPIO45 wake |
| AXP2101 PMIC | battery voltage, battery percentage, USB/charge state, PEK interrupt bits | Home battery badge, Clock details, Power short menu, Power long sleep, wake guard |
| SHTC3 | temperature, relative humidity, sensor ID and CRC status | Home temperature summary, Environment overview, sensor-details page |
| QMI8658 | fixed-point acceleration, gyroscope, die temperature, motion magnitude, dominant axis | Motion overview, Motion Events, tilt-driven games |
| ES8311 + I2S | codec state, PCM RX/TX streams | Audio diagnostics, alarm chime, Voice Notes record/playback |

Raw handles stay inside native Rust owners. Lua apps and renderers receive only bounded snapshots, button events, or debounced motion events.

## Native IMU event pipeline

`src/imu.rs` decodes QMI8658 frames into fixed-point axis values. `src/imu_events.rs` converts those snapshots into discrete native events:

```text
QMI8658 fixed-point sample
  -> strongest planar / rotational axis
  -> threshold evaluation
  -> stable-sample debounce
  -> release latch or cooldown
  -> TILT / SHAKE / ROTATE / LEVEL event
  -> diagnostics screen or native game bridge
```

The bridge samples only when Motion Events or an IMU-driven game needs it. It avoids continuous background redraw churn and prevents raw I2C access from crossing the feature boundary.

Default event protections include:

- stable-sample qualification before tilt and level events
- release-to-neutral latching before repeated tilt or rotate events
- cooldown windows for shake and rotate events
- fixed-point thresholds adjustable from the Motion Events screen
- immediate redraw on detected events with a slower diagnostics heartbeat

Motion-driven games compose that same event boundary:

| Game | Sensor mapping | Native responsibility |
| --- | --- | --- |
| Tilt Maze | Debounced planar tilt moves the player through the maze | Maze parse, collision checks, goal detection, dirty cells, render commands |
| Motion 2048 | Debounced planar tilt becomes one board swipe | Slide, merge, spawn, score, outcome, board redraw |
| Sokoban Tilt | Debounced planar tilt moves the player and pushes crates | Level parse, wall checks, push rules, goals, dirty cells |

The game scripts declare bounded app content. Native Rust owns state mutation, sensor interpretation, and panel refresh decisions.

## Display and refresh ownership

The SSD1677 panel is native `800 × 480`. UI screens render to a logical portrait `480 × 800` framebuffer through `src/orientation.rs`.

`src/panel_refresh.rs` is the single refresh policy boundary. It tracks partial refreshes and requests a global-base refresh for:

- periodic ghost cleanup after the configured partial-refresh limit
- wake restoration
- safety fallbacks
- Reader clear-ghosting actions
- Power short-press display-maintenance actions

The physical Power key does not write the panel directly. A short Power press opens `src/power_key_menu.rs`; selecting `Clear ghosting now` queues the shared manual global refresh path. A long Power press enters the existing sleep-image path.

## Power-key and BOOT-button ownership

`src/power_key.rs` decodes AXP2101 PEK interrupt status. Long-press classification has priority over short-press classification when both sticky bits are observed.

```text
Power short press -> display-maintenance menu
Power long press  -> sleep-image mode
Power wake press  -> restore retained route after wake guard
```

GPIO0 BOOT remains the UI navigation key:

```text
BOOT short -> contextual action, including NAV H / NAV V toggle on keyboards
BOOT long  -> hierarchical Back
```

## Reader boundary

`src/reader.rs` owns TXT and EPUB library rows, per-book state, bookmarks, preferences, and bounded pagination. `src/epub.rs` is an isolated EPUB parser boundary: it reads a bounded ZIP central directory, extracts stored or DEFLATE members, resolves `META-INF/container.xml`, parses OPF manifest and spine records, flattens XHTML text, and builds TOC rows.

The large-archive compatibility path is file-backed. `ZipArchive` retains the SD path, archive length, and a bounded central-directory index rather than reading the complete EPUB into RAM. Selected OPF, navigation, and XHTML members are opened and extracted on demand. The parser reserves a bounded flattened-text buffer up front, rejects archives outside explicit limits, skips malformed missing spine references, and excludes EPUB3 `nav` documents from readable chapter pagination.

```text
Archive bytes             <= 64 MiB on SD
Central-directory bytes   <= 2 MiB retained temporarily
ZIP entries               <= 4096
OPF manifest rows         <= 4096
OPF spine rows            <= 4096
Flattened EPUB text       <= 7 MiB retained for the Reader session
Chapter page anchors      <= 16384
```

Reader state lives below:

```text
/sdcard/RUSTMIX/READER/
  STATE.TXT
  POSITS.TXT
  RECENT.TXT
  MARKS.TXT
  PREFS.TXT
  CACHE/<8HEX>.CCH
```

Reader writes use FAT 8.3-safe `.TMP` and `.BAK` siblings. Bookmarks retain byte offsets as authoritative anchors.

### SD Unicode Indic EPUB typography

`src/reader_unicode.rs` adds an optional Reader-only font boundary for Devanagari and Gujarati EPUB text:

```text
flattened UTF-8 EPUB text
  -> script detection
  -> cluster-aware pagination
  -> active Reader-size FONTS.TXT selection
  -> bounded RWF1 pack load from /sdcard/RUSTMIX/FONTS
  -> longest-prefix shaped cluster lookup
  -> clipped mixed Latin / Indic glyph drawing
```

The local browser builder under `tools/font-builder/` uses locally supplied **Noto Sans Devanagari** and **Noto Sans Gujarati** files. Browser canvas shaping rasterizes corpus-specific conjuncts, matras, virama sequences, and Gujarati clusters into compact 1-bpp `.RWF` entries. Raw font files are not committed and are not copied to the device.

The active session loads only the required script packs for the selected Reader size. Missing packs are a readable Reader state (`FONT?`) rather than a boot failure. Resume positions, chapter anchors, and bookmarks remain source UTF-8 byte offsets, so one visual shaped cluster can map to several Unicode code points without changing persistence semantics.

## Voice Notes boundary

Voice-note friendly-title editing reuses the shared keyboard-grid navigator: BOOT short toggles `NAV H` / `NAV V`, rotary movement follows the active axis, and keyboard `SAVE` / `CANCEL` actions keep long BOOT as hierarchical cancel/back.

`src/voice_notes.rs` owns host-testable WAV framing, catalog scanning, bounded playback streaming, microphone gain, pause/resume state, and stale recording cleanup. `src/voice_note_metadata.rs` owns title, timestamp, gain, and SD-capacity presentation helpers.

Native ES8311 and I2S handles remain owned by `AudioRuntime` in the main loop. Voice Notes do not move hardware handles into a worker.

```text
/sdcard/RUSTMIX/VOICE/
  VOICE###.WAV
  INDEX.TXT
  META.TXT
  SETTINGS.TXT
```

Recording streams through `VOICE###.TMP`, finalizes a PCM16 mono 16 kHz WAV header, and refuses destructive overwrite of an existing WAV.

## Dictionary boundary

`src/dictionary.rs` reuses the Rustmix X4 SD pack without executing its Lua UI:

```text
/sdcard/RUSTMIX/APPS/DICT/
  INDEX.TXT
  DATA/*.JSN
```

The native engine enforces bounded query and shard sizes, validates relative shard paths, performs exact lookup with prefix fallback, and cycles wildcard matches. `src/keyboard_navigation.rs` supplies shared H/V keyboard-grid navigation.

## Calendar boundary

`src/calendar.rs` owns Gregorian date math, bounded SD event parsing, daily agenda state, and personal-event persistence.

```text
/sdcard/RUSTMIX/APPS/CALENDAR/
  EVENTS.TXT
  EVENTS.TMP
  EVENTS.BAK
  US2026.TXT
```

Personal rows in `EVENTS.TXT` are writable. `US2026.TXT` is read-only. `HINDU26.TXT` is intentionally excluded. Calendar does not create or modify RTC alarms.

## Wi-Fi transfer boundary

`src/wifi_transfer.rs` owns an explicit LAN-only portal rooted at `/sdcard/RUSTMIX`. It is off after boot and starts only from Settings. Requests require the displayed session code, remain root-confined, use bounded stream buffers, and write replacement files atomically.

Protected paths include device configuration and internal sidecars such as:

```text
WIFI.TXT
ALARMS.TXT
DISPLAY.TXT
WEATHER.TXT
VOICE/META.TXT
VOICE/SETTINGS.TXT
APPS/CALENDAR/EVENTS.TMP
APPS/CALENDAR/EVENTS.BAK
```

## Games and Lua boundary

`src/lua_runtime/` implements a bounded safe subset for SD-loaded app manifests and scripts. Scripts can declare canvas content and game initialization, but they do not receive panel SPI, framebuffer transport, networking, raw I2C, or arbitrary hardware access.

Native Rust bridges own mutable state and dirty-region decisions:

| App | Native module | Interaction model |
| --- | --- | --- |
| Hello Grid | bounded static canvas path | SD Lua foundation sample |
| Sudoku | `src/games/sudoku.rs` | Rotary navigation, BOOT-short H/V axis toggle, SELECT edit/commit |
| Minesweeper | `src/games/minesweeper.rs` | Rotary navigation, reveal/flag action state, first-reveal safety |
| Tilt Maze | `src/games/tilt_maze.rs` | Debounced QMI8658 tilt |
| Motion 2048 | `src/games/motion_2048.rs` | Debounced QMI8658 tilt swipes |
| Sokoban Tilt | `src/games/sokoban_tilt.rs` | Debounced QMI8658 tilt with crate pushes |

Dirty rectangles flow into the shared refresh policy. Invalid geometry requests a safe global-base refresh fallback instead of exposing raw panel operations to the app.

## Main-task safety and worker boundary

The ESP-IDF main task has a configured 16 KiB stack and remains the narrow hardware-orchestration owner. `src/main.rs` owns long-lived peripheral handles, screen routing, display-refresh requests, sleep transitions, alarm coordination, and compact snapshots. `AppState` is heap-boxed so the UI model does not consume the main task stack.

Heavy operations are deliberately moved away from the main task:

| Operation | Boundary | Stack or buffer policy | Returned value |
| --- | --- | --- | --- |
| Full EPUB parse | short-lived `epub-parser` thread | preferred 48 KiB worker stack, 32 KiB fragmentation fallback, plus 4 KiB contiguous-heap start guard | heap-owned bounded EPUB document |
| EPUB title lookup | deferred until selected EPUB open | Library scan uses FAT filename first; parsed OPF metadata replaces it after open | compact session title |
| HTTPS weather fetch | `runtime_worker::run_named_worker("weather-fetch", ...)` | 64 KiB worker stack, bounded 8 KiB response | parsed weather snapshot or classified error |
| Lua app open | `runtime_worker::run_named_worker("lua-loader", ...)` | 32 KiB worker stack, bounded script size | compact native Lua session and canvas |
| Wi-Fi transfer portal | ESP-IDF HTTP server task | 24 KiB task stack, 4 KiB streaming chunks, 64 MiB upload cap | compact lifecycle snapshot |
| Voice Notes PCM record/playback | cooperative main-loop chunks | bounded I2S RX/TX buffers, streamed `.TMP` finalization | compact UI progress snapshots |
| Indic Reader font pack load | visible-page SD streaming boundary | selected `.RWF` files remain on SD; only glyphs referenced by the current page are retained, capped at 2048 page glyphs | bounded visible-page RWF subset |

`src/runtime_worker.rs` logs memory snapshots before and after generic named workers, joins the short-lived thread, maps worker-start and panic failures into explicit errors, and returns a compact result to the main loop.

The EPUB parser worker uses a fragmentation-aware start boundary. Network startup and TLS may reduce the largest contiguous internal-heap block even when total heap remains healthy. The parser prefers a 48 KiB task stack, falls back to 32 KiB for the file-backed path when necessary, reserves a 4 KiB pthread-bookkeeping guard, records the largest internal block before spawning, and returns a readable preflight error instead of blindly exhausting internal heap.

Library scanning is deliberately filename-first. It does not spawn an EPUB title thread for every row. OPF metadata replaces the fallback title after the selected EPUB has opened. This avoids repetitive worker-stack allocation immediately before full parsing and keeps the largest internal-heap block available for the parser boundary. The Library renderer follows the active selection through a seven-row scrolling window so a larger SD collection remains visible and reachable.

EPUB session creation is first-page-first. The parser worker still returns a bounded flattened document, but the main task no longer walks the complete book to build chapter-page totals before showing content. The current page is rendered immediately, page anchors grow lazily through the existing bounded cache path, and chapter labels use a `+` suffix until the active chapter total is complete. For Indic text, `.RWF` files are streamed from SD and only the visible page's shaped glyph subset remains in RAM. This protects PSRAM for large books such as multi-volume Sanskrit/Hindi EPUBs.

`src/runtime_memory.rs` records:

```text
main-stack-high-water-bytes
heap-free-internal-bytes
heap-largest-internal-block-bytes
heap-free-psram-bytes
```

This makes memory pressure visible in monitor logs and prevents stack-heavy parsing, TLS, or Lua loading from silently consuming the main-loop stack.

Panel SPI ownership never moves to worker threads. The refresh coordinator remains main-task-owned, so all UI, Reader, games, manual ghost cleanup, and wake restoration share one proven panel transport path.

## Storage boundary

`src/storage.rs` provides a bounded browser facade with root confinement, directory-first sorting, retry-limited scans, FAT metadata fallback classification, and bounded previews.

Writable exceptions are deliberately narrow:

- global display preferences
- Reader state
- Voice Notes WAV and sidecar files
- Calendar personal events
- explicit Wi-Fi portal writes

## Screenshot-driven user documentation

[`docs/USER_GUIDE.md`](USER_GUIDE.md) is the operating reference for the product UI. Reference images live in [`/screenshots`](../screenshots/) and cover Home, Reader, Productivity, Calendar, Voice Notes, Games, Tools, Settings, sensors, weather, Wi-Fi transfer, and sleep-image mode.

## Validation boundary

```text
scripts/validate.sh
  cargo +stable fmt --all -- --check
  scripts/validate_source_contract.sh
  scripts/test-host.sh
```

`test-host.sh` explicitly resolves the stable native target before running library tests so `.cargo/config.toml` cannot leak the Xtensa default into host compilation.

GitHub Actions runs the same formatting, static-contract, and native-target test flow on Ubuntu. Embedded firmware generation remains a local ESP toolchain operation.
