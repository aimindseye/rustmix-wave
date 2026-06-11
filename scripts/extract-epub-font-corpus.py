#!/usr/bin/env python3
"""Extract a bounded UTF-8 text corpus from EPUB XHTML files for RWF generation."""
from __future__ import annotations

import html
import re
import sys
import zipfile
from pathlib import Path

TAG = re.compile(r"<[^>]+>")
SPACE = re.compile(r"\s+")


def extract(path: Path) -> str:
    pieces: list[str] = []
    with zipfile.ZipFile(path) as archive:
        for name in archive.namelist():
            if not name.lower().endswith((".xhtml", ".html", ".htm")):
                continue
            text = archive.read(name).decode("utf-8", errors="ignore")
            text = html.unescape(TAG.sub(" ", text))
            pieces.append(SPACE.sub(" ", text).strip())
    return "\n".join(piece for piece in pieces if piece)


def main(argv: list[str]) -> int:
    if len(argv) < 2:
        print("usage: extract-epub-font-corpus.py BOOK.epub [BOOK.epub ...]", file=sys.stderr)
        return 2
    for value in argv[1:]:
        print(extract(Path(value)))
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
