// build-time rasterised bitmap fonts for e-ink rendering
// TTFs rasterised by build.rs via fontdue into 1-bit tables in flash
// zero heap, zero parsing at runtime
//
// five size tiers: 0=XSmall  1=Small  2=Medium  3=Large  4=XLarge
//
// Reader font-family selection is CrossInk-style: fonts are firmware-static assets
// generated at build time. OS chrome uses a fixed Inter bitmap family so
// Settings/Home/internal shell layout stays metric-stable across screens.

pub mod bitmap;

#[allow(clippy::all)]
pub mod font_data {
    include!(concat!(env!("OUT_DIR"), "/font_data.rs"));
}

use crate::rustmix_x4::x4_kernel::drivers::strip::StripBuffer;
use bitmap::BitmapFont;

pub const FONT_SIZE_COUNT: usize = 5;

pub const FONT_SIZE_NAMES: &[&str] = &["XSmall", "Small", "Medium", "Large", "XLarge"];

pub const READER_FONT_SOURCE_COUNT: u8 = 4;
pub const READER_FONT_SOURCE_NAMES: &[&str] = &["Bookerly", "Charis", "Bitter", "Lexend"];

pub const UI_FONT_SOURCE_COUNT: u8 = 3;
pub const UI_FONT_SOURCE_NAMES: &[&str] = &["Inter", "Inter", "Inter"];

#[inline]
pub fn reader_font_source_name(idx: u8) -> &'static str {
    READER_FONT_SOURCE_NAMES
        .get(idx as usize)
        .copied()
        .unwrap_or("Bookerly")
}

#[inline]
pub fn ui_font_source_name(idx: u8) -> &'static str {
    UI_FONT_SOURCE_NAMES
        .get(idx as usize)
        .copied()
        .unwrap_or("Inter")
}

#[derive(Clone, Copy)]
pub struct UiFonts {
    pub body: &'static BitmapFont,
    pub heading: &'static BitmapFont,
}

impl UiFonts {
    pub fn for_size(_idx: u8) -> Self {
        Self {
            body: ui_body_font_fixed(),
            heading: ui_heading_font_fixed(),
        }
    }

    pub fn for_source_size(_source: u8, idx: u8) -> Self {
        Self::for_size(idx)
    }
}

#[inline]
pub fn font_size_name(idx: u8) -> &'static str {
    FONT_SIZE_NAMES
        .get(idx as usize)
        .copied()
        .unwrap_or("Small")
}

#[inline]
pub const fn max_size_idx() -> u8 {
    (FONT_SIZE_COUNT - 1) as u8
}

macro_rules! size_match {
    ($idx:expr, $xsmall:path, $small:path, $medium:path, $large:path, $xlarge:path) => {
        match $idx {
            0 => &$xsmall,
            1 => &$small,
            2 => &$medium,
            3 => &$large,
            4 => &$xlarge,
            _ => &$small,
        }
    };
}

pub fn body_font(idx: u8) -> &'static BitmapFont {
    size_match!(
        idx,
        font_data::REGULAR_BODY_XSMALL,
        font_data::REGULAR_BODY_SMALL,
        font_data::REGULAR_BODY_MEDIUM,
        font_data::REGULAR_BODY_LARGE,
        font_data::REGULAR_BODY_XLARGE
    )
}

pub fn heading_font(idx: u8) -> &'static BitmapFont {
    size_match!(
        idx,
        font_data::REGULAR_HEADING_XSMALL,
        font_data::REGULAR_HEADING_SMALL,
        font_data::REGULAR_HEADING_MEDIUM,
        font_data::REGULAR_HEADING_LARGE,
        font_data::REGULAR_HEADING_XLARGE
    )
}

fn charis_body_font(idx: u8) -> &'static BitmapFont {
    size_match!(
        idx,
        font_data::CHARIS_REGULAR_BODY_XSMALL,
        font_data::CHARIS_REGULAR_BODY_SMALL,
        font_data::CHARIS_REGULAR_BODY_MEDIUM,
        font_data::CHARIS_REGULAR_BODY_LARGE,
        font_data::CHARIS_REGULAR_BODY_XLARGE
    )
}
fn charis_heading_font(idx: u8) -> &'static BitmapFont {
    size_match!(
        idx,
        font_data::CHARIS_REGULAR_HEADING_XSMALL,
        font_data::CHARIS_REGULAR_HEADING_SMALL,
        font_data::CHARIS_REGULAR_HEADING_MEDIUM,
        font_data::CHARIS_REGULAR_HEADING_LARGE,
        font_data::CHARIS_REGULAR_HEADING_XLARGE
    )
}
fn bitter_body_font(idx: u8) -> &'static BitmapFont {
    size_match!(
        idx,
        font_data::BITTER_REGULAR_BODY_XSMALL,
        font_data::BITTER_REGULAR_BODY_SMALL,
        font_data::BITTER_REGULAR_BODY_MEDIUM,
        font_data::BITTER_REGULAR_BODY_LARGE,
        font_data::BITTER_REGULAR_BODY_XLARGE
    )
}
fn bitter_heading_font(idx: u8) -> &'static BitmapFont {
    size_match!(
        idx,
        font_data::BITTER_REGULAR_HEADING_XSMALL,
        font_data::BITTER_REGULAR_HEADING_SMALL,
        font_data::BITTER_REGULAR_HEADING_MEDIUM,
        font_data::BITTER_REGULAR_HEADING_LARGE,
        font_data::BITTER_REGULAR_HEADING_XLARGE
    )
}
fn lexend_body_font(idx: u8) -> &'static BitmapFont {
    size_match!(
        idx,
        font_data::LEXEND_REGULAR_BODY_XSMALL,
        font_data::LEXEND_REGULAR_BODY_SMALL,
        font_data::LEXEND_REGULAR_BODY_MEDIUM,
        font_data::LEXEND_REGULAR_BODY_LARGE,
        font_data::LEXEND_REGULAR_BODY_XLARGE
    )
}
fn lexend_heading_font(idx: u8) -> &'static BitmapFont {
    size_match!(
        idx,
        font_data::LEXEND_REGULAR_HEADING_XSMALL,
        font_data::LEXEND_REGULAR_HEADING_SMALL,
        font_data::LEXEND_REGULAR_HEADING_MEDIUM,
        font_data::LEXEND_REGULAR_HEADING_LARGE,
        font_data::LEXEND_REGULAR_HEADING_XLARGE
    )
}

fn inter_body_font(idx: u8) -> &'static BitmapFont {
    size_match!(
        idx,
        font_data::INTER_REGULAR_BODY_XSMALL,
        font_data::INTER_REGULAR_BODY_SMALL,
        font_data::INTER_REGULAR_BODY_MEDIUM,
        font_data::INTER_REGULAR_BODY_LARGE,
        font_data::INTER_REGULAR_BODY_XLARGE
    )
}
fn inter_heading_font(idx: u8) -> &'static BitmapFont {
    size_match!(
        idx,
        font_data::INTER_REGULAR_HEADING_XSMALL,
        font_data::INTER_REGULAR_HEADING_SMALL,
        font_data::INTER_REGULAR_HEADING_MEDIUM,
        font_data::INTER_REGULAR_HEADING_LARGE,
        font_data::INTER_REGULAR_HEADING_XLARGE
    )
}
fn inter_bold_body_font(idx: u8) -> &'static BitmapFont {
    size_match!(
        idx,
        font_data::INTER_BOLD_BODY_XSMALL,
        font_data::INTER_BOLD_BODY_SMALL,
        font_data::INTER_BOLD_BODY_MEDIUM,
        font_data::INTER_BOLD_BODY_LARGE,
        font_data::INTER_BOLD_BODY_XLARGE
    )
}

pub const UI_BODY_SIZE_IDX: u8 = 1;
pub const UI_HEADING_SIZE_IDX: u8 = 0;
pub const UI_CHROME_SIZE_IDX: u8 = 0;

// CrossInk internal list pages use a slightly larger Inter tier than
// compact chrome.  Settings, Tools, Games, and general internal list
// surfaces use MEDIUM. Reader tab lists also use MEDIUM now; their
// row stride is slightly taller so text remains footer-safe.
pub const UI_LIST_SIZE_IDX: u8 = 2;
pub const UI_READER_LIST_SIZE_IDX: u8 = 2;

#[inline]
pub fn ui_body_font_fixed() -> &'static BitmapFont {
    usable_or_fallback(
        inter_body_font(UI_BODY_SIZE_IDX),
        body_font(UI_BODY_SIZE_IDX),
    )
}

#[inline]
pub fn ui_heading_font_fixed() -> &'static BitmapFont {
    usable_or_fallback(
        inter_heading_font(UI_HEADING_SIZE_IDX),
        heading_font(UI_HEADING_SIZE_IDX),
    )
}

#[inline]
pub fn ui_section_font_fixed() -> &'static BitmapFont {
    usable_or_fallback(inter_bold_body_font(UI_BODY_SIZE_IDX), ui_body_font_fixed())
}

#[inline]
pub fn ui_list_font_fixed() -> &'static BitmapFont {
    usable_or_fallback(
        inter_body_font(UI_LIST_SIZE_IDX),
        body_font(UI_LIST_SIZE_IDX),
    )
}

#[inline]
pub fn ui_list_section_font_fixed() -> &'static BitmapFont {
    usable_or_fallback(inter_bold_body_font(UI_LIST_SIZE_IDX), ui_list_font_fixed())
}

#[inline]
pub fn ui_reader_list_font_fixed() -> &'static BitmapFont {
    usable_or_fallback(
        inter_body_font(UI_READER_LIST_SIZE_IDX),
        body_font(UI_READER_LIST_SIZE_IDX),
    )
}

#[inline]
fn usable_or_fallback(
    candidate: &'static BitmapFont,
    fallback: &'static BitmapFont,
) -> &'static BitmapFont {
    if candidate.glyph('A').advance > 0 {
        candidate
    } else {
        fallback
    }
}

pub fn reader_body_font(source: u8, idx: u8) -> &'static BitmapFont {
    let fallback = body_font(idx);
    usable_or_fallback(
        match source {
            1 => charis_body_font(idx),
            2 => bitter_body_font(idx),
            3 => lexend_body_font(idx),
            _ => fallback,
        },
        fallback,
    )
}

pub fn reader_heading_font(source: u8, idx: u8) -> &'static BitmapFont {
    let fallback = heading_font(idx);
    usable_or_fallback(
        match source {
            1 => charis_heading_font(idx),
            2 => bitter_heading_font(idx),
            3 => lexend_heading_font(idx),
            _ => fallback,
        },
        fallback,
    )
}

pub fn ui_body_font(_source: u8, _idx: u8) -> &'static BitmapFont {
    ui_body_font_fixed()
}

pub fn ui_heading_font(_source: u8, _idx: u8) -> &'static BitmapFont {
    ui_heading_font_fixed()
}

pub fn chrome_font() -> &'static BitmapFont {
    usable_or_fallback(
        inter_body_font(UI_CHROME_SIZE_IDX),
        body_font(UI_CHROME_SIZE_IDX),
    )
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Style {
    Regular,
    Bold,
    Italic,
    Heading,
}

#[derive(Clone, Copy)]
pub struct FontSet {
    regular: &'static BitmapFont,
    bold: &'static BitmapFont,
    italic: &'static BitmapFont,
    heading: &'static BitmapFont,
}

impl FontSet {
    fn from_fonts(
        regular: &'static BitmapFont,
        bold_candidate: &'static BitmapFont,
        italic_candidate: &'static BitmapFont,
        heading: &'static BitmapFont,
    ) -> Self {
        let regular = usable_or_fallback(regular, body_font(1));
        let heading = usable_or_fallback(heading, heading_font(1));
        let bold = usable_or_fallback(bold_candidate, regular);
        let italic = usable_or_fallback(italic_candidate, regular);
        Self {
            regular,
            bold,
            italic,
            heading,
        }
    }

    pub fn for_size(idx: u8) -> Self {
        Self::for_source_size(0, idx)
    }

    pub fn for_source_size(source: u8, idx: u8) -> Self {
        match source {
            1 => Self::from_fonts(
                charis_body_font(idx),
                size_match!(
                    idx,
                    font_data::CHARIS_BOLD_BODY_XSMALL,
                    font_data::CHARIS_BOLD_BODY_SMALL,
                    font_data::CHARIS_BOLD_BODY_MEDIUM,
                    font_data::CHARIS_BOLD_BODY_LARGE,
                    font_data::CHARIS_BOLD_BODY_XLARGE
                ),
                size_match!(
                    idx,
                    font_data::CHARIS_ITALIC_BODY_XSMALL,
                    font_data::CHARIS_ITALIC_BODY_SMALL,
                    font_data::CHARIS_ITALIC_BODY_MEDIUM,
                    font_data::CHARIS_ITALIC_BODY_LARGE,
                    font_data::CHARIS_ITALIC_BODY_XLARGE
                ),
                charis_heading_font(idx),
            ),
            2 => Self::from_fonts(
                bitter_body_font(idx),
                size_match!(
                    idx,
                    font_data::BITTER_BOLD_BODY_XSMALL,
                    font_data::BITTER_BOLD_BODY_SMALL,
                    font_data::BITTER_BOLD_BODY_MEDIUM,
                    font_data::BITTER_BOLD_BODY_LARGE,
                    font_data::BITTER_BOLD_BODY_XLARGE
                ),
                size_match!(
                    idx,
                    font_data::BITTER_ITALIC_BODY_XSMALL,
                    font_data::BITTER_ITALIC_BODY_SMALL,
                    font_data::BITTER_ITALIC_BODY_MEDIUM,
                    font_data::BITTER_ITALIC_BODY_LARGE,
                    font_data::BITTER_ITALIC_BODY_XLARGE
                ),
                bitter_heading_font(idx),
            ),
            3 => Self::from_fonts(
                lexend_body_font(idx),
                size_match!(
                    idx,
                    font_data::LEXEND_BOLD_BODY_XSMALL,
                    font_data::LEXEND_BOLD_BODY_SMALL,
                    font_data::LEXEND_BOLD_BODY_MEDIUM,
                    font_data::LEXEND_BOLD_BODY_LARGE,
                    font_data::LEXEND_BOLD_BODY_XLARGE
                ),
                size_match!(
                    idx,
                    font_data::LEXEND_ITALIC_BODY_XSMALL,
                    font_data::LEXEND_ITALIC_BODY_SMALL,
                    font_data::LEXEND_ITALIC_BODY_MEDIUM,
                    font_data::LEXEND_ITALIC_BODY_LARGE,
                    font_data::LEXEND_ITALIC_BODY_XLARGE
                ),
                lexend_heading_font(idx),
            ),
            _ => Self::from_fonts(
                body_font(idx),
                size_match!(
                    idx,
                    font_data::BOLD_BODY_XSMALL,
                    font_data::BOLD_BODY_SMALL,
                    font_data::BOLD_BODY_MEDIUM,
                    font_data::BOLD_BODY_LARGE,
                    font_data::BOLD_BODY_XLARGE
                ),
                size_match!(
                    idx,
                    font_data::ITALIC_BODY_XSMALL,
                    font_data::ITALIC_BODY_SMALL,
                    font_data::ITALIC_BODY_MEDIUM,
                    font_data::ITALIC_BODY_LARGE,
                    font_data::ITALIC_BODY_XLARGE
                ),
                heading_font(idx),
            ),
        }
    }

    #[inline]
    pub fn font(&self, style: Style) -> &'static BitmapFont {
        match style {
            Style::Regular => self.regular,
            Style::Bold => self.bold,
            Style::Italic => self.italic,
            Style::Heading => self.heading,
        }
    }

    #[inline]
    pub fn line_height(&self, style: Style) -> u16 {
        self.font(style).line_height
    }
    #[inline]
    pub fn ascent(&self, style: Style) -> u16 {
        self.font(style).ascent
    }
    #[inline]
    pub fn advance(&self, ch: char, style: Style) -> u8 {
        self.font(style).advance(ch)
    }
    #[inline]
    pub fn advance_byte(&self, b: u8, style: Style) -> u8 {
        self.font(style).advance(bitmap::byte_to_char(b))
    }

    #[inline]
    pub fn draw_char(
        &self,
        strip: &mut StripBuffer,
        ch: char,
        style: Style,
        cx: i32,
        baseline: i32,
    ) -> u8 {
        self.font(style).draw_char(strip, ch, cx, baseline)
    }

    pub fn draw_bytes(
        &self,
        strip: &mut StripBuffer,
        text: &[u8],
        style: Style,
        cx: i32,
        baseline: i32,
    ) -> i32 {
        self.font(style).draw_bytes(strip, text, cx, baseline)
    }

    pub fn draw_str(
        &self,
        strip: &mut StripBuffer,
        text: &str,
        style: Style,
        cx: i32,
        baseline: i32,
    ) -> i32 {
        self.font(style).draw_str(strip, text, cx, baseline)
    }
}
