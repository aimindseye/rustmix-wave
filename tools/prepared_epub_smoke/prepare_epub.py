#!/usr/bin/env python3
"""
Prepare an EPUB into Rustmix's existing /FCACHE/<BOOKID>/ prepared glyph cache.

This is intentionally host-side:
- extracts ordered EPUB spine text
- writes a temporary UTF-8 TXT file
- invokes tools/prepared_txt_real_vfnt to emit VFNT/VRUN cache files

The X4 still renders only prepared glyph runs; it does not shape EPUB text on-device.
"""

from __future__ import annotations

import argparse
import html
import re
import shutil
import subprocess
import sys
import tempfile
import zipfile
from html.parser import HTMLParser
from pathlib import Path
from xml.etree import ElementTree as ET


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


def ns_tag(name: str) -> str:
    return name.split("}", 1)[-1]


def find_container_opf(zf: zipfile.ZipFile) -> str:
    data = zf.read("META-INF/container.xml")
    root = ET.fromstring(data)
    for elem in root.iter():
        if ns_tag(elem.tag) == "rootfile":
            path = elem.attrib.get("full-path")
            if path:
                return path
    raise ValueError("container.xml has no rootfile full-path")


def posix_dirname(path: str) -> str:
    if "/" not in path:
        return ""
    return path.rsplit("/", 1)[0]


def resolve(base: str, href: str) -> str:
    base_dir = posix_dirname(base)
    if not base_dir:
        return href
    return f"{base_dir}/{href}".replace("//", "/")


def extract_spine_text(epub: Path) -> tuple[str, str]:
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
                    manifest[item_id] = (resolve(opf_path, href), media_type)

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


def run() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--epub", required=True, type=Path)
    parser.add_argument("--device-path", required=True)
    parser.add_argument("--latin-font", required=True, type=Path)
    parser.add_argument("--devanagari-font", required=True, type=Path)
    parser.add_argument("--out", required=True, type=Path, help="Usually the SD card FCACHE directory")
    parser.add_argument("--title")
    parser.add_argument("--latin-size", default="18")
    parser.add_argument("--devanagari-size", default="22")
    parser.add_argument("--line-height")
    parser.add_argument("--page-width", default="464")
    parser.add_argument("--page-height", default="730")
    parser.add_argument("--margin-x", default="0")
    parser.add_argument("--margin-y", default="4")
    parser.add_argument("--keep-work", action="store_true")
    args = parser.parse_args()

    repo = Path(__file__).resolve().parents[2]
    manifest = repo / "tools/prepared_txt_real_vfnt/Cargo.toml"
    if not manifest.exists():
        print(f"missing prepared TXT tool: {manifest}", file=sys.stderr)
        return 2

    text, detected_title = extract_spine_text(args.epub)
    title = args.title or detected_title

    work_root = Path(tempfile.mkdtemp(prefix="rustmix-prepared-epub-"))
    txt_path = work_root / f"{args.epub.stem}.txt"
    txt_path.write_text(text, encoding="utf-8")

    cmd = [
        "cargo",
        "run",
        "--manifest-path",
        str(manifest),
        "--release",
        "--",
        "--book",
        str(txt_path),
        "--device-path",
        args.device_path,
        "--latin-font",
        str(args.latin_font),
        "--devanagari-font",
        str(args.devanagari_font),
        "--out",
        str(args.out),
        "--title",
        title,
        "--latin-size",
        str(args.latin_size),
        "--devanagari-size",
        str(args.devanagari_size),
        "--page-width",
        str(args.page_width),
        "--page-height",
        str(args.page_height),
        "--margin-x",
        str(args.margin_x),
        "--margin-y",
        str(args.margin_y),
    ]

    if args.line_height:
        cmd.extend(["--line-height", str(args.line_height)])

    print(f"extracted_text={txt_path}")
    print("running prepared TXT VFNT/VRUN generator...")
    result = subprocess.run(cmd, cwd=repo)

    if args.keep_work:
        print(f"work_dir={work_root}")
    else:
        shutil.rmtree(work_root, ignore_errors=True)

    return result.returncode


if __name__ == "__main__":
    raise SystemExit(run())
