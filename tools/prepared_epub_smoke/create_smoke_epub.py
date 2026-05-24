#!/usr/bin/env python3
"""
Create a tiny mixed English + Devanagari EPUB for prepared EPUB smoke testing.
"""

from __future__ import annotations

import argparse
import zipfile
from pathlib import Path


MIMETYPE = "application/epub+zip"

CONTAINER = """<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>
"""

OPF = """<?xml version="1.0" encoding="UTF-8"?>
<package version="3.0" unique-identifier="bookid" xmlns="http://www.idpf.org/2007/opf">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:identifier id="bookid">rustmix-prepared-epub-smoke</dc:identifier>
    <dc:title>Prepared EPUB Smoke</dc:title>
    <dc:language>en</dc:language>
  </metadata>
  <manifest>
    <item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav"/>
    <item id="chap1" href="chap1.xhtml" media-type="application/xhtml+xml"/>
  </manifest>
  <spine>
    <itemref idref="chap1"/>
  </spine>
</package>
"""

NAV = """<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
  <head><title>Navigation</title></head>
  <body>
    <nav epub:type="toc" xmlns:epub="http://www.idpf.org/2007/ops">
      <ol><li><a href="chap1.xhtml">Chapter 1</a></li></ol>
    </nav>
  </body>
</html>
"""

CHAPTER = """<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
  <head><title>Prepared EPUB Smoke</title></head>
  <body>
    <h1>Prepared EPUB Smoke</h1>
    <p>Rustmix prepared EPUB rendering smoke test.</p>
    <p>Hindi: नमस्ते दुनिया</p>
    <p>Sanskrit: धर्मक्षेत्रे कुरुक्षेत्रे</p>
    <p>Mixed: Rustmix नमस्ते Rustmix</p>
  </body>
</html>
"""


def run() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("output", type=Path)
    args = parser.parse_args()

    args.output.parent.mkdir(parents=True, exist_ok=True)

    with zipfile.ZipFile(args.output, "w") as zf:
        zf.writestr("mimetype", MIMETYPE, compress_type=zipfile.ZIP_STORED)
        zf.writestr("META-INF/container.xml", CONTAINER)
        zf.writestr("OEBPS/content.opf", OPF)
        zf.writestr("OEBPS/nav.xhtml", NAV)
        zf.writestr("OEBPS/chap1.xhtml", CHAPTER)

    print(args.output)
    return 0


if __name__ == "__main__":
    raise SystemExit(run())
