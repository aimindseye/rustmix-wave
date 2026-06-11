//! Reader-only mixed Latin and SD-loaded Indic glyph rendering.
//!
//! The browser font builder rasterizes shaped Devanagari and Gujarati clusters
//! with locally supplied Noto Sans fonts. Firmware keeps a small bounded pack in
//! the active Reader session and falls back to the existing Latin strike for
//! ASCII text and missing clusters.

use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::{DrawTarget, Pixel, Point},
};

use super::typography::{Text, TextBounds, UiTextStyle};
use crate::reader_unicode::{ReaderUnicodeFonts, ReaderUnicodeGlyph};

#[must_use]
pub fn reader_line_width(text: &str, latin: UiTextStyle, fonts: &ReaderUnicodeFonts) -> i32 {
    let mut width = 0;
    let mut offset = 0;
    while offset < text.len() {
        let remaining = &text[offset..];
        if let Some(glyph) = fonts.longest_prefix(remaining) {
            width += i32::from(glyph.advance.max(0));
            offset += glyph.sequence.len();
            continue;
        }
        let character = remaining
            .chars()
            .next()
            .expect("offset remains on UTF-8 boundary");
        let mut encoded = [0_u8; 4];
        width += latin.text_width(character.encode_utf8(&mut encoded));
        offset += character.len_utf8();
    }
    width
}

pub fn draw_reader_unicode_line<D>(
    display: &mut D,
    text: &str,
    baseline: Point,
    latin: UiTextStyle,
    fonts: &ReaderUnicodeFonts,
    bounds: TextBounds,
) -> Result<Point, D::Error>
where
    D: DrawTarget<Color = BinaryColor>,
{
    let mut cursor = baseline;
    let mut offset = 0;
    while offset < text.len() {
        let remaining = &text[offset..];
        if let Some(glyph) = fonts.longest_prefix(remaining) {
            draw_unicode_glyph(display, cursor, glyph, bounds)?;
            cursor.x += i32::from(glyph.advance.max(0));
            offset += glyph.sequence.len();
            continue;
        }
        let character = remaining
            .chars()
            .next()
            .expect("offset remains on UTF-8 boundary");
        let mut encoded = [0_u8; 4];
        let value = character.encode_utf8(&mut encoded);
        cursor = Text::new(value, cursor, latin).draw_clipped(display, bounds)?;
        offset += character.len_utf8();
    }
    Ok(cursor)
}

fn draw_unicode_glyph<D>(
    display: &mut D,
    baseline: Point,
    glyph: &ReaderUnicodeGlyph,
    bounds: TextBounds,
) -> Result<(), D::Error>
where
    D: DrawTarget<Color = BinaryColor>,
{
    let stride = (usize::from(glyph.width) + 7) / 8;
    for row in 0..usize::from(glyph.height) {
        for column in 0..usize::from(glyph.width) {
            let byte = glyph.bitmap[row * stride + column / 8];
            if byte & (0x80 >> (column % 8)) == 0 {
                continue;
            }
            let point = Point::new(
                baseline.x + i32::from(glyph.left) + column as i32,
                baseline.y + i32::from(glyph.top) + row as i32,
            );
            if point.x >= bounds.left
                && point.x < bounds.right
                && point.y >= bounds.top
                && point.y < bounds.bottom
            {
                display.draw_iter(core::iter::once(Pixel(point, BinaryColor::On)))?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use embedded_graphics::{mock_display::MockDisplay, pixelcolor::BinaryColor, prelude::Point};

    use super::{draw_reader_unicode_line, reader_line_width};
    use crate::{
        app::{display::DisplayPreferences, typography::TextBounds},
        reader::BookFontSize,
        reader_unicode::{
            ReaderUnicodeFontPack, ReaderUnicodeFonts, ReaderUnicodeGlyph, ReaderUnicodeScript,
        },
    };

    fn fonts() -> ReaderUnicodeFonts {
        ReaderUnicodeFonts {
            devanagari: Some(ReaderUnicodeFontPack {
                script: ReaderUnicodeScript::Devanagari,
                size: BookFontSize::Medium,
                line_height: 24,
                glyphs: vec![ReaderUnicodeGlyph {
                    sequence: "श्री".into(),
                    width: 8,
                    height: 8,
                    advance: 9,
                    left: 0,
                    top: -8,
                    bitmap: vec![0xFF; 8],
                }],
            }),
            gujarati: None,
            warning: None,
        }
    }

    #[test]
    fn measures_and_draws_mixed_latin_and_shaped_cluster_line() {
        let style = DisplayPreferences::default().body_style();
        let fonts = fonts();
        assert!(reader_line_width("A श्री B", style, &fonts) > 9);
        let mut display = MockDisplay::<BinaryColor>::new();
        display.set_allow_overdraw(true);
        let end = draw_reader_unicode_line(
            &mut display,
            "श्री",
            Point::new(4, 16),
            style,
            &fonts,
            TextBounds::new(0, 0, 64, 64),
        )
        .unwrap();
        assert_eq!(end.x, 13);
    }
}
