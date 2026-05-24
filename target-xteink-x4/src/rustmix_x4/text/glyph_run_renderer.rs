//! In-memory prepared glyph run rendering.
//!
//! This module draws positioned glyph records through the bitmap renderer. It
//! does not shape text, perform layout, or connect to app/display renderers.

use super::font_assets::VfntFont;
use super::glyph_bitmap_renderer::{
    GlyphBitmapRenderer, GlyphBlitMode, GlyphPoint, GlyphRenderError, MonochromeRenderTarget,
};
use super::glyph_cache::{FontAssetId, PositionedGlyph};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GlyphRunRenderError {
    MissingFont,
    MissingGlyph,
    UnsupportedBitmapFormat,
    InvalidRun,
    InvalidGlyphRecord,
    GlyphRenderFailed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GlyphRunRenderOptions {
    pub mode: GlyphBlitMode,
}

impl Default for GlyphRunRenderOptions {
    fn default() -> Self {
        Self {
            mode: GlyphBlitMode::Transparent,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GlyphRunRenderRequest<'a> {
    pub glyphs: &'a [PositionedGlyph],
    pub options: GlyphRunRenderOptions,
}

pub trait PreparedFontLookup<'font> {
    fn font_for_asset(&self, font: FontAssetId) -> Option<&'font VfntFont<'font>>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SingleFontLookup<'font> {
    pub font_id: FontAssetId,
    pub font: &'font VfntFont<'font>,
}

impl<'font> PreparedFontLookup<'font> for SingleFontLookup<'font> {
    fn font_for_asset(&self, font: FontAssetId) -> Option<&'font VfntFont<'font>> {
        if self.font_id == font {
            Some(self.font)
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LoadedPreparedFont<'font> {
    pub font_id: FontAssetId,
    pub font: &'font VfntFont<'font>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SliceFontLookup<'fonts, 'font> {
    pub fonts: &'fonts [LoadedPreparedFont<'font>],
}

impl<'font> PreparedFontLookup<'font> for SliceFontLookup<'_, 'font> {
    fn font_for_asset(&self, font: FontAssetId) -> Option<&'font VfntFont<'font>> {
        self.fonts
            .iter()
            .find(|loaded| loaded.font_id == font)
            .map(|loaded| loaded.font)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GlyphRunRenderer;

impl GlyphRunRenderer {
    pub fn render<'font, T, F>(
        target: &mut T,
        fonts: &F,
        request: GlyphRunRenderRequest<'_>,
    ) -> Result<(), GlyphRunRenderError>
    where
        T: MonochromeRenderTarget,
        F: PreparedFontLookup<'font>,
    {
        for glyph in request.glyphs {
            let font = fonts
                .font_for_asset(glyph.font)
                .ok_or(GlyphRunRenderError::MissingFont)?;

            GlyphBitmapRenderer::render_glyph_id(
                target,
                font,
                glyph.glyph_id,
                GlyphPoint {
                    x: glyph.x,
                    y: glyph.y,
                },
                request.options.mode,
            )
            .map_err(map_render_error)?;
        }

        Ok(())
    }
}

fn map_render_error(err: GlyphRenderError) -> GlyphRunRenderError {
    match err {
        GlyphRenderError::MissingGlyph => GlyphRunRenderError::MissingGlyph,
        GlyphRenderError::UnsupportedBitmapFormat => GlyphRunRenderError::UnsupportedBitmapFormat,
        GlyphRenderError::InvalidBitmapStride | GlyphRenderError::InvalidBitmapData => {
            GlyphRunRenderError::InvalidGlyphRecord
        }
        GlyphRenderError::TargetOutOfBounds => GlyphRunRenderError::GlyphRenderFailed,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        GlyphRunRenderError, GlyphRunRenderOptions, GlyphRunRenderRequest, GlyphRunRenderer,
        LoadedPreparedFont, PreparedFontLookup, SingleFontLookup, SliceFontLookup,
    };
    use crate::rustmix_x4::text::font_assets::{
        VFNT_GLYPH_BITMAP_LEN, VFNT_GLYPH_METRICS_LEN, VFNT_HEADER_LEN, VFNT_MAGIC, VFNT_VERSION,
        VfntFont,
    };
    use crate::rustmix_x4::text::glyph_bitmap_renderer::{
        GlyphBlitMode, MonoBitmapViewMut, MonochromeRenderTarget,
    };
    use crate::rustmix_x4::text::glyph_cache::{FontAssetId, PositionedGlyph};

    const SCRIPT_LATIN: u16 = 1;
    const BITMAP_FORMAT_ONE_BPP: u16 = 1;
    const BITMAP_FORMAT_TWO_BPP: u16 = 2;

    fn push_u16(data: &mut Vec<u8>, value: u16) {
        data.extend_from_slice(&value.to_le_bytes());
    }

    fn push_i16(data: &mut Vec<u8>, value: i16) {
        data.extend_from_slice(&value.to_le_bytes());
    }

    fn push_u32(data: &mut Vec<u8>, value: u32) {
        data.extend_from_slice(&value.to_le_bytes());
    }

    fn synthetic_font_bytes(bitmap_format: u16, second_pattern: u8) -> Vec<u8> {
        let glyph_count = 2usize;
        let metrics_offset = VFNT_HEADER_LEN;
        let bitmap_index_offset = metrics_offset + glyph_count * VFNT_GLYPH_METRICS_LEN;
        let bitmap_data_offset = bitmap_index_offset + glyph_count * VFNT_GLYPH_BITMAP_LEN;
        let bitmap_data = [0b1111_0000, second_pattern];

        let mut data = Vec::new();
        data.extend_from_slice(&VFNT_MAGIC);
        push_u16(&mut data, VFNT_VERSION);
        push_u16(&mut data, VFNT_HEADER_LEN as u16);
        push_u32(&mut data, 0);
        push_u16(&mut data, 8);
        push_u16(&mut data, 10);
        push_i16(&mut data, 8);
        push_i16(&mut data, -2);
        push_u32(&mut data, glyph_count as u32);
        push_u32(&mut data, metrics_offset as u32);
        push_u32(&mut data, bitmap_index_offset as u32);
        push_u32(&mut data, bitmap_data_offset as u32);
        push_u32(&mut data, bitmap_data.len() as u32);
        push_u16(&mut data, SCRIPT_LATIN);
        push_u16(&mut data, bitmap_format);

        push_metrics(&mut data, 1, 4, 1);
        push_metrics(&mut data, 2, 4, 1);
        push_bitmap(&mut data, 1, 0, 1, 1);
        push_bitmap(&mut data, 2, 1, 1, 1);
        data.extend_from_slice(&bitmap_data);

        data
    }

    fn push_metrics(data: &mut Vec<u8>, glyph_id: u32, width: u16, height: u16) {
        push_u32(data, glyph_id);
        push_i16(data, width as i16);
        push_i16(data, 0);
        push_i16(data, 0);
        push_i16(data, 0);
        push_u16(data, width);
        push_u16(data, height);
    }

    fn push_bitmap(data: &mut Vec<u8>, glyph_id: u32, offset: u32, len: u32, row_stride: u16) {
        push_u32(data, glyph_id);
        push_u32(data, offset);
        push_u32(data, len);
        push_u16(data, row_stride);
        push_u16(data, 0);
    }

    fn positioned(font: FontAssetId, glyph_id: u32, x: i16, y: i16) -> PositionedGlyph {
        PositionedGlyph {
            font,
            glyph_id,
            x,
            y,
            advance_x: 0,
            advance_y: 0,
            cluster: 0,
        }
    }

    fn render_run<'font, F>(
        target: &mut MonoBitmapViewMut<'_>,
        lookup: &F,
        glyphs: &[PositionedGlyph],
        mode: GlyphBlitMode,
    ) -> Result<(), GlyphRunRenderError>
    where
        F: PreparedFontLookup<'font>,
    {
        GlyphRunRenderer::render(
            target,
            lookup,
            GlyphRunRenderRequest {
                glyphs,
                options: GlyphRunRenderOptions { mode },
            },
        )
    }

    #[test]
    fn renders_multiple_positioned_glyphs_into_memory() {
        let font_bytes = synthetic_font_bytes(BITMAP_FORMAT_ONE_BPP, 0b1001_0000);
        let font = VfntFont::parse(&font_bytes).unwrap();
        let lookup = SingleFontLookup {
            font_id: FontAssetId(1),
            font: &font,
        };
        let glyphs = [
            positioned(FontAssetId(1), 1, 0, 0),
            positioned(FontAssetId(1), 2, 5, 0),
            positioned(FontAssetId(1), 1, 0, 1),
        ];
        let mut data = [0u8; 4];
        let mut target = MonoBitmapViewMut::new(16, 2, 2, &mut data).unwrap();

        render_run(&mut target, &lookup, &glyphs, GlyphBlitMode::Transparent).unwrap();

        assert!(target.pixel(0, 0));
        assert!(target.pixel(3, 0));
        assert!(target.pixel(5, 0));
        assert!(!target.pixel(6, 0));
        assert!(!target.pixel(7, 0));
        assert!(target.pixel(8, 0));
        assert!(target.pixel(0, 1));
        assert!(target.pixel(3, 1));
    }

    #[test]
    fn renders_glyphs_in_order() {
        let font_bytes = synthetic_font_bytes(BITMAP_FORMAT_ONE_BPP, 0b0101_0000);
        let font = VfntFont::parse(&font_bytes).unwrap();
        let lookup = SingleFontLookup {
            font_id: FontAssetId(1),
            font: &font,
        };
        let glyphs = [
            positioned(FontAssetId(1), 1, 0, 0),
            positioned(FontAssetId(1), 2, 1, 0),
        ];
        let mut data = [0u8; 1];
        let mut target = MonoBitmapViewMut::new(8, 1, 1, &mut data).unwrap();

        render_run(&mut target, &lookup, &glyphs, GlyphBlitMode::Opaque).unwrap();

        assert!(target.pixel(0, 0));
        assert!(!target.pixel(1, 0));
        assert!(target.pixel(2, 0));
        assert!(!target.pixel(3, 0));
        assert!(target.pixel(4, 0));
    }

    #[test]
    fn empty_run_is_noop() {
        let font_bytes = synthetic_font_bytes(BITMAP_FORMAT_ONE_BPP, 0b1000_0000);
        let font = VfntFont::parse(&font_bytes).unwrap();
        let lookup = SingleFontLookup {
            font_id: FontAssetId(1),
            font: &font,
        };
        let mut data = [0b1010_0000];
        let before = data;
        let mut target = MonoBitmapViewMut::new(8, 1, 1, &mut data).unwrap();

        render_run(&mut target, &lookup, &[], GlyphBlitMode::Opaque).unwrap();

        assert_eq!(data, before);
    }

    #[test]
    fn missing_font_returns_error() {
        let font_bytes = synthetic_font_bytes(BITMAP_FORMAT_ONE_BPP, 0b1000_0000);
        let font = VfntFont::parse(&font_bytes).unwrap();
        let lookup = SingleFontLookup {
            font_id: FontAssetId(1),
            font: &font,
        };
        let glyphs = [positioned(FontAssetId(2), 1, 0, 0)];
        let mut data = [0u8; 1];
        let mut target = MonoBitmapViewMut::new(8, 1, 1, &mut data).unwrap();

        assert_eq!(
            render_run(&mut target, &lookup, &glyphs, GlyphBlitMode::Transparent),
            Err(GlyphRunRenderError::MissingFont)
        );
    }

    #[test]
    fn missing_glyph_returns_error() {
        let font_bytes = synthetic_font_bytes(BITMAP_FORMAT_ONE_BPP, 0b1000_0000);
        let font = VfntFont::parse(&font_bytes).unwrap();
        let lookup = SingleFontLookup {
            font_id: FontAssetId(1),
            font: &font,
        };
        let glyphs = [positioned(FontAssetId(1), 99, 0, 0)];
        let mut data = [0u8; 1];
        let mut target = MonoBitmapViewMut::new(8, 1, 1, &mut data).unwrap();

        assert_eq!(
            render_run(&mut target, &lookup, &glyphs, GlyphBlitMode::Transparent),
            Err(GlyphRunRenderError::MissingGlyph)
        );
    }

    #[test]
    fn unsupported_bitmap_format_returns_error() {
        let font_bytes = synthetic_font_bytes(BITMAP_FORMAT_TWO_BPP, 0b1000_0000);
        let font = VfntFont::parse(&font_bytes).unwrap();
        let lookup = SingleFontLookup {
            font_id: FontAssetId(1),
            font: &font,
        };
        let glyphs = [positioned(FontAssetId(1), 1, 0, 0)];
        let mut data = [0u8; 1];
        let mut target = MonoBitmapViewMut::new(8, 1, 1, &mut data).unwrap();

        assert_eq!(
            render_run(&mut target, &lookup, &glyphs, GlyphBlitMode::Transparent),
            Err(GlyphRunRenderError::UnsupportedBitmapFormat)
        );
    }

    #[test]
    fn transparent_run_preserves_background() {
        let font_bytes = synthetic_font_bytes(BITMAP_FORMAT_ONE_BPP, 0b1000_0000);
        let font = VfntFont::parse(&font_bytes).unwrap();
        let lookup = SingleFontLookup {
            font_id: FontAssetId(1),
            font: &font,
        };
        let glyphs = [positioned(FontAssetId(1), 2, 0, 0)];
        let mut data = [0b0100_0000];
        let mut target = MonoBitmapViewMut::new(8, 1, 1, &mut data).unwrap();

        render_run(&mut target, &lookup, &glyphs, GlyphBlitMode::Transparent).unwrap();

        assert!(target.pixel(0, 0));
        assert!(target.pixel(1, 0));
    }

    #[test]
    fn opaque_run_clears_background_for_zero_bits() {
        let font_bytes = synthetic_font_bytes(BITMAP_FORMAT_ONE_BPP, 0b1000_0000);
        let font = VfntFont::parse(&font_bytes).unwrap();
        let lookup = SingleFontLookup {
            font_id: FontAssetId(1),
            font: &font,
        };
        let glyphs = [positioned(FontAssetId(1), 2, 0, 0)];
        let mut data = [0b0100_0000];
        let mut target = MonoBitmapViewMut::new(8, 1, 1, &mut data).unwrap();

        render_run(&mut target, &lookup, &glyphs, GlyphBlitMode::Opaque).unwrap();

        assert!(target.pixel(0, 0));
        assert!(!target.pixel(1, 0));
    }

    #[test]
    fn run_renderer_uses_each_glyph_position() {
        let font_bytes = synthetic_font_bytes(BITMAP_FORMAT_ONE_BPP, 0b1000_0000);
        let font = VfntFont::parse(&font_bytes).unwrap();
        let lookup = SingleFontLookup {
            font_id: FontAssetId(1),
            font: &font,
        };
        let glyphs = [
            positioned(FontAssetId(1), 2, 2, 0),
            positioned(FontAssetId(1), 2, 6, 1),
        ];
        let mut data = [0u8; 4];
        let mut target = MonoBitmapViewMut::new(16, 2, 2, &mut data).unwrap();

        render_run(&mut target, &lookup, &glyphs, GlyphBlitMode::Transparent).unwrap();

        assert!(target.pixel(2, 0));
        assert!(!target.pixel(2, 1));
        assert!(target.pixel(6, 1));
        assert!(!target.pixel(6, 0));
    }

    #[test]
    fn run_renderer_supports_clipping_through_bitmap_renderer() {
        let font_bytes = synthetic_font_bytes(BITMAP_FORMAT_ONE_BPP, 0b1111_0000);
        let font = VfntFont::parse(&font_bytes).unwrap();
        let lookup = SingleFontLookup {
            font_id: FontAssetId(1),
            font: &font,
        };
        let glyphs = [
            positioned(FontAssetId(1), 2, -2, 0),
            positioned(FontAssetId(1), 2, 7, 0),
        ];
        let mut data = [0u8; 1];
        let mut target = MonoBitmapViewMut::new(8, 1, 1, &mut data).unwrap();

        render_run(&mut target, &lookup, &glyphs, GlyphBlitMode::Transparent).unwrap();

        assert!(target.pixel(0, 0));
        assert!(target.pixel(1, 0));
        assert!(target.pixel(7, 0));
    }

    #[test]
    fn single_font_lookup_returns_only_matching_font() {
        let font_bytes = synthetic_font_bytes(BITMAP_FORMAT_ONE_BPP, 0b1000_0000);
        let font = VfntFont::parse(&font_bytes).unwrap();
        let lookup = SingleFontLookup {
            font_id: FontAssetId(4),
            font: &font,
        };

        assert!(lookup.font_for_asset(FontAssetId(4)).is_some());
        assert!(lookup.font_for_asset(FontAssetId(5)).is_none());
    }

    #[test]
    fn slice_font_lookup_selects_requested_font() {
        let first_bytes = synthetic_font_bytes(BITMAP_FORMAT_ONE_BPP, 0b1000_0000);
        let second_bytes = synthetic_font_bytes(BITMAP_FORMAT_ONE_BPP, 0b0100_0000);
        let first = VfntFont::parse(&first_bytes).unwrap();
        let second = VfntFont::parse(&second_bytes).unwrap();
        let loaded = [
            LoadedPreparedFont {
                font_id: FontAssetId(1),
                font: &first,
            },
            LoadedPreparedFont {
                font_id: FontAssetId(2),
                font: &second,
            },
        ];
        let lookup = SliceFontLookup { fonts: &loaded };
        let glyphs = [positioned(FontAssetId(2), 2, 0, 0)];
        let mut data = [0u8; 1];
        let mut target = MonoBitmapViewMut::new(8, 1, 1, &mut data).unwrap();

        render_run(&mut target, &lookup, &glyphs, GlyphBlitMode::Transparent).unwrap();

        assert!(!target.pixel(0, 0));
        assert!(target.pixel(1, 0));
    }
}
