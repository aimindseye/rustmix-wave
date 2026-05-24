//! Read-only VFNT asset loading and catalog binding.
//!
//! This module parses borrowed font bytes into semantic loaded faces and
//! selects a preferred face by script. It does not scan storage, render glyphs,
//! or shape text.

use super::font_assets::{VfntFont, VfntParseError};
use super::script::ScriptClass;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FontAssetRef<'a> {
    pub name: &'a str,
    pub bytes: &'a [u8],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LoadedFontFace<'a> {
    pub name: &'a str,
    pub script: ScriptClass,
    pub font: VfntFont<'a>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FontAssetReadError {
    Parse(VfntParseError),
    NoAssets,
    DuplicateScript,
    MissingPreferredFont,
}

impl From<VfntParseError> for FontAssetReadError {
    fn from(err: VfntParseError) -> Self {
        Self::Parse(err)
    }
}

pub trait FontAssetReader<'a> {
    fn load_face(&self, asset: FontAssetRef<'a>) -> Result<LoadedFontFace<'a>, FontAssetReadError>;
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StaticFontAssetReader;

impl<'a> FontAssetReader<'a> for StaticFontAssetReader {
    fn load_face(&self, asset: FontAssetRef<'a>) -> Result<LoadedFontFace<'a>, FontAssetReadError> {
        let font = VfntFont::parse(asset.bytes)?;
        let script = font.header().script;

        Ok(LoadedFontFace {
            name: asset.name,
            script,
            font,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LoadedFontSet<'faces, 'font> {
    faces: &'faces [LoadedFontFace<'font>],
}

impl<'faces, 'font> LoadedFontSet<'faces, 'font> {
    pub const fn new(faces: &'faces [LoadedFontFace<'font>]) -> Self {
        Self { faces }
    }

    pub const fn faces(&self) -> &'faces [LoadedFontFace<'font>] {
        self.faces
    }

    pub const fn is_empty(&self) -> bool {
        self.faces.is_empty()
    }

    pub fn validate(self) -> Result<Self, FontAssetReadError> {
        if self.faces.is_empty() {
            return Err(FontAssetReadError::NoAssets);
        }

        let mut left = 0usize;
        while left < self.faces.len() {
            let mut right = left + 1;
            while right < self.faces.len() {
                if self.faces[left].script == self.faces[right].script {
                    return Err(FontAssetReadError::DuplicateScript);
                }
                right += 1;
            }
            left += 1;
        }

        Ok(self)
    }

    pub fn font_for_script(&self, script: ScriptClass) -> Option<&'faces LoadedFontFace<'font>> {
        select_font_for_script(script, self.faces)
    }

    pub fn fallback_font(&self) -> Option<&'faces LoadedFontFace<'font>> {
        fallback_font(self.faces)
    }
}

pub trait LoadedFontLookup<'faces, 'font> {
    fn font_for_script(&self, script: ScriptClass) -> Option<&'faces LoadedFontFace<'font>>;
    fn fallback_font(&self) -> Option<&'faces LoadedFontFace<'font>>;
}

impl<'faces, 'font> LoadedFontLookup<'faces, 'font> for LoadedFontSet<'faces, 'font> {
    fn font_for_script(&self, script: ScriptClass) -> Option<&'faces LoadedFontFace<'font>> {
        LoadedFontSet::font_for_script(self, script)
    }

    fn fallback_font(&self) -> Option<&'faces LoadedFontFace<'font>> {
        LoadedFontSet::fallback_font(self)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FontFallbackSelection<'faces, 'font> {
    pub requested: ScriptClass,
    pub face: &'faces LoadedFontFace<'font>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FontCatalogBinding<'faces, 'font> {
    loaded: LoadedFontSet<'faces, 'font>,
}

impl<'faces, 'font> FontCatalogBinding<'faces, 'font> {
    pub const fn new(faces: &'faces [LoadedFontFace<'font>]) -> Self {
        Self {
            loaded: LoadedFontSet::new(faces),
        }
    }

    pub const fn loaded(&self) -> LoadedFontSet<'faces, 'font> {
        self.loaded
    }

    pub fn select(&self, script: ScriptClass) -> Option<FontFallbackSelection<'faces, 'font>> {
        self.loaded
            .font_for_script(script)
            .map(|face| FontFallbackSelection {
                requested: script,
                face,
            })
    }
}

pub fn select_font_for_script<'faces, 'font>(
    script: ScriptClass,
    loaded: &'faces [LoadedFontFace<'font>],
) -> Option<&'faces LoadedFontFace<'font>> {
    if loaded.is_empty() {
        return None;
    }

    if matches!(script, ScriptClass::Unknown | ScriptClass::Common) {
        return fallback_font(loaded);
    }

    find_script(loaded, script).or_else(|| fallback_font(loaded))
}

fn fallback_font<'faces, 'font>(
    loaded: &'faces [LoadedFontFace<'font>],
) -> Option<&'faces LoadedFontFace<'font>> {
    find_script(loaded, ScriptClass::Latin).or_else(|| loaded.first())
}

fn find_script<'faces, 'font>(
    loaded: &'faces [LoadedFontFace<'font>],
    script: ScriptClass,
) -> Option<&'faces LoadedFontFace<'font>> {
    loaded.iter().find(|face| face.script == script)
}

#[cfg(test)]
mod tests {
    use super::{
        FontAssetReadError, FontAssetReader, FontAssetRef, FontCatalogBinding, LoadedFontFace,
        LoadedFontSet, StaticFontAssetReader, select_font_for_script,
    };
    use crate::rustmix_x4::text::ScriptClass;
    use crate::rustmix_x4::text::font_assets::{
        VFNT_GLYPH_BITMAP_LEN, VFNT_GLYPH_METRICS_LEN, VFNT_HEADER_LEN, VFNT_MAGIC, VFNT_VERSION,
        VfntParseError,
    };

    const SCRIPT_LATIN: u16 = 1;
    const SCRIPT_DEVANAGARI: u16 = 2;
    const SCRIPT_GUJARATI: u16 = 3;
    const BITMAP_FORMAT_ONE_BPP: u16 = 1;

    fn push_u16(data: &mut Vec<u8>, value: u16) {
        data.extend_from_slice(&value.to_le_bytes());
    }

    fn push_i16(data: &mut Vec<u8>, value: i16) {
        data.extend_from_slice(&value.to_le_bytes());
    }

    fn push_u32(data: &mut Vec<u8>, value: u32) {
        data.extend_from_slice(&value.to_le_bytes());
    }

    fn synthetic_font_bytes(script: u16) -> Vec<u8> {
        let glyph_count = 1usize;
        let metrics_offset = VFNT_HEADER_LEN;
        let bitmap_index_offset = metrics_offset + glyph_count * VFNT_GLYPH_METRICS_LEN;
        let bitmap_data_offset = bitmap_index_offset + glyph_count * VFNT_GLYPH_BITMAP_LEN;
        let bitmap_data = [0xAA, 0x55];

        let mut data = Vec::new();
        data.extend_from_slice(&VFNT_MAGIC);
        push_u16(&mut data, VFNT_VERSION);
        push_u16(&mut data, VFNT_HEADER_LEN as u16);
        push_u32(&mut data, 0);
        push_u16(&mut data, 18);
        push_u16(&mut data, 24);
        push_i16(&mut data, 18);
        push_i16(&mut data, -6);
        push_u32(&mut data, glyph_count as u32);
        push_u32(&mut data, metrics_offset as u32);
        push_u32(&mut data, bitmap_index_offset as u32);
        push_u32(&mut data, bitmap_data_offset as u32);
        push_u32(&mut data, bitmap_data.len() as u32);
        push_u16(&mut data, script);
        push_u16(&mut data, BITMAP_FORMAT_ONE_BPP);

        push_u32(&mut data, 7);
        push_i16(&mut data, 10);
        push_i16(&mut data, 0);
        push_i16(&mut data, 1);
        push_i16(&mut data, 12);
        push_u16(&mut data, 8);
        push_u16(&mut data, 12);

        push_u32(&mut data, 7);
        push_u32(&mut data, 0);
        push_u32(&mut data, bitmap_data.len() as u32);
        push_u16(&mut data, 1);
        push_u16(&mut data, 0);

        data.extend_from_slice(&bitmap_data);
        data
    }

    fn load<'a>(name: &'a str, bytes: &'a [u8]) -> LoadedFontFace<'a> {
        StaticFontAssetReader
            .load_face(FontAssetRef { name, bytes })
            .unwrap()
    }

    #[test]
    fn loads_latin_vfnt_face_from_bytes() {
        let bytes = synthetic_font_bytes(SCRIPT_LATIN);
        let face = load("latin", &bytes);

        assert_eq!(face.name, "latin");
        assert_eq!(face.script, ScriptClass::Latin);
        assert_eq!(face.font.glyph_count(), 1);
    }

    #[test]
    fn loads_devanagari_vfnt_face_from_bytes() {
        let bytes = synthetic_font_bytes(SCRIPT_DEVANAGARI);
        let face = load("devanagari", &bytes);

        assert_eq!(face.script, ScriptClass::Devanagari);
        assert_eq!(face.font.glyph(7).unwrap().bitmap_data, &[0xAA, 0x55]);
    }

    #[test]
    fn loads_gujarati_vfnt_face_from_bytes() {
        let bytes = synthetic_font_bytes(SCRIPT_GUJARATI);
        let face = load("gujarati", &bytes);

        assert_eq!(face.script, ScriptClass::Gujarati);
    }

    #[test]
    fn rejects_invalid_vfnt_asset() {
        let err = StaticFontAssetReader
            .load_face(FontAssetRef {
                name: "invalid",
                bytes: &[],
            })
            .unwrap_err();

        assert_eq!(
            err,
            FontAssetReadError::Parse(VfntParseError::TruncatedHeader)
        );
    }

    #[test]
    fn selects_exact_script_font() {
        let latin_bytes = synthetic_font_bytes(SCRIPT_LATIN);
        let devanagari_bytes = synthetic_font_bytes(SCRIPT_DEVANAGARI);
        let faces = [
            load("latin", &latin_bytes),
            load("devanagari", &devanagari_bytes),
        ];
        let binding = FontCatalogBinding::new(&faces);

        let selected = binding.select(ScriptClass::Devanagari).unwrap();
        assert_eq!(selected.requested, ScriptClass::Devanagari);
        assert_eq!(selected.face.name, "devanagari");
    }

    #[test]
    fn falls_back_to_latin_for_unknown_script() {
        let latin_bytes = synthetic_font_bytes(SCRIPT_LATIN);
        let gujarati_bytes = synthetic_font_bytes(SCRIPT_GUJARATI);
        let faces = [
            load("gujarati", &gujarati_bytes),
            load("latin", &latin_bytes),
        ];

        let selected = select_font_for_script(ScriptClass::Unknown, &faces).unwrap();
        assert_eq!(selected.name, "latin");
    }

    #[test]
    fn falls_back_to_latin_when_devanagari_missing() {
        let latin_bytes = synthetic_font_bytes(SCRIPT_LATIN);
        let gujarati_bytes = synthetic_font_bytes(SCRIPT_GUJARATI);
        let faces = [
            load("gujarati", &gujarati_bytes),
            load("latin", &latin_bytes),
        ];

        let selected = select_font_for_script(ScriptClass::Devanagari, &faces).unwrap();
        assert_eq!(selected.name, "latin");
    }

    #[test]
    fn falls_back_to_first_available_when_latin_missing() {
        let gujarati_bytes = synthetic_font_bytes(SCRIPT_GUJARATI);
        let faces = [load("gujarati", &gujarati_bytes)];

        let selected = select_font_for_script(ScriptClass::Devanagari, &faces).unwrap();
        assert_eq!(selected.name, "gujarati");
    }

    #[test]
    fn returns_none_when_no_fonts_loaded() {
        let faces = [];
        let set = LoadedFontSet::new(&faces);

        assert!(set.is_empty());
        assert!(set.font_for_script(ScriptClass::Latin).is_none());
        assert!(set.validate().is_err());
    }

    #[test]
    fn loaded_face_borrows_original_asset_bytes() {
        let bytes = synthetic_font_bytes(SCRIPT_LATIN);
        let face = load("latin", &bytes);

        assert_eq!(face.font.data().as_ptr(), bytes.as_ptr());
        assert_eq!(face.font.data().len(), bytes.len());
    }

    #[test]
    fn duplicate_script_validation_reports_error() {
        let first_bytes = synthetic_font_bytes(SCRIPT_LATIN);
        let second_bytes = synthetic_font_bytes(SCRIPT_LATIN);
        let faces = [
            load("latin-a", &first_bytes),
            load("latin-b", &second_bytes),
        ];

        assert_eq!(
            LoadedFontSet::new(&faces).validate(),
            Err(FontAssetReadError::DuplicateScript)
        );
    }
}
