# Rustmix Wave v1.0.0

Rustmix Wave v1.0.0 is the first stable firmware release for the Waveshare ESP32-S3 3.97-inch e-paper board.

## Highlights

### Reader

* TXT and EPUB reading
* EPUB table of contents
* Per-book resume positions
* Bookmarks
* Reader preferences
* Multiple e-paper-friendly font profiles
* Paragraph alignment, themes, orientation, and progress display

### Voice Notes

* Native ES8311 microphone capture
* PCM16 mono 16 kHz WAV recording
* Saved-WAV playback through the onboard audio path
* LOW, NORMAL, HIGH, and BOOST microphone gain profiles
* Persistent microphone-gain selection
* Pause and resume recording
* Friendly titles while preserving FAT 8.3 WAV filenames
* Recording date and time
* Available SD-card storage display
* Delete confirmation
* LAN export and download shortcut

### Offline Dictionary

* Native Rust dictionary UI
* Complete Rustmix X4 prefix-shard Dictionary pack compatibility
* Exact-word lookup
* Prefix lookup and wildcard cycling
* Shared BOOT-short H/V keyboard navigation

### Calendar

* Native monthly Calendar
* U.S. holiday pack support
* Personal events
* Daily agenda
* Event details
* Create, edit, and delete personal events
* U.S. holidays remain read-only
* Recovery-safe `EVENTS.TMP -> EVENTS.TXT` persistence with `EVENTS.BAK` rollback

### Sensors and board services

* PCF85063 RTC clock and alarm scheduling
* GPIO45 RTC alarm wake
* AXP2101 battery, USB, charging, and Power-key handling
* SHTC3 temperature and humidity
* QMI8658 motion diagnostics
* Wi-Fi, SNTP time synchronization, and weather
* SD-card-backed Reader, Dictionary, Calendar, Voice Notes, apps, and sleep images

### Motion games

* Tilt Maze: planar tilt moves through maze cells
* Motion 2048: planar tilt performs tile swipes
* Sokoban Tilt: planar tilt moves the player and pushes crates
* Sudoku and Minesweeper: BOOT-short H/V grid navigation

### Power-key behavior

* Short physical Power press opens the display-maintenance menu
* **Clear ghosting now** performs a manual global e-paper refresh
* Long physical Power press enters random sleep-image mode
* Wake restores the previous route after the quiet guard interval

## Safe flashing instructions

Extract the firmware ZIP and flash the release ELF with:

```bash
./waveshare-epd397-rust-app-v1.0.0-flash-release.sh \
  waveshare-epd397-rust-app-v1.0.0.elf
```

From a source checkout, use:

```bash
./scripts/flash-release.sh \
  dist/waveshare-epd397-rust-app-v1.0.0.elf
```

The supported release artifact is the ELF file.

Do **not** use `espflash write-bin` with address `0x0`. That is a low-level raw-address operation and is not a supported Rustmix Wave installation method.

## Documentation

* `README.md`
* `docs/USER_GUIDE.md`
* `docs/ARCHITECTURE.md`
* `docs/RELEASE.md`
* `docs/SD_CARD_SETUP.md`
* `docs/PHYSICAL_SMOKE_TEST.md`
