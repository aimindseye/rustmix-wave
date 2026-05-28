# Rustmix-Wave X4 Reader Reuse Map v0

## Scope

This document scans the existing Rustmix X4 reader/UI code that is still present
in this repository and classifies what should be reused for Rustmix-Wave.

This slice is documentation and validation only. It does not change runtime
behavior.

## Current accepted Rustmix-Wave boundary

Rustmix-Wave must preserve this working path:

- DisplayBackendAdapter
- ShellDisplayBridge
- ReaderDisplaySurface
- SD-backed ReaderStorage
- GPIO3 reserved for EPD_BUSY
- GPIO4 Button_Up
- GPIO5 Button_Function
- GPIO6 Button_Down

The guiding rule is:

Reuse reader/UI/product logic. Do not reuse X4 hardware ownership.

## Reuse categories

### Reuse as-is

These files should be portable or nearly portable. They should be moved into a
shared/core location only when needed by a runtime deliverable.

| File | Reason |
|---|---|
| `core/src/models/book_id.rs` | portable model or path helper with low hardware coupling |
| `core/src/models/book_identity.rs` | portable model or path helper with low hardware coupling |
| `core/src/models/storage_layout.rs` | portable model or path helper with low hardware coupling |
| `core/src/models/storage_path_helpers.rs` | portable model or path helper with low hardware coupling |


### Reuse with adapter

These files are valuable, but must be adapted to Rustmix-Wave boundaries. They
must not directly own display pins, SPI, SD buses, or input GPIOs.

| File | Reason |
|---|---|
| `core/src/apps/reader/mod.rs` | reader logic is useful, but display/input/storage must go through Waveshare boundaries |
| `core/src/models/book_identity_title_cache.rs` | reader logic is useful, but display/input/storage must go through Waveshare boundaries |
| `core/src/models/bookmark.rs` | state model is useful, but persistence path must stay Waveshare SD-backed |
| `core/src/models/prepared_cache_metadata.rs` | reader logic is useful, but display/input/storage must go through Waveshare boundaries |
| `core/src/models/progress.rs` | state model is useful, but persistence path must stay Waveshare SD-backed |
| `core/src/models/reader_file.rs` | reader logic is useful, but display/input/storage must go through Waveshare boundaries |
| `core/src/models/reader_meta.rs` | reader logic is useful, but display/input/storage must go through Waveshare boundaries |
| `core/src/models/reader_runtime.rs` | reader logic is useful, but display/input/storage must go through Waveshare boundaries |
| `core/src/models/reader_state_io.rs` | state model is useful, but persistence path must stay Waveshare SD-backed |
| `core/src/models/reader_viewport.rs` | reader logic is useful, but display/input/storage must go through Waveshare boundaries |
| `core/src/models/sd_font_catalog.rs` | font/glyph logic may be portable, but pixels must draw through ReaderDisplaySurface |


### Copy concepts only

These files are useful for product behavior and visual design, but should not be
ported line-for-line.

| File | Reason |
|---|---|
| `core/src/apps/mod.rs` | UI/product behavior is useful, but implementation likely mixes app shell or target assumptions |
| `core/src/hal/input.rs` | UI/product behavior is useful, but implementation likely mixes app shell or target assumptions |
| `core/src/hal/mod.rs` | UI/product behavior is useful, but implementation likely mixes app shell or target assumptions |
| `core/src/hal/storage.rs` | UI/product behavior is useful, but implementation likely mixes app shell or target assumptions |
| `core/src/models/display_drawing_abstractions.rs` | UI/product behavior is useful, but implementation likely mixes app shell or target assumptions |
| `core/src/models/input_semantic_mapping.rs` | UI/product behavior is useful, but implementation likely mixes app shell or target assumptions |
| `core/src/models/lua_app_catalog.rs` | UI/product behavior is useful, but implementation likely mixes app shell or target assumptions |
| `core/src/models/lua_app_discovery.rs` | UI/product behavior is useful, but implementation likely mixes app shell or target assumptions |
| `core/src/models/lua_app_manifest.rs` | UI/product behavior is useful, but implementation likely mixes app shell or target assumptions |
| `core/src/models/lua_app_runtime.rs` | UI/product behavior is useful, but implementation likely mixes app shell or target assumptions |
| `core/src/models/lua_app_storage.rs` | UI/product behavior is useful, but implementation likely mixes app shell or target assumptions |
| `core/src/models/lua_host_api.rs` | UI/product behavior is useful, but implementation likely mixes app shell or target assumptions |
| `core/src/models/mod.rs` | UI/product behavior is useful, but implementation likely mixes app shell or target assumptions |
| `core/src/models/state.rs` | UI/product behavior is useful, but implementation likely mixes app shell or target assumptions |
| `core/src/models/theme.rs` | UI/product behavior is useful, but implementation likely mixes app shell or target assumptions |
| `core/src/os.rs` | UI/product behavior is useful, but implementation likely mixes app shell or target assumptions |
| `core/src/services/mod.rs` | UI/product behavior is useful, but implementation likely mixes app shell or target assumptions |
| `core/src/services/storage_service.rs` | UI/product behavior is useful, but implementation likely mixes app shell or target assumptions |
| `core/src/ui/activity.rs` | UI/product behavior is useful, but implementation likely mixes app shell or target assumptions |
| `vendor/smol-epub/src/async_io.rs` | UI/product behavior is useful, but implementation likely mixes app shell or target assumptions |
| `vendor/smol-epub/src/cache.rs` | UI/product behavior is useful, but implementation likely mixes app shell or target assumptions |
| `vendor/smol-epub/src/css.rs` | UI/product behavior is useful, but implementation likely mixes app shell or target assumptions |
| `vendor/smol-epub/src/epub.rs` | UI/product behavior is useful, but implementation likely mixes app shell or target assumptions |
| `vendor/smol-epub/src/html_strip.rs` | UI/product behavior is useful, but implementation likely mixes app shell or target assumptions |
| `vendor/smol-epub/src/jpeg.rs` | UI/product behavior is useful, but implementation likely mixes app shell or target assumptions |
| `vendor/smol-epub/src/lib.rs` | UI/product behavior is useful, but implementation likely mixes app shell or target assumptions |
| `vendor/smol-epub/src/png.rs` | UI/product behavior is useful, but implementation likely mixes app shell or target assumptions |
| `vendor/smol-epub/src/xml.rs` | UI/product behavior is useful, but implementation likely mixes app shell or target assumptions |
| `vendor/smol-epub/src/zip.rs` | UI/product behavior is useful, but implementation likely mixes app shell or target assumptions |


### Do not reuse

These files are X4-specific or hardware/runtime-specific. They may be referenced
for understanding, but should not be included in Rustmix-Wave runtime.

| File | Reason |
|---|---|
| `core/src/hal/power.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `core/src/models/wifi_transfer_config.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `core/src/services/power_manager.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `hal-xteink-x4/src/display_smoke.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `hal-xteink-x4/src/lib.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `hal-xteink-x4/src/storage.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/apps/app_catalog.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/apps/app_list_model.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/apps/home.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/apps/manager.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/apps/mod.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/apps/reader_state.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/contracts/boundary_contract.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/contracts/display.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/contracts/input.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/contracts/input_semantics.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/contracts/mod.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/contracts/storage.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/contracts/storage_path_helpers.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/contracts/storage_state_contract.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/display/display_geometry_runtime.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/display/redraw_policy_runtime.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/imported/mod.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/imported/x4_reader_runtime.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/input/active_semantic_mapper.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/io/mod.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/io/reader_state_runtime.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/io/storage_readonly_adapter.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/io/storage_readonly_boundary.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/io/storage_readonly_x4_bridge.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/io/storage_state.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/io/storage_state_adapter.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/io/storage_state_runtime.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/lua/calendar_script.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/lua/catalog_bridge.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/lua/daily_mantra_script.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/lua/daily_mantra_vm_bridge.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/lua/dictionary.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/lua/flashcards_script.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/lua/game_stub_script.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/lua/mod.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/lua/panchang_script.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/lua/sd_manifest_reader_bridge.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/lua/tool_stub_script.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/mod.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/network/biscuit_wifi.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/network/network_time.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/network/time_status.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/network/upload.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/network/wifi_scan.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/physical/display_x4_backend.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/physical/input_x4_backend.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/physical/spi_bus_runtime.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/physical/spi_bus_runtime_contract.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/runtime_adapter_contracts.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/sleep/sleep_screen_mode.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/state/bookmark_state_io_adapter.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/state/metadata_state_io_adapter.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/state/mod.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/state/progress_state_io_adapter.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/state/state_registry_adapter.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/state/theme_state_io_adapter.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/text/font_asset_reader.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/text/font_assets.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/text/font_catalog.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/text/glyph_bitmap_renderer.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/text/glyph_cache.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/text/glyph_run.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/text/glyph_run_renderer.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/text/layout.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/text/mod.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/text/script.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/text/sd_font_selection.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/text/sd_vfn_runtime.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/text/static_font_assets.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/text/text_run.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/ui.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/ui/biscuit_files.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/ui/biscuit_home.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| `target-xteink-x4/src/rustmix_x4/ui/biscuit_home_apps.rs` | hardware/runtime ownership is X4-specific and must not be ported directly |
| ... | 51 more candidates omitted from this doc. |


## Concrete migration list

### Reader layout

Use X4 reader layout concepts for:

- compact header
- small footer
- page number and progress
- margins
- line spacing
- selected book title display

Port target:

- create a Rustmix-Wave reader layout module that renders only through
  ReaderDisplaySurface
- remove heavy black bars and large inner border from the current Wave reader
- use more of the 480x800 logical screen

Do not port:

- X4 display driver calls
- X4 framebuffer byte orientation
- X4 refresh ownership

### TXT wrapping and pagination

Use X4 wrapping/pagination concepts for:

- word wrapping
- line counting
- page breaks by rendered lines
- clamped next/previous navigation
- clean chapter/title handling where possible

Port target:

- replace current byte-stride pagination with layout-aware TXT pagination
- keep ReaderStorage as the content source
- keep button navigation already accepted on Waveshare

Do not port:

- assumptions tied to X4 display update policy
- assumptions tied to X4 storage mount paths unless abstracted

### Font rendering

Use X4 font/glyph concepts for:

- bitmap font metrics
- glyph rendering helpers
- line height calculation
- title/body/footer font roles

Port target:

- add a Wave font layer that emits pixels/rects through ReaderDisplaySurface
- prefer a readable body font over the current debug 5x7 scaled text
- keep 5x7 only for debug or tiny status labels

Do not port:

- hardware framebuffer ownership
- SSD1677-specific byte layout
- direct display RAM writes

### Book browser

Use X4 book browser concepts for:

- filename-to-title derivation
- list density
- selected row highlight
- empty state
- sorting
- future recent-books list

Port target:

- keep /sdcard/BOOKS/*.txt scanning
- keep GPIO6/GPIO4/GPIO5 navigation
- improve visible rows and selected-row contrast

Do not port:

- touch/card UI assumptions
- X4 input router assumptions

### Progress and bookmarks later

Use X4 progress/bookmark models for:

- current book identity
- page index
- page count
- bookmark entries
- state file naming

Port target later:

- first add read-only resume display
- then add progress save after navigation
- then add bookmark add/list
- keep writes under a Rustmix-Wave SD state directory

Do not port yet:

- automatic writes during early reader UI work
- any X4 state path that conflicts with the Wave SD layout

## Recommended next deliverables

1. Rustmix-Wave X4 Reader Layout Port v0
   - port layout concepts only
   - keep current byte/layout pagination for one slice
   - use selected book title
   - reduce chrome and wasted space

2. Rustmix-Wave X4 TXT Layout Pagination Port v0
   - replace byte-stride pages with word/line pagination
   - use X4 pagination concepts where portable

3. Rustmix-Wave X4 Font Layer Port v0
   - port readable bitmap font/glyph concepts
   - render through ReaderDisplaySurface only

4. Rustmix-Wave Book Browser UI Polish v0
   - port X4 list concepts
   - keep Wave buttons and SD path

5. Rustmix-Wave Progress Resume v0
   - port progress model
   - add Wave-owned state path
   - no bookmarks until resume is stable

## Non-goals

- No runtime behavior change.
- No code migration in this slice.
- No display backend changes.
- No input changes.
- No storage changes.
- No EPUB port.
- No bookmark/progress persistence.
