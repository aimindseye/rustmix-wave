//! Global user-interface typography for RustMix Wave.
//!
//! UI text uses pre-rasterized 1-bpp glyph atlases derived from the Inter and
//! Atkinson Hyperlegible families. v0.13.2 shifts all profiles upward for the
//! physical e-paper panel and adds a bounded Detail role for dense diagnostic
//! values. Every user-facing role is larger than its v0.13.1 counterpart.

use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::{DrawTarget, Pixel, Point},
};

use super::display::{DisplayPreferences, UiFontFamily, UiFontSize};

mod assets;

/// One rasterized printable-ASCII glyph relative to its text baseline.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Glyph {
    pub offset: u32,
    pub width: u8,
    pub height: u8,
    pub advance: u8,
    pub left: i8,
    pub top: i8,
}

/// One complete printable-ASCII bitmap-font strike.
#[derive(Clone, Copy, Debug)]
pub struct BitmapFont {
    pub glyphs: &'static [Glyph; 95],
    pub bitmap: &'static [u8],
    pub line_height: u8,
}

impl BitmapFont {
    #[must_use]
    fn glyph(self, character: char) -> Glyph {
        let code = character as u32;
        let index = if (32..=126).contains(&code) {
            (code - 32) as usize
        } else {
            ('?' as usize) - 32
        };
        self.glyphs[index]
    }
}

/// Semantic UI text role. A size profile selects the concrete raster strike.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiTextRole {
    /// Dense diagnostics, paths and compact values. Still larger than v0.13.1.
    Detail,
    /// Standard labels, descriptions, header subtitles and footers.
    Body,
    /// Menu titles, section headings and selected-row emphasis.
    Heading,
    /// Product titles and large sensor values.
    Large,
}

/// Half-open clipping rectangle for bounded text drawing.
///
/// Reader pages use this guard so body glyphs cannot cross the shared body
/// viewport even when a proportional bitmap strike contains a wide glyph.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TextBounds {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl TextBounds {
    #[must_use]
    pub const fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    #[must_use]
    pub const fn width(self) -> i32 {
        self.right - self.left
    }

    #[must_use]
    const fn contains(self, point: Point) -> bool {
        point.x >= self.left && point.x < self.right && point.y >= self.top && point.y < self.bottom
    }
}

/// Transparent UI text style used by the firmware-local bitmap renderer.
#[derive(Clone, Copy, Debug)]
pub struct UiTextStyle {
    font: &'static BitmapFont,
    color: BinaryColor,
}

impl UiTextStyle {
    #[must_use]
    pub const fn new(font: &'static BitmapFont, color: BinaryColor) -> Self {
        Self { font, color }
    }

    #[must_use]
    pub const fn line_height(self) -> u8 {
        self.font.line_height
    }

    /// Measure one printable-ASCII Reader line using this bitmap strike.
    /// Unsupported characters follow the same `?` fallback as drawing.
    #[must_use]
    pub fn text_width(self, text: &str) -> i32 {
        text.chars()
            .filter(|character| *character != '\n')
            .map(|character| i32::from(self.font.glyph(character).advance))
            .sum()
    }
}

/// Small drop-in drawable mirroring the `Text::new(...).draw(...)` call shape
/// used by the existing shell screens.
pub struct Text<'a> {
    text: &'a str,
    baseline: Point,
    style: UiTextStyle,
}

impl<'a> Text<'a> {
    #[must_use]
    pub const fn new(text: &'a str, baseline: Point, style: UiTextStyle) -> Self {
        Self {
            text,
            baseline,
            style,
        }
    }

    /// Draw transparent text and return the cursor position after the final
    /// glyph. Printable ASCII is embedded; other characters use `?` safely.
    pub fn draw<D>(&self, display: &mut D) -> Result<Point, D::Error>
    where
        D: DrawTarget<Color = BinaryColor>,
    {
        self.draw_with_bounds(display, None)
    }

    /// Draw transparent text while discarding pixels outside one half-open
    /// viewport. The returned cursor still advances through the full string so
    /// callers can use this as a final rendering guard without altering source
    /// byte anchors or pagination state.
    pub fn draw_clipped<D>(&self, display: &mut D, bounds: TextBounds) -> Result<Point, D::Error>
    where
        D: DrawTarget<Color = BinaryColor>,
    {
        self.draw_with_bounds(display, Some(bounds))
    }

    fn draw_with_bounds<D>(
        &self,
        display: &mut D,
        bounds: Option<TextBounds>,
    ) -> Result<Point, D::Error>
    where
        D: DrawTarget<Color = BinaryColor>,
    {
        let start_x = self.baseline.x;
        let mut cursor = self.baseline;
        for character in self.text.chars() {
            if character == '\n' {
                cursor.x = start_x;
                cursor.y += i32::from(self.style.font.line_height);
                continue;
            }
            let glyph = self.style.font.glyph(character);
            draw_glyph(display, cursor, glyph, self.style, bounds)?;
            cursor.x += i32::from(glyph.advance);
        }
        Ok(cursor)
    }
}

fn draw_glyph<D>(
    display: &mut D,
    baseline: Point,
    glyph: Glyph,
    style: UiTextStyle,
    bounds: Option<TextBounds>,
) -> Result<(), D::Error>
where
    D: DrawTarget<Color = BinaryColor>,
{
    let stride = (usize::from(glyph.width) + 7) / 8;
    let offset = glyph.offset as usize;
    for row in 0..usize::from(glyph.height) {
        for column in 0..usize::from(glyph.width) {
            let byte = style.font.bitmap[offset + row * stride + column / 8];
            if byte & (0x80 >> (column % 8)) == 0 {
                continue;
            }
            let point = Point::new(
                baseline.x + i32::from(glyph.left) + column as i32,
                baseline.y + i32::from(glyph.top) + row as i32,
            );
            if bounds.map_or(true, |clip| clip.contains(point)) {
                display.draw_iter(core::iter::once(Pixel(point, style.color)))?;
            }
        }
    }
    Ok(())
}

/// Resolve a firmware-local bitmap strike for one family, profile and role.
#[must_use]
pub const fn style_for(
    family: UiFontFamily,
    size: UiFontSize,
    role: UiTextRole,
    color: BinaryColor,
) -> UiTextStyle {
    use assets::*;
    use UiFontFamily::{AtkinsonHyperlegible, Inter};
    use UiFontSize::{Compact, Large, Standard};
    use UiTextRole::{Body, Detail, Heading, Large as LargeRole};

    let font = match (family, size, role) {
        (Inter, Compact, Detail) => &INTER_COMPACT_DETAIL,
        (Inter, Compact, Body) => &INTER_COMPACT_BODY,
        (Inter, Compact, Heading) => &INTER_COMPACT_HEADING,
        (Inter, Compact, LargeRole) => &INTER_COMPACT_LARGE,
        (Inter, Standard, Detail) => &INTER_STANDARD_DETAIL,
        (Inter, Standard, Body) => &INTER_STANDARD_BODY,
        (Inter, Standard, Heading) => &INTER_STANDARD_HEADING,
        (Inter, Standard, LargeRole) => &INTER_STANDARD_LARGE,
        (Inter, Large, Detail) => &INTER_LARGE_DETAIL,
        (Inter, Large, Body) => &INTER_LARGE_BODY,
        (Inter, Large, Heading) => &INTER_LARGE_HEADING,
        (Inter, Large, LargeRole) => &INTER_LARGE_LARGE,
        (AtkinsonHyperlegible, Compact, Detail) => &ATKINSON_COMPACT_DETAIL,
        (AtkinsonHyperlegible, Compact, Body) => &ATKINSON_COMPACT_BODY,
        (AtkinsonHyperlegible, Compact, Heading) => &ATKINSON_COMPACT_HEADING,
        (AtkinsonHyperlegible, Compact, LargeRole) => &ATKINSON_COMPACT_LARGE,
        (AtkinsonHyperlegible, Standard, Detail) => &ATKINSON_STANDARD_DETAIL,
        (AtkinsonHyperlegible, Standard, Body) => &ATKINSON_STANDARD_BODY,
        (AtkinsonHyperlegible, Standard, Heading) => &ATKINSON_STANDARD_HEADING,
        (AtkinsonHyperlegible, Standard, LargeRole) => &ATKINSON_STANDARD_LARGE,
        (AtkinsonHyperlegible, Large, Detail) => &ATKINSON_LARGE_DETAIL,
        (AtkinsonHyperlegible, Large, Body) => &ATKINSON_LARGE_BODY,
        (AtkinsonHyperlegible, Large, Heading) => &ATKINSON_LARGE_HEADING,
        (AtkinsonHyperlegible, Large, LargeRole) => &ATKINSON_LARGE_LARGE,
    };
    UiTextStyle::new(font, color)
}

impl DisplayPreferences {
    #[must_use]
    pub const fn text_style(self, role: UiTextRole, color: BinaryColor) -> UiTextStyle {
        style_for(self.font_family, self.font_size, role, color)
    }

    #[must_use]
    pub const fn detail_style(self) -> UiTextStyle {
        self.text_style(UiTextRole::Detail, BinaryColor::On)
    }

    #[must_use]
    pub const fn body_style(self) -> UiTextStyle {
        self.text_style(UiTextRole::Body, BinaryColor::On)
    }

    #[must_use]
    pub const fn heading_style(self) -> UiTextStyle {
        self.text_style(UiTextRole::Heading, BinaryColor::On)
    }

    #[must_use]
    pub const fn large_style(self) -> UiTextStyle {
        self.text_style(UiTextRole::Large, BinaryColor::On)
    }

    #[must_use]
    pub const fn header_title_style(self) -> UiTextStyle {
        self.text_style(UiTextRole::Large, BinaryColor::Off)
    }

    #[must_use]
    pub const fn header_subtitle_style(self) -> UiTextStyle {
        self.text_style(UiTextRole::Body, BinaryColor::Off)
    }

    #[must_use]
    pub const fn navigation_style(self) -> UiTextStyle {
        self.heading_style()
    }

    #[must_use]
    pub const fn footer_style(self) -> UiTextStyle {
        self.body_style()
    }
}

#[cfg(test)]
mod tests {
    use embedded_graphics::{mock_display::MockDisplay, pixelcolor::BinaryColor, prelude::Point};

    use super::{Text, TextBounds};
    use crate::app::display::{DisplayPreferences, UiFontFamily, UiFontSize};

    #[test]
    fn renders_inter_standard_ascii_text() {
        let mut display = MockDisplay::<BinaryColor>::new();
        display.set_allow_overdraw(true);
        let style = DisplayPreferences::default().body_style();
        // Keep the representative ASCII sample inside MockDisplay's
        // default 64 × 64 surface after the v0.13.2 readability scaling.
        let cursor = Text::new("RustMix", Point::new(0, 24), style)
            .draw(&mut display)
            .unwrap();
        assert!(cursor.x > 0);
    }

    #[test]
    fn clipped_text_guard_discards_pixels_outside_bounds() {
        let mut display = MockDisplay::<BinaryColor>::new();
        display.set_allow_overdraw(true);
        let style = DisplayPreferences::default().body_style();
        let cursor = Text::new("RustMix", Point::new(0, 24), style)
            .draw_clipped(&mut display, TextBounds::new(0, 0, 10, 64))
            .unwrap();
        assert!(cursor.x > 10);
    }

    #[test]
    fn standard_profile_is_readability_scaled() {
        let preferences = DisplayPreferences::default();
        assert!(preferences.detail_style().line_height() >= 17);
        assert!(preferences.body_style().line_height() >= 20);
        assert!(preferences.heading_style().line_height() >= 26);
    }

    #[test]
    fn resolves_hyperlegible_large_profile() {
        let preferences = DisplayPreferences {
            font_family: UiFontFamily::AtkinsonHyperlegible,
            font_size: UiFontSize::Large,
        };
        assert!(
            preferences.heading_style().line_height() >= preferences.body_style().line_height()
        );
    }
}
