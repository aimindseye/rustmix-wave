//! Contract-only glyph cache and prepared run types.
//!
//! This module defines lookup keys and prepared run records for future glyph
//! cache integration. It does not load fonts, allocate cache storage, or render
//! glyph bitmaps.

pub const VRUN_MAGIC: [u8; 4] = *b"VRUN";
pub const VRUN_VERSION: u16 = 1;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FontAssetId(pub u32);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GlyphCacheKey {
    pub font: FontAssetId,
    pub glyph_id: u32,
    pub pixel_size: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GlyphBitmapRef {
    pub offset: u32,
    pub len: u32,
    pub width: u16,
    pub height: u16,
    pub row_stride: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GlyphCacheStatus {
    Ready,
    MissingFont,
    MissingGlyph,
    UnsupportedFormat,
}

pub trait GlyphCacheLookup {
    fn lookup(&self, key: GlyphCacheKey) -> Result<GlyphBitmapRef, GlyphCacheStatus>;
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct EmptyGlyphCache;

impl GlyphCacheLookup for EmptyGlyphCache {
    fn lookup(&self, key: GlyphCacheKey) -> Result<GlyphBitmapRef, GlyphCacheStatus> {
        if key.font.0 == 0 {
            Err(GlyphCacheStatus::MissingFont)
        } else {
            Err(GlyphCacheStatus::MissingGlyph)
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PreparedTextRunKey {
    pub font: FontAssetId,
    pub source_hash: u32,
    pub pixel_size: u16,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PreparedRunId(pub u32);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VrunHeader {
    pub magic: [u8; 4],
    pub version: u16,
    pub run_count: u32,
    pub glyph_count: u32,
    pub cluster_count: u32,
}

impl VrunHeader {
    pub const fn uses_expected_magic(self) -> bool {
        self.magic[0] == VRUN_MAGIC[0]
            && self.magic[1] == VRUN_MAGIC[1]
            && self.magic[2] == VRUN_MAGIC[2]
            && self.magic[3] == VRUN_MAGIC[3]
    }

    pub const fn uses_supported_version(self) -> bool {
        self.version == VRUN_VERSION
    }

    pub const fn is_supported(self) -> bool {
        self.uses_expected_magic() && self.uses_supported_version()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PositionedGlyph {
    pub font: FontAssetId,
    pub glyph_id: u32,
    pub x: i16,
    pub y: i16,
    pub advance_x: i16,
    pub advance_y: i16,
    pub cluster: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TextCluster {
    pub source_start: u32,
    pub source_len: u32,
    pub first_glyph: u32,
    pub glyph_count: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PreparedGlyphRun<'a> {
    pub id: PreparedRunId,
    pub key: PreparedTextRunKey,
    pub glyphs: &'a [PositionedGlyph],
    pub clusters: &'a [TextCluster],
}

#[cfg(test)]
mod tests {
    use super::{
        EmptyGlyphCache, FontAssetId, GlyphCacheKey, GlyphCacheLookup, GlyphCacheStatus,
        VRUN_MAGIC, VRUN_VERSION, VrunHeader,
    };

    #[test]
    fn vrun_header_rejects_wrong_version() {
        let header = VrunHeader {
            magic: VRUN_MAGIC,
            version: VRUN_VERSION + 1,
            run_count: 1,
            glyph_count: 4,
            cluster_count: 2,
        };

        assert!(header.uses_expected_magic());
        assert!(!header.uses_supported_version());
        assert!(!header.is_supported());
    }

    #[test]
    fn empty_glyph_cache_reports_missing_glyph() {
        let cache = EmptyGlyphCache;
        let key = GlyphCacheKey {
            font: FontAssetId(7),
            glyph_id: 42,
            pixel_size: 22,
        };

        assert_eq!(cache.lookup(key), Err(GlyphCacheStatus::MissingGlyph));
    }
}
