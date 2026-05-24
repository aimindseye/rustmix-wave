//! In-memory VFNT glyph bitmap rendering.
//!
//! This module renders parsed 1bpp glyph bitmaps into a borrowed monochrome
//! buffer. It is not connected to e-paper display or app text rendering.

use super::font_assets::{FontBitmapFormat, VfntFont, VfntGlyph, VfntParseError};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GlyphRenderError {
    MissingGlyph,
    UnsupportedBitmapFormat,
    InvalidBitmapStride,
    InvalidBitmapData,
    TargetOutOfBounds,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GlyphBlitMode {
    Transparent,
    Opaque,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GlyphPoint {
    pub x: i16,
    pub y: i16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GlyphRenderRequest<'a> {
    pub glyph: VfntGlyph<'a>,
    pub bitmap_format: FontBitmapFormat,
    pub origin: GlyphPoint,
    pub mode: GlyphBlitMode,
}

pub trait MonochromeRenderTarget {
    fn width(&self) -> u16;
    fn height(&self) -> u16;
    fn set_pixel(&mut self, x: u16, y: u16, on: bool);
    fn pixel(&self, x: u16, y: u16) -> bool;
}

#[derive(Debug, Eq, PartialEq)]
pub struct MonoBitmapViewMut<'a> {
    width: u16,
    height: u16,
    row_stride: u16,
    data: &'a mut [u8],
}

impl<'a> MonoBitmapViewMut<'a> {
    pub fn new(
        width: u16,
        height: u16,
        row_stride: u16,
        data: &'a mut [u8],
    ) -> Result<Self, GlyphRenderError> {
        let min_stride = min_row_stride(width)?;
        if row_stride < min_stride {
            return Err(GlyphRenderError::InvalidBitmapStride);
        }

        let required_len = usize::from(row_stride)
            .checked_mul(usize::from(height))
            .ok_or(GlyphRenderError::TargetOutOfBounds)?;
        if data.len() < required_len {
            return Err(GlyphRenderError::TargetOutOfBounds);
        }

        Ok(Self {
            width,
            height,
            row_stride,
            data,
        })
    }

    pub const fn row_stride(&self) -> u16 {
        self.row_stride
    }

    fn pixel_offset(&self, x: u16, y: u16) -> Option<(usize, u8)> {
        if x >= self.width || y >= self.height {
            return None;
        }

        let row_start = usize::from(y).checked_mul(usize::from(self.row_stride))?;
        let byte_offset = row_start.checked_add(usize::from(x / 8))?;
        let bit_mask = 0x80 >> (x % 8);
        Some((byte_offset, bit_mask))
    }
}

impl MonochromeRenderTarget for MonoBitmapViewMut<'_> {
    fn width(&self) -> u16 {
        self.width
    }

    fn height(&self) -> u16 {
        self.height
    }

    fn set_pixel(&mut self, x: u16, y: u16, on: bool) {
        let Some((byte_offset, bit_mask)) = self.pixel_offset(x, y) else {
            return;
        };
        let Some(byte) = self.data.get_mut(byte_offset) else {
            return;
        };

        if on {
            *byte |= bit_mask;
        } else {
            *byte &= !bit_mask;
        }
    }

    fn pixel(&self, x: u16, y: u16) -> bool {
        self.pixel_offset(x, y)
            .and_then(|(byte_offset, bit_mask)| {
                self.data.get(byte_offset).map(|byte| byte & bit_mask != 0)
            })
            .unwrap_or(false)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GlyphBitmapRenderer;

impl GlyphBitmapRenderer {
    pub fn render<T: MonochromeRenderTarget>(
        target: &mut T,
        request: GlyphRenderRequest<'_>,
    ) -> Result<(), GlyphRenderError> {
        if request.bitmap_format != FontBitmapFormat::OneBpp {
            return Err(GlyphRenderError::UnsupportedBitmapFormat);
        }

        render_one_bpp(target, request)
    }

    pub fn render_glyph_id<T: MonochromeRenderTarget>(
        target: &mut T,
        font: &VfntFont<'_>,
        glyph_id: u32,
        origin: GlyphPoint,
        mode: GlyphBlitMode,
    ) -> Result<(), GlyphRenderError> {
        let glyph = font.glyph(glyph_id).map_err(map_glyph_error)?;
        Self::render(
            target,
            GlyphRenderRequest {
                glyph,
                bitmap_format: font.header().bitmap_format,
                origin,
                mode,
            },
        )
    }
}

fn render_one_bpp<T: MonochromeRenderTarget>(
    target: &mut T,
    request: GlyphRenderRequest<'_>,
) -> Result<(), GlyphRenderError> {
    let width = request.glyph.metrics.width;
    let height = request.glyph.metrics.height;
    let row_stride = request.glyph.bitmap.row_stride;
    let min_stride = min_row_stride(width)?;
    if row_stride < min_stride {
        return Err(GlyphRenderError::InvalidBitmapStride);
    }

    let required_len = usize::from(row_stride)
        .checked_mul(usize::from(height))
        .ok_or(GlyphRenderError::InvalidBitmapData)?;
    if request.glyph.bitmap_data.len() < required_len {
        return Err(GlyphRenderError::InvalidBitmapData);
    }

    for src_y in 0..height {
        let dst_y = i32::from(request.origin.y) + i32::from(src_y);
        if dst_y < 0 || dst_y >= i32::from(target.height()) {
            continue;
        }

        let row_start = usize::from(src_y) * usize::from(row_stride);
        for src_x in 0..width {
            let dst_x = i32::from(request.origin.x) + i32::from(src_x);
            if dst_x < 0 || dst_x >= i32::from(target.width()) {
                continue;
            }

            let byte = request.glyph.bitmap_data[row_start + usize::from(src_x / 8)];
            let bit_mask = 0x80 >> (src_x % 8);
            let on = byte & bit_mask != 0;
            if on || request.mode == GlyphBlitMode::Opaque {
                target.set_pixel(dst_x as u16, dst_y as u16, on);
            }
        }
    }

    Ok(())
}

fn min_row_stride(width: u16) -> Result<u16, GlyphRenderError> {
    let stride = usize::from(width).div_ceil(8);
    u16::try_from(stride).map_err(|_| GlyphRenderError::InvalidBitmapStride)
}

fn map_glyph_error(err: VfntParseError) -> GlyphRenderError {
    match err {
        VfntParseError::GlyphNotFound => GlyphRenderError::MissingGlyph,
        _ => GlyphRenderError::InvalidBitmapData,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        GlyphBitmapRenderer, GlyphBlitMode, GlyphPoint, GlyphRenderError, GlyphRenderRequest,
        MonoBitmapViewMut, MonochromeRenderTarget,
    };
    use crate::rustmix_x4::text::font_assets::{
        FontBitmapFormat, VFNT_GLYPH_BITMAP_LEN, VFNT_GLYPH_METRICS_LEN, VFNT_HEADER_LEN,
        VFNT_MAGIC, VFNT_VERSION, VfntFont, VfntGlyph, VfntGlyphBitmap, VfntGlyphMetrics,
    };

    const SCRIPT_LATIN: u16 = 1;
    const BITMAP_FORMAT_ONE_BPP: u16 = 1;

    fn glyph<'a>(width: u16, height: u16, row_stride: u16, bitmap_data: &'a [u8]) -> VfntGlyph<'a> {
        VfntGlyph {
            metrics: VfntGlyphMetrics {
                glyph_id: 7,
                advance_x: width as i16,
                advance_y: 0,
                bearing_x: 0,
                bearing_y: 0,
                width,
                height,
            },
            bitmap: VfntGlyphBitmap {
                glyph_id: 7,
                offset: 0,
                len: bitmap_data.len() as u32,
                row_stride,
            },
            bitmap_data,
        }
    }

    fn target<'a>(
        width: u16,
        height: u16,
        row_stride: u16,
        data: &'a mut [u8],
    ) -> MonoBitmapViewMut<'a> {
        MonoBitmapViewMut::new(width, height, row_stride, data).unwrap()
    }

    fn render<'a>(
        target: &mut MonoBitmapViewMut<'_>,
        glyph: VfntGlyph<'a>,
        origin: GlyphPoint,
        mode: GlyphBlitMode,
    ) -> Result<(), GlyphRenderError> {
        GlyphBitmapRenderer::render(
            target,
            GlyphRenderRequest {
                glyph,
                bitmap_format: FontBitmapFormat::OneBpp,
                origin,
                mode,
            },
        )
    }

    fn push_u16(data: &mut Vec<u8>, value: u16) {
        data.extend_from_slice(&value.to_le_bytes());
    }

    fn push_i16(data: &mut Vec<u8>, value: i16) {
        data.extend_from_slice(&value.to_le_bytes());
    }

    fn push_u32(data: &mut Vec<u8>, value: u32) {
        data.extend_from_slice(&value.to_le_bytes());
    }

    fn synthetic_font_bytes(bitmap_format: u16) -> Vec<u8> {
        let glyph_count = 1usize;
        let metrics_offset = VFNT_HEADER_LEN;
        let bitmap_index_offset = metrics_offset + glyph_count * VFNT_GLYPH_METRICS_LEN;
        let bitmap_data_offset = bitmap_index_offset + glyph_count * VFNT_GLYPH_BITMAP_LEN;
        let bitmap_data = [0b1000_0000];

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

        push_u32(&mut data, 7);
        push_i16(&mut data, 1);
        push_i16(&mut data, 0);
        push_i16(&mut data, 0);
        push_i16(&mut data, 0);
        push_u16(&mut data, 1);
        push_u16(&mut data, 1);

        push_u32(&mut data, 7);
        push_u32(&mut data, 0);
        push_u32(&mut data, bitmap_data.len() as u32);
        push_u16(&mut data, 1);
        push_u16(&mut data, 0);

        data.extend_from_slice(&bitmap_data);
        data
    }

    #[test]
    fn target_set_and_get_pixel_round_trip() {
        let mut data = [0u8; 4];
        let mut target = target(12, 2, 2, &mut data);

        target.set_pixel(0, 0, true);
        target.set_pixel(7, 0, true);
        target.set_pixel(8, 1, true);
        assert!(target.pixel(0, 0));
        assert!(target.pixel(7, 0));
        assert!(target.pixel(8, 1));

        target.set_pixel(7, 0, false);
        assert!(!target.pixel(7, 0));
        assert_eq!(target.row_stride(), 2);
    }

    #[test]
    fn renders_one_bpp_glyph_pixels() {
        let bitmap = [0b1010_0000, 0b0101_0000];
        let glyph = glyph(4, 2, 1, &bitmap);
        let mut data = [0u8; 2];
        let mut target = target(8, 2, 1, &mut data);

        render(
            &mut target,
            glyph,
            GlyphPoint { x: 0, y: 0 },
            GlyphBlitMode::Transparent,
        )
        .unwrap();

        assert!(target.pixel(0, 0));
        assert!(!target.pixel(1, 0));
        assert!(target.pixel(2, 0));
        assert!(!target.pixel(3, 0));
        assert!(!target.pixel(0, 1));
        assert!(target.pixel(1, 1));
        assert!(!target.pixel(2, 1));
        assert!(target.pixel(3, 1));
    }

    #[test]
    fn clips_glyph_at_right_edge() {
        let bitmap = [0b1111_0000];
        let glyph = glyph(4, 1, 1, &bitmap);
        let mut data = [0u8; 1];
        let mut target = target(5, 1, 1, &mut data);

        render(
            &mut target,
            glyph,
            GlyphPoint { x: 3, y: 0 },
            GlyphBlitMode::Transparent,
        )
        .unwrap();

        assert!(!target.pixel(2, 0));
        assert!(target.pixel(3, 0));
        assert!(target.pixel(4, 0));
    }

    #[test]
    fn clips_glyph_at_left_edge_with_negative_origin() {
        let bitmap = [0b1111_0000];
        let glyph = glyph(4, 1, 1, &bitmap);
        let mut data = [0u8; 1];
        let mut target = target(4, 1, 1, &mut data);

        render(
            &mut target,
            glyph,
            GlyphPoint { x: -2, y: 0 },
            GlyphBlitMode::Transparent,
        )
        .unwrap();

        assert!(target.pixel(0, 0));
        assert!(target.pixel(1, 0));
        assert!(!target.pixel(2, 0));
    }

    #[test]
    fn clips_glyph_at_bottom_edge() {
        let bitmap = [0b1000_0000, 0b0100_0000, 0b0010_0000];
        let glyph = glyph(3, 3, 1, &bitmap);
        let mut data = [0u8; 2];
        let mut target = target(8, 2, 1, &mut data);

        render(
            &mut target,
            glyph,
            GlyphPoint { x: 0, y: 1 },
            GlyphBlitMode::Transparent,
        )
        .unwrap();

        assert!(!target.pixel(0, 0));
        assert!(target.pixel(0, 1));
        assert!(!target.pixel(1, 1));
    }

    #[test]
    fn honors_source_row_stride() {
        let bitmap = [0b1000_0000, 0xFF, 0b0100_0000, 0xFF];
        let glyph = glyph(2, 2, 2, &bitmap);
        let mut data = [0u8; 2];
        let mut target = target(8, 2, 1, &mut data);

        render(
            &mut target,
            glyph,
            GlyphPoint { x: 0, y: 0 },
            GlyphBlitMode::Transparent,
        )
        .unwrap();

        assert!(target.pixel(0, 0));
        assert!(!target.pixel(1, 0));
        assert!(!target.pixel(0, 1));
        assert!(target.pixel(1, 1));
    }

    #[test]
    fn transparent_blit_preserves_background_for_zero_bits() {
        let bitmap = [0b1000_0000];
        let glyph = glyph(2, 1, 1, &bitmap);
        let mut data = [0b1100_0000];
        let mut target = target(8, 1, 1, &mut data);

        render(
            &mut target,
            glyph,
            GlyphPoint { x: 0, y: 0 },
            GlyphBlitMode::Transparent,
        )
        .unwrap();

        assert!(target.pixel(0, 0));
        assert!(target.pixel(1, 0));
    }

    #[test]
    fn opaque_blit_clears_background_for_zero_bits() {
        let bitmap = [0b1000_0000];
        let glyph = glyph(2, 1, 1, &bitmap);
        let mut data = [0b1100_0000];
        let mut target = target(8, 1, 1, &mut data);

        render(
            &mut target,
            glyph,
            GlyphPoint { x: 0, y: 0 },
            GlyphBlitMode::Opaque,
        )
        .unwrap();

        assert!(target.pixel(0, 0));
        assert!(!target.pixel(1, 0));
    }

    #[test]
    fn rejects_row_stride_too_small() {
        let bitmap = [0u8; 1];
        let glyph = glyph(9, 1, 1, &bitmap);
        let mut data = [0u8; 2];
        let mut target = target(16, 1, 2, &mut data);

        assert_eq!(
            render(
                &mut target,
                glyph,
                GlyphPoint { x: 0, y: 0 },
                GlyphBlitMode::Transparent,
            ),
            Err(GlyphRenderError::InvalidBitmapStride)
        );
    }

    #[test]
    fn rejects_short_bitmap_data() {
        let bitmap = [0u8; 1];
        let glyph = glyph(8, 2, 1, &bitmap);
        let mut data = [0u8; 2];
        let mut target = target(8, 2, 1, &mut data);

        assert_eq!(
            render(
                &mut target,
                glyph,
                GlyphPoint { x: 0, y: 0 },
                GlyphBlitMode::Transparent,
            ),
            Err(GlyphRenderError::InvalidBitmapData)
        );
    }

    #[test]
    fn rejects_unsupported_bitmap_format() {
        let bitmap = [0b1000_0000];
        let glyph = glyph(1, 1, 1, &bitmap);
        let mut data = [0u8; 1];
        let mut target = target(8, 1, 1, &mut data);

        assert_eq!(
            GlyphBitmapRenderer::render(
                &mut target,
                GlyphRenderRequest {
                    glyph,
                    bitmap_format: FontBitmapFormat::TwoBpp,
                    origin: GlyphPoint { x: 0, y: 0 },
                    mode: GlyphBlitMode::Transparent,
                },
            ),
            Err(GlyphRenderError::UnsupportedBitmapFormat)
        );
    }

    #[test]
    fn returns_missing_for_absent_glyph_when_rendering_from_font() {
        let font_bytes = synthetic_font_bytes(BITMAP_FORMAT_ONE_BPP);
        let font = VfntFont::parse(&font_bytes).unwrap();
        let mut data = [0u8; 1];
        let mut target = target(8, 1, 1, &mut data);

        assert_eq!(
            GlyphBitmapRenderer::render_glyph_id(
                &mut target,
                &font,
                99,
                GlyphPoint { x: 0, y: 0 },
                GlyphBlitMode::Transparent,
            ),
            Err(GlyphRenderError::MissingGlyph)
        );
    }

    #[test]
    fn render_glyph_id_uses_font_bitmap_format() {
        let font_bytes = synthetic_font_bytes(BITMAP_FORMAT_ONE_BPP);
        let font = VfntFont::parse(&font_bytes).unwrap();
        let mut data = [0u8; 1];
        let mut target = target(8, 1, 1, &mut data);

        GlyphBitmapRenderer::render_glyph_id(
            &mut target,
            &font,
            7,
            GlyphPoint { x: 2, y: 0 },
            GlyphBlitMode::Transparent,
        )
        .unwrap();

        assert!(!target.pixel(1, 0));
        assert!(target.pixel(2, 0));
    }
}
