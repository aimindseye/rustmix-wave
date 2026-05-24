//! SD font catalog contract for reader-installed downloadable fonts.
//!
//! This model is intentionally metadata-only. The first SD-font slice adds
//! a stable manifest contract and validators without changing the active
//! reader font renderer.

use heapless::String;

pub const SD_FONT_ROOT_DIR: &str = "FONTS";
pub const SD_FONT_ROOT_PATH: &str = "/RUSTMIX/FONTS";
pub const SD_FONT_MANIFEST_FILE: &str = "MANIFEST.TXT";
pub const SD_FONT_MANIFEST_PATH: &str = "/RUSTMIX/FONTS/MANIFEST.TXT";
pub const SD_FONT_FILE_EXTENSION: &str = "VFN";
pub const SD_FONT_MAX_FILE_BYTES: u32 = 512 * 1024;
pub const SD_FONT_MAX_GLYPHS: u16 = 2048;
pub const SD_FONT_ID_MAX: usize = 8;
pub const SD_FONT_DISPLAY_NAME_MAX: usize = 32;
pub const SD_FONT_PATH_MAX: usize = 12;
pub const SD_FONT_CRC_HEX_LEN: usize = 8;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SdFontScriptModel {
    Latin,
    Devanagari,
    Gujarati,
    Symbols,
}

impl SdFontScriptModel {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Latin => "Latin",
            Self::Devanagari => "Devanagari",
            Self::Gujarati => "Gujarati",
            Self::Symbols => "Symbols",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        if value.eq_ignore_ascii_case("latin") {
            Some(Self::Latin)
        } else if value.eq_ignore_ascii_case("devanagari") {
            Some(Self::Devanagari)
        } else if value.eq_ignore_ascii_case("gujarati") {
            Some(Self::Gujarati)
        } else if value.eq_ignore_ascii_case("symbols") {
            Some(Self::Symbols)
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SdFontStyleModel {
    Regular,
    Bold,
    Italic,
    BoldItalic,
}

impl SdFontStyleModel {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Regular => "Regular",
            Self::Bold => "Bold",
            Self::Italic => "Italic",
            Self::BoldItalic => "BoldItalic",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        if value.eq_ignore_ascii_case("regular") {
            Some(Self::Regular)
        } else if value.eq_ignore_ascii_case("bold") {
            Some(Self::Bold)
        } else if value.eq_ignore_ascii_case("italic") {
            Some(Self::Italic)
        } else if value.eq_ignore_ascii_case("bolditalic")
            || value.eq_ignore_ascii_case("bold_italic")
            || value.eq_ignore_ascii_case("bold-italic")
        {
            Some(Self::BoldItalic)
        } else {
            None
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SdFontCatalogRecordModel {
    pub font_id: String<SD_FONT_ID_MAX>,
    pub display_name: String<SD_FONT_DISPLAY_NAME_MAX>,
    pub script: SdFontScriptModel,
    pub style: SdFontStyleModel,
    pub pixel_size: u8,
    pub file_name: String<SD_FONT_PATH_MAX>,
    pub file_size_bytes: u32,
    pub crc32_hex: String<SD_FONT_CRC_HEX_LEN>,
    pub glyph_count: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SdFontCatalogErrorModel {
    EmptyLine,
    WrongRecordKind,
    WrongFieldCount,
    UnsafeFontId,
    DisplayNameTooLong,
    UnknownScript,
    UnknownStyle,
    InvalidPixelSize,
    UnsafeFileName,
    InvalidFileSize,
    FileTooLarge,
    InvalidCrc32,
    InvalidGlyphCount,
    TooManyGlyphs,
}

pub fn parse_sd_font_manifest_line(
    line: &str,
) -> Result<Option<SdFontCatalogRecordModel>, SdFontCatalogErrorModel> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return Ok(None);
    }

    let mut fields = trimmed.split('|');
    let kind = fields
        .next()
        .ok_or(SdFontCatalogErrorModel::WrongFieldCount)?;
    let font_id = fields
        .next()
        .ok_or(SdFontCatalogErrorModel::WrongFieldCount)?;
    let display_name = fields
        .next()
        .ok_or(SdFontCatalogErrorModel::WrongFieldCount)?;
    let script = fields
        .next()
        .ok_or(SdFontCatalogErrorModel::WrongFieldCount)?;
    let style = fields
        .next()
        .ok_or(SdFontCatalogErrorModel::WrongFieldCount)?;
    let pixel_size = fields
        .next()
        .ok_or(SdFontCatalogErrorModel::WrongFieldCount)?;
    let file_name = fields
        .next()
        .ok_or(SdFontCatalogErrorModel::WrongFieldCount)?;
    let file_size = fields
        .next()
        .ok_or(SdFontCatalogErrorModel::WrongFieldCount)?;
    let crc32_hex = fields
        .next()
        .ok_or(SdFontCatalogErrorModel::WrongFieldCount)?;
    let glyph_count = fields
        .next()
        .ok_or(SdFontCatalogErrorModel::WrongFieldCount)?;
    if fields.next().is_some() {
        return Err(SdFontCatalogErrorModel::WrongFieldCount);
    }

    if kind != "FONT" {
        return Err(SdFontCatalogErrorModel::WrongRecordKind);
    }
    if !is_safe_sd_font_id(font_id) {
        return Err(SdFontCatalogErrorModel::UnsafeFontId);
    }
    let script = SdFontScriptModel::parse(script).ok_or(SdFontCatalogErrorModel::UnknownScript)?;
    let style = SdFontStyleModel::parse(style).ok_or(SdFontCatalogErrorModel::UnknownStyle)?;
    let pixel_size = pixel_size
        .parse::<u8>()
        .map_err(|_| SdFontCatalogErrorModel::InvalidPixelSize)?;
    if !(8..=36).contains(&pixel_size) {
        return Err(SdFontCatalogErrorModel::InvalidPixelSize);
    }
    if !is_safe_sd_font_file_name(file_name) {
        return Err(SdFontCatalogErrorModel::UnsafeFileName);
    }
    let file_size_bytes = file_size
        .parse::<u32>()
        .map_err(|_| SdFontCatalogErrorModel::InvalidFileSize)?;
    if file_size_bytes == 0 {
        return Err(SdFontCatalogErrorModel::InvalidFileSize);
    }
    if file_size_bytes > SD_FONT_MAX_FILE_BYTES {
        return Err(SdFontCatalogErrorModel::FileTooLarge);
    }
    if !is_crc32_hex(crc32_hex) {
        return Err(SdFontCatalogErrorModel::InvalidCrc32);
    }
    let glyph_count = glyph_count
        .parse::<u16>()
        .map_err(|_| SdFontCatalogErrorModel::InvalidGlyphCount)?;
    if glyph_count == 0 {
        return Err(SdFontCatalogErrorModel::InvalidGlyphCount);
    }
    if glyph_count > SD_FONT_MAX_GLYPHS {
        return Err(SdFontCatalogErrorModel::TooManyGlyphs);
    }

    let mut font_id_buf = String::<SD_FONT_ID_MAX>::new();
    font_id_buf
        .push_str(font_id)
        .map_err(|_| SdFontCatalogErrorModel::UnsafeFontId)?;
    let mut display_name_buf = String::<SD_FONT_DISPLAY_NAME_MAX>::new();
    display_name_buf
        .push_str(display_name)
        .map_err(|_| SdFontCatalogErrorModel::DisplayNameTooLong)?;
    let mut file_name_buf = String::<SD_FONT_PATH_MAX>::new();
    file_name_buf
        .push_str(file_name)
        .map_err(|_| SdFontCatalogErrorModel::UnsafeFileName)?;
    let mut crc32_buf = String::<SD_FONT_CRC_HEX_LEN>::new();
    crc32_buf
        .push_str(crc32_hex)
        .map_err(|_| SdFontCatalogErrorModel::InvalidCrc32)?;

    Ok(Some(SdFontCatalogRecordModel {
        font_id: font_id_buf,
        display_name: display_name_buf,
        script,
        style,
        pixel_size,
        file_name: file_name_buf,
        file_size_bytes,
        crc32_hex: crc32_buf,
        glyph_count,
    }))
}

pub fn is_safe_sd_font_id(value: &str) -> bool {
    if value.is_empty() || value.len() > SD_FONT_ID_MAX {
        return false;
    }
    value
        .bytes()
        .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
}

pub fn is_safe_sd_font_file_name(value: &str) -> bool {
    let Some((base, ext)) = value.rsplit_once('.') else {
        return false;
    };
    if !ext.eq_ignore_ascii_case(SD_FONT_FILE_EXTENSION) {
        return false;
    }
    is_safe_sd_font_id(base)
}

pub fn is_crc32_hex(value: &str) -> bool {
    value.len() == SD_FONT_CRC_HEX_LEN && value.bytes().all(|b| b.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::{
        SdFontCatalogErrorModel, SdFontScriptModel, SdFontStyleModel, is_safe_sd_font_file_name,
        parse_sd_font_manifest_line,
    };

    #[test]
    fn parses_font_manifest_record() {
        let rec = parse_sd_font_manifest_line(
            "FONT|BITTR14|Bitter|Latin|Regular|14|BITTR14.VFN|32768|A1B2C3D4|512",
        )
        .unwrap()
        .unwrap();
        assert_eq!(rec.font_id.as_str(), "BITTR14");
        assert_eq!(rec.display_name.as_str(), "Bitter");
        assert_eq!(rec.script, SdFontScriptModel::Latin);
        assert_eq!(rec.style, SdFontStyleModel::Regular);
        assert_eq!(rec.pixel_size, 14);
        assert_eq!(rec.file_name.as_str(), "BITTR14.VFN");
        assert_eq!(rec.file_size_bytes, 32768);
        assert_eq!(rec.crc32_hex.as_str(), "A1B2C3D4");
        assert_eq!(rec.glyph_count, 512);
    }

    #[test]
    fn rejects_unsafe_names_and_paths() {
        assert!(!is_safe_sd_font_file_name("Bitter14.vfnt"));
        assert!(!is_safe_sd_font_file_name("BITTER14.TTF"));
        assert!(!is_safe_sd_font_file_name("FONTS/BIT14.VFN"));
        assert!(is_safe_sd_font_file_name("BITTR14.VFN"));
    }

    #[test]
    fn ignores_blank_and_comment_lines() {
        assert_eq!(parse_sd_font_manifest_line("").unwrap(), None);
        assert_eq!(parse_sd_font_manifest_line("# comment").unwrap(), None);
    }

    #[test]
    fn validates_size_crc_and_glyph_count() {
        assert_eq!(
            parse_sd_font_manifest_line(
                "FONT|BITTR14|Bitter|Latin|Regular|14|BITTR14.VFN|0|A1B2C3D4|512"
            )
            .unwrap_err(),
            SdFontCatalogErrorModel::InvalidFileSize
        );
        assert_eq!(
            parse_sd_font_manifest_line(
                "FONT|BITTR14|Bitter|Latin|Regular|14|BITTR14.VFN|32768|bad|512"
            )
            .unwrap_err(),
            SdFontCatalogErrorModel::InvalidCrc32
        );
        assert_eq!(
            parse_sd_font_manifest_line(
                "FONT|BITTR14|Bitter|Latin|Regular|14|BITTR14.VFN|32768|A1B2C3D4|0"
            )
            .unwrap_err(),
            SdFontCatalogErrorModel::InvalidGlyphCount
        );
    }
}
