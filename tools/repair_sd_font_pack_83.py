#!/usr/bin/env python3
"""Repair an existing /RUSTMIX/FONTS folder to X4 8.3-safe physical names.

This renames .VFNT files to .VFN, renames UI-MANIFEST.TXT to UIFONTS.TXT,
and updates MANIFEST.TXT / UIFONTS.TXT references. The internal file magic
remains VFNT; only the SD filename changes to fit FAT 8.3 constraints.
"""
from __future__ import annotations
import argparse
import re
import shutil
from pathlib import Path


def is_83(name: str) -> bool:
    if "." not in name:
        return bool(re.fullmatch(r"[A-Z0-9_]{1,8}", name))
    base, ext = name.rsplit(".", 1)
    return bool(re.fullmatch(r"[A-Z0-9_]{1,8}", base)) and bool(re.fullmatch(r"[A-Z0-9_]{1,3}", ext))


def rewrite_text(path: Path) -> None:
    if path.exists() and path.is_file():
        text = path.read_text(encoding="utf-8")
        text = text.replace("UI-MANIFEST.TXT", "UIFONTS.TXT")
        text = text.replace(".VFNT", ".VFN")
        path.write_text(text, encoding="utf-8")


def repair(font_dir: Path, dry_run: bool = False) -> int:
    if not font_dir.exists():
        raise SystemExit(f"missing font folder: {font_dir}")

    actions: list[str] = []
    for path in sorted(font_dir.iterdir()):
        if path.name == ".DS_Store" or path.name.startswith("._"):
            actions.append(f"delete {path.name}")
            if not dry_run:
                path.unlink(missing_ok=True)
        elif path.is_dir() and path.name == "__MACOSX":
            actions.append(f"delete {path.name}/")
            if not dry_run:
                shutil.rmtree(path)
        elif path.is_file() and path.suffix.upper() == ".VFNT":
            target = path.with_suffix(".VFN")
            actions.append(f"rename {path.name} -> {target.name}")
            if not dry_run:
                if target.exists():
                    path.unlink()
                else:
                    path.rename(target)

    old_ui = font_dir / "UI-MANIFEST.TXT"
    new_ui = font_dir / "UIFONTS.TXT"
    if old_ui.exists():
        actions.append("rename UI-MANIFEST.TXT -> UIFONTS.TXT")
        if not dry_run:
            if new_ui.exists():
                old_ui.unlink()
            else:
                old_ui.rename(new_ui)

    if not dry_run:
        for name in ("MANIFEST.TXT", "UIFONTS.TXT", "README.TXT"):
            rewrite_text(font_dir / name)

    bad = [p.name for p in font_dir.iterdir() if not is_83(p.name)]
    for action in actions:
        print(action)
    if bad:
        print("non-8.3 names remain:")
        for name in bad:
            print(name)
        return 1
    print(f"font folder is 8.3-safe: {font_dir}")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("font_dir", type=Path, help="Path to /RUSTMIX/FONTS")
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()
    return repair(args.font_dir, args.dry_run)


if __name__ == "__main__":
    raise SystemExit(main())
