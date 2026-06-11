# SD Unicode Reader font packs

Rustmix Wave v1.1 adds optional SD-loaded Reader packs for Devanagari and Gujarati EPUB text. The firmware repository intentionally does not distribute raw TTF or OTF files.

## Recommended local fonts

Use:

```text
Noto Sans Devanagari Regular
Noto Sans Gujarati Regular
```

Download them from the official Noto projects, then select the local files in the browser builder. The generated `.RWF` files are compact 1-bpp shaped-cluster packs for Rustmix Wave Reader use.

## Why the builder uses shaped clusters

Indic scripts cannot be rendered correctly by drawing Unicode code points independently. Matras, virama sequences, conjuncts, reph, combining marks, and Gujarati clusters require shaping. The browser builder delegates shaping to the browser canvas and exports the resulting glyph clusters. Firmware performs longest-prefix cluster lookup while preserving source UTF-8 byte anchors.

## Generate a corpus from EPUB files

```bash
python3 scripts/extract-epub-font-corpus.py \
  "Garud puran.epub" \
  "Srimad Bhagavat Mahapuran Volume 1 Sanskrit Hindi.epub" \
  "વાલ્મીકી રામાયણ.epub" \
  > indic-corpus.txt
```

The corpus extractor reads local EPUB XHTML files only. Keep copyrighted books outside the source repository.

## Build packs in the browser

Open:

```text
tools/font-builder/index.html
```

Select the local Noto Sans font files and `indic-corpus.txt`. The tool downloads one archive:

```text
rustmix-indic-font-pack.zip
```

Extract the ZIP. It contains:

```text
FONTS.TXT
NSD16.RWF
NSD20.RWF
NSD24.RWF
NSD28.RWF
NSG16.RWF
NSG20.RWF
NSG24.RWF
NSG28.RWF
```

Devanagari-only or Gujarati-only packs are valid when only one script is needed. The single-ZIP workflow avoids browser multiple-download blocking, which can otherwise leave `FONTS.TXT` referencing files that were never saved.

## Install to the SD card

Place the generated files in a directory, then run:

```bash
./scripts/install-indic-font-pack.sh ~/Downloads/rustmix-indic-font-pack.zip /Volumes/YOUR_SD_CARD
./scripts/verify-indic-font-pack.sh /Volumes/YOUR_SD_CARD
```

The device path is:

```text
/RUSTMIX/FONTS/
  FONTS.TXT
  *.RWF
```

## Runtime behavior

- Latin Reader typography remains embedded and unchanged.
- Devanagari and Gujarati ranges are preserved during EPUB normalization.
- Pagination does not split a virama-based cluster across lines.
- The active Reader session streams the selected script pack from SD and retains only shaped glyphs required by the visible page.
- Missing packs display an install message and `FONT?` badge instead of crashing.
- Bookmarks and resume positions remain UTF-8 source-byte anchors.


## Lighter e-paper raster setting

The browser builder exposes a 1-bit alpha threshold. Higher values discard more anti-aliased edge pixels and create a lighter glyph on the e-paper panel. Start with:

```text
160 — balanced light
```

Use `128` only when a lighter page is too thin. Use `192` when Noto Sans Regular still appears too dark. Changing this setting requires regenerating and reinstalling the `.RWF` ZIP; firmware does not contain the raw font files.

## Large-book visible-page loading

Large Hindi and Sanskrit EPUBs can consume most PSRAM with flattened UTF-8 text. Rustmix Wave therefore does not retain every bitmap from the selected `.RWF` file. It streams the SD pack, keeps only glyphs referenced by the visible page, and reloads that bounded subset after page navigation. EPUB session creation also displays the first page before lazily extending the page-anchor index.

## Font-pack bounds

Each `.RWF` pack is limited to:

```text
1 MiB per pack
8192 shaped glyph clusters
64 UTF-8 bytes per cluster key
```

These limits keep Reader memory use explicit and bounded.

## Audit large EPUB fixtures before flashing

The Reader opens ZIP metadata through a file-backed archive boundary so large books do not require the complete `.epub` file in RAM. Local fixtures can be checked before copying them onto the SD card:

```bash
python3 scripts/audit-indic-epub-fixture.py \
  "Garud puran.epub" \
  "Srimad Bhagavat Mahapuran Volume 1 Sanskrit Hindi.epub" \
  "વાલ્મીકી રામાયણ.epub"
```

The embedded bounds are:

```text
64 MiB archive on SD
2 MiB ZIP central directory
4096 ZIP entries
4096 OPF manifest entries
4096 OPF spine entries
7 MiB flattened Reader text
16384 chapter page anchors
```

Missing malformed spine references such as a dangling `cover` id are skipped when readable XHTML chapters remain. EPUB3 `nav` rows are used for the table of contents but are excluded from reading pagination.
