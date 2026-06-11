#!/usr/bin/env python3
"""Audit EPUB structure against the Rustmix Wave bounded large-archive Reader policy."""
from __future__ import annotations

import argparse
import html
import re
import sys
import xml.etree.ElementTree as ET
import zipfile
from pathlib import Path

ARCHIVE_MAX = 64 * 1024 * 1024
CENTRAL_DIRECTORY_MAX = 2 * 1024 * 1024
ENTRY_MAX = 4096
MANIFEST_MAX = 4096
SPINE_MAX = 4096
REFLOW_TEXT_MAX = 7 * 1024 * 1024


def local(tag: str) -> str:
    return tag.rsplit('}', 1)[-1]


def flatten_xhtml(raw: bytes) -> str:
    text = raw.decode('utf-8', errors='replace')
    text = re.sub(r'<[^>]+>', ' ', text)
    text = html.unescape(text)
    return ' '.join(text.split())


def audit(path: Path) -> list[str]:
    errors: list[str] = []
    size = path.stat().st_size
    if size > ARCHIVE_MAX:
        errors.append(f'archive bytes {size} exceed {ARCHIVE_MAX}')
    with zipfile.ZipFile(path) as archive:
        infos = archive.infolist()
        if len(infos) > ENTRY_MAX:
            errors.append(f'ZIP entries {len(infos)} exceed {ENTRY_MAX}')
        central_size = sum(46 + len(info.filename.encode('utf-8')) + len(info.extra) + len(info.comment) for info in infos)
        if central_size > CENTRAL_DIRECTORY_MAX:
            errors.append(f'central-directory bytes {central_size} exceed {CENTRAL_DIRECTORY_MAX}')
        try:
            container = ET.fromstring(archive.read('META-INF/container.xml'))
            package_path = next(element.attrib['full-path'] for element in container.iter() if local(element.tag) == 'rootfile')
            package = ET.fromstring(archive.read(package_path))
        except Exception as exc:  # noqa: BLE001 - user-facing audit utility
            errors.append(f'container/OPF parse failed: {exc}')
            return errors
        manifest = {
            element.attrib.get('id', ''): element.attrib
            for element in package.iter()
            if local(element.tag) == 'item' and element.attrib.get('id')
        }
        spine = [
            element.attrib.get('idref', '')
            for element in package.iter()
            if local(element.tag) == 'itemref' and element.attrib.get('idref')
        ]
        if len(manifest) > MANIFEST_MAX:
            errors.append(f'manifest entries {len(manifest)} exceed {MANIFEST_MAX}')
        if len(spine) > SPINE_MAX:
            errors.append(f'spine entries {len(spine)} exceed {SPINE_MAX}')
        package_dir = package_path.rsplit('/', 1)[0] if '/' in package_path else ''
        missing: list[str] = []
        skipped_nav = 0
        flattened_bytes = 0
        for idref in spine:
            item = manifest.get(idref)
            if not item:
                missing.append(idref)
                continue
            properties = item.get('properties', '').split()
            if 'nav' in properties:
                skipped_nav += 1
                continue
            href = item.get('href', '').split('#', 1)[0]
            member = f'{package_dir}/{href}' if package_dir else href
            try:
                flattened_bytes += len(flatten_xhtml(archive.read(member)).encode('utf-8'))
            except KeyError:
                missing.append(idref)
        if flattened_bytes > REFLOW_TEXT_MAX:
            errors.append(f'flattened text bytes {flattened_bytes} exceed {REFLOW_TEXT_MAX}')
        print(
            'indic-epub-fixture-audit '
            f'file={path.name!r} bytes={size} entries={len(infos)} central={central_size} '
            f'manifest={len(manifest)} spine={len(spine)} skipped-nav={skipped_nav} '
            f'skipped-missing={len(missing)} flattened-bytes={flattened_bytes} status={"ok" if not errors else "failed"}'
        )
        if missing:
            print(f'indic-epub-fixture-audit skipped-missing-ids={missing[:8]}')
    return errors


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument('epub', nargs='+', type=Path)
    args = parser.parse_args()
    failed = False
    for path in args.epub:
        errors = audit(path)
        for error in errors:
            print(f'indic-epub-fixture-audit error file={path.name!r} detail={error}', file=sys.stderr)
        failed |= bool(errors)
    if failed:
        return 1
    print('indic-epub-fixture-audit=ok')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
