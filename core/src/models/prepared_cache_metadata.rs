use core::fmt::Write as _;

use heapless::{String, Vec};
use serde::{Deserialize, Serialize};

use super::reader_viewport::ReaderViewportModel;
use super::state::ReaderOrientationModel;

pub const PREPARED_CACHE_ROOT_DIR: &str = "FCACHE";
pub const PREPARED_CACHE_ROOT_PATH: &str = "/FCACHE";
pub const PREPARED_META_FILE: &str = "META.TXT";
pub const PREPARED_FONTS_INDEX_FILE: &str = "FONTS.IDX";
pub const PREPARED_PAGES_INDEX_FILE: &str = "PAGES.IDX";
pub const PREPARED_BOOK_ID_LEN: usize = 8;
pub const PREPARED_BOOK_ID_MAX: usize = 16;
pub const PREPARED_SOURCE_MAX: usize = 128;
pub const PREPARED_CACHE_FILE_MAX: usize = 12;
pub const PREPARED_CACHE_PATH_MAX: usize = 64;
pub const PREPARED_MAX_PAGES: usize = 192;
pub const PREPARED_MAX_GLYPHS: usize = 1024;
pub const PREPARED_VRUN_MAGIC: [u8; 4] = *b"VRUN";
pub const PREPARED_VRUN_VERSION: u16 = 1;
pub const PREPARED_VRUN_HEADER_LEN: usize = 20;
pub const PREPARED_VRUN_GLYPH_RECORD_LEN: usize = 20;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PreparedCacheKindModel {
    Txt,
    Epub,
    Epu,
    #[default]
    Unknown,
}

impl PreparedCacheKindModel {
    pub fn from_source_path(source: &str) -> Self {
        match extension_ascii(source) {
            Some(ext) if eq_ascii_ignore_case(ext, "TXT") => Self::Txt,
            Some(ext) if eq_ascii_ignore_case(ext, "EPUB") => Self::Epub,
            Some(ext) if eq_ascii_ignore_case(ext, "EPU") => Self::Epu,
            _ => Self::Unknown,
        }
    }

    pub const fn is_prepared_reader_kind(self) -> bool {
        matches!(self, Self::Txt | Self::Epub | Self::Epu)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PreparedCacheErrorClassModel {
    #[default]
    Missing,
    InvalidMeta,
    MismatchedBook,
    InvalidIndex,
    MissingFont,
    InvalidFont,
    InvalidPage,
    TooLarge,
    Unsupported,
}

impl PreparedCacheErrorClassModel {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Missing => "MISSING",
            Self::InvalidMeta => "META",
            Self::MismatchedBook => "BOOK",
            Self::InvalidIndex => "INDEX",
            Self::MissingFont => "FONT_MISSING",
            Self::InvalidFont => "FONT",
            Self::InvalidPage => "PAGE",
            Self::TooLarge => "TOO_LARGE",
            Self::Unsupported => "UNSUPPORTED",
        }
    }

    pub const fn is_safe_fallback(self) -> bool {
        matches!(self, Self::Missing)
    }

    pub const fn is_diagnostic_worthy(self) -> bool {
        !self.is_safe_fallback()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PreparedCacheStatusModel {
    #[default]
    MissingFallback,
    Ready,
    Diagnostic(PreparedCacheErrorClassModel),
}

impl PreparedCacheStatusModel {
    pub const fn from_error(error: PreparedCacheErrorClassModel) -> Self {
        if error.is_safe_fallback() {
            Self::MissingFallback
        } else {
            Self::Diagnostic(error)
        }
    }

    pub const fn should_fallback_silently(self) -> bool {
        matches!(self, Self::MissingFallback)
    }

    pub const fn should_show_diagnostic(self) -> bool {
        matches!(self, Self::Diagnostic(_))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreparedCacheBookIdModel {
    pub value: String<PREPARED_BOOK_ID_MAX>,
}

impl PreparedCacheBookIdModel {
    pub fn new(value: &str) -> Option<Self> {
        if !is_valid_prepared_book_id(value) {
            return None;
        }
        let mut out = String::new();
        for b in value.bytes() {
            out.push((b as char).to_ascii_uppercase()).ok()?;
        }
        Some(Self { value: out })
    }

    pub fn from_u32(value: u32) -> Self {
        let mut out = String::new();
        let _ = write!(out, "{value:08X}");
        Self { value: out }
    }

    pub fn as_str(&self) -> &str {
        self.value.as_str()
    }

    pub fn root_path(&self) -> String<PREPARED_CACHE_PATH_MAX> {
        prepared_cache_book_path(self.as_str()).unwrap_or_default()
    }

    pub fn child_path(&self, file_name: &str) -> Option<String<PREPARED_CACHE_PATH_MAX>> {
        prepared_cache_file_path(self.as_str(), file_name)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreparedCacheManifestModel {
    pub book_id: PreparedCacheBookIdModel,
    pub source: String<PREPARED_SOURCE_MAX>,
    pub page_count: u16,
    pub kind: PreparedCacheKindModel,
    #[serde(default)]
    pub orientation: ReaderOrientationModel,
    #[serde(default = "default_reader_logical_width")]
    pub logical_width: u16,
    #[serde(default = "default_reader_logical_height")]
    pub logical_height: u16,
}

impl PreparedCacheManifestModel {
    pub fn chapter_page_model(&self) -> PreparedCacheChapterPageModel {
        PreparedCacheChapterPageModel {
            chapter_count: if matches!(
                self.kind,
                PreparedCacheKindModel::Epub | PreparedCacheKindModel::Epu
            ) {
                1
            } else {
                0
            },
            total_pages: self.page_count,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreparedCacheLayoutKeyModel {
    pub orientation: ReaderOrientationModel,
    pub logical_width: u16,
    pub logical_height: u16,
    pub content_width: u16,
    pub content_height: u16,
}

impl PreparedCacheLayoutKeyModel {
    pub const fn from_viewport(viewport: ReaderViewportModel) -> Self {
        Self {
            orientation: viewport.orientation,
            logical_width: viewport.logical_width,
            logical_height: viewport.logical_height,
            content_width: viewport.content_region.width,
            content_height: viewport.content_region.height,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreparedCacheFontIndexModel {
    pub latin: String<PREPARED_CACHE_FILE_MAX>,
    pub devanagari: String<PREPARED_CACHE_FILE_MAX>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreparedCachePageRecordModel {
    pub page_index: u16,
    pub file_name: String<PREPARED_CACHE_FILE_MAX>,
    pub glyph_count: u16,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreparedCachePageListModel {
    pub pages: Vec<PreparedCachePageRecordModel, PREPARED_MAX_PAGES>,
}

impl PreparedCachePageListModel {
    pub const fn new() -> Self {
        Self { pages: Vec::new() }
    }

    pub fn total_pages(&self) -> u16 {
        u16::try_from(self.pages.len()).unwrap_or(u16::MAX)
    }

    pub fn is_empty(&self) -> bool {
        self.pages.is_empty()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreparedCacheChapterPageModel {
    pub chapter_count: u16,
    pub total_pages: u16,
}

impl PreparedCacheChapterPageModel {
    pub const fn is_valid(self) -> bool {
        self.total_pages > 0 && (self.total_pages as usize) <= PREPARED_MAX_PAGES
    }
}

pub type PreparedCacheResult<T> = Result<T, PreparedCacheErrorClassModel>;

pub fn is_valid_prepared_book_id(book_id: &str) -> bool {
    book_id.len() == PREPARED_BOOK_ID_LEN && book_id.bytes().all(|b| b.is_ascii_hexdigit())
}

pub fn is_valid_prepared_cache_file(file_name: &str) -> bool {
    !file_name.is_empty()
        && file_name.len() <= PREPARED_CACHE_FILE_MAX
        && file_name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'_')
}

pub fn prepared_cache_book_path(book_id: &str) -> Option<String<PREPARED_CACHE_PATH_MAX>> {
    let id = PreparedCacheBookIdModel::new(book_id)?;
    let mut out = String::new();
    out.push_str(PREPARED_CACHE_ROOT_PATH).ok()?;
    out.push('/').ok()?;
    out.push_str(id.as_str()).ok()?;
    Some(out)
}

pub fn prepared_cache_relative_book_path(book_id: &str) -> Option<String<PREPARED_CACHE_PATH_MAX>> {
    let id = PreparedCacheBookIdModel::new(book_id)?;
    let mut out = String::new();
    out.push_str(PREPARED_CACHE_ROOT_DIR).ok()?;
    out.push('/').ok()?;
    out.push_str(id.as_str()).ok()?;
    Some(out)
}

pub fn prepared_cache_file_path(
    book_id: &str,
    file_name: &str,
) -> Option<String<PREPARED_CACHE_PATH_MAX>> {
    if !is_valid_prepared_cache_file(file_name) {
        return None;
    }
    let id = PreparedCacheBookIdModel::new(book_id)?;
    let mut out = String::new();
    out.push_str(PREPARED_CACHE_ROOT_PATH).ok()?;
    out.push('/').ok()?;
    out.push_str(id.as_str()).ok()?;
    out.push('/').ok()?;
    out.push_str(file_name).ok()?;
    Some(out)
}

pub fn prepared_cache_relative_file_path(
    book_id: &str,
    file_name: &str,
) -> Option<String<PREPARED_CACHE_PATH_MAX>> {
    if !is_valid_prepared_cache_file(file_name) {
        return None;
    }
    let id = PreparedCacheBookIdModel::new(book_id)?;
    let mut out = String::new();
    out.push_str(PREPARED_CACHE_ROOT_DIR).ok()?;
    out.push('/').ok()?;
    out.push_str(id.as_str()).ok()?;
    out.push('/').ok()?;
    out.push_str(file_name).ok()?;
    Some(out)
}

pub fn parse_prepared_meta(input: &str) -> PreparedCacheResult<PreparedCacheManifestModel> {
    let mut book_id = "";
    let mut source = "";
    let mut page_count = None;
    let mut kind = PreparedCacheKindModel::Unknown;
    let mut orientation = ReaderOrientationModel::Portrait;
    let mut logical_width = None;
    let mut logical_height = None;

    for (key, value) in key_value_lines(input) {
        match key {
            "book_id" => book_id = value,
            "source" => source = value,
            "page_count" => page_count = parse_u16(value),
            "kind" => kind = parse_kind(value),
            "orientation" | "reader_orientation" => {
                if let Some(value) = parse_reader_orientation(value) {
                    orientation = value;
                }
            }
            "logical_width" => logical_width = parse_u16(value),
            "logical_height" => logical_height = parse_u16(value),
            _ => {}
        }
    }

    let book_id =
        PreparedCacheBookIdModel::new(book_id).ok_or(PreparedCacheErrorClassModel::InvalidMeta)?;
    let page_count = page_count.ok_or(PreparedCacheErrorClassModel::InvalidMeta)?;
    if page_count == 0 || usize::from(page_count) > PREPARED_MAX_PAGES {
        return Err(PreparedCacheErrorClassModel::InvalidMeta);
    }

    let mut source_out = String::new();
    push_str_truncated(&mut source_out, source);
    if matches!(kind, PreparedCacheKindModel::Unknown) {
        kind = PreparedCacheKindModel::from_source_path(source);
    }
    let viewport = ReaderViewportModel::for_orientation(orientation);
    let logical_width = logical_width.unwrap_or(viewport.logical_width);
    let logical_height = logical_height.unwrap_or(viewport.logical_height);

    Ok(PreparedCacheManifestModel {
        book_id,
        source: source_out,
        page_count,
        kind,
        orientation,
        logical_width,
        logical_height,
    })
}

pub fn parse_prepared_fonts_index(input: &str) -> PreparedCacheResult<PreparedCacheFontIndexModel> {
    let mut latin = "";
    let mut devanagari = "";
    for (key, value) in key_value_lines(input) {
        match key {
            "Latin" => latin = value,
            "Devanagari" => devanagari = value,
            _ => {}
        }
    }
    if !is_valid_prepared_cache_file(latin) || !is_valid_prepared_cache_file(devanagari) {
        return Err(PreparedCacheErrorClassModel::MissingFont);
    }
    Ok(PreparedCacheFontIndexModel {
        latin: string_from_str(latin).ok_or(PreparedCacheErrorClassModel::InvalidIndex)?,
        devanagari: string_from_str(devanagari)
            .ok_or(PreparedCacheErrorClassModel::InvalidIndex)?,
    })
}

pub fn parse_prepared_pages_index(input: &str) -> PreparedCacheResult<PreparedCachePageListModel> {
    let mut list = PreparedCachePageListModel::new();
    for raw in input.lines() {
        let file = raw.trim();
        if file.is_empty() {
            continue;
        }
        if !is_valid_prepared_cache_file(file) {
            return Err(PreparedCacheErrorClassModel::InvalidIndex);
        }
        let index = u16::try_from(list.pages.len())
            .map_err(|_| PreparedCacheErrorClassModel::InvalidIndex)?;
        let record = PreparedCachePageRecordModel {
            page_index: index,
            file_name: string_from_str(file).ok_or(PreparedCacheErrorClassModel::InvalidIndex)?,
            glyph_count: 0,
        };
        list.pages
            .push(record)
            .map_err(|_| PreparedCacheErrorClassModel::InvalidIndex)?;
    }
    if list.pages.is_empty() {
        Err(PreparedCacheErrorClassModel::InvalidIndex)
    } else {
        Ok(list)
    }
}

pub fn parse_prepared_page_record(
    page_index: u16,
    file_name: &str,
    input: &[u8],
) -> PreparedCacheResult<PreparedCachePageRecordModel> {
    if !is_valid_prepared_cache_file(file_name) {
        return Err(PreparedCacheErrorClassModel::InvalidIndex);
    }
    if input.len() < PREPARED_VRUN_HEADER_LEN {
        return Err(PreparedCacheErrorClassModel::InvalidPage);
    }
    if input[0..4] != PREPARED_VRUN_MAGIC {
        return Err(PreparedCacheErrorClassModel::InvalidPage);
    }
    if read_u16(input, 4)? != PREPARED_VRUN_VERSION {
        return Err(PreparedCacheErrorClassModel::InvalidPage);
    }
    if usize::from(read_u16(input, 6)?) < PREPARED_VRUN_HEADER_LEN {
        return Err(PreparedCacheErrorClassModel::InvalidPage);
    }
    let glyph_count_u32 = read_u32(input, 8)?;
    let glyph_count =
        usize::try_from(glyph_count_u32).map_err(|_| PreparedCacheErrorClassModel::InvalidPage)?;
    if glyph_count > PREPARED_MAX_GLYPHS {
        return Err(PreparedCacheErrorClassModel::TooLarge);
    }
    let records_len = glyph_count
        .checked_mul(PREPARED_VRUN_GLYPH_RECORD_LEN)
        .ok_or(PreparedCacheErrorClassModel::InvalidPage)?;
    checked_range(input.len(), PREPARED_VRUN_HEADER_LEN, records_len)?;

    Ok(PreparedCachePageRecordModel {
        page_index,
        file_name: string_from_str(file_name).ok_or(PreparedCacheErrorClassModel::InvalidIndex)?,
        glyph_count: u16::try_from(glyph_count)
            .map_err(|_| PreparedCacheErrorClassModel::TooLarge)?,
    })
}

pub fn classify_prepared_cache_error(
    error: PreparedCacheErrorClassModel,
) -> PreparedCacheStatusModel {
    PreparedCacheStatusModel::from_error(error)
}

pub const fn missing_cache_is_safe_fallback() -> bool {
    PreparedCacheErrorClassModel::Missing.is_safe_fallback()
}

fn key_value_lines(input: &str) -> impl Iterator<Item = (&str, &str)> {
    input.lines().filter_map(|line| {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            return None;
        }
        let (key, value) = line.split_once('=')?;
        Some((key.trim(), value.trim()))
    })
}

fn default_reader_logical_width() -> u16 {
    ReaderViewportModel::for_orientation(ReaderOrientationModel::Portrait).logical_width
}

fn default_reader_logical_height() -> u16 {
    ReaderViewportModel::for_orientation(ReaderOrientationModel::Portrait).logical_height
}

fn parse_reader_orientation(input: &str) -> Option<ReaderOrientationModel> {
    if input == "0" || eq_ascii_ignore_case(input, "portrait") {
        Some(ReaderOrientationModel::Portrait)
    } else if input == "1" || eq_ascii_ignore_case(input, "inverted") {
        Some(ReaderOrientationModel::Inverted)
    } else if input == "2"
        || eq_ascii_ignore_case(input, "landscape_cw")
        || eq_ascii_ignore_case(input, "landscape-cw")
        || eq_ascii_ignore_case(input, "landscape cw")
    {
        Some(ReaderOrientationModel::LandscapeCw)
    } else if input == "3"
        || eq_ascii_ignore_case(input, "landscape_ccw")
        || eq_ascii_ignore_case(input, "landscape-ccw")
        || eq_ascii_ignore_case(input, "landscape ccw")
    {
        Some(ReaderOrientationModel::LandscapeCcw)
    } else {
        None
    }
}

fn parse_kind(input: &str) -> PreparedCacheKindModel {
    if eq_ascii_ignore_case(input, "txt") {
        PreparedCacheKindModel::Txt
    } else if eq_ascii_ignore_case(input, "epub") {
        PreparedCacheKindModel::Epub
    } else if eq_ascii_ignore_case(input, "epu") {
        PreparedCacheKindModel::Epu
    } else {
        PreparedCacheKindModel::Unknown
    }
}

fn parse_u16(input: &str) -> Option<u16> {
    let mut value = 0u16;
    if input.is_empty() {
        return None;
    }
    for b in input.bytes() {
        if !b.is_ascii_digit() {
            return None;
        }
        value = value.checked_mul(10)?.checked_add(u16::from(b - b'0'))?;
    }
    Some(value)
}

fn string_from_str<const N: usize>(input: &str) -> Option<String<N>> {
    let mut out = String::new();
    out.push_str(input).ok()?;
    Some(out)
}

fn push_str_truncated<const N: usize>(out: &mut String<N>, input: &str) {
    for ch in input.chars() {
        if out.push(ch).is_err() {
            break;
        }
    }
}

fn extension_ascii(path: &str) -> Option<&str> {
    let name = path.rsplit(['/', '\\']).next().unwrap_or(path);
    let dot = name.rfind('.')?;
    Some(&name[dot + 1..])
}

fn eq_ascii_ignore_case(a: &str, b: &str) -> bool {
    a.len() == b.len()
        && a.bytes()
            .zip(b.bytes())
            .all(|(left, right)| left.eq_ignore_ascii_case(&right))
}

fn checked_range(total: usize, start: usize, len: usize) -> PreparedCacheResult<()> {
    let end = start
        .checked_add(len)
        .ok_or(PreparedCacheErrorClassModel::InvalidPage)?;
    if start <= total && end <= total {
        Ok(())
    } else {
        Err(PreparedCacheErrorClassModel::InvalidPage)
    }
}

fn read_u16(data: &[u8], off: usize) -> PreparedCacheResult<u16> {
    let bytes = data
        .get(off..off + 2)
        .ok_or(PreparedCacheErrorClassModel::InvalidPage)?;
    Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn read_u32(data: &[u8], off: usize) -> PreparedCacheResult<u32> {
    let bytes = data
        .get(off..off + 4)
        .ok_or(PreparedCacheErrorClassModel::InvalidPage)?;
    Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vrun_page(glyph_count: u32) -> heapless::Vec<u8, 128> {
        let mut out = heapless::Vec::<u8, 128>::new();
        out.extend_from_slice(&PREPARED_VRUN_MAGIC).unwrap();
        out.extend_from_slice(&PREPARED_VRUN_VERSION.to_le_bytes())
            .unwrap();
        out.extend_from_slice(&(PREPARED_VRUN_HEADER_LEN as u16).to_le_bytes())
            .unwrap();
        out.extend_from_slice(&glyph_count.to_le_bytes()).unwrap();
        out.extend_from_slice(&0u32.to_le_bytes()).unwrap();
        out.extend_from_slice(&0u32.to_le_bytes()).unwrap();
        for _ in 0..glyph_count {
            out.extend_from_slice(&1u32.to_le_bytes()).unwrap();
            out.extend_from_slice(&42u32.to_le_bytes()).unwrap();
            out.extend_from_slice(&0i16.to_le_bytes()).unwrap();
            out.extend_from_slice(&0i16.to_le_bytes()).unwrap();
            out.extend_from_slice(&0u32.to_le_bytes()).unwrap();
            out.extend_from_slice(&0u32.to_le_bytes()).unwrap();
        }
        out
    }

    #[test]
    fn yearly_h_meta_is_parsed_as_prepared_txt() {
        let meta = "book_id=15D1296A\nsource=/YEARLY_H.TXT\npage_count=17\n";
        let manifest = parse_prepared_meta(meta).unwrap();
        assert_eq!(manifest.book_id.as_str(), "15D1296A");
        assert_eq!(manifest.page_count, 17);
        assert_eq!(manifest.kind, PreparedCacheKindModel::Txt);
        assert!(manifest.chapter_page_model().is_valid());
    }

    #[test]
    fn mixed_epub_meta_is_parsed_as_prepared_epub() {
        let meta = "book_id=15D1296A\nsource=/MIXED_EP.EPU\nkind=epub\npage_count=1\n";
        let manifest = parse_prepared_meta(meta).unwrap();
        assert_eq!(manifest.kind, PreparedCacheKindModel::Epub);
        assert_eq!(manifest.chapter_page_model().chapter_count, 1);
        assert_eq!(manifest.chapter_page_model().total_pages, 1);
    }

    #[test]
    fn missing_cache_is_fallback_not_runtime_error() {
        let status = classify_prepared_cache_error(PreparedCacheErrorClassModel::Missing);
        assert_eq!(status, PreparedCacheStatusModel::MissingFallback);
        assert!(status.should_fallback_silently());
        assert!(!status.should_show_diagnostic());
    }

    #[test]
    fn malformed_metadata_is_diagnostic_worthy() {
        let err = parse_prepared_meta("book_id=15D1296A\npage_count=0\n").unwrap_err();
        assert_eq!(err, PreparedCacheErrorClassModel::InvalidMeta);
        let status = classify_prepared_cache_error(err);
        assert!(status.should_show_diagnostic());
    }

    #[test]
    fn book_ids_and_cache_paths_are_8dot3_safe() {
        assert!(is_valid_prepared_book_id("15D1296A"));
        assert!(!is_valid_prepared_book_id("15D1296AX"));
        assert_eq!(
            prepared_cache_book_path("15d1296a").unwrap().as_str(),
            "/FCACHE/15D1296A"
        );
        assert_eq!(
            prepared_cache_file_path("15D1296A", "META.TXT")
                .unwrap()
                .as_str(),
            "/FCACHE/15D1296A/META.TXT"
        );
        assert_eq!(
            prepared_cache_relative_file_path("15D1296A", "PAGES.IDX")
                .unwrap()
                .as_str(),
            "FCACHE/15D1296A/PAGES.IDX"
        );
    }

    #[test]
    fn page_index_parses_current_pages_idx_format() {
        let pages = parse_prepared_pages_index("00000000.VRN\n00000001.VRN\n").unwrap();
        assert_eq!(pages.total_pages(), 2);
        assert_eq!(pages.pages[0].page_index, 0);
        assert_eq!(pages.pages[1].file_name.as_str(), "00000001.VRN");
    }

    #[test]
    fn fonts_index_parses_current_named_font_slots() {
        let fonts = parse_prepared_fonts_index("Latin=LATIN.VFNT\nDevanagari=DEVA.VFNT\n").unwrap();
        assert_eq!(fonts.latin.as_str(), "LATIN.VFNT");
        assert_eq!(fonts.devanagari.as_str(), "DEVA.VFNT");
    }

    #[test]
    fn prepared_page_record_parses_vrun_header() {
        let page = vrun_page(2);
        let record = parse_prepared_page_record(0, "00000000.VRN", page.as_slice()).unwrap();
        assert_eq!(record.page_index, 0);
        assert_eq!(record.glyph_count, 2);
    }

    #[test]
    fn invalid_page_binary_is_diagnostic_worthy() {
        let err = parse_prepared_page_record(0, "00000000.VRN", b"bad").unwrap_err();
        assert_eq!(err, PreparedCacheErrorClassModel::InvalidPage);
        assert!(PreparedCacheStatusModel::from_error(err).should_show_diagnostic());
    }

    #[test]
    fn prepared_meta_defaults_to_portrait_viewport_key_fields() {
        let meta = "book_id=15D1296A\nsource=/YEARLY_H.TXT\npage_count=17\n";
        let manifest = parse_prepared_meta(meta).unwrap();
        assert_eq!(manifest.orientation, ReaderOrientationModel::Portrait);
        assert_eq!(manifest.logical_width, 480);
        assert_eq!(manifest.logical_height, 800);
    }

    #[test]
    fn prepared_meta_accepts_landscape_viewport_key_fields_without_enabling_ui() {
        let meta =
            "book_id=15D1296A\nsource=/YEARLY_H.TXT\npage_count=17\norientation=landscape_cw\n";
        let manifest = parse_prepared_meta(meta).unwrap();
        assert_eq!(manifest.orientation, ReaderOrientationModel::LandscapeCw);
        assert_eq!(manifest.logical_width, 800);
        assert_eq!(manifest.logical_height, 480);

        let key = PreparedCacheLayoutKeyModel::from_viewport(ReaderViewportModel::for_orientation(
            manifest.orientation,
        ));
        assert_eq!(key.orientation, ReaderOrientationModel::LandscapeCw);
        assert_eq!(key.logical_width, 800);
        assert_eq!(key.logical_height, 480);
        assert_eq!(key.content_width, 768);
    }
}
