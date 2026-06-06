//! Reader-specific body typography.
//!
//! Reader pages deliberately use an independent font preference boundary so
//! global UI typography remains stable. Inter reuses the existing UI strike;
//! Atkinson Hyperlegible Next Medium, DejaVu Serif and Literata Medium use
//! generated printable-ASCII Reader-only bitmap strikes. TXT normalization
//! converts unsupported punctuation before layout.

use embedded_graphics::pixelcolor::BinaryColor;

use super::{
    display::{UiFontFamily, UiFontSize},
    reader_atkinson_next_assets::{
        ATKINSON_NEXT_LARGE, ATKINSON_NEXT_MEDIUM, ATKINSON_NEXT_SMALL, ATKINSON_NEXT_XLARGE,
    },
    reader_literata_assets::{LITERATA_LARGE, LITERATA_MEDIUM, LITERATA_SMALL, LITERATA_XLARGE},
    reader_serif_assets::{SERIF_LARGE, SERIF_MEDIUM, SERIF_SMALL, SERIF_XLARGE},
    typography::{style_for, UiTextRole, UiTextStyle},
};
use crate::reader::{BookFont, BookFontSize, ReadingTheme};

/// Resolve one Reader body strike without affecting global UI preferences.
#[must_use]
pub const fn reader_body_style(
    family: BookFont,
    size: BookFontSize,
    _theme: ReadingTheme,
) -> UiTextStyle {
    match family {
        BookFont::Inter => style_for(
            UiFontFamily::Inter,
            ui_profile(size),
            ui_role(size),
            BinaryColor::On,
        ),
        BookFont::AtkinsonHyperlegible => {
            UiTextStyle::new(atkinson_next_font(size), BinaryColor::On)
        }
        BookFont::Serif => UiTextStyle::new(serif_font(size), BinaryColor::On),
        BookFont::Literata => UiTextStyle::new(literata_font(size), BinaryColor::On),
    }
}

#[must_use]
const fn ui_profile(size: BookFontSize) -> UiFontSize {
    match size {
        BookFontSize::Small => UiFontSize::Compact,
        BookFontSize::Medium => UiFontSize::Standard,
        BookFontSize::Large | BookFontSize::XLarge => UiFontSize::Large,
    }
}

#[must_use]
const fn ui_role(size: BookFontSize) -> UiTextRole {
    match size {
        BookFontSize::Small | BookFontSize::Medium | BookFontSize::Large => UiTextRole::Body,
        BookFontSize::XLarge => UiTextRole::Heading,
    }
}

#[must_use]
const fn atkinson_next_font(size: BookFontSize) -> &'static super::typography::BitmapFont {
    match size {
        BookFontSize::Small => &ATKINSON_NEXT_SMALL,
        BookFontSize::Medium => &ATKINSON_NEXT_MEDIUM,
        BookFontSize::Large => &ATKINSON_NEXT_LARGE,
        BookFontSize::XLarge => &ATKINSON_NEXT_XLARGE,
    }
}

#[must_use]
const fn serif_font(size: BookFontSize) -> &'static super::typography::BitmapFont {
    match size {
        BookFontSize::Small => &SERIF_SMALL,
        BookFontSize::Medium => &SERIF_MEDIUM,
        BookFontSize::Large => &SERIF_LARGE,
        BookFontSize::XLarge => &SERIF_XLARGE,
    }
}

#[must_use]
const fn literata_font(size: BookFontSize) -> &'static super::typography::BitmapFont {
    match size {
        BookFontSize::Small => &LITERATA_SMALL,
        BookFontSize::Medium => &LITERATA_MEDIUM,
        BookFontSize::Large => &LITERATA_LARGE,
        BookFontSize::XLarge => &LITERATA_XLARGE,
    }
}

#[cfg(test)]
mod tests {
    use super::reader_body_style;
    use crate::reader::{BookFont, BookFontSize, ReadingTheme};

    #[test]
    fn resolves_all_reader_body_profiles() {
        for family in [
            BookFont::Inter,
            BookFont::AtkinsonHyperlegible,
            BookFont::Serif,
            BookFont::Literata,
        ] {
            for size in [
                BookFontSize::Small,
                BookFontSize::Medium,
                BookFontSize::Large,
                BookFontSize::XLarge,
            ] {
                assert!(reader_body_style(family, size, ReadingTheme::Classic).line_height() > 0);
            }
        }
    }
}
