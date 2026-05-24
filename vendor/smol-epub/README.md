# smol-epub

Minimal `no_std` EPUB parser for memory-constrained embedded targets.
Streaming decompression, HTML stripping, CSS resolution, and optional
1-bit image decoders. Works anywhere `alloc` is available.

Designed for devices with as little as ~140 KB of heap.

## What it does

An EPUB file is a ZIP archive containing XHTML chapters, metadata,
and images. smol-epub handles the full pipeline:

- **zip** — central-directory parser, streaming DEFLATE extraction
  (via miniz_oxide)
- **epub** — `container.xml` → OPF → spine, metadata, and TOC
- **xml** — minimal tag/attribute scanner for EPUB metadata
- **css** — CSS property parser for EPUB stylesheets
- **html_strip** — single-pass streaming HTML-to-styled-text converter;
  emits inline style markers for bold, italic, headings, block quotes,
  and image references
- **cache** — chapter decompress-and-strip pipeline with cache metadata
- **png** — PNG decoder → 1-bit Floyd–Steinberg dithered bitmap
- **jpeg** — JPEG decoder → 1-bit Floyd–Steinberg dithered bitmap

The `images` feature (enabled by default) gates the `png` and `jpeg`
modules.

## I/O model

All functions that read from an external source take a generic closure:

```rust
FnMut(offset: u32, buf: &mut [u8]) -> Result<usize, E>
```

SD cards, flash, `std::fs::File`, in-memory buffers — anything that
supports random-access reads. The crate never assumes a storage backend.

## Usage

```rust
use smol_epub::zip::{self, ZipIndex};
use smol_epub::epub::{self, EpubMeta, EpubSpine, EpubToc};

// build ZIP index from the EPUB file's central directory
let mut zip = ZipIndex::new();
let (cd_offset, cd_size) = ZipIndex::parse_eocd(&tail_buf, file_size)?;
// ... read central directory into cd_buf ...
zip.parse_central_directory(&cd_buf)?;

// parse EPUB structure
let container = zip::extract_entry(/* ... */)?;
let mut opf_path = [0u8; epub::OPF_PATH_CAP];
let opf_len = epub::parse_container(&container, &mut opf_path)?;

// extract metadata and reading-order spine
let mut meta = EpubMeta::new();
let mut spine = EpubSpine::new();
epub::parse_opf(&opf_data, opf_dir, &zip, &mut meta, &mut spine)?;

// optionally parse the table of contents
let mut toc = EpubToc::new();
if let Some(src) = epub::find_toc_source(&opf_data, opf_dir, &zip) {
    epub::parse_toc(src, &toc_data, toc_dir, &spine, &zip, &mut toc);
}

// stream-decompress + HTML-strip a chapter
let n = smol_epub::cache::stream_strip_entry(
    &entry, local_offset, read_fn, output_fn,
)?;
```

## Image decoders

The `png` and `jpeg` modules decode images to 1-bit monochrome bitmaps
using Floyd–Steinberg dithering — intended for e-ink displays. Each
format provides three variants:

- `decode_{png,jpeg}_fit` — from an in-memory `&[u8]`
- `decode_{png,jpeg}_streaming` — from a stored (uncompressed) ZIP entry via read closure
- `decode_{png,jpeg}_deflate_streaming` — from a DEFLATE-compressed ZIP entry via read closure

All variants accept `max_w` / `max_h` and integer-downscale to fit.
Output is a `DecodedImage`: packed 1-bit MSB-first, row-major, where
a set bit means black (ink).

## Memory

Typical peak heap on an embedded target:

- ZIP index parse: ~5 KB
- chapter stream-strip (DEFLATE): ~51 KB
- PNG streaming decode: ~90 KB
- JPEG streaming decode: ~30 KB
- JPEG DEFLATE streaming decode: ~79 KB

Large internal structs (e.g. `DecompressorOxide` at ~11 KB) are always
heap-allocated. Stack usage is kept low throughout.

## License

MIT or Apache-2.0, at your option.