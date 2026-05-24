#!/usr/bin/env python3
"""Generate firmware-static Rustmix font mappings from local .VFN files.

This tool does not ship font binaries. Run it locally after generating a VFN
font pack from fonts you are allowed to use. It rewrites
`target-xteink-x4/src/rustmix_x4/text/static_font_assets.rs` so Reader/UI font
choices are compiled into firmware instead of loaded from SD during rendering.
"""
from __future__ import annotations

import argparse
import os
from pathlib import Path
import textwrap

DEFAULT_FONTS_DIR = Path("examples/sd-card/RUSTMIX/FONTS")
DEFAULT_OUT = Path("target-xteink-x4/src/rustmix_x4/text/static_font_assets.rs")

READER_MAP = [
    (1, "Charis", "CHARIS18.VFN"),
    (2, "Bitter", "BITTER18.VFN"),
    (3, "Lexend Deca", "LEXEND18.VFN"),
]
UI_MAP = [
    (1, "Inter", "INTER14.VFN"),
    (2, "Lexend Deca UI", "LEXUI14.VFN"),
]

HEADER = b"VFNT"
MAX_BYTES = 64 * 1024


def rust_ident(name: str) -> str:
    return name.replace(".", "_").replace("-", "_").upper()


def validate_vfn(path: Path) -> None:
    data = path.read_bytes()
    if len(data) < 4 or data[:4] != HEADER:
        raise ValueError(f"{path} does not start with VFNT magic")
    if len(data) > MAX_BYTES:
        raise ValueError(f"{path} is too large: {len(data)} bytes > {MAX_BYTES}")


def include_const(name: str, rel: str) -> str:
    ident = rust_ident(name)
    return f'const {ident}: &[u8] = include_bytes!("{rel}");\n'


def rel_from_out(out: Path, file_path: Path) -> str:
    return str(file_path.resolve().relative_to(out.parent.resolve()).as_posix()) if False else str(Path("../../../../../") / file_path)


def relative_include_path(out: Path, file_path: Path) -> str:
    # include_bytes! is relative to the generated source file.
    rel = os.path.relpath(Path.cwd().joinpath(file_path).resolve(), out.parent.resolve())
    return rel.replace(os.sep, "/")


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--fonts-dir", type=Path, default=DEFAULT_FONTS_DIR)
    ap.add_argument("--out", type=Path, default=DEFAULT_OUT)
    ap.add_argument("--allow-missing", action="store_true", help="skip missing optional font files")
    args = ap.parse_args()

    root = Path.cwd()
    out = args.out
    fonts_dir = args.fonts_dir

    consts = []
    reader_arms = []
    ui_arms = []

    for source, display, file_name in READER_MAP:
        path = fonts_dir / file_name
        if not path.exists():
            if args.allow_missing:
                continue
            raise FileNotFoundError(path)
        validate_vfn(path)
        ident = rust_ident(file_name)
        rel = relative_include_path(out, path)
        consts.append(include_const(file_name, rel))
        reader_arms.append(f"        {source} => Some({ident}), // {display}\n")

    for source, display, file_name in UI_MAP:
        path = fonts_dir / file_name
        if not path.exists():
            if args.allow_missing:
                continue
            raise FileNotFoundError(path)
        validate_vfn(path)
        ident = rust_ident(file_name)
        rel = relative_include_path(out, path)
        consts.append(include_const(file_name, rel))
        ui_arms.append(f"        {source} => Some({ident}), // {display}\n")

    content = f'''//! Generated build-time static font asset bridge for Rustmix X4.\n//!\n//! Do not edit by hand. Regenerate with:\n//!   python3 tools/build_static_font_assets_from_vfn.py\n\nuse core::sync::atomic::{{AtomicU8, Ordering}};\n\nuse embedded_graphics::{{pixelcolor::BinaryColor, prelude::*, primitives::PrimitiveStyle}};\n\nuse crate::rustmix_x4::text::sd_vfn_runtime;\nuse crate::rustmix_x4::x4_apps::fonts::bitmap::BitmapFont;\nuse crate::rustmix_x4::x4_apps::ui::{{Alignment, Region}};\nuse crate::rustmix_x4::x4_kernel::drivers::strip::StripBuffer;\n\npub const STATIC_READER_FONT_MARKER: &str = "reader-fonts=x4-reader-crossink-static-fonts-ok";\npub const STATIC_UI_FONT_MARKER: &str = "ui-fonts=x4-ui-crossink-static-fonts-ok";\n\nstatic UI_FONT_SOURCE: AtomicU8 = AtomicU8::new(0);\n\n{''.join(consts)}\npub fn set_ui_font_source(source: u8) {{\n    UI_FONT_SOURCE.store(source.min(2), Ordering::Relaxed);\n}}\n\npub fn ui_font_source() -> u8 {{\n    UI_FONT_SOURCE.load(Ordering::Relaxed)\n}}\n\npub fn reader_font_for_source(source: u8) -> Option<&'static [u8]> {{\n    match source {{\n{''.join(reader_arms)}        _ => None,\n    }}\n}}\n\npub fn ui_font_for_source(source: u8) -> Option<&'static [u8]> {{\n    match source {{\n{''.join(ui_arms)}        _ => None,\n    }}\n}}\n\npub fn draw_ui_text(\n    strip: &mut StripBuffer,\n    region: Region,\n    text: &str,\n    _fallback: &'static BitmapFont,\n    alignment: Alignment,\n    inverted: bool,\n) -> bool {{\n    if inverted || !text.is_ascii() || !region.intersects(strip.logical_window()) {{\n        return false;\n    }}\n\n    let Some(data) = ui_font_for_source(ui_font_source()) else {{\n        return false;\n    }};\n    let Some(metrics) = sd_vfn_runtime::metrics(data) else {{\n        return false;\n    }};\n\n    region\n        .to_rect()\n        .into_styled(PrimitiveStyle::with_fill(BinaryColor::Off))\n        .draw(strip)\n        .ok();\n\n    let width = sd_vfn_runtime::measure_str(data, text) as u32;\n    let height = u32::from(metrics.line_height.max(1));\n    let pos = alignment.position(region, embedded_graphics::geometry::Size::new(width, height));\n    let baseline = pos.y + i32::from(metrics.ascent.max(1));\n    sd_vfn_runtime::draw_str(strip, data, text, pos.x, baseline);\n    true\n}}\n'''
    out.write_text(content, encoding="utf-8")
    print(f"wrote {out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
