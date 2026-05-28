# Rustmix-Wave Reader Port Recon v0

## Status

This is a reconnaissance-only deliverable.

It does **not** port the reader, does **not** enable reader code in
`target-waveshare-epd397`, does **not** enable real rotary input, and does
**not** reuse GPIO3 for input.

Current accepted Rustmix-Wave runtime path remains:

```text
DisplayBackendAdapter
  -> ShellDisplayBridge
  -> Rustmix-Wave rotary-first home UI
```

GPIO3 remains reserved for `EPD_BUSY`.

## Audit summary

Candidate counts from repository scan:

```text
reader/content candidates:          216
storage/state candidates:           232
X4/hardware assumption candidates:   281
Waveshare target candidates:         23
```

These are candidate files from static keyword scanning. They are intentionally
review-oriented; they do not mean every listed file should be ported.

## Reader/content candidates

- `.github/ISSUE_TEMPLATE/x4-hal-extraction-task.md` — hits: reader
- `.github/workflows/x4-firmware-release.yml` — hits: txt
- `AGENTS.md` — hits: book, font, page, reader
- `ARCHITECTURE.md` — hits: book, dictionary, flashcard, font, reader, txt
- `Cargo.toml` — hits: epub, reader
- `README.md` — hits: book, bookmark, books, dictionary, epub, flashcard, font, progress, reader, txt
- `ROADMAP.md` — hits: book, bookmark, font, library, page, progress, reader
- `SCOPE.md` — hits: book, bookmark, dictionary, epub, font, page, progress, reader, txt
- `SCREENSHOTS.md` — hits: book, bookmark, books, dictionary, library, page, reader
- `USERGUIDE.md` — hits: book, books, dictionary, epub, flashcard, font, progress, reader, txt
- `core/src/apps/mod.rs` — hits: reader
- `core/src/apps/reader/mod.rs` — hits: book, bookmark, epub, library, page, progress, reader
- `core/src/hal/input.rs` — hits: reader
- `core/src/hal/power.rs` — hits: reader
- `core/src/models/book_id.rs` — hits: book
- `core/src/models/book_identity.rs` — hits: book
- `core/src/models/book_identity_title_cache.rs` — hits: book, books, epub, reader, txt
- `core/src/models/bookmark.rs` — hits: book, bookmark, page, reader
- `core/src/models/display_drawing_abstractions.rs` — hits: library, progress, reader
- `core/src/models/input_semantic_mapping.rs` — hits: library, page, reader
- `core/src/models/lua_app_catalog.rs` — hits: reader
- `core/src/models/lua_app_manifest.rs` — hits: reader, txt
- `core/src/models/lua_app_storage.rs` — hits: page, txt
- `core/src/models/mod.rs` — hits: book, bookmark, books, font, library, page, progress, reader, txt
- `core/src/models/prepared_cache_metadata.rs` — hits: book, epub, font, page, reader, txt
- `core/src/models/progress.rs` — hits: book, font, page, progress, reader
- `core/src/models/reader_file.rs` — hits: book, books, epub, library, reader, txt
- `core/src/models/reader_meta.rs` — hits: book, epub, reader, txt
- `core/src/models/reader_runtime.rs` — hits: book, bookmark, books, epub, library, page, reader, txt
- `core/src/models/reader_state_io.rs` — hits: book, bookmark, books, epub, font, page, progress, reader, txt
- `core/src/models/reader_viewport.rs` — hits: reader
- `core/src/models/sd_font_catalog.rs` — hits: font, reader, txt
- `core/src/models/state.rs` — hits: book, font, progress, reader, txt
- `core/src/models/storage_layout.rs` — hits: book, bookmark, books, progress, reader, txt
- `core/src/models/storage_path_helpers.rs` — hits: book, bookmark, books, epub, library, progress, reader, txt
- `core/src/models/theme.rs` — hits: book, font, reader
- `core/src/models/wifi_transfer_config.rs` — hits: book
- `core/src/os.rs` — hits: reader
- `docs/rustmix-wave/architecture.md` — hits: book, bookmark, books, dictionary, flashcard, font, library, progress, reader
- `docs/rustmix-wave/bootstrap-v0.md` — hits: reader
- `docs/rustmix-wave/display-backend-import-v0.md` — hits: reader
- `docs/rustmix-wave/migration-plan.md` — hits: book, bookmark, books, dictionary, flashcard, font, library, progress, reader
- `docs/rustmix-wave/reader-port-recon-v0.md` — hits: book, bookmark, books, dictionary, epub, flashcard, font, library, page, progress
- `docs/rustmix-wave/shell-bridge-ui-import-v0.md` — hits: reader
- `docs/rustmix-wave/ui-direction.md` — hits: reader
- `docs/rustmix-wave/voice-layer.md` — hits: reader
- `hal-waveshare-epd397/src/lib.rs` — hits: book, books, dictionary, flashcard, reader
- `hal-xteink-x4/src/display_smoke.rs` — hits: book, bookmark, books, epub, library, page, reader, txt
- `hal-xteink-x4/src/lib.rs` — hits: library, page, reader
- `scripts/audit_rustmix_wave_reader_port_recon_v0.py` — hits: book, bookmark, books, dictionary, epub, flashcard, font, library, page, progress
- `scripts/build_x4_firmware_artifacts.sh` — hits: txt
- `scripts/build_x4_release_firmware.sh` — hits: txt
- `scripts/check_repo_hygiene.sh` — hits: reader, txt
- `scripts/create_rustfirmware_bin.sh` — hits: font, txt
- `scripts/deploy/check_deploy_ready.sh` — hits: reader, txt
- `scripts/install_flashcards_image_cards_sd.sh` — hits: flashcard
- `scripts/install_flashcards_sd_app.sh` — hits: flashcard
- `scripts/validate_rustmix_wave_display_backend_import_v0.py` — hits: reader
- `scripts/validate_rustmix_wave_reader_port_recon_v0.py` — hits: book, bookmark, epub, progress, reader
- `scripts/validate_rustmix_wave_shell_bridge_ui_import_v0.py` — hits: reader
- `scripts/validate_x4_flash_ota_slot_policy.sh` — hits: reader
- `src/apps/app_category_dashboard.rs` — hits: book, bookmark, dictionary, flashcard, library, reader
- `target-waveshare-epd397/README.md` — hits: reader
- `target-xteink-x4/Cargo.toml` — hits: epub, font
- `target-xteink-x4/build.rs` — hits: book, books, font
- `target-xteink-x4/src/rustmix_x4/apps/app_catalog.rs` — hits: book, books, reader
- `target-xteink-x4/src/rustmix_x4/apps/app_list_model.rs` — hits: reader
- `target-xteink-x4/src/rustmix_x4/apps/home.rs` — hits: book, bookmark, books, dictionary, flashcard, font, library, page, progress, reader
- `target-xteink-x4/src/rustmix_x4/apps/manager.rs` — hits: book, bookmark, books, epub, font, page, progress, reader, txt
- `target-xteink-x4/src/rustmix_x4/apps/mod.rs` — hits: reader
- `target-xteink-x4/src/rustmix_x4/apps/reader_state.rs` — hits: book, bookmark, books, epub, font, page, progress, reader, txt
- `target-xteink-x4/src/rustmix_x4/contracts/input.rs` — hits: reader
- `target-xteink-x4/src/rustmix_x4/contracts/input_semantics.rs` — hits: book, bookmark, library, page, reader
- `target-xteink-x4/src/rustmix_x4/contracts/storage.rs` — hits: book, bookmark, epub, library, progress, reader, txt
- `target-xteink-x4/src/rustmix_x4/contracts/storage_path_helpers.rs` — hits: book, bookmark, progress, txt
- `target-xteink-x4/src/rustmix_x4/contracts/storage_state_contract.rs` — hits: book, bookmark, epub, progress, txt
- `target-xteink-x4/src/rustmix_x4/display/display_geometry_runtime.rs` — hits: page, reader
- `target-xteink-x4/src/rustmix_x4/display/redraw_policy_runtime.rs` — hits: book, bookmark, epub, page, progress, reader
- `target-xteink-x4/src/rustmix_x4/imported/mod.rs` — hits: reader
- `target-xteink-x4/src/rustmix_x4/imported/x4_reader_runtime.rs` — hits: book, bookmark, epub, reader
- ... 136 more candidates omitted from this recon doc.


## Storage/state candidates

- `.github/ISSUE_TEMPLATE/x4-hal-extraction-task.md` — hits: file
- `.github/workflows/ci.yml` — hits: cache
- `.github/workflows/x4-firmware-release.yml` — hits: cache, file, path
- `AGENTS.md` — hits: card, path, sd
- `ARCHITECTURE.md` — hits: cache, card, directory, file, path, sd, state
- `Cargo.toml` — hits: card, file, sd
- `README.md` — hits: bookmark, cache, card, file, path, progress, sd, state
- `ROADMAP.md` — hits: bookmark, cache, file, path, progress, sd, state
- `SCOPE.md` — hits: bookmark, cache, card, file, path, progress, sd, state
- `SCREENSHOTS.md` — hits: bookmark, card, file, sd, state
- `USERGUIDE.md` — hits: cache, card, file, progress, sd, state
- `core/Cargo.toml` — hits: card
- `core/src/apps/reader/mod.rs` — hits: bookmark, file, path, progress, state
- `core/src/hal/display.rs` — hits: file, path, sd, state
- `core/src/hal/input.rs` — hits: state
- `core/src/hal/mod.rs` — hits: sd, state
- `core/src/hal/power.rs` — hits: file, state
- `core/src/hal/storage.rs` — hits: card, directory, file, path, sd, state
- `core/src/models/book_id.rs` — hits: file, path
- `core/src/models/book_identity.rs` — hits: file, path
- `core/src/models/book_identity_title_cache.rs` — hits: cache, file, path
- `core/src/models/bookmark.rs` — hits: bookmark, path
- `core/src/models/display_drawing_abstractions.rs` — hits: cache, file, progress
- `core/src/models/input_semantic_mapping.rs` — hits: file, path
- `core/src/models/lua_app_catalog.rs` — hits: path, sd, state
- `core/src/models/lua_app_discovery.rs` — hits: cache, file, path, sd, state
- `core/src/models/lua_app_manifest.rs` — hits: file, path, sd
- `core/src/models/lua_app_runtime.rs` — hits: state
- `core/src/models/lua_app_storage.rs` — hits: cache, file, path, state
- `core/src/models/lua_host_api.rs` — hits: file, state
- `core/src/models/mod.rs` — hits: bookmark, cache, file, path, progress, sd, state
- `core/src/models/prepared_cache_metadata.rs` — hits: cache, file, path, state
- `core/src/models/progress.rs` — hits: path, progress
- `core/src/models/reader_file.rs` — hits: file, path, sd
- `core/src/models/reader_meta.rs` — hits: file, path
- `core/src/models/reader_runtime.rs` — hits: bookmark, file, path, state
- `core/src/models/reader_state_io.rs` — hits: bookmark, file, path, progress, state
- `core/src/models/reader_viewport.rs` — hits: cache, state
- `core/src/models/sd_font_catalog.rs` — hits: file, path, sd
- `core/src/models/state.rs` — hits: cache, file, path, progress, sd, state
- `core/src/models/storage_layout.rs` — hits: bookmark, cache, file, path, progress, state
- `core/src/models/storage_path_helpers.rs` — hits: bookmark, cache, file, path, progress, state
- `core/src/models/wifi_transfer_config.rs` — hits: cache, card, file, path, sd
- `core/src/os.rs` — hits: card, sd, state
- `docs/rustmix-wave/architecture.md` — hits: bookmark, card, path, progress, state
- `docs/rustmix-wave/migration-plan.md` — hits: bookmark, card, path, progress, state
- `docs/rustmix-wave/reader-port-recon-v0.md` — hits: bookmark, cache, card, directory, fat, file, path, progress, sd, state
- `docs/rustmix-wave/shell-bridge-ui-import-v0.md` — hits: path
- `docs/rustmix-wave/ui-direction.md` — hits: state
- `docs/rustmix-wave/voice-layer.md` — hits: sd, state
- `espflash.toml` — hits: file
- `hal-waveshare-epd397/Cargo.toml` — hits: path
- `hal-waveshare-epd397/src/lib.rs` — hits: card, path, sd, state
- `hal-xteink-x4/Cargo.toml` — hits: path
- `hal-xteink-x4/src/display.rs` — hits: sd
- `hal-xteink-x4/src/display_smoke.rs` — hits: bookmark, file, path, sd, state
- `hal-xteink-x4/src/input.rs` — hits: state
- `hal-xteink-x4/src/lib.rs` — hits: file, sd
- `hal-xteink-x4/src/power.rs` — hits: state
- `hal-xteink-x4/src/storage.rs` — hits: card, file, path, state
- `scripts/audit_remaining_pulp_runtime_dependencies.sh` — hits: path
- `scripts/audit_rustmix_wave_reader_port_recon_v0.py` — hits: bookmark, cache, card, directory, fat, file, path, progress, sd, state
- `scripts/build_x4_firmware_artifacts.sh` — hits: directory
- `scripts/build_x4_release_firmware.sh` — hits: file
- `scripts/check_repo_hygiene.sh` — hits: cache, path
- `scripts/create_rustfirmware_bin.sh` — hits: cache, file, sd, state
- `scripts/deploy/check_deploy_ready.sh` — hits: state
- `scripts/flash_x4_standard_partition_table.sh` — hits: card, sd
- `scripts/install_flashcards_image_cards_sd.sh` — hits: card, path, sd
- `scripts/install_flashcards_sd_app.sh` — hits: card, sd
- `scripts/validate_rustmix_wave_display_backend_import_v0.py` — hits: file, path, sd
- `scripts/validate_rustmix_wave_reader_port_recon_v0.py` — hits: bookmark, file, path, progress, sd
- `scripts/validate_rustmix_wave_repository_bootstrap_v0.py` — hits: file, path
- `scripts/validate_rustmix_wave_shell_bridge_ui_import_v0.py` — hits: file, path, state
- `scripts/validate_x4_flash_ota_slot_policy.sh` — hits: path
- `scripts/validate_x4_standard_partition_table_compatibility.py` — hits: file, path
- `src/apps/app_category_dashboard.rs` — hits: bookmark, card, file, state
- `support/rustmix-lua-vm/src/lib.rs` — hits: path, sd
- `target-waveshare-epd397/Cargo.toml` — hits: file, path
- `target-waveshare-epd397/README.md` — hits: state
- ... 152 more candidates omitted from this recon doc.


## X4-specific hardware/display/input assumption candidates

- `.github/ISSUE_TEMPLATE/x4-hal-extraction-task.md` — hits: display, input, x4, xteink
- `.github/workflows/ci.yml` — hits: pulp, x4, xteink
- `.github/workflows/x4-firmware-release.yml` — hits: x4, xteink
- `AGENTS.md` — hits: pulp, x4, xteink
- `ARCHITECTURE.md` — hits: button, buttons, display, input, pulp, spi, ssd, x4, xteink
- `Cargo.toml` — hits: epd, x4, xteink
- `README.md` — hits: busy, display, epd, gpio, input, pulp, spi, ssd, x4, xteink
- `ROADMAP.md` — hits: display, x4, xteink
- `SCOPE.md` — hits: display, input, pulp, x4, xteink
- `SCREENSHOTS.md` — hits: display, x4, xteink
- `USERGUIDE.md` — hits: display, input, x4, xteink
- `core/src/apps/reader/mod.rs` — hits: x4
- `core/src/hal/display.rs` — hits: display, epd, spi, x4
- `core/src/hal/input.rs` — hits: adc, button, input, x4
- `core/src/hal/mod.rs` — hits: battery, button, display, epd, input, spi, x4
- `core/src/hal/power.rs` — hits: adc, battery, x4
- `core/src/hal/storage.rs` — hits: display
- `core/src/models/book_id.rs` — hits: x4
- `core/src/models/book_identity.rs` — hits: display
- `core/src/models/book_identity_title_cache.rs` — hits: display, input, x4
- `core/src/models/bookmark.rs` — hits: display
- `core/src/models/display_drawing_abstractions.rs` — hits: battery, display, x4
- `core/src/models/input_semantic_mapping.rs` — hits: button, buttons, input
- `core/src/models/lua_app_catalog.rs` — hits: display, input
- `core/src/models/lua_app_discovery.rs` — hits: display, input
- `core/src/models/lua_app_manifest.rs` — hits: display, input
- `core/src/models/lua_app_runtime.rs` — hits: button
- `core/src/models/lua_app_storage.rs` — hits: display
- `core/src/models/lua_host_api.rs` — hits: battery, display, input
- `core/src/models/mod.rs` — hits: button, display, input, x4
- `core/src/models/prepared_cache_metadata.rs` — hits: input
- `core/src/models/reader_file.rs` — hits: display, x4
- `core/src/models/reader_meta.rs` — hits: display
- `core/src/models/reader_runtime.rs` — hits: x4, xteink
- `core/src/models/reader_state_io.rs` — hits: display
- `core/src/models/reader_viewport.rs` — hits: x4
- `core/src/models/sd_font_catalog.rs` — hits: display
- `core/src/models/state.rs` — hits: button, buttons, display, input, pulp, x4
- `core/src/models/storage_layout.rs` — hits: x4
- `core/src/models/storage_path_helpers.rs` — hits: epd, input, x4
- `core/src/os.rs` — hits: battery, display, x4
- `core/src/ui/activity.rs` — hits: input
- `docs/rustmix-wave/architecture.md` — hits: display, epd, x4, xteink
- `docs/rustmix-wave/bootstrap-v0.md` — hits: display, epd, input, x4
- `docs/rustmix-wave/display-backend-import-v0.md` — hits: busy, display, epd, gpio, input, spi, x4
- `docs/rustmix-wave/migration-plan.md` — hits: display, input, x4
- `docs/rustmix-wave/reader-port-recon-v0.md` — hits: adc, battery, busy, button, buttons, display, epd, gpio, input, pulp
- `docs/rustmix-wave/shell-bridge-ui-import-v0.md` — hits: busy, display, epd, gpio, input
- `docs/rustmix-wave/ui-direction.md` — hits: display, x4
- `docs/rustmix-wave/voice-layer.md` — hits: display, x4, xteink
- `espflash.toml` — hits: x4, xteink
- `hal-waveshare-epd397/Cargo.toml` — hits: epd
- `hal-waveshare-epd397/README.md` — hits: busy, button, display, epd, gpio, input
- `hal-waveshare-epd397/src/lib.rs` — hits: battery, busy, display, epd, gpio, input, spi, ssd, x4
- `hal-xteink-x4/Cargo.toml` — hits: x4, xteink
- `hal-xteink-x4/src/display.rs` — hits: display, epd, spi, x4, xteink
- `hal-xteink-x4/src/display_smoke.rs` — hits: battery, busy, display, epd, input, spi, ssd, x4, xteink
- `hal-xteink-x4/src/input.rs` — hits: adc, button, buttons, input, x4, xteink
- `hal-xteink-x4/src/lib.rs` — hits: adc, battery, display, epd, input, spi, ssd, x4, xteink
- `hal-xteink-x4/src/power.rs` — hits: adc, battery, spi, x4, xteink
- `hal-xteink-x4/src/storage.rs` — hits: x4, xteink
- `scripts/audit_remaining_pulp_runtime_dependencies.sh` — hits: pulp, x4, xteink
- `scripts/audit_rustmix_wave_reader_port_recon_v0.py` — hits: adc, battery, busy, button, buttons, display, epd, gpio, input, pulp
- `scripts/build_x4_firmware_artifacts.sh` — hits: x4, xteink
- `scripts/build_x4_release_firmware.sh` — hits: x4, xteink
- `scripts/check_repo_hygiene.sh` — hits: pulp, x4, xteink
- `scripts/create_rustfirmware_bin.sh` — hits: x4, xteink
- `scripts/deploy/check_deploy_ready.sh` — hits: x4, xteink
- `scripts/erase_x4_otadata_select_app0.sh` — hits: x4
- `scripts/flash_x4_release_bin.sh` — hits: x4
- `scripts/flash_x4_rustmix_app0.sh` — hits: x4, xteink
- `scripts/flash_x4_standard_partition_table.sh` — hits: x4, xteink
- `scripts/read_x4_partition_table.sh` — hits: x4, xteink
- `scripts/validate_rustmix_wave_display_backend_import_v0.py` — hits: busy, display, epd, gpio, input, spi
- `scripts/validate_rustmix_wave_reader_port_recon_v0.py` — hits: busy, display, epd, gpio, input, x4
- `scripts/validate_rustmix_wave_repository_bootstrap_v0.py` — hits: busy, epd, gpio, x4
- `scripts/validate_rustmix_wave_shell_bridge_ui_import_v0.py` — hits: busy, display, epd, gpio, input
- `scripts/validate_x4_flash_ota_slot_policy.sh` — hits: spi, x4, xteink
- `scripts/validate_x4_standard_partition_table_compatibility.py` — hits: spi, x4, xteink
- `scripts/validate_x4_standard_partition_table_compatibility.sh` — hits: x4
- ... 201 more candidates omitted from this recon doc.


## Existing Rustmix-Wave/Waveshare candidates

- `Cargo.toml` — hits: epd397, waveshare
- `README.md` — hits: DisplayBackendAdapter, ShellDisplayBridge, epd397, rustmix-wave, waveshare
- `SCOPE.md` — hits: waveshare
- `docs/rustmix-wave/architecture.md` — hits: DisplayBackendAdapter, ShellDisplayBridge, epd397, rustmix-wave, waveshare
- `docs/rustmix-wave/bootstrap-v0.md` — hits: epd397, rustmix-wave, waveshare
- `docs/rustmix-wave/display-backend-import-v0.md` — hits: DisplayBackendAdapter, ShellDisplayBridge, epd397, rustmix-wave, waveshare
- `docs/rustmix-wave/migration-plan.md` — hits: DisplayBackendAdapter, ShellDisplayBridge, rustmix-wave, waveshare
- `docs/rustmix-wave/reader-port-recon-v0.md` — hits: DisplayBackendAdapter, ShellDisplayBridge, epd397, rustmix-wave, waveshare
- `docs/rustmix-wave/shell-bridge-ui-import-v0.md` — hits: DisplayBackendAdapter, ShellDisplayBridge, rustmix-wave, waveshare
- `docs/rustmix-wave/ui-direction.md` — hits: rustmix-wave, waveshare
- `docs/rustmix-wave/voice-layer.md` — hits: rustmix-wave, waveshare
- `hal-waveshare-epd397/Cargo.toml` — hits: epd397, rustmix-wave, waveshare
- `hal-waveshare-epd397/README.md` — hits: DisplayBackendAdapter, ShellDisplayBridge, epd397, waveshare
- `hal-waveshare-epd397/src/lib.rs` — hits: DisplayBackendAdapter, ShellDisplayBridge, epd397, rustmix-wave, waveshare
- `scripts/audit_rustmix_wave_reader_port_recon_v0.py` — hits: DisplayBackendAdapter, ShellDisplayBridge, epd397, rustmix-wave, waveshare
- `scripts/validate_rustmix_wave_display_backend_import_v0.py` — hits: DisplayBackendAdapter, ShellDisplayBridge, epd397, rustmix-wave, waveshare
- `scripts/validate_rustmix_wave_reader_port_recon_v0.py` — hits: DisplayBackendAdapter, ShellDisplayBridge, epd397, rustmix-wave, waveshare
- `scripts/validate_rustmix_wave_repository_bootstrap_v0.py` — hits: epd397, rustmix-wave, waveshare
- `scripts/validate_rustmix_wave_shell_bridge_ui_import_v0.py` — hits: DisplayBackendAdapter, ShellDisplayBridge, epd397, rustmix-wave, waveshare
- `target-waveshare-epd397/Cargo.toml` — hits: epd397, rustmix-wave, waveshare
- `target-waveshare-epd397/README.md` — hits: epd397, rustmix-wave, waveshare
- `target-waveshare-epd397/build.rs` — hits: epd397, waveshare
- `target-waveshare-epd397/src/main.rs` — hits: DisplayBackendAdapter, ShellDisplayBridge, epd397, rustmix-wave, waveshare


## Reusable reader/core module categories

The first port should prefer Rustmix X4 reader logic that is independent of
X4 GPIO, X4 display pins, and X4 input wiring.

Reusable candidates should be grouped into these categories before code moves:

1. **Book/library model**
   - book identity
   - title metadata
   - recent books
   - library listing
   - content type selection

2. **Reader state model**
   - current book
   - current page / offset
   - bookmark model
   - progress save/restore
   - settings affecting layout

3. **Content parsers and page preparation**
   - TXT parsing
   - EPUB parsing where portable
   - page/chapter navigation
   - cache metadata
   - prepared assets where not tied to X4 display dimensions

4. **Font and glyph logic**
   - VFN/custom font handling where portable
   - text shaping assumptions that are display-independent
   - bitmap glyph rendering if it can target a generic 1bpp surface

5. **App/data formats**
   - dictionary shard conventions
   - flashcard topic conventions
   - Lua app folder conventions
   - sleep image conversion conventions, after display target dimensions are handled

## X4-only assumptions to isolate

Do not directly copy modules that assume:

- ESP32-C3 or Xteink X4 target triples.
- X4 e-paper pin map.
- X4 display dimensions or orientation.
- X4 button ladder or GPIO input behavior.
- GPIO3 input ownership.
- X4 battery/power behavior.
- X4 partition/flash assumptions.
- Direct display-driver ownership from reader code.
- Direct SD/FAT ownership from reader code.

For Rustmix-Wave, GPIO3 is `EPD_BUSY` and must not be used for rotary or
button input.

## Proposed display adapter

Reader code should not write directly to `DisplayBackendAdapter`.

The Waveshare reader port should target a reader-facing display surface, then
that surface should flush through `ShellDisplayBridge`.

Recommended boundary:

```rust
pub trait ReaderDisplaySurface {
    fn logical_width(&self) -> u32;   // 480 for portrait
    fn logical_height(&self) -> u32;  // 800 for portrait
    fn clear(&mut self);
    fn fill_rect(&mut self, x: u32, y: u32, w: u32, h: u32, black: bool);
    fn draw_mono_bitmap(&mut self, x: u32, y: u32, w: u32, h: u32, data: &[u8]);
    fn flush(&mut self) -> anyhow::Result<()>;
}
```

First implementation:

```text
ReaderDisplaySurface
  -> ShellDisplayBridge
  -> DisplayBackendAdapter
  -> free Waveshare display backend
```

Rules:

- Keep portrait logical coordinates at `480x800`.
- Keep native RAM mapping inside `ShellDisplayBridge`.
- Do not expose SPI/DC/RST/BUSY to reader code.
- Do not reintroduce the old Focus Hub `EpaperDisplay::new` wrapper.
- Start with full refresh only; partial refresh can come later.

## Proposed SD/storage compatibility path

Reader code should not mount or own SD directly in the first port.

Recommended boundary:

```rust
pub trait ReaderStorage {
    fn list_books(&mut self) -> anyhow::Result<BookList>;
    fn read_file_chunk(&mut self, path: &str, offset: usize, buf: &mut [u8]) -> anyhow::Result<usize>;
    fn read_state_file(&mut self, path: &str, buf: &mut [u8]) -> anyhow::Result<usize>;
    fn write_state_file(&mut self, path: &str, data: &[u8]) -> anyhow::Result<()>;
}
```

Initial compatibility strategy:

- Preserve Rustmix X4 SD data formats where possible.
- Keep reader state files compatible until a Waveshare-specific migration is needed.
- Keep book/progress/bookmark semantics stable.
- Add a Waveshare storage adapter later after display UI remains stable.
- Prefer a root such as `/RUSTMIX` or the existing Rustmix SD root if already established by the X4 repo.
- Do not change dictionary/flashcard/Lua app file formats during the first reader port.

## Concrete port plan

### Step 1 — Reader module inventory freeze

Commit this recon doc and keep it as the source of truth for first reader
migration decisions.

### Step 2 — Reader boundary traits

Add reader-facing traits only:

- `ReaderDisplaySurface`
- `ReaderStorage`
- `ReaderInputEvent` model, but no real input wiring yet

No UI or reader behavior should change in this step.

### Step 3 — Host/unit compile slice

Move or reference reusable reader/core modules into a portable crate/module
without ESP32-S3 hardware dependencies.

Expected output:

- builds on host where possible
- no direct GPIO/SPI/display pin usage
- no direct `ShellDisplayBridge` dependency in parser/state code

### Step 4 — Display adapter slice

Implement `ReaderDisplaySurface` for `ShellDisplayBridge`.

Expected output:

- render static reader placeholder page
- render title/header/footer
- full refresh only
- no SD reading yet

### Step 5 — Storage adapter slice

Add Waveshare `ReaderStorage` implementation.

Expected output:

- list books from SD
- read one TXT file or prepared page
- read/write progress in compatible format

### Step 6 — Minimal reader screen

Wire selected `READER` row to a simulated open path first, then a real open path.

Expected output:

- dashboard can open Reader screen
- Reader screen can show first page/title
- Back returns to home later
- real rotary input still can remain simulated until pin audit is complete

### Step 7 — Real rotary input after pin audit

Only after display/storage reader slices are stable:

- audit rotary pins
- confirm no GPIO3 usage
- add input reader
- map rotary turn to page/menu navigation
- map press to open/select
- map hold to voice later

## First code migration recommendation

The first actual code migration after this recon should **not** be EPUB.

Recommended first migration:

```text
ReaderDisplaySurface + static reader placeholder
```

Then:

```text
ReaderStorage + TXT/simple book listing
```

Only after those pass should EPUB/cache/font-heavy logic move.

## Acceptance markers for future reader migration

Suggested future markers:

```text
RAW-RUSTMIX-WAVE-READER-BOUNDARY-V0-OK
RAW-RUSTMIX-WAVE-READER-DISPLAY-PLACEHOLDER-OK
RAW-RUSTMIX-WAVE-READER-STORAGE-LIST-OK
RAW-RUSTMIX-WAVE-READER-TXT-FIRST-PAGE-OK
```

## Non-goals

- No reader code is ported in this recon slice.
- No real rotary input is enabled.
- No GPIO3 input usage is allowed.
- No audio/voice implementation is included.
- No existing X4 reader code is deleted.
