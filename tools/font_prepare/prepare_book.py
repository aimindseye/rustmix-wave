#!/usr/bin/env python3
"""
Rustmix font preparation workflow.

One tool for:
- prepared TXT cache generation
- prepared EPUB cache generation
- cache validation
- exact SD upload instructions

The generated cache stays compatible with the current X4 Reader path:
/FCACHE/<BOOKID>/
"""

from __future__ import annotations

import argparse
import html
import os
import re
import shutil
import subprocess
import sys
import tempfile
import zipfile
from dataclasses import dataclass
from html.parser import HTMLParser
from pathlib import Path
from xml.etree import ElementTree as ET


DEFAULT_LATIN_FONT_NAMES = [
    "NotoSans-Regular.ttf",
    "NotoSans-Medium.ttf",
    "NotoSans-VariableFont_wdth,wght.ttf",
]

DEFAULT_DEVANAGARI_FONT_NAMES = [
    "NotoSansDevanagari-Regular.ttf",
    "NotoSansDevanagari-Medium.ttf",
    "NotoSansDevanagari-VariableFont_wdth,wght.ttf",
]

REQUIRED_CACHE_FILES = [
    "META.TXT",
    "FONTS.IDX",
    "PAGES.IDX",
]

FIRMWARE_MAX_PAGE_BYTES = 24 * 1024

RECOMMENDED_CACHE_FILES = [
    "LAT18.VFN",
    "DEV22.VFN",
    "P000.VRN",
]


@dataclass
class PreparedResult:
    book_id: str | None
    cache_dir: Path | None
    stdout: str


class TextExtractor(HTMLParser):
    BLOCK_TAGS = {
        "p",
        "div",
        "section",
        "article",
        "br",
        "h1",
        "h2",
        "h3",
        "h4",
        "h5",
        "h6",
        "li",
        "tr",
    }

    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.parts: list[str] = []
        self.skip_depth = 0

    def handle_starttag(self, tag: str, attrs) -> None:
        tag = tag.lower()
        if tag in {"script", "style", "svg"}:
            self.skip_depth += 1
        elif tag in self.BLOCK_TAGS:
            self._newline()

    def handle_endtag(self, tag: str) -> None:
        tag = tag.lower()
        if tag in {"script", "style", "svg"} and self.skip_depth:
            self.skip_depth -= 1
        elif tag in self.BLOCK_TAGS:
            self._newline()

    def handle_data(self, data: str) -> None:
        if self.skip_depth:
            return
        data = re.sub(r"\s+", " ", data)
        if data.strip():
            self.parts.append(data)

    def _newline(self) -> None:
        if self.parts and not self.parts[-1].endswith("\n"):
            self.parts.append("\n")

    def text(self) -> str:
        raw = "".join(self.parts)
        raw = html.unescape(raw)
        raw = re.sub(r"[ \t]+\n", "\n", raw)
        raw = re.sub(r"\n{3,}", "\n\n", raw)
        return raw.strip()


def repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def ns_tag(name: str) -> str:
    return name.split("}", 1)[-1]


def posix_dirname(path: str) -> str:
    if "/" not in path:
        return ""
    return path.rsplit("/", 1)[0]


def resolve_opf_path(base: str, href: str) -> str:
    base_dir = posix_dirname(base)
    if not base_dir:
        return href
    return f"{base_dir}/{href}".replace("//", "/")


def find_container_opf(zf: zipfile.ZipFile) -> str:
    data = zf.read("META-INF/container.xml")
    root = ET.fromstring(data)
    for elem in root.iter():
        if ns_tag(elem.tag) == "rootfile":
            path = elem.attrib.get("full-path")
            if path:
                return path
    raise ValueError("container.xml has no rootfile full-path")


def extract_epub_text(epub: Path) -> tuple[str, str]:
    with zipfile.ZipFile(epub) as zf:
        opf_path = find_container_opf(zf)
        opf_root = ET.fromstring(zf.read(opf_path))

        title = ""
        for elem in opf_root.iter():
            if ns_tag(elem.tag) == "title" and elem.text and elem.text.strip():
                title = elem.text.strip()
                break

        manifest: dict[str, tuple[str, str]] = {}
        for elem in opf_root.iter():
            if ns_tag(elem.tag) == "item":
                item_id = elem.attrib.get("id")
                href = elem.attrib.get("href")
                media_type = elem.attrib.get("media-type", "")
                if item_id and href:
                    manifest[item_id] = (resolve_opf_path(opf_path, href), media_type)

        spine_ids: list[str] = []
        for elem in opf_root.iter():
            if ns_tag(elem.tag) == "itemref":
                item_id = elem.attrib.get("idref")
                if item_id:
                    spine_ids.append(item_id)

        chunks: list[str] = []
        for item_id in spine_ids:
            entry = manifest.get(item_id)
            if not entry:
                continue

            href, media_type = entry
            if not (
                "html" in media_type
                or href.lower().endswith((".xhtml", ".html", ".htm"))
            ):
                continue

            try:
                data = zf.read(href)
            except KeyError:
                continue

            parser = TextExtractor()
            parser.feed(data.decode("utf-8", errors="replace"))
            text = parser.text()
            if text:
                chunks.append(text)

    if not chunks:
        raise ValueError("EPUB spine produced no extractable text")

    return "\n\n".join(chunks) + "\n", title or epub.stem


def find_font(fonts_dir: Path | None, explicit: Path | None, names: list[str], label: str) -> Path:
    if explicit:
        if explicit.exists():
            return explicit
        raise FileNotFoundError(f"{label} font does not exist: {explicit}")

    if not fonts_dir:
        raise FileNotFoundError(
            f"{label} font not provided. Use --{label}-font or --fonts-dir."
        )

    candidates: list[Path] = []
    for name in names:
        candidates.extend(fonts_dir.rglob(name))

    if candidates:
        candidates.sort(key=lambda p: (len(str(p)), str(p)))
        return candidates[0]

    expected = ", ".join(names)
    raise FileNotFoundError(
        f"Could not find {label} font under {fonts_dir}. Expected one of: {expected}"
    )


def prepared_txt_manifest() -> Path:
    manifest = repo_root() / "tools/prepared_txt_real_vfnt/Cargo.toml"
    if not manifest.exists():
        raise FileNotFoundError(f"missing prepared TXT generator: {manifest}")
    return manifest


def clean_generated_artifacts() -> None:
    paths = [
        repo_root() / "tools/prepared_txt_real_vfnt/target",
        repo_root() / "tools/prepared_epub_smoke/__pycache__",
        repo_root() / "tools/font_prepare/__pycache__",
    ]

    for path in paths:
        if path.exists():
            print(f"removing generated artifact: {path}")
            shutil.rmtree(path, ignore_errors=True)


def run_prepared_txt_generator(
    book: Path,
    device_path: str,
    latin_font: Path,
    devanagari_font: Path,
    out: Path,
    title: str | None,
    latin_size: int,
    devanagari_size: int,
    line_height: int | None,
    page_width: int,
    page_height: int,
    margin_x: int,
    margin_y: int,
) -> PreparedResult:
    cmd = [
        "cargo",
        "run",
        "--manifest-path",
        str(prepared_txt_manifest()),
        "--release",
        "--",
        "--book",
        str(book),
        "--device-path",
        device_path,
        "--latin-font",
        str(latin_font),
        "--devanagari-font",
        str(devanagari_font),
        "--out",
        str(out),
        "--latin-size",
        str(latin_size),
        "--devanagari-size",
        str(devanagari_size),
        "--page-width",
        str(page_width),
        "--page-height",
        str(page_height),
        "--margin-x",
        str(margin_x),
        "--margin-y",
        str(margin_y),
    ]

    if title:
        cmd.extend(["--title", title])

    if line_height:
        cmd.extend(["--line-height", str(line_height)])

    print("running prepared font generator...")
    print(" ".join(shell_quote(part) for part in cmd))

    completed = subprocess.run(
        cmd,
        cwd=repo_root(),
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    )

    print(completed.stdout, end="")

    if completed.returncode != 0:
        raise RuntimeError(f"prepared font generator failed with code {completed.returncode}")

    book_id = parse_line_value(completed.stdout, "book_id")
    cache_path_raw = parse_line_value(completed.stdout, "prepared cache")

    cache_dir = Path(cache_path_raw) if cache_path_raw else None
    return PreparedResult(book_id=book_id, cache_dir=cache_dir, stdout=completed.stdout)


def parse_line_value(text: str, key: str) -> str | None:
    prefix = f"{key}="
    alt_prefix = f"{key}:"

    for line in text.splitlines():
        stripped = line.strip()
        if stripped.startswith(prefix):
            return stripped[len(prefix):].strip()
        if stripped.startswith(alt_prefix):
            return stripped[len(alt_prefix):].strip()

    return None


def shell_quote(value: str) -> str:
    if not value:
        return "''"
    if re.fullmatch(r"[A-Za-z0-9_./:=,+@%-]+", value):
        return value
    return "'" + value.replace("'", "'\"'\"'") + "'"


def print_upload_instructions(cache_dir: Path | None, book_id: str | None) -> None:
    print()
    print("=== SD upload instructions ===")

    if not cache_dir:
        print("Cache directory was not detected from generator output.")
        print("Inspect the generator output above and upload the generated /FCACHE/<BOOKID> directory.")
        return

    book_id = book_id or cache_dir.name

    print(f"Local cache directory:")
    print(f"  {cache_dir}")
    print()
    print("Browser SD Manager target:")
    print(f"  /FCACHE/{book_id}/")
    print()
    print("Using the X4 browser SD Manager:")
    print("  1. Open Wi-Fi Transfer on X4.")
    print("  2. Open http://x4.local/ or the shown IP address.")
    print("  3. At SD root, create/open: FCACHE")
    print(f"  4. Inside FCACHE, create/open: {book_id}")
    print(f"  5. Upload every file from: {cache_dir}")
    print()
    print("Files to upload:")
    if cache_dir.exists():
        for path in sorted(cache_dir.iterdir()):
            if path.is_file():
                print(f"  {path.name}")
    else:
        print("  cache directory does not exist on this host")

    print()
    print("Optional direct SD copy if the SD card is mounted on your host:")
    print(f"  mkdir -p /Volumes/<SDCARD>/FCACHE/{book_id}")
    print(f"  cp {shell_quote(str(cache_dir))}/* /Volumes/<SDCARD>/FCACHE/{book_id}/")


def validate_cache(cache_dir: Path, device_path: str | None) -> int:
    print(f"validating cache: {cache_dir}")

    if not cache_dir.exists():
        print("ERROR: cache directory does not exist")
        return 1

    if not cache_dir.is_dir():
        print("ERROR: cache path is not a directory")
        return 1

    failed = False

    for name in REQUIRED_CACHE_FILES:
        path = cache_dir / name
        if not path.exists():
            print(f"ERROR: missing required file: {name}")
            failed = True
        elif path.stat().st_size == 0:
            print(f"ERROR: required file is empty: {name}")
            failed = True
        else:
            print(f"OK: {name} ({path.stat().st_size} bytes)")

    for name in RECOMMENDED_CACHE_FILES:
        path = cache_dir / name
        if not path.exists():
            print(f"WARN: missing expected smoke file: {name}")
        elif path.stat().st_size == 0:
            print(f"ERROR: file is empty: {name}")
            failed = True
        else:
            print(f"OK: {name} ({path.stat().st_size} bytes)")

    meta = cache_dir / "META.TXT"
    source = None
    if meta.exists():
        for line in meta.read_text(encoding="utf-8", errors="replace").splitlines():
            if line.startswith("source="):
                source = line.split("=", 1)[1].strip()
                break

    if source:
        print(f"OK: META source={source}")
    else:
        print("WARN: META.TXT source= line not found")

    if device_path:
        if source and not source_matches(source, device_path):
            print(f"ERROR: META source does not match device path")
            print(f"  source={source}")
            print(f"  device_path={device_path}")
            failed = True
        else:
            print(f"OK: device path matches: {device_path}")

    fonts_idx = cache_dir / "FONTS.IDX"
    if fonts_idx.exists():
        fonts_text = fonts_idx.read_text(encoding="utf-8", errors="replace")
        if "LAT18.VFN" not in fonts_text:
            print("WARN: FONTS.IDX does not mention LAT18.VFN")
        if "DEV22.VFN" not in fonts_text:
            print("WARN: FONTS.IDX does not mention DEV22.VFN")

    pages_idx = cache_dir / "PAGES.IDX"
    if pages_idx.exists():
        pages_text = pages_idx.read_text(encoding="utf-8", errors="replace")
        page_refs = re.findall(r"P[0-9]{3}\.VRN", pages_text)
        if not page_refs:
            print("WARN: PAGES.IDX does not reference Pxxx.VRN pages")
        else:
            print(f"OK: pages referenced: {', '.join(page_refs[:8])}")

        largest_page_size = 0
        largest_page_name = ""
        for page_name in page_refs:
            page_path = cache_dir / page_name
            if page_path.exists():
                size = page_path.stat().st_size
                if size > largest_page_size:
                    largest_page_size = size
                    largest_page_name = page_name

        if largest_page_size:
            if largest_page_size > FIRMWARE_MAX_PAGE_BYTES:
                print(
                    f"ERROR: largest page {largest_page_name} is {largest_page_size} bytes; "
                    f"firmware limit is {FIRMWARE_MAX_PAGE_BYTES} bytes"
                )
                failed = True
            else:
                print(
                    f"OK: largest page {largest_page_name} is {largest_page_size} bytes "
                    f"<= firmware limit {FIRMWARE_MAX_PAGE_BYTES}"
                )

    if failed:
        print()
        print("cache validation FAILED")
        return 1

    print()
    print("cache validation OK")
    return 0


def source_matches(cache_source: str, device_path: str) -> bool:
    cache_source = cache_source.strip().lstrip("/\\")
    device_path = device_path.strip().lstrip("/\\")
    return (
        cache_source.lower() == device_path.lower()
        or Path(cache_source).name.lower() == Path(device_path).name.lower()
    )


def add_common_prepare_args(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("--device-path", required=True, help="Exact filename/path as seen by X4")
    parser.add_argument("--fonts-dir", type=Path, help="Directory searched recursively for Noto fonts")
    parser.add_argument("--latin-font", type=Path, help="Path to NotoSans-Regular.ttf")
    parser.add_argument("--devanagari-font", type=Path, help="Path to NotoSansDevanagari-Regular.ttf")
    parser.add_argument("--out", required=True, type=Path, help="Output FCACHE directory, e.g. /tmp/FCACHE")
    parser.add_argument("--title")
    parser.add_argument("--latin-size", type=int, default=18)
    parser.add_argument("--devanagari-size", type=int, default=22)
    parser.add_argument("--line-height", type=int)
    parser.add_argument("--page-width", type=int, default=464)
    parser.add_argument("--page-height", type=int, default=730)
    parser.add_argument("--margin-x", type=int, default=0)
    parser.add_argument("--margin-y", type=int, default=4)
    parser.add_argument("--clean", action="store_true", help="Remove generated local target/cache artifacts first")


def command_txt(args: argparse.Namespace) -> int:
    if args.clean:
        clean_generated_artifacts()

    latin_font = find_font(args.fonts_dir, args.latin_font, DEFAULT_LATIN_FONT_NAMES, "latin")
    devanagari_font = find_font(
        args.fonts_dir,
        args.devanagari_font,
        DEFAULT_DEVANAGARI_FONT_NAMES,
        "devanagari",
    )

    result = run_prepared_txt_generator(
        book=args.book,
        device_path=args.device_path,
        latin_font=latin_font,
        devanagari_font=devanagari_font,
        out=args.out,
        title=args.title,
        latin_size=args.latin_size,
        devanagari_size=args.devanagari_size,
        line_height=args.line_height,
        page_width=args.page_width,
        page_height=args.page_height,
        margin_x=args.margin_x,
        margin_y=args.margin_y,
    )

    print_upload_instructions(result.cache_dir, result.book_id)

    if result.cache_dir:
        return validate_cache(result.cache_dir, args.device_path)

    return 0


def command_epub(args: argparse.Namespace) -> int:
    if args.clean:
        clean_generated_artifacts()

    latin_font = find_font(args.fonts_dir, args.latin_font, DEFAULT_LATIN_FONT_NAMES, "latin")
    devanagari_font = find_font(
        args.fonts_dir,
        args.devanagari_font,
        DEFAULT_DEVANAGARI_FONT_NAMES,
        "devanagari",
    )

    text, detected_title = extract_epub_text(args.book)
    title = args.title or detected_title

    work_dir = Path(tempfile.mkdtemp(prefix="rustmix-font-epub-"))
    txt_path = work_dir / f"{args.book.stem}.txt"
    txt_path.write_text(text, encoding="utf-8")

    print(f"extracted_text={txt_path}")

    try:
        result = run_prepared_txt_generator(
            book=txt_path,
            device_path=args.device_path,
            latin_font=latin_font,
            devanagari_font=devanagari_font,
            out=args.out,
            title=title,
            latin_size=args.latin_size,
            devanagari_size=args.devanagari_size,
            line_height=args.line_height,
            page_width=args.page_width,
            page_height=args.page_height,
            margin_x=args.margin_x,
            margin_y=args.margin_y,
        )

        print_upload_instructions(result.cache_dir, result.book_id)

        rc = 0
        if result.cache_dir:
            rc = validate_cache(result.cache_dir, args.device_path)

        if args.keep_work:
            print(f"work_dir={work_dir}")
        else:
            shutil.rmtree(work_dir, ignore_errors=True)

        return rc
    except Exception:
        if args.keep_work:
            print(f"work_dir={work_dir}")
        else:
            shutil.rmtree(work_dir, ignore_errors=True)
        raise


def command_validate(args: argparse.Namespace) -> int:
    return validate_cache(args.cache, args.device_path)


def command_clean(args: argparse.Namespace) -> int:
    clean_generated_artifacts()
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Prepare Rustmix TXT/EPUB font caches for X4 Reader"
    )
    sub = parser.add_subparsers(dest="command", required=True)

    txt = sub.add_parser("txt", help="Prepare a TXT book")
    txt.add_argument("--book", required=True, type=Path)
    add_common_prepare_args(txt)
    txt.set_defaults(func=command_txt)

    epub = sub.add_parser("epub", help="Prepare an EPUB book")
    epub.add_argument("--book", required=True, type=Path)
    epub.add_argument("--keep-work", action="store_true")
    add_common_prepare_args(epub)
    epub.set_defaults(func=command_epub)

    validate = sub.add_parser("validate", help="Validate a generated /FCACHE/<BOOKID> directory")
    validate.add_argument("--cache", required=True, type=Path)
    validate.add_argument("--device-path")
    validate.set_defaults(func=command_validate)

    clean = sub.add_parser("clean", help="Remove generated tool target/cache artifacts")
    clean.set_defaults(func=command_clean)

    return parser


def main() -> int:
    parser = build_parser()
    args = parser.parse_args()

    try:
        return args.func(args)
    except Exception as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
