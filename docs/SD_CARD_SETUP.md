# SD-card setup

Use a FAT-formatted SD card. Rustmix Wave mounts it at `/sdcard` and expects the following product tree:

```text
/RUSTMIX/
  WIFI.TXT
  WEATHER.TXT
  ALARMS.TXT
  DISPLAY.TXT
  BOOKS/
  READER/
    CACHE/
  FONTS/
    FONTS.TXT
    *.RWF
  VOICE/
  SLEEP/
    *.BMP
  APPS/
    HGRID/
    SUDOKU/
    MINES/
    TILTMAZE/
    M2048/
    SOKOBAN/
    DICT/
      INDEX.TXT
      DATA/*.JSN
    CALENDAR/
      EVENTS.TXT
      US2026.TXT
```

## Install bundled examples

```bash
./scripts/install-sd-examples.sh /Volumes/YOUR_SD_CARD
```

Existing paths are preserved by default. Use `--force` only when deliberately replacing bundled example files:

```bash
./scripts/install-sd-examples.sh --force /Volumes/YOUR_SD_CARD
```

The generic installer preserves an existing Dictionary and Calendar tree. Use the dedicated installers for intentional complete-pack replacement.

## Wi-Fi

Copy or edit `/RUSTMIX/WIFI.TXT`:

```text
ssid=YOUR_NETWORK
password=YOUR_PASSWORD
timezone=America/New_York
ntp_server=pool.ntp.org
```

Do not commit real credentials.

## Weather

Optional `/RUSTMIX/WEATHER.TXT` example:

```text
provider=open-meteo
location=New York, NY
latitude=40.7128
longitude=-74.0060
timezone=America/New_York
refresh_minutes=30
```

## Alarms

Optional `/RUSTMIX/ALARMS.TXT` example:

```text
snooze_minutes=10
alarm=Workday,07:30,weekdays,on,recurring
alarm=Weekend,09:00,weekends,off,recurring
alarm=Appointment,16:45,2026-06-10,on,once
```

Calendar personal events remain separate from alarms.

## Display preferences

`/RUSTMIX/DISPLAY.TXT` supports:

```text
font_family=inter|atkinson-hyperlegible
font_size=compact|standard|large
```

## Sleep images

Files below `/RUSTMIX/SLEEP` must be uncompressed monochrome Windows BMP files:

```text
800 × 480
1-bpp
```

Install bundled samples:

```bash
./scripts/install-sleep-images.sh /Volumes/YOUR_SD_CARD
```

## Reader books and state

Copy TXT, EPUB, or FAT-friendly `.EPU` books into:

```text
/RUSTMIX/BOOKS
```

The device creates Reader state automatically:

```text
/RUSTMIX/READER/STATE.TXT
/RUSTMIX/READER/POSITS.TXT
/RUSTMIX/READER/RECENT.TXT
/RUSTMIX/READER/MARKS.TXT
/RUSTMIX/READER/PREFS.TXT
/RUSTMIX/READER/CACHE/<8HEX>.CCH
```

Reader writes use `.TMP` and `.BAK` siblings for recovery.

## Devanagari and Gujarati Reader fonts

Generate SD Unicode packs locally with `tools/font-builder/index.html`. Use local **Noto Sans Devanagari** and **Noto Sans Gujarati** files and a corpus extracted from the EPUBs. The builder downloads one `rustmix-indic-font-pack.zip` archive so browser multiple-download protection cannot omit pack files:

```bash
python3 scripts/extract-epub-font-corpus.py book1.epub book2.epub > indic-corpus.txt
./scripts/install-indic-font-pack.sh ~/Downloads/rustmix-indic-pack /Volumes/YOUR_SD_CARD
./scripts/verify-indic-font-pack.sh /Volumes/YOUR_SD_CARD
```

See [`UNICODE_FONTS.md`](UNICODE_FONTS.md).

## Voice Notes

The device creates:

```text
/RUSTMIX/VOICE/VOICE###.WAV
/RUSTMIX/VOICE/INDEX.TXT
/RUSTMIX/VOICE/META.TXT
/RUSTMIX/VOICE/SETTINGS.TXT
```

Do not hand-edit sidecars while the device is active.

## Complete Dictionary pack

Install from a local `rustmix-x4-firmware` checkout:

```bash
./scripts/install-dictionary-x4-pack.sh \
  --force \
  --x4-repo /Users/piyushdaiya/Documents/projects/rustmix-x4-firmware \
  /Volumes/YOUR_SD_CARD
```

Verify representative lookups:

```bash
./scripts/verify-dictionary-x4-pack.sh /Volumes/YOUR_SD_CARD
```

## U.S.-only Calendar pack

Install from a local X4 checkout:

```bash
./scripts/install-calendar-x4-pack.sh \
  --force \
  --x4-repo /Users/piyushdaiya/Documents/projects/rustmix-x4-firmware \
  /Volumes/YOUR_SD_CARD
```

The installer includes `EVENTS.TXT` and `US2026.TXT`, and explicitly excludes `HINDU26.TXT`.

Calendar personal-event writes use:

```text
EVENTS.TMP -> EVENTS.TXT
EVENTS.BAK retained for rollback
```
