#!/usr/bin/env python3
"""Install local font ZIPs into Rustmix firmware-static font source folders.

No font binaries are distributed by the overlay. This script copies fonts from
local ZIPs into target-xteink-x4/assets/fonts-static/* so build.rs can rasterize
firmware-static BitmapFont assets at compile time.
"""
from __future__ import annotations
import argparse, shutil, zipfile, re
from pathlib import Path

FAMILIES = {
    'charis': ['charis'],
    'bitter': ['bitter'],
    'inter': ['inter'],
    'lexend': ['lexend', 'lexenddeca', 'lexend_deca', 'lexend deca'],
}
STYLES = ['Regular', 'Bold', 'Italic', 'BoldItalic']

def norm(s: str) -> str:
    return re.sub(r'[^a-z0-9]+', '', s.lower())

def extract(zips: list[Path], tmp: Path) -> list[Path]:
    tmp.mkdir(parents=True, exist_ok=True)
    files = []
    for zp in zips:
        with zipfile.ZipFile(zp) as z:
            z.extractall(tmp / zp.stem)
    for p in tmp.rglob('*'):
        if p.suffix.lower() in ('.ttf', '.otf'):
            files.append(p)
    return files

def family_for(path: Path) -> str | None:
    n = norm(path.name)
    for fam, keys in FAMILIES.items():
        if any(norm(k) in n for k in keys):
            return fam
    return None

def style_for(path: Path) -> str:
    n = norm(path.name)
    if 'bolditalic' in n or ('bold' in n and 'italic' in n): return 'BoldItalic'
    if 'bold' in n: return 'Bold'
    if 'italic' in n: return 'Italic'
    return 'Regular'

def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument('--charis-zip', type=Path)
    ap.add_argument('--families-zip', type=Path)
    ap.add_argument('--out', type=Path, default=Path('target-xteink-x4/assets/fonts-static'))
    ap.add_argument('--clean', action='store_true')
    args = ap.parse_args()
    zips = [p for p in [args.charis_zip, args.families_zip] if p]
    if not zips: raise SystemExit('provide at least one font zip')
    if args.clean and args.out.exists(): shutil.rmtree(args.out)
    tmp = Path('/tmp/rustmix-static-font-zips')
    if tmp.exists(): shutil.rmtree(tmp)
    fonts = extract(zips, tmp)
    copied = 0
    for src in fonts:
        fam = family_for(src)
        if not fam: continue
        sty = style_for(src)
        dst_dir = args.out / fam
        dst_dir.mkdir(parents=True, exist_ok=True)
        dst = dst_dir / f'{fam.capitalize()}-{sty}{src.suffix.lower()}'
        shutil.copy2(src, dst)
        copied += 1
        print(f'{fam}:{sty} <- {src.name}')
    if copied == 0:
        raise SystemExit('no supported font files found in zips')
    print(f'installed {copied} font files under {args.out}')
    print('Next: cargo clean -p target-xteink-x4 && cargo build -p target-xteink-x4 --release --target riscv32imc-unknown-none-elf --features lua-vm')
    return 0
if __name__ == '__main__':
    raise SystemExit(main())
