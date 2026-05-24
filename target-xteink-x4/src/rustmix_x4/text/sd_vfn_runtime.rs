//! Runtime support for SD-installed Rustmix `.VFN` files.
//!
//! Physical files use 8.3-safe `.VFN` names, but their internal header remains
//! the existing `VFNT` binary contract. Reader fonts are loaded per ReaderApp;
//! UI fonts are loaded into a static buffer from `/RUSTMIX/FONTS/UIFONTS.TXT`
//! only when the Settings UI font source is changed or loaded.

use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use crate::rustmix_x4::text::font_assets::VfntFont;
use crate::rustmix_x4::x4_kernel::drivers::strip::StripBuffer;
use crate::rustmix_x4::x4_kernel::kernel::KernelHandle;

pub const MAX_SD_VFN_BYTES: usize = 16 * 1024;
pub const SD_UI_FONT_MARKER: &str = "ui-fonts=x4-ui-sd-font-runtime-ok";

static UI_FONT_ENABLED: AtomicBool = AtomicBool::new(false);
static UI_FONT_LEN: AtomicUsize = AtomicUsize::new(0);
static mut UI_FONT_BUF: [u8; MAX_SD_VFN_BYTES] = [0; MAX_SD_VFN_BYTES];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SdVfnMetrics {
    pub line_height: u16,
    pub ascent: u16,
}

pub fn vfn_file_name_is_safe(name: &str) -> bool {
    let Some((base, ext)) = name.rsplit_once('.') else {
        return false;
    };
    if !ext.eq_ignore_ascii_case("VFN") && !ext.eq_ignore_ascii_case("VFNT") {
        return false;
    }
    !base.is_empty()
        && base.len() <= 8
        && base
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
}

pub fn vfnt_bytes_valid(data: &[u8]) -> bool {
    VfntFont::parse(data).is_ok()
}

pub fn metrics(data: &[u8]) -> Option<SdVfnMetrics> {
    let font = VfntFont::parse(data).ok()?;
    let header = font.header();
    Some(SdVfnMetrics {
        line_height: header.line_height.max(1),
        ascent: header.ascent.max(1) as u16,
    })
}

pub fn advance_char(data: &[u8], ch: char) -> Option<u8> {
    let font = VfntFont::parse(data).ok()?;
    let glyph = font.glyph(ch as u32).ok()?;
    Some(glyph.metrics.advance_x.max(1).min(255) as u8)
}

pub fn draw_char(
    strip: &mut StripBuffer,
    data: &[u8],
    ch: char,
    cx: i32,
    baseline: i32,
) -> Option<u8> {
    draw_char_color(strip, data, ch, cx, baseline, true)
}

pub fn draw_char_color(
    strip: &mut StripBuffer,
    data: &[u8],
    ch: char,
    cx: i32,
    baseline: i32,
    black: bool,
) -> Option<u8> {
    let font = VfntFont::parse(data).ok()?;
    let glyph = font.glyph(ch as u32).ok()?;
    let width = glyph.metrics.width as usize;
    let height = glyph.metrics.height as usize;
    let stride = glyph.bitmap.row_stride as usize;
    if width > 0 && height > 0 && stride > 0 {
        let gx = cx + i32::from(glyph.metrics.bearing_x);
        let gy = baseline - i32::from(glyph.metrics.bearing_y);
        strip.blit_1bpp(glyph.bitmap_data, 0, width, height, stride, gx, gy, black);
    }
    Some(glyph.metrics.advance_x.max(1).min(255) as u8)
}

pub fn measure_str(data: &[u8], text: &str) -> u16 {
    text.chars()
        .map(|ch| {
            if ch.is_ascii() {
                advance_char(data, ch).unwrap_or(8) as u16
            } else {
                8
            }
        })
        .sum()
}

pub fn draw_str(strip: &mut StripBuffer, data: &[u8], text: &str, cx: i32, baseline: i32) -> i32 {
    draw_str_color(strip, data, text, cx, baseline, true)
}

pub fn draw_str_color(
    strip: &mut StripBuffer,
    data: &[u8],
    text: &str,
    cx: i32,
    baseline: i32,
    black: bool,
) -> i32 {
    let mut x = cx;
    for ch in text.chars() {
        if ch.is_ascii() {
            x += i32::from(draw_char_color(strip, data, ch, x, baseline, black).unwrap_or(8));
        } else {
            x += 8;
        }
    }
    x
}

pub fn disable_ui_font() {
    UI_FONT_ENABLED.store(false, Ordering::Relaxed);
    UI_FONT_LEN.store(0, Ordering::Relaxed);
}

fn ui_font_data<R>(f: impl FnOnce(&[u8]) -> R) -> Option<R> {
    if !UI_FONT_ENABLED.load(Ordering::Relaxed) {
        return None;
    }
    let len = UI_FONT_LEN.load(Ordering::Relaxed);
    if len == 0 || len > MAX_SD_VFN_BYTES {
        return None;
    }
    let data = unsafe { &UI_FONT_BUF[..len] };
    Some(f(data))
}

pub fn load_ui_font_from_sd(_k: &mut KernelHandle<'_>, _source: u8) -> bool {
    // Stack-safe recovery: global UI SD-font rendering is disabled. The previous
    // implementation performed async FAT reads from Settings/Home UI paths and could
    // overflow the ESP32-C3 main stack. Keep UI Font as a persisted setting only.
    disable_ui_font();
    false
}

pub fn load_first_ui_font_from_sd(_k: &mut KernelHandle<'_>) -> bool {
    disable_ui_font();
    false
}

fn font_file_from_manifest_slot(manifest: &str, slot: u8) -> Option<&str> {
    let mut seen = 0u8;
    for raw in manifest.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || !line.starts_with("FONT|") {
            continue;
        }
        let mut parts = line.split('|');
        let _tag = parts.next()?;
        let _id = parts.next()?;
        let _name = parts.next()?;
        let _script = parts.next()?;
        let _style = parts.next()?;
        let _px = parts.next()?;
        let file = parts.next()?.trim();
        if seen == slot && vfn_file_name_is_safe(file) {
            return Some(file);
        }
        seen = seen.saturating_add(1);
    }
    None
}

pub fn draw_ui_text(
    _strip: &mut StripBuffer,
    _region: crate::rustmix_x4::x4_apps::ui::Region,
    _text: &str,
    _fallback: &'static crate::rustmix_x4::x4_apps::fonts::bitmap::BitmapFont,
    _alignment: crate::rustmix_x4::x4_apps::ui::Alignment,
    _inverted: bool,
) -> bool {
    false
}
