# Font Preparation Tool

This tool prepares Rustmix Reader font caches for the X4.

It supports:

```text
- TXT books
- EPUB books
- cache validation
- exact SD upload instructions
- generated artifact cleanup
```

The output stays compatible with the current X4 Reader prepared cache layout:

```text
/FCACHE/<BOOKID>/
  META.TXT
  FONTS.IDX
  PAGES.IDX
  LAT18.VFN
  DEV22.VFN
  P000.VRN
  ...
```

## Required fonts

Use Google Noto fonts:

```text
Latin:
  NotoSans-Regular.ttf

Devanagari:
  NotoSansDevanagari-Regular.ttf
```

Example downloaded font layout:

```text
/Users/<you>/Downloads/Noto_Sans,Noto_Sans_Devanagari/
  Noto_Sans/static/NotoSans-Regular.ttf
  Noto_Sans_Devanagari/static/NotoSansDevanagari-Regular.ttf
```

You can either pass the two font files directly, or pass `--fonts-dir` and the tool will search recursively.

## Clean generated artifacts

Run before committing:

```bash
python3 tools/font_prepare/prepare_book.py clean
```

This removes generated local build/cache artifacts such as:

```text
tools/prepared_txt_real_vfnt/target
tools/prepared_epub_smoke/__pycache__
tools/font_prepare/__pycache__
```

## Prepare TXT

One command:

```bash
python3 tools/font_prepare/prepare_book.py txt \
  --book /path/to/BOOK.TXT \
  --device-path BOOK.TXT \
  --fonts-dir /Users/<you>/Downloads/Noto_Sans,Noto_Sans_Devanagari \
  --out /tmp/FCACHE \
  --clean
```

Or explicit font paths:

```bash
python3 tools/font_prepare/prepare_book.py txt \
  --book /path/to/BOOK.TXT \
  --device-path BOOK.TXT \
  --latin-font /path/to/NotoSans-Regular.ttf \
  --devanagari-font /path/to/NotoSansDevanagari-Regular.ttf \
  --out /tmp/FCACHE
```

## Prepare EPUB

One command:

```bash
python3 tools/font_prepare/prepare_book.py epub \
  --book /path/to/BOOK.EPUB \
  --device-path BOOK.EPUB \
  --fonts-dir /Users/<you>/Downloads/Noto_Sans,Noto_Sans_Devanagari \
  --out /tmp/FCACHE \
  --keep-work
```

If the X4 file browser shows a shortened filename, use that exact filename as `--device-path`.

Example:

```bash
python3 tools/font_prepare/prepare_book.py epub \
  --book /tmp/MIXED_EPUB.EPUB \
  --device-path MIXED_EP.EPU \
  --fonts-dir /Users/<you>/Downloads/Noto_Sans,Noto_Sans_Devanagari \
  --out /tmp/FCACHE
```

## Validate cache

```bash
python3 tools/font_prepare/prepare_book.py validate \
  --cache /tmp/FCACHE/<BOOKID> \
  --device-path BOOK.EPUB
```

The validator checks:

```text
- META.TXT exists and is non-empty
- FONTS.IDX exists and is non-empty
- PAGES.IDX exists and is non-empty
- LAT18.VFN exists
- DEV22.VFN exists
- P000.VRN exists
- META.TXT source= matches the X4 device path
```

## SD upload using browser SD Manager

After preparation, the tool prints exact upload instructions.

Generic workflow:

```text
1. Open Wi-Fi Transfer on X4.
2. Open http://x4.local/ or the shown IP address.
3. At SD root, create/open: FCACHE
4. Inside FCACHE, create/open: <BOOKID>
5. Upload all files from local cache directory:
   /tmp/FCACHE/<BOOKID>
6. Open the book in Reader.
```

Expected Reader behavior:

```text
- Prepared cache opens from /FCACHE/<BOOKID>
- Header shows prepared-cache mode
- Latin and Devanagari render from VFNT/VRUN files
- Normal EPUB/TXT parsing is bypassed when cache is valid
```

## Notes

```text
- Font shaping and rasterization stay host-side.
- X4 only renders prepared glyph runs.
- No network API is used.
- Generated target directories must not be committed.
```


## Reader settings alignment

The X4 Settings app now stores prepared-reader preferences in `_X4/SETTINGS.TXT`:

```text
prepared_font_profile=0   # 0=Compact, 1=Balanced, 2=Large
prepared_fallback_policy=0 # 0=Visible, 1=Latin, 2=Reject
```

Meaning:

```text
Prepared profile:
- Compact: smaller generated glyph sizes, more text per page
- Balanced: default profile for mixed Latin + Devanagari
- Large: larger generated glyph sizes for easier reading

Fallback:
- Visible: unsupported/missing glyphs should be visible
- Latin: prefer Latin transliteration/fallback where available
- Reject: fail preparation when unsupported script text is detected
```

Important: prepared books are generated host-side. Changing these settings on X4 does not reshape an already-generated cache. Regenerate the prepared cache with matching settings, then upload it to:

```text
/FCACHE/<BOOKID>/
```
