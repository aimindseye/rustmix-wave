# Consolidated physical smoke test

For screen names, navigation controls, and reference images, see [`USER_GUIDE.md`](USER_GUIDE.md).

Run this checklist after a release build or any cross-cutting runtime change.

## Build and boot

1. Run `./scripts/validate.sh`.
2. Run `cargo +esp build -Z build-std=std,panic_abort --release --target xtensa-esp32s3-espidf`.
3. Flash with `./scripts/flash.sh monitor`.
4. Confirm boot reaches the Home screen without panic or reset loops.
5. Confirm the displayed version is `1.1.0` and the SD Unicode Indic Reader readiness markers appear.

## Power key and display refresh

1. Press Power briefly and confirm the display-maintenance menu opens.
2. Select `Clear ghosting now` and confirm a clean global refresh returns to the underlying screen.
3. Press Power briefly, select Cancel, and confirm no sleep transition.
4. Hold Power and confirm random sleep-image mode starts and network services suspend.
5. Wait for the wake quiet guard and press Power to restore the prior route.

## Reader

1. Open one TXT book and one EPUB or `.EPU` book.
2. Confirm staged loading, page navigation, Reader Options, preferences, TOC behavior, and bookmark add/remove.
3. Reboot and confirm Continue Reading restores the prior book and page.
4. Confirm `/RUSTMIX/READER/POSITS.TXT` and `CACHE/<8HEX>.CCH` exist.

## Dictionary

1. Open `Tools > Dictionary`.
2. Confirm `CAB`, `BARN`, and `CALENDAR` exact lookup.
3. Confirm `AAR*` prefix lookup and result cycling.
4. Press BOOT briefly and confirm `NAV H` / `NAV V` switches without moving the selected key.
5. Hold BOOT and confirm hierarchical Back.

## Calendar

1. Open `Productivity > Calendar`.
2. Confirm U.S. event markers and daily agenda rendering.
3. Create, edit, and delete one personal event.
4. Confirm U.S. holiday rows remain read-only.
5. Confirm agenda summary, pagination, first row, and footer do not overlap.
6. Confirm `EVENTS.TMP` is absent after successful write and `EVENTS.BAK` is retained.

## Voice Notes

1. Record a note, pause, resume, and save.
2. Confirm a new `VOICE###.WAV` file persists after reboot.
3. Confirm gain selection persists, metadata is readable, playback works, and delete confirmation works.
4. Confirm LAN export displays a path and protected sidecars are not exposed.

## Network, alarms, and settings

1. Confirm Wi-Fi connection and SNTP status.
2. Start the explicit Wi-Fi transfer portal, access it with the displayed code, then stop it.
3. Confirm an alarm can sound, snooze, and dismiss.
4. Confirm alarm behavior is not hidden by the Power-key display menu.
5. Confirm Display settings persist after reboot.

## Games and sensors

1. Open Sudoku and verify rotary movement, BOOT-short axis toggle, edit, and commit.
2. Open one motion game and verify debounced IMU movement.
3. Open Environment and Motion diagnostic screens.
4. Run the audio test chime.

## Text-editor layout alignment

1. Open Voice Notes, select a saved WAV, and choose **Edit friendly title**.
2. Confirm the header reads **VOICE NOTE TITLE / EDIT FRIENDLY TITLE**.
3. Confirm the shared grid keyboard is visible and defaults to `NAV H`.
4. Press BOOT briefly and confirm `NAV V` appears without moving the selected key.
5. Use `SAVE` to persist a friendly title and confirm the internal `VOICE###.WAV` filename remains unchanged.
6. Reopen the title editor, hold BOOT, and confirm the edit is cancelled without saving.
7. Open Calendar, create or edit a personal event, and confirm the status strip shows a compact `YYYY-MM-DD` date plus `NAV H` or `NAV V` without overlap.
8. Confirm the Calendar editor footer is fully visible.

## v1.1 SD Unicode Indic EPUB Reader

Before copying the fixture books to SD, run:

```bash
python3 scripts/audit-indic-epub-fixture.py \
  "Garud puran.epub" \
  "Srimad Bhagavat Mahapuran Volume 1 Sanskrit Hindi.epub" \
  "વાલ્મીકી રામાયણ.epub"
```

Confirm `indic-epub-fixture-audit=ok`. During boot, confirm `rustmix-wave=reader-epub-large-archive-file-backed-ready`.


1. Generate Noto Sans Devanagari and Noto Sans Gujarati `.RWF` packs with `tools/font-builder/index.html`.
2. Install and verify `/RUSTMIX/FONTS/FONTS.TXT` and the generated packs.
3. Open the supplied Garud Puran EPUB and confirm Devanagari text renders instead of question marks.
4. Inspect words with matras and conjuncts such as `श्री`, `कृष्ण`, and `धर्म`.
5. Change Reader font size and confirm the matching Reader-size pack loads.
6. Bookmark a Devanagari page, reboot, and confirm resume and bookmark anchors.
7. Open the supplied Srimad Bhagavat Mahapuran EPUB and navigate multiple chapters.
8. Open the supplied Gujarati Ramayan EPUB and confirm Gujarati text renders.
9. Confirm monitor logs show `reader-library-scroll-epub-title-defer-ready` and `reader-library-scan status=completed ... title-policy=fat-filename-first opf-title=after-open`.
10. Copy at least twelve TXT / EPUB books into `/RUSTMIX/BOOKS`, open Library, move beyond row seven, and confirm the visible rows scroll while every book remains reachable.
11. Confirm monitor logs show `reader-epub-parser-fragmentation-aware-stack-ready` and `epub-parser-worker status=starting` with either the preferred `stack-bytes=49152` or fallback `stack-bytes=32768`, plus `guard-bytes=4096`.
12. Remove one required `.RWF` pack temporarily and confirm the readable `FONT?` install state.
13. Restore the pack and recheck an English EPUB and TXT file.

## v1.1.0-r9 Indic large-book and raster verification

1. Regenerate the Noto Sans Devanagari and Noto Sans Gujarati pack with alpha threshold `160`.
2. Install and verify the ZIP under `/RUSTMIX/FONTS`.
3. Open `BGV1.EPU` and confirm the first page appears without the generic font-pack install message.
4. Confirm monitor output reports `reader-epub-first-page status=ready` and `reader-unicode-page-fonts status=loaded`.
5. Move forward and backward and confirm the visible-page glyph subset reloads without losing bookmarks or resume anchors.
6. Open `GARUDP.EPU` and `RAYN.EPU` and confirm the regenerated glyphs are lighter than the pre-r9 pack.
