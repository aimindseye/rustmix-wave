//! Compact bitmap font asset contracts.
//!
//! These types describe the Rustmix font asset layout without parsing files or
//! drawing glyph bitmaps. Runtime loading and rendering are intentionally left
//! to later integration work.

use super::font_catalog::FontDescriptor;
use super::script::ScriptClass;
use core::convert::TryFrom;

pub const VFNT_MAGIC: [u8; 4] = *b"VFNT";
pub const VFNT_VERSION: u16 = 1;
pub const VFNT_HEADER_LEN: usize = 44;
pub const VFNT_GLYPH_METRICS_LEN: usize = 16;
pub const VFNT_GLYPH_BITMAP_LEN: usize = 16;

const SCRIPT_COMMON: u16 = 0;
const SCRIPT_LATIN: u16 = 1;
const SCRIPT_DEVANAGARI: u16 = 2;
const SCRIPT_GUJARATI: u16 = 3;
const SCRIPT_UNKNOWN: u16 = u16::MAX;

const BITMAP_FORMAT_ONE_BPP: u16 = 1;
const BITMAP_FORMAT_TWO_BPP: u16 = 2;
const BITMAP_FORMAT_FOUR_BPP: u16 = 4;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(clippy::enum_variant_names)]
pub enum FontBitmapFormat {
    OneBpp,
    TwoBpp,
    FourBpp,
}

impl FontBitmapFormat {
    pub const fn bits_per_pixel(self) -> u8 {
        match self {
            Self::OneBpp => 1,
            Self::TwoBpp => 2,
            Self::FourBpp => 4,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VfntParseError {
    TruncatedHeader,
    InvalidMagic,
    UnsupportedVersion,
    UnsupportedBitmapFormat,
    InvalidHeaderLength,
    InvalidTableOffset,
    InvalidTableLength,
    InvalidGlyphCount,
    InvalidBitmapRange,
    InvalidMetricsRecord,
    InvalidBitmapRecord,
    GlyphNotFound,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VfntHeader {
    pub magic: [u8; 4],
    pub version: u16,
    pub header_len: u16,
    pub flags: u32,
    pub pixel_size: u16,
    pub line_height: u16,
    pub ascent: i16,
    pub descent: i16,
    pub glyph_count: u32,
    pub metrics_offset: u32,
    pub bitmap_index_offset: u32,
    pub bitmap_data_offset: u32,
    pub bitmap_data_len: u32,
    pub script: ScriptClass,
    pub bitmap_format: FontBitmapFormat,
}

impl VfntHeader {
    pub const fn uses_expected_magic(self) -> bool {
        self.magic[0] == VFNT_MAGIC[0]
            && self.magic[1] == VFNT_MAGIC[1]
            && self.magic[2] == VFNT_MAGIC[2]
            && self.magic[3] == VFNT_MAGIC[3]
    }

    pub const fn uses_supported_version(self) -> bool {
        self.version == VFNT_VERSION
    }

    pub const fn is_supported(self) -> bool {
        self.uses_expected_magic()
            && self.uses_supported_version()
            && self.header_len > 0
            && self.pixel_size > 0
            && self.line_height > 0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VfntGlyphMetrics {
    pub glyph_id: u32,
    pub advance_x: i16,
    pub advance_y: i16,
    pub bearing_x: i16,
    pub bearing_y: i16,
    pub width: u16,
    pub height: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VfntGlyphBitmap {
    pub glyph_id: u32,
    pub offset: u32,
    pub len: u32,
    pub row_stride: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VfntAssetInfo {
    pub descriptor: FontDescriptor,
    pub header: VfntHeader,
}

impl VfntAssetInfo {
    pub fn is_supported(self) -> bool {
        self.header.is_supported() && self.descriptor.script == self.header.script
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VfntGlyph<'a> {
    pub metrics: VfntGlyphMetrics,
    pub bitmap: VfntGlyphBitmap,
    pub bitmap_data: &'a [u8],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VfntFont<'a> {
    data: &'a [u8],
    header: VfntHeader,
    metrics_table: &'a [u8],
    bitmap_index_table: &'a [u8],
    bitmap_data: &'a [u8],
}

impl<'a> VfntFont<'a> {
    pub fn parse(data: &'a [u8]) -> Result<Self, VfntParseError> {
        let header = parse_header(data)?;

        if header.glyph_count == 0 {
            return Err(VfntParseError::InvalidGlyphCount);
        }

        let glyph_count =
            usize::try_from(header.glyph_count).map_err(|_| VfntParseError::InvalidGlyphCount)?;
        let metrics_len = glyph_count
            .checked_mul(VFNT_GLYPH_METRICS_LEN)
            .ok_or(VfntParseError::InvalidGlyphCount)?;
        let bitmap_index_len = glyph_count
            .checked_mul(VFNT_GLYPH_BITMAP_LEN)
            .ok_or(VfntParseError::InvalidGlyphCount)?;

        let metrics_table = checked_table_slice(data, header.metrics_offset, metrics_len)?;
        let bitmap_index_table =
            checked_table_slice(data, header.bitmap_index_offset, bitmap_index_len)?;
        let bitmap_data_len = usize::try_from(header.bitmap_data_len)
            .map_err(|_| VfntParseError::InvalidTableLength)?;
        let bitmap_data = checked_table_slice(data, header.bitmap_data_offset, bitmap_data_len)?;

        for idx in 0..glyph_count {
            let record = bitmap_record_at(bitmap_index_table, idx)?;
            checked_bitmap_range(bitmap_data, record.offset, record.len)?;
        }

        Ok(Self {
            data,
            header,
            metrics_table,
            bitmap_index_table,
            bitmap_data,
        })
    }

    pub const fn header(&self) -> &VfntHeader {
        &self.header
    }

    pub const fn glyph_count(&self) -> u32 {
        self.header.glyph_count
    }

    pub const fn data(&self) -> &'a [u8] {
        self.data
    }

    pub fn metrics_for_glyph(&self, glyph_id: u32) -> Result<VfntGlyphMetrics, VfntParseError> {
        let glyph_count = usize::try_from(self.header.glyph_count)
            .map_err(|_| VfntParseError::InvalidGlyphCount)?;
        for idx in 0..glyph_count {
            let metrics = metrics_record_at(self.metrics_table, idx)?;
            if metrics.glyph_id == glyph_id {
                return Ok(metrics);
            }
        }
        Err(VfntParseError::GlyphNotFound)
    }

    pub fn bitmap_for_glyph(&self, glyph_id: u32) -> Result<VfntGlyphBitmap, VfntParseError> {
        let glyph_count = usize::try_from(self.header.glyph_count)
            .map_err(|_| VfntParseError::InvalidGlyphCount)?;
        for idx in 0..glyph_count {
            let bitmap = bitmap_record_at(self.bitmap_index_table, idx)?;
            if bitmap.glyph_id == glyph_id {
                checked_bitmap_range(self.bitmap_data, bitmap.offset, bitmap.len)?;
                return Ok(bitmap);
            }
        }
        Err(VfntParseError::GlyphNotFound)
    }

    pub fn glyph(&self, glyph_id: u32) -> Result<VfntGlyph<'a>, VfntParseError> {
        let metrics = self.metrics_for_glyph(glyph_id)?;
        let bitmap = self.bitmap_for_glyph(glyph_id)?;
        let bitmap_data = checked_bitmap_range(self.bitmap_data, bitmap.offset, bitmap.len)?;

        Ok(VfntGlyph {
            metrics,
            bitmap,
            bitmap_data,
        })
    }
}

fn parse_header(data: &[u8]) -> Result<VfntHeader, VfntParseError> {
    if data.len() < VFNT_HEADER_LEN {
        return Err(VfntParseError::TruncatedHeader);
    }

    let magic = [data[0], data[1], data[2], data[3]];
    if magic != VFNT_MAGIC {
        return Err(VfntParseError::InvalidMagic);
    }

    let version = read_u16(data, 4).ok_or(VfntParseError::TruncatedHeader)?;
    if version != VFNT_VERSION {
        return Err(VfntParseError::UnsupportedVersion);
    }

    let header_len = read_u16(data, 6).ok_or(VfntParseError::TruncatedHeader)?;
    if usize::from(header_len) < VFNT_HEADER_LEN || usize::from(header_len) > data.len() {
        return Err(VfntParseError::InvalidHeaderLength);
    }

    let bitmap_format_code = read_u16(data, 42).ok_or(VfntParseError::TruncatedHeader)?;
    let bitmap_format = decode_bitmap_format(bitmap_format_code)?;

    let header = VfntHeader {
        magic,
        version,
        header_len,
        flags: read_u32(data, 8).ok_or(VfntParseError::TruncatedHeader)?,
        pixel_size: read_u16(data, 12).ok_or(VfntParseError::TruncatedHeader)?,
        line_height: read_u16(data, 14).ok_or(VfntParseError::TruncatedHeader)?,
        ascent: read_i16(data, 16).ok_or(VfntParseError::TruncatedHeader)?,
        descent: read_i16(data, 18).ok_or(VfntParseError::TruncatedHeader)?,
        glyph_count: read_u32(data, 20).ok_or(VfntParseError::TruncatedHeader)?,
        metrics_offset: read_u32(data, 24).ok_or(VfntParseError::TruncatedHeader)?,
        bitmap_index_offset: read_u32(data, 28).ok_or(VfntParseError::TruncatedHeader)?,
        bitmap_data_offset: read_u32(data, 32).ok_or(VfntParseError::TruncatedHeader)?,
        bitmap_data_len: read_u32(data, 36).ok_or(VfntParseError::TruncatedHeader)?,
        script: decode_script(read_u16(data, 40).ok_or(VfntParseError::TruncatedHeader)?),
        bitmap_format,
    };

    if !header.is_supported() {
        return Err(VfntParseError::InvalidHeaderLength);
    }

    Ok(header)
}

fn checked_table_slice(data: &[u8], offset: u32, len: usize) -> Result<&[u8], VfntParseError> {
    let start = usize::try_from(offset).map_err(|_| VfntParseError::InvalidTableOffset)?;
    if start > data.len() {
        return Err(VfntParseError::InvalidTableOffset);
    }
    let end = start
        .checked_add(len)
        .ok_or(VfntParseError::InvalidTableLength)?;
    if end > data.len() {
        return Err(VfntParseError::InvalidTableLength);
    }
    Ok(&data[start..end])
}

fn checked_bitmap_range(
    bitmap_data: &[u8],
    offset: u32,
    len: u32,
) -> Result<&[u8], VfntParseError> {
    let start = usize::try_from(offset).map_err(|_| VfntParseError::InvalidBitmapRange)?;
    let byte_len = usize::try_from(len).map_err(|_| VfntParseError::InvalidBitmapRange)?;
    let end = start
        .checked_add(byte_len)
        .ok_or(VfntParseError::InvalidBitmapRange)?;
    if end > bitmap_data.len() {
        return Err(VfntParseError::InvalidBitmapRange);
    }
    Ok(&bitmap_data[start..end])
}

fn metrics_record_at(table: &[u8], idx: usize) -> Result<VfntGlyphMetrics, VfntParseError> {
    let start = idx
        .checked_mul(VFNT_GLYPH_METRICS_LEN)
        .ok_or(VfntParseError::InvalidMetricsRecord)?;
    let end = start
        .checked_add(VFNT_GLYPH_METRICS_LEN)
        .ok_or(VfntParseError::InvalidMetricsRecord)?;
    let record = table
        .get(start..end)
        .ok_or(VfntParseError::InvalidMetricsRecord)?;

    Ok(VfntGlyphMetrics {
        glyph_id: read_u32(record, 0).ok_or(VfntParseError::InvalidMetricsRecord)?,
        advance_x: read_i16(record, 4).ok_or(VfntParseError::InvalidMetricsRecord)?,
        advance_y: read_i16(record, 6).ok_or(VfntParseError::InvalidMetricsRecord)?,
        bearing_x: read_i16(record, 8).ok_or(VfntParseError::InvalidMetricsRecord)?,
        bearing_y: read_i16(record, 10).ok_or(VfntParseError::InvalidMetricsRecord)?,
        width: read_u16(record, 12).ok_or(VfntParseError::InvalidMetricsRecord)?,
        height: read_u16(record, 14).ok_or(VfntParseError::InvalidMetricsRecord)?,
    })
}

fn bitmap_record_at(table: &[u8], idx: usize) -> Result<VfntGlyphBitmap, VfntParseError> {
    let start = idx
        .checked_mul(VFNT_GLYPH_BITMAP_LEN)
        .ok_or(VfntParseError::InvalidBitmapRecord)?;
    let end = start
        .checked_add(VFNT_GLYPH_BITMAP_LEN)
        .ok_or(VfntParseError::InvalidBitmapRecord)?;
    let record = table
        .get(start..end)
        .ok_or(VfntParseError::InvalidBitmapRecord)?;

    Ok(VfntGlyphBitmap {
        glyph_id: read_u32(record, 0).ok_or(VfntParseError::InvalidBitmapRecord)?,
        offset: read_u32(record, 4).ok_or(VfntParseError::InvalidBitmapRecord)?,
        len: read_u32(record, 8).ok_or(VfntParseError::InvalidBitmapRecord)?,
        row_stride: read_u16(record, 12).ok_or(VfntParseError::InvalidBitmapRecord)?,
    })
}

fn decode_script(code: u16) -> ScriptClass {
    match code {
        SCRIPT_COMMON => ScriptClass::Common,
        SCRIPT_LATIN => ScriptClass::Latin,
        SCRIPT_DEVANAGARI => ScriptClass::Devanagari,
        SCRIPT_GUJARATI => ScriptClass::Gujarati,
        SCRIPT_UNKNOWN => ScriptClass::Unknown,
        _ => ScriptClass::Unknown,
    }
}

fn decode_bitmap_format(code: u16) -> Result<FontBitmapFormat, VfntParseError> {
    match code {
        BITMAP_FORMAT_ONE_BPP => Ok(FontBitmapFormat::OneBpp),
        BITMAP_FORMAT_TWO_BPP => Ok(FontBitmapFormat::TwoBpp),
        BITMAP_FORMAT_FOUR_BPP => Ok(FontBitmapFormat::FourBpp),
        _ => Err(VfntParseError::UnsupportedBitmapFormat),
    }
}

fn read_u16(data: &[u8], offset: usize) -> Option<u16> {
    let bytes = data.get(offset..offset.checked_add(2)?)?;
    Some(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn read_i16(data: &[u8], offset: usize) -> Option<i16> {
    let bytes = data.get(offset..offset.checked_add(2)?)?;
    Some(i16::from_le_bytes([bytes[0], bytes[1]]))
}

fn read_u32(data: &[u8], offset: usize) -> Option<u32> {
    let bytes = data.get(offset..offset.checked_add(4)?)?;
    Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

#[cfg(test)]
mod tests {
    use super::{
        BITMAP_FORMAT_ONE_BPP, FontBitmapFormat, SCRIPT_DEVANAGARI, VFNT_GLYPH_BITMAP_LEN,
        VFNT_GLYPH_METRICS_LEN, VFNT_HEADER_LEN, VFNT_MAGIC, VFNT_VERSION, VfntFont,
        VfntGlyphBitmap, VfntGlyphMetrics, VfntHeader, VfntParseError,
    };
    use crate::rustmix_x4::text::ScriptClass;

    fn supported_header() -> VfntHeader {
        VfntHeader {
            magic: VFNT_MAGIC,
            version: VFNT_VERSION,
            header_len: VFNT_HEADER_LEN as u16,
            flags: 0,
            pixel_size: 22,
            line_height: 28,
            ascent: 22,
            descent: -6,
            glyph_count: 128,
            metrics_offset: 64,
            bitmap_index_offset: 2048,
            bitmap_data_offset: 4096,
            bitmap_data_len: 8192,
            script: ScriptClass::Devanagari,
            bitmap_format: FontBitmapFormat::OneBpp,
        }
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

    fn patch_u16(data: &mut [u8], offset: usize, value: u16) {
        data[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }

    fn patch_u32(data: &mut [u8], offset: usize, value: u32) {
        data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn push_header(
        data: &mut Vec<u8>,
        glyph_count: u32,
        metrics_offset: u32,
        bitmap_index_offset: u32,
        bitmap_data_offset: u32,
        bitmap_data_len: u32,
    ) {
        data.extend_from_slice(&VFNT_MAGIC);
        push_u16(data, VFNT_VERSION);
        push_u16(data, VFNT_HEADER_LEN as u16);
        push_u32(data, 0);
        push_u16(data, 22);
        push_u16(data, 28);
        push_i16(data, 22);
        push_i16(data, -6);
        push_u32(data, glyph_count);
        push_u32(data, metrics_offset);
        push_u32(data, bitmap_index_offset);
        push_u32(data, bitmap_data_offset);
        push_u32(data, bitmap_data_len);
        push_u16(data, SCRIPT_DEVANAGARI);
        push_u16(data, BITMAP_FORMAT_ONE_BPP);
    }

    fn push_metrics(data: &mut Vec<u8>, glyph_id: u32, advance_x: i16, width: u16, height: u16) {
        push_u32(data, glyph_id);
        push_i16(data, advance_x);
        push_i16(data, 0);
        push_i16(data, 1);
        push_i16(data, 18);
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

    fn valid_font_bytes() -> Vec<u8> {
        let glyph_count = 2usize;
        let metrics_offset = VFNT_HEADER_LEN;
        let bitmap_index_offset = metrics_offset + glyph_count * VFNT_GLYPH_METRICS_LEN;
        let bitmap_data_offset = bitmap_index_offset + glyph_count * VFNT_GLYPH_BITMAP_LEN;
        let bitmap_data = [0x11, 0x22, 0x33, 0x44, 0x55];

        let mut data = Vec::new();
        push_header(
            &mut data,
            glyph_count as u32,
            metrics_offset as u32,
            bitmap_index_offset as u32,
            bitmap_data_offset as u32,
            bitmap_data.len() as u32,
        );
        push_metrics(&mut data, 11, 12, 8, 16);
        push_metrics(&mut data, 22, 14, 9, 18);
        push_bitmap(&mut data, 11, 0, 2, 1);
        push_bitmap(&mut data, 22, 2, 3, 1);
        data.extend_from_slice(&bitmap_data);

        data
    }

    #[test]
    fn vfnt_header_rejects_wrong_magic() {
        let mut header = supported_header();
        header.magic = *b"FONT";
        assert!(!header.uses_expected_magic());
        assert!(!header.is_supported());
    }

    #[test]
    fn vfnt_header_accepts_supported_contract() {
        let header = supported_header();
        assert!(header.uses_expected_magic());
        assert!(header.uses_supported_version());
        assert!(header.is_supported());
        assert_eq!(header.bitmap_format.bits_per_pixel(), 1);
    }

    #[test]
    fn glyph_contracts_preserve_bitmap_offsets_and_metrics() {
        let metrics = VfntGlyphMetrics {
            glyph_id: 42,
            advance_x: 12,
            advance_y: 0,
            bearing_x: 1,
            bearing_y: 18,
            width: 10,
            height: 20,
        };
        let bitmap = VfntGlyphBitmap {
            glyph_id: metrics.glyph_id,
            offset: 256,
            len: 40,
            row_stride: 2,
        };

        assert_eq!(bitmap.glyph_id, metrics.glyph_id);
        assert_eq!(bitmap.row_stride, 2);
    }

    #[test]
    fn vfnt_parse_rejects_empty_input() {
        assert_eq!(VfntFont::parse(&[]), Err(VfntParseError::TruncatedHeader));
    }

    #[test]
    fn vfnt_parse_rejects_wrong_magic() {
        let mut data = valid_font_bytes();
        data[0..4].copy_from_slice(b"FONT");

        assert_eq!(VfntFont::parse(&data), Err(VfntParseError::InvalidMagic));
    }

    #[test]
    fn vfnt_parse_rejects_unsupported_version() {
        let mut data = valid_font_bytes();
        patch_u16(&mut data, 4, VFNT_VERSION + 1);

        assert_eq!(
            VfntFont::parse(&data),
            Err(VfntParseError::UnsupportedVersion)
        );
    }

    #[test]
    fn vfnt_parse_rejects_unsupported_bitmap_format() {
        let mut data = valid_font_bytes();
        patch_u16(&mut data, 42, 9);

        assert_eq!(
            VfntFont::parse(&data),
            Err(VfntParseError::UnsupportedBitmapFormat)
        );
    }

    #[test]
    fn vfnt_parse_rejects_invalid_header_length() {
        let mut data = valid_font_bytes();
        patch_u16(&mut data, 6, (VFNT_HEADER_LEN - 1) as u16);

        assert_eq!(
            VfntFont::parse(&data),
            Err(VfntParseError::InvalidHeaderLength)
        );
    }

    #[test]
    fn vfnt_parse_rejects_short_metrics_table() {
        let mut data = valid_font_bytes();
        data.truncate(VFNT_HEADER_LEN + VFNT_GLYPH_METRICS_LEN);

        assert_eq!(
            VfntFont::parse(&data),
            Err(VfntParseError::InvalidTableLength)
        );
    }

    #[test]
    fn vfnt_parse_rejects_short_bitmap_index() {
        let mut data = valid_font_bytes();
        data.truncate(VFNT_HEADER_LEN + 2 * VFNT_GLYPH_METRICS_LEN + VFNT_GLYPH_BITMAP_LEN);

        assert_eq!(
            VfntFont::parse(&data),
            Err(VfntParseError::InvalidTableLength)
        );
    }

    #[test]
    fn vfnt_parse_rejects_table_offset_outside_data() {
        let mut data = valid_font_bytes();
        patch_u32(&mut data, 24, 9999);

        assert_eq!(
            VfntFont::parse(&data),
            Err(VfntParseError::InvalidTableOffset)
        );
    }

    #[test]
    fn vfnt_parse_rejects_bitmap_range_outside_data() {
        let mut data = valid_font_bytes();
        let second_bitmap_offset = VFNT_HEADER_LEN + 2 * VFNT_GLYPH_METRICS_LEN + 16;
        patch_u32(&mut data, second_bitmap_offset + 4, 4);
        patch_u32(&mut data, second_bitmap_offset + 8, 4);

        assert_eq!(
            VfntFont::parse(&data),
            Err(VfntParseError::InvalidBitmapRange)
        );
    }

    #[test]
    fn vfnt_lookup_returns_missing_for_unknown_glyph() {
        let data = valid_font_bytes();
        let font = VfntFont::parse(&data).unwrap();

        assert_eq!(
            font.metrics_for_glyph(99),
            Err(VfntParseError::GlyphNotFound)
        );
        assert_eq!(font.glyph(99), Err(VfntParseError::GlyphNotFound));
    }

    #[test]
    fn vfnt_lookup_returns_metrics_and_bitmap_for_known_glyph() {
        let data = valid_font_bytes();
        let font = VfntFont::parse(&data).unwrap();
        let glyph = font.glyph(22).unwrap();

        assert_eq!(glyph.metrics.glyph_id, 22);
        assert_eq!(glyph.metrics.advance_x, 14);
        assert_eq!(glyph.metrics.width, 9);
        assert_eq!(glyph.bitmap.glyph_id, 22);
        assert_eq!(glyph.bitmap.offset, 2);
        assert_eq!(glyph.bitmap.len, 3);
        assert_eq!(glyph.bitmap_data, &[0x33, 0x44, 0x55]);
    }

    #[test]
    fn vfnt_parse_accepts_valid_minimal_font() {
        let data = valid_font_bytes();
        let font = VfntFont::parse(&data).unwrap();

        assert_eq!(font.header().script, ScriptClass::Devanagari);
        assert_eq!(font.glyph_count(), 2);
        assert_eq!(font.data().len(), data.len());
        assert_eq!(font.bitmap_for_glyph(11).unwrap().len, 2);
    }
}
