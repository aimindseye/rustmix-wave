#!/usr/bin/env python3
from pathlib import Path
import re
import sys

ROOT = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else Path.cwd().resolve()
OUT = ROOT / "docs" / "rustmix-wave" / "reader-port-recon-v0.md"

EXCLUDE_DIRS = {
    ".git",
    "target",
    "local_patches",
    ".embuild",
    ".cargo",
}

TEXT_EXTS = {
    ".rs",
    ".toml",
    ".md",
    ".txt",
    ".sh",
    ".py",
    ".json",
    ".yml",
    ".yaml",
}

READER_PATTERNS = [
    "reader",
    "book",
    "books",
    "epub",
    "txt",
    "page",
    "bookmark",
    "progress",
    "font",
    "dictionary",
    "flashcard",
    "library",
]

STORAGE_PATTERNS = [
    "sd",
    "card",
    "fat",
    "file",
    "directory",
    "path",
    "state",
    "progress",
    "bookmark",
    "cache",
]

HARDWARE_PATTERNS = [
    "x4",
    "xteink",
    "gpio",
    "button",
    "buttons",
    "input",
    "spi",
    "ssd",
    "epd",
    "busy",
    "adc",
    "battery",
    "display",
    "pulp",
]

WAVESHARE_PATTERNS = [
    "waveshare",
    "rustmix-wave",
    "epd397",
    "ShellDisplayBridge",
    "DisplayBackendAdapter",
]


def should_skip(path: Path) -> bool:
    parts = set(path.relative_to(ROOT).parts)
    return bool(parts & EXCLUDE_DIRS)


def read_text(path: Path) -> str:
    try:
        return path.read_text(errors="ignore")
    except Exception:
        return ""


def matches_any(text: str, path_text: str, patterns: list[str]) -> list[str]:
    haystack = (path_text + "\n" + text).lower()
    return [p for p in patterns if p.lower() in haystack]


def collect_candidates() -> dict[str, list[tuple[str, list[str]]]]:
    groups: dict[str, list[tuple[str, list[str]]]] = {
        "reader_content_candidates": [],
        "storage_state_candidates": [],
        "x4_hardware_assumption_candidates": [],
        "waveshare_target_candidates": [],
    }

    for path in sorted(ROOT.rglob("*")):
        if not path.is_file() or should_skip(path):
            continue

        if path.suffix not in TEXT_EXTS:
            continue

        rel = str(path.relative_to(ROOT))
        text = read_text(path)

        reader_hits = matches_any(text, rel, READER_PATTERNS)
        storage_hits = matches_any(text, rel, STORAGE_PATTERNS)
        hardware_hits = matches_any(text, rel, HARDWARE_PATTERNS)
        wave_hits = matches_any(text, rel, WAVESHARE_PATTERNS)

        if reader_hits:
            groups["reader_content_candidates"].append((rel, reader_hits))
        if storage_hits:
            groups["storage_state_candidates"].append((rel, storage_hits))
        if hardware_hits:
            groups["x4_hardware_assumption_candidates"].append((rel, hardware_hits))
        if wave_hits:
            groups["waveshare_target_candidates"].append((rel, wave_hits))

    return groups


def render_list(items: list[tuple[str, list[str]]], limit: int = 80) -> str:
    if not items:
        return "- No candidate files found.\n"

    lines = []
    for rel, hits in items[:limit]:
        hit_text = ", ".join(sorted(set(hits))[:10])
        lines.append(f"- `{rel}` — hits: {hit_text}")
    if len(items) > limit:
        lines.append(f"- ... {len(items) - limit} more candidates omitted from this recon doc.")
    return "\n".join(lines) + "\n"


def main() -> None:
    groups = collect_candidates()

    doc = f"""# Rustmix-Wave Reader Port Recon v0

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
reader/content candidates:          {len(groups["reader_content_candidates"])}
storage/state candidates:           {len(groups["storage_state_candidates"])}
X4/hardware assumption candidates:   {len(groups["x4_hardware_assumption_candidates"])}
Waveshare target candidates:         {len(groups["waveshare_target_candidates"])}
```

These are candidate files from static keyword scanning. They are intentionally
review-oriented; they do not mean every listed file should be ported.

## Reader/content candidates

{render_list(groups["reader_content_candidates"])}

## Storage/state candidates

{render_list(groups["storage_state_candidates"])}

## X4-specific hardware/display/input assumption candidates

{render_list(groups["x4_hardware_assumption_candidates"])}

## Existing Rustmix-Wave/Waveshare candidates

{render_list(groups["waveshare_target_candidates"])}

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
pub trait ReaderDisplaySurface {{
    fn logical_width(&self) -> u32;   // 480 for portrait
    fn logical_height(&self) -> u32;  // 800 for portrait
    fn clear(&mut self);
    fn fill_rect(&mut self, x: u32, y: u32, w: u32, h: u32, black: bool);
    fn draw_mono_bitmap(&mut self, x: u32, y: u32, w: u32, h: u32, data: &[u8]);
    fn flush(&mut self) -> anyhow::Result<()>;
}}
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
pub trait ReaderStorage {{
    fn list_books(&mut self) -> anyhow::Result<BookList>;
    fn read_file_chunk(&mut self, path: &str, offset: usize, buf: &mut [u8]) -> anyhow::Result<usize>;
    fn read_state_file(&mut self, path: &str, buf: &mut [u8]) -> anyhow::Result<usize>;
    fn write_state_file(&mut self, path: &str, data: &[u8]) -> anyhow::Result<()>;
}}
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
"""

    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(doc)

    print("rustmix-wave-reader-port-recon-v0-audit=ok")
    print(f"wrote {OUT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
