//! Shared text layout request types.

use super::font_catalog::FontFallbackChain;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LayoutDirection {
    LeftToRight,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LayoutBox {
    pub x: i16,
    pub y: i16,
    pub width: u16,
    pub height: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TextLayoutStyle {
    pub direction: LayoutDirection,
    pub line_height: u16,
    pub paragraph_gap: u16,
    pub max_lines: u8,
}

impl TextLayoutStyle {
    pub const fn sleep_screen() -> Self {
        Self {
            direction: LayoutDirection::LeftToRight,
            line_height: 28,
            paragraph_gap: 10,
            max_lines: 10,
        }
    }

    pub const fn compact_ui() -> Self {
        Self {
            direction: LayoutDirection::LeftToRight,
            line_height: 18,
            paragraph_gap: 4,
            max_lines: 6,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TextBlock<'a> {
    pub text: &'a str,
    pub bounds: LayoutBox,
    pub style: TextLayoutStyle,
}

#[derive(Clone, Copy, Debug)]
pub struct TextLayoutRequest<'a> {
    pub block: TextBlock<'a>,
    pub fonts: FontFallbackChain<'a>,
}
