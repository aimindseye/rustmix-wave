//! Generated build-time static font asset bridge for Rustmix X4.
//!
//! Do not edit by hand. Regenerate with:
//!   python3 tools/build_static_font_assets_from_vfn.py

use core::sync::atomic::{AtomicU8, Ordering};

use crate::rustmix_x4::x4_apps::fonts::bitmap::BitmapFont;
use crate::rustmix_x4::x4_apps::ui::{Alignment, Region};
use crate::rustmix_x4::x4_kernel::drivers::strip::StripBuffer;

pub const STATIC_READER_FONT_MARKER: &str = "reader-fonts=x4-reader-crossink-static-fonts-ok";
pub const STATIC_UI_FONT_MARKER: &str = "ui-fonts=x4-ui-crossink-static-fonts-ok";

static UI_FONT_SOURCE: AtomicU8 = AtomicU8::new(0);

const CHARIS18_VFN: &[u8] =
    include_bytes!("../../../../examples/sd-card/RUSTMIX/FONTS/CHARIS18.VFN");
const BITTER18_VFN: &[u8] =
    include_bytes!("../../../../examples/sd-card/RUSTMIX/FONTS/BITTER18.VFN");
const LEXEND18_VFN: &[u8] =
    include_bytes!("../../../../examples/sd-card/RUSTMIX/FONTS/LEXEND18.VFN");
pub fn set_ui_font_source(source: u8) {
    UI_FONT_SOURCE.store(source.min(2), Ordering::Relaxed);
}

pub fn ui_font_source() -> u8 {
    UI_FONT_SOURCE.load(Ordering::Relaxed)
}

pub fn reader_font_for_source(source: u8) -> Option<&'static [u8]> {
    match source {
        1 => Some(CHARIS18_VFN), // Charis
        2 => Some(BITTER18_VFN), // Bitter
        3 => Some(LEXEND18_VFN), // Lexend Deca
        _ => None,
    }
}

pub fn draw_ui_text(
    _strip: &mut StripBuffer,
    _region: Region,
    _text: &str,
    _fallback: &'static BitmapFont,
    _alignment: Alignment,
    _inverted: bool,
) -> bool {
    // Design option 1: UI chrome uses the compiled Inter bitmap family in
    // x4_apps::fonts. Keep this older static-VFN UI bridge dormant so stale
    // persisted UI font-source values cannot mix VFN metrics into Home,
    // Settings, or future tabbed internal pages.
    false
}
