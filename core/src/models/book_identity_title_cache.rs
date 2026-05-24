use core::fmt::Write as _;

use heapless::String;
use serde::{Deserialize, Serialize};

pub const TITLE_CACHE_DIR: &str = "_x4";
pub const TITLE_CACHE_FILE: &str = "TITLES.BIN";
pub const TITLE_CACHE_PATH: &str = "_x4/TITLES.BIN";
pub const HOST_TITLE_MAP_FILE: &str = "TITLEMAP.TSV";
pub const HOST_TITLE_MAP_PATH: &str = "_x4/TITLEMAP.TSV";
pub const TITLE_CACHE_RECORD_MAX: usize = 128;
pub const TITLE_CACHE_KEY_MAX: usize = 13;
pub const TITLE_CACHE_TITLE_MAX: usize = 64;
pub const DISPLAY_TITLE_MAX: usize = 96;
pub const SOURCE_PATH_MAX: usize = 128;
pub const BOOK_ID_HEX_LEN: usize = 8;

const FNV1A32_OFFSET: u32 = 0x811c_9dc5;
const FNV1A32_PRIME: u32 = 0x0100_0193;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum BookFileFormatModel {
    Txt,
    Epub,
    Epu,
    Md,
    #[default]
    Unknown,
}

impl BookFileFormatModel {
    pub fn from_path(path: &str) -> Self {
        match extension_ascii(path) {
            Some(ext) if ext.eq_ignore_ascii_case("TXT") => Self::Txt,
            Some(ext) if ext.eq_ignore_ascii_case("EPUB") => Self::Epub,
            Some(ext) if ext.eq_ignore_ascii_case("EPU") => Self::Epu,
            Some(ext) if ext.eq_ignore_ascii_case("MD") => Self::Md,
            _ => Self::Unknown,
        }
    }

    pub const fn is_reader_supported(self) -> bool {
        matches!(self, Self::Txt | Self::Epub | Self::Epu | Self::Md)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StableBookIdModel {
    pub hex8: String<BOOK_ID_HEX_LEN>,
}

impl StableBookIdModel {
    pub fn from_path(path: &str) -> Self {
        let mut normalized: String<SOURCE_PATH_MAX> = String::new();
        push_normalized_book_id_source(&mut normalized, path);
        Self::from_normalized_source(normalized.as_str())
    }

    pub fn from_normalized_source(source: &str) -> Self {
        Self::from_u32(fnv1a32(source.as_bytes()))
    }

    pub fn from_u32(value: u32) -> Self {
        let mut hex8 = String::new();
        let _ = write!(hex8, "{value:08X}");
        Self { hex8 }
    }

    pub fn as_str(&self) -> &str {
        self.hex8.as_str()
    }

    pub fn is_8dot3_safe(&self) -> bool {
        is_8dot3_book_id(self.hex8.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TitleCacheRecordModel {
    pub file_name: String<TITLE_CACHE_KEY_MAX>,
    pub display_title: String<TITLE_CACHE_TITLE_MAX>,
}

impl TitleCacheRecordModel {
    pub fn new(file_name: &str, display_title: &str) -> Option<Self> {
        let file_name = normalize_title_cache_key(file_name);
        let display_title = normalize_title_cache_title(display_title);
        if file_name.is_empty() || display_title.is_empty() {
            None
        } else {
            Some(Self {
                file_name,
                display_title,
            })
        }
    }

    pub fn matches_file_name(&self, file_name: &str) -> bool {
        self.file_name
            .as_str()
            .eq_ignore_ascii_case(normalize_title_cache_key(file_name).as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookIdentityTitleModel {
    pub book_id: StableBookIdModel,
    pub source_path: String<SOURCE_PATH_MAX>,
    pub cache_key: String<TITLE_CACHE_KEY_MAX>,
    pub display_title: String<DISPLAY_TITLE_MAX>,
    pub format: BookFileFormatModel,
}

impl BookIdentityTitleModel {
    pub fn from_path(path: &str) -> Self {
        let mut source_path = String::new();
        push_truncated(&mut source_path, path);
        Self {
            book_id: StableBookIdModel::from_path(path),
            cache_key: normalize_title_cache_key(path),
            display_title: fallback_display_title_from_filename(path),
            format: BookFileFormatModel::from_path(path),
            source_path,
        }
    }

    pub fn apply_title_cache_record(&mut self, record: &TitleCacheRecordModel) -> bool {
        if !record.matches_file_name(self.cache_key.as_str()) {
            return false;
        }
        self.display_title.clear();
        push_truncated(&mut self.display_title, record.display_title.as_str());
        true
    }
}

pub fn normalize_title_cache_key(path_or_name: &str) -> String<TITLE_CACHE_KEY_MAX> {
    let mut out = String::new();
    for ch in file_name_from_path(path_or_name).trim().chars() {
        if out.len() >= TITLE_CACHE_KEY_MAX {
            break;
        }
        if ch.is_ascii() && !ch.is_ascii_whitespace() {
            let _ = out.push(ch.to_ascii_uppercase());
        }
    }
    out
}

pub fn normalize_display_title(input: &str) -> String<DISPLAY_TITLE_MAX> {
    normalize_title_text(input)
}

pub fn normalize_title_cache_title(input: &str) -> String<TITLE_CACHE_TITLE_MAX> {
    normalize_title_text(input)
}

pub fn fallback_display_title_from_filename(path_or_name: &str) -> String<DISPLAY_TITLE_MAX> {
    let file_name = file_name_from_path(path_or_name).trim();
    let stem = strip_reader_extension(file_name);
    let uppercase_like = stem
        .bytes()
        .filter(u8::is_ascii_alphabetic)
        .all(|b| !b.is_ascii_lowercase());
    let mut out: String<DISPLAY_TITLE_MAX> = String::new();
    let mut prev_space = false;
    let mut start_word = true;

    for mut ch in stem.chars() {
        if out.len() >= DISPLAY_TITLE_MAX {
            break;
        }
        if matches!(ch, '_' | '-' | '.') || ch.is_whitespace() {
            if !prev_space && !out.is_empty() {
                let _ = out.push(' ');
                prev_space = true;
                start_word = true;
            }
            continue;
        }
        if uppercase_like && ch.is_ascii_alphabetic() {
            ch = if start_word {
                ch.to_ascii_uppercase()
            } else {
                ch.to_ascii_lowercase()
            };
        }
        if out.push(ch).is_err() {
            break;
        }
        prev_space = false;
        start_word = false;
    }

    while out.ends_with(' ') {
        out.pop();
    }
    if out.is_empty() {
        push_truncated(&mut out, file_name);
    }
    out
}

pub fn parse_title_cache_record(line: &[u8]) -> Option<TitleCacheRecordModel> {
    let line = trim_line_end(line);
    let tab = line.iter().position(|&b| b == b'\t')?;
    let file_part = core::str::from_utf8(&line[..tab]).ok()?;
    let title_part = core::str::from_utf8(&line[tab + 1..]).ok()?;
    TitleCacheRecordModel::new(file_part, title_part)
}

pub fn write_title_cache_record(
    record: &TitleCacheRecordModel,
) -> Option<String<TITLE_CACHE_RECORD_MAX>> {
    let mut out = String::new();
    out.push_str(record.file_name.as_str()).ok()?;
    out.push('\t').ok()?;
    out.push_str(record.display_title.as_str()).ok()?;
    out.push('\n').ok()?;
    Some(out)
}

pub fn make_title_cache_record(
    file_name: &str,
    title: &str,
) -> Option<String<TITLE_CACHE_RECORD_MAX>> {
    write_title_cache_record(&TitleCacheRecordModel::new(file_name, title)?)
}

pub fn is_8dot3_book_id(book_id: &str) -> bool {
    book_id.len() == BOOK_ID_HEX_LEN && book_id.bytes().all(|b| b.is_ascii_hexdigit())
}

pub fn is_8dot3_cache_key(key: &str) -> bool {
    let bytes = key.as_bytes();
    if bytes.is_empty() || bytes.len() > TITLE_CACHE_KEY_MAX {
        return false;
    }

    let mut dots = 0usize;
    let mut base = 0usize;
    let mut ext = 0usize;
    let mut in_ext = false;

    for &b in bytes {
        if b == b'.' {
            dots += 1;
            if dots > 1 || base == 0 {
                return false;
            }
            in_ext = true;
            continue;
        }
        if !is_cache_key_byte(b) {
            return false;
        }
        if in_ext {
            ext += 1;
        } else {
            base += 1;
        }
    }

    base > 0 && base <= 8 && (!in_ext || (ext > 0 && ext <= 4))
}

fn push_normalized_book_id_source<const N: usize>(out: &mut String<N>, path: &str) {
    for ch in path.trim().chars() {
        let ch = if ch == '\\' { '/' } else { ch };
        if !ch.is_ascii_whitespace() && out.push(ch.to_ascii_uppercase()).is_err() {
            break;
        }
    }
}

fn normalize_title_text<const N: usize>(input: &str) -> String<N> {
    let mut out = String::new();
    let mut prev_space = false;
    for ch in input.trim().chars() {
        if out.len() >= N {
            break;
        }
        if ch.is_whitespace() {
            if !prev_space && !out.is_empty() {
                let _ = out.push(' ');
                prev_space = true;
            }
            continue;
        }
        if out.push(ch).is_err() {
            break;
        }
        prev_space = false;
    }
    while out.ends_with(' ') {
        out.pop();
    }
    out
}

fn push_truncated<const N: usize>(out: &mut String<N>, input: &str) {
    for ch in input.chars() {
        if out.push(ch).is_err() {
            break;
        }
    }
}

fn file_name_from_path(path_or_name: &str) -> &str {
    let a = path_or_name.rfind('/');
    let b = path_or_name.rfind('\\');
    let start = match (a, b) {
        (Some(x), Some(y)) => x.max(y) + 1,
        (Some(x), None) => x + 1,
        (None, Some(y)) => y + 1,
        _ => 0,
    };
    &path_or_name[start..]
}

fn strip_reader_extension(file_name: &str) -> &str {
    let Some(dot) = file_name.rfind('.') else {
        return file_name;
    };
    let ext = &file_name[dot + 1..];
    if ext.eq_ignore_ascii_case("TXT")
        || ext.eq_ignore_ascii_case("EPUB")
        || ext.eq_ignore_ascii_case("EPU")
        || ext.eq_ignore_ascii_case("MD")
    {
        &file_name[..dot]
    } else {
        file_name
    }
}

fn extension_ascii(path: &str) -> Option<&str> {
    let name = file_name_from_path(path);
    let dot = name.rfind('.')?;
    Some(&name[dot + 1..])
}

fn trim_line_end(mut line: &[u8]) -> &[u8] {
    while matches!(line.last(), Some(b'\n' | b'\r')) {
        line = &line[..line.len() - 1];
    }
    line
}

fn is_cache_key_byte(b: u8) -> bool {
    b.is_ascii_uppercase() || b.is_ascii_digit() || matches!(b, b'~' | b'_' | b'-')
}

fn fnv1a32(bytes: &[u8]) -> u32 {
    let mut hash = FNV1A32_OFFSET;
    for &b in bytes {
        hash = (hash ^ u32::from(b)).wrapping_mul(FNV1A32_PRIME);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alice_long_title_fallback_preserves_apostrophe_and_spaces() {
        let title =
            fallback_display_title_from_filename("/books/Alice's Adventures in Wonderland.epub");
        assert_eq!(title.as_str(), "Alice's Adventures in Wonderland");
    }

    #[test]
    fn yearly_h_identity_is_stable_and_safe() {
        let ident = BookIdentityTitleModel::from_path("YEARLY_H.TXT");
        assert_eq!(ident.cache_key.as_str(), "YEARLY_H.TXT");
        assert_eq!(ident.display_title.as_str(), "Yearly H");
        assert_eq!(ident.format, BookFileFormatModel::Txt);
        assert!(ident.book_id.is_8dot3_safe());
    }

    #[test]
    fn epub_and_txt_title_fallback_strip_reader_extensions() {
        assert_eq!(
            fallback_display_title_from_filename("THEHO~26.EPU").as_str(),
            "Theho~26"
        );
        assert_eq!(
            fallback_display_title_from_filename("NOTES.TXT").as_str(),
            "Notes"
        );
    }

    #[test]
    fn display_title_normalization_collapses_spaces_but_preserves_case() {
        let title = normalize_display_title("  Alice's   Adventures   in Wonderland  ");
        assert_eq!(title.as_str(), "Alice's Adventures in Wonderland");
    }

    #[test]
    fn cache_key_normalization_is_ascii_uppercase_and_bounded() {
        assert_eq!(
            normalize_title_cache_key("books/alice.epub").as_str(),
            "ALICE.EPUB"
        );
        assert_eq!(
            normalize_title_cache_key("YEARLY_H.TXT").as_str(),
            "YEARLY_H.TXT"
        );
        assert!(is_8dot3_cache_key("YEARLY_H.TXT"));
    }

    #[test]
    fn title_cache_record_round_trips_current_titles_bin_line_format() {
        let rec =
            TitleCacheRecordModel::new("ALICES~1.EPU", "Alice's Adventures in Wonderland").unwrap();
        let line = write_title_cache_record(&rec).unwrap();
        assert_eq!(
            line.as_str(),
            "ALICES~1.EPU\tAlice's Adventures in Wonderland\n"
        );
        assert_eq!(parse_title_cache_record(line.as_bytes()).unwrap(), rec);
    }

    #[test]
    fn title_cache_record_parser_accepts_crlf_and_applies_to_identity() {
        let rec = parse_title_cache_record(b"ALICES~1.EPU\tAlice's Adventures in Wonderland\r\n")
            .unwrap();
        let mut ident = BookIdentityTitleModel::from_path("ALICES~1.EPU");
        assert!(ident.apply_title_cache_record(&rec));
        assert_eq!(
            ident.display_title.as_str(),
            "Alice's Adventures in Wonderland"
        );
    }

    #[test]
    fn stable_book_id_normalizes_case_and_slashes() {
        let a = StableBookIdModel::from_path("books/yearly_h.txt");
        let b = StableBookIdModel::from_path("BOOKS\\YEARLY_H.TXT");
        assert_eq!(a, b);
        assert!(is_8dot3_book_id(a.as_str()));
    }
}
