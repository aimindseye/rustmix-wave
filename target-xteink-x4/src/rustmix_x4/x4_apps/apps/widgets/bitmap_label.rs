use core::convert::Infallible;

use embedded_graphics::{pixelcolor::BinaryColor, prelude::*, primitives::PrimitiveStyle};

use crate::rustmix_x4::text::static_font_assets;
use crate::rustmix_x4::x4_apps::fonts::bitmap::BitmapFont;
use crate::rustmix_x4::x4_apps::ui::{Alignment, Region};
use crate::rustmix_x4::x4_kernel::drivers::strip::StripBuffer;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BitmapTextWeight {
    Regular,
    Medium,
    SemiBold,
}

const MEDIUM_EXTRA_PASSES: &[(i32, i32)] = &[(1, 0)];
const SEMIBOLD_EXTRA_PASSES: &[(i32, i32)] = &[(1, 0), (0, 1)];

impl BitmapTextWeight {
    #[inline]
    fn extra_passes(self) -> &'static [(i32, i32)] {
        match self {
            Self::Regular => &[],
            // E-ink tuned Inter Medium: preserve source metrics and add one
            // horizontal pass so Regular glyphs do not look washed out on X4.
            Self::Medium => MEDIUM_EXTRA_PASSES,
            // E-ink tuned Inter SemiBold: add horizontal plus light vertical
            // reinforcement for titles, selected tabs, selected cards, and pills.
            Self::SemiBold => SEMIBOLD_EXTRA_PASSES,
        }
    }

    #[inline]
    const fn right_padding(self) -> u32 {
        match self {
            Self::Regular => 0,
            Self::Medium | Self::SemiBold => 1,
        }
    }
}

pub struct BitmapLabel<'a> {
    region: Region,
    text: &'a str,
    font: &'static BitmapFont,
    alignment: Alignment,
    inverted: bool,
    weight: BitmapTextWeight,
}

impl<'a> BitmapLabel<'a> {
    pub fn new(region: Region, text: &'a str, font: &'static BitmapFont) -> Self {
        Self {
            region,
            text,
            font,
            alignment: Alignment::CenterLeft,
            inverted: false,
            weight: BitmapTextWeight::Medium,
        }
    }

    pub const fn alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }

    pub const fn inverted(mut self, inverted: bool) -> Self {
        self.inverted = inverted;
        self
    }

    pub const fn weight(mut self, weight: BitmapTextWeight) -> Self {
        self.weight = weight;
        self
    }

    pub const fn regular(self) -> Self {
        self.weight(BitmapTextWeight::Regular)
    }

    pub const fn medium(self) -> Self {
        self.weight(BitmapTextWeight::Medium)
    }

    pub const fn semibold(self) -> Self {
        self.weight(BitmapTextWeight::SemiBold)
    }

    pub fn draw(&self, strip: &mut StripBuffer) -> Result<(), Infallible> {
        draw_bitmap_text(
            strip,
            self.region,
            self.text,
            self.font,
            self.alignment,
            self.inverted,
            self.weight,
        )
    }
}

pub struct BitmapDynLabel<const N: usize> {
    region: Region,
    buffer: [u8; N],
    len: usize,
    font: &'static BitmapFont,
    alignment: Alignment,
    inverted: bool,
    weight: BitmapTextWeight,
}

impl<const N: usize> BitmapDynLabel<N> {
    pub fn new(region: Region, font: &'static BitmapFont) -> Self {
        Self {
            region,
            buffer: [0u8; N],
            len: 0,
            font,
            alignment: Alignment::CenterLeft,
            inverted: false,
            weight: BitmapTextWeight::Medium,
        }
    }

    pub const fn alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }

    pub const fn inverted(mut self, inverted: bool) -> Self {
        self.inverted = inverted;
        self
    }

    pub const fn weight(mut self, weight: BitmapTextWeight) -> Self {
        self.weight = weight;
        self
    }

    pub const fn regular(self) -> Self {
        self.weight(BitmapTextWeight::Regular)
    }

    pub const fn medium(self) -> Self {
        self.weight(BitmapTextWeight::Medium)
    }

    pub const fn semibold(self) -> Self {
        self.weight(BitmapTextWeight::SemiBold)
    }

    pub fn set_text(&mut self, text: &str) {
        let bytes = text.as_bytes();
        let n = bytes.len().min(N);
        self.buffer[..n].copy_from_slice(&bytes[..n]);
        self.len = n;
    }

    pub fn clear_text(&mut self) {
        self.len = 0;
    }

    pub fn text(&self) -> &str {
        core::str::from_utf8(&self.buffer[..self.len]).unwrap_or("")
    }

    pub fn draw(&self, strip: &mut StripBuffer) -> Result<(), Infallible> {
        draw_bitmap_text(
            strip,
            self.region,
            self.text(),
            self.font,
            self.alignment,
            self.inverted,
            self.weight,
        )
    }
}

impl<const N: usize> core::fmt::Write for BitmapDynLabel<N> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let bytes = s.as_bytes();
        let available = N - self.len;
        let n = bytes.len().min(available);
        self.buffer[self.len..self.len + n].copy_from_slice(&bytes[..n]);
        self.len += n;
        Ok(())
    }
}

fn draw_bitmap_text(
    strip: &mut StripBuffer,
    region: Region,
    text: &str,
    font: &'static BitmapFont,
    alignment: Alignment,
    inverted: bool,
    weight: BitmapTextWeight,
) -> Result<(), Infallible> {
    if !region.intersects(strip.logical_window()) {
        return Ok(());
    }
    if static_font_assets::draw_ui_text(strip, region, text, font, alignment, inverted) {
        return Ok(());
    }

    let (bg, fg) = if inverted {
        (BinaryColor::On, BinaryColor::Off)
    } else {
        (BinaryColor::Off, BinaryColor::On)
    };

    region
        .to_rect()
        .into_styled(PrimitiveStyle::with_fill(bg))
        .draw(strip)?;

    draw_weighted_aligned(strip, region, text, font, alignment, fg, weight);

    Ok(())
}

fn draw_weighted_aligned(
    strip: &mut StripBuffer,
    region: Region,
    text: &str,
    font: &'static BitmapFont,
    alignment: Alignment,
    fg: BinaryColor,
    weight: BitmapTextWeight,
) {
    if text.is_empty() {
        return;
    }

    let text_w = (font.measure_str(text) as u32).saturating_add(weight.right_padding());
    let text_h = font.line_height as u32;
    let top_left = alignment.position(region, Size::new(text_w, text_h));
    let baseline = top_left.y + font.ascent as i32;
    draw_weighted_str(strip, text, font, fg, top_left.x, baseline, weight);
}

fn draw_weighted_str(
    strip: &mut StripBuffer,
    text: &str,
    font: &'static BitmapFont,
    fg: BinaryColor,
    cx: i32,
    baseline: i32,
    weight: BitmapTextWeight,
) -> i32 {
    let mut x = cx;
    for ch in text.chars() {
        let advance = font.draw_char_fg(strip, ch, fg, x, baseline) as i32;
        for (dx, dy) in weight.extra_passes() {
            let _ = font.draw_char_fg(strip, ch, fg, x + dx, baseline + dy);
        }
        x += advance;
    }
    x
}
