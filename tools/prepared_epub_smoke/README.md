# Prepared EPUB Book Smoke

This tool path prepares EPUB content offline and reuses the existing Rustmix prepared TXT cache format.

## What it proves

```text
- EPUB spine text is extracted on a host machine.
- Mixed English + Devanagari text is shaped/rasterized host-side.
- The output cache is written as VFNT + VRUN files.
- The X4 Reader detects /FCACHE/<BOOKID>/META.TXT for an EPUB.
- When cache exists, the Reader renders prepared pages instead of parsing the EPUB normally.
```

## Generate a tiny smoke EPUB

```bash
python3 tools/prepared_epub_smoke/create_smoke_epub.py /tmp/MIXED_EPUB.EPUB
```

Copy `/tmp/MIXED_EPUB.EPUB` to the SD card root.

## Prepare cache

Use real Noto font paths on your Mac.

```bash
python3 tools/prepared_epub_smoke/prepare_epub.py \
  --epub /tmp/MIXED_EPUB.EPUB \
  --device-path MIXED_EPUB.EPUB \
  --latin-font /path/to/NotoSans-Regular.ttf \
  --devanagari-font /path/to/NotoSansDevanagari-Regular.ttf \
  --out /Volumes/<SDCARD>/FCACHE
```

The `--device-path` must match the EPUB filename/path as seen by the X4 Reader.

## SD layout

```text
/
  MIXED_EPUB.EPUB
  FCACHE/
    <BOOKID>/
      META.TXT
      FONTS.IDX
      PAGES.IDX
      LAT18.VFN
      DEV22.VFN
      P000.VRN
      ...
```

## Device validation

```text
1. Flash firmware.
2. Open Files / Library.
3. Open MIXED_EPUB.EPUB.
4. Reader should open prepared pages directly.
5. English and Devanagari glyphs should render from prepared cache.
6. Next / Prev should page through the prepared EPUB cache.
```
