#![allow(dead_code)]

extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::rustmix_x4::contracts::storage_path_helpers::{
    RustmixStateFileKind, RustmixStoragePathHelpers,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RustmixReaderFormat {
    Txt,
    Epub,
    Unknown,
}

impl RustmixReaderFormat {
    pub fn from_path(path: &str) -> Self {
        let path = path.as_bytes();
        if path.len() >= 5 && path[path.len() - 5..].eq_ignore_ascii_case(b".epub") {
            Self::Epub
        } else if path.len() >= 4 && path[path.len() - 4..].eq_ignore_ascii_case(b".txt") {
            Self::Txt
        } else {
            Self::Unknown
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Txt => "txt",
            Self::Epub => "epub",
            Self::Unknown => "unknown",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "txt" => Self::Txt,
            "epub" => Self::Epub,
            _ => Self::Unknown,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RustmixBookId(String);

impl RustmixBookId {
    pub fn from_path(path: &str) -> Self {
        Self(fingerprint_path(path))
    }

    pub fn from_encoded(encoded: String) -> Self {
        Self(encoded)
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub fn is_empty(&self) -> bool {
        self.0.trim().is_empty()
    }

    pub fn hex8(&self) -> Option<[u8; RustmixStoragePathHelpers::BOOK_ID_LEN]> {
        let raw = self
            .as_str()
            .strip_prefix("bk-")
            .unwrap_or_else(|| self.as_str());
        let mut out = [b'0'; RustmixStoragePathHelpers::BOOK_ID_LEN];
        let mut pos = 0usize;

        for byte in raw.bytes() {
            if byte.is_ascii_hexdigit() {
                out[pos] = byte.to_ascii_uppercase();
                pos += 1;
                if pos == out.len() {
                    break;
                }
            }
        }

        if pos == 0 { None } else { Some(out) }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RustmixBookIdentity {
    pub book_id: RustmixBookId,
    pub source_path: String,
    pub display_title: String,
    pub format: RustmixReaderFormat,
    pub fingerprint_kind: &'static str,
}

impl RustmixBookIdentity {
    pub fn from_path(path: &str) -> Self {
        Self {
            book_id: RustmixBookId::from_path(path),
            source_path: path.to_string(),
            display_title: display_title(path),
            format: RustmixReaderFormat::from_path(path),
            fingerprint_kind: FINGERPRINT_KIND,
        }
    }

    pub fn with_display_title(mut self, title: &str) -> Self {
        let title = title.trim();
        if !title.is_empty() {
            self.display_title = title.to_string();
        }
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RustmixReaderStateLayout {
    pub book_id: RustmixBookId,
    pub state_dir: &'static str,
    pub progress_file: String,
    pub bookmark_file: String,
    pub meta_file: String,
    pub theme_file: String,
    pub bookmarks_index_file: &'static str,
}

impl RustmixReaderStateLayout {
    pub fn for_book_id(book_id: &RustmixBookId) -> Option<Self> {
        Some(Self {
            book_id: book_id.clone(),
            state_dir: STATE_DIR,
            progress_file: typed_state_file_for(book_id, RustmixStateFileKind::Progress)?,
            bookmark_file: typed_state_file_for(book_id, RustmixStateFileKind::Bookmark)?,
            meta_file: typed_state_file_for(book_id, RustmixStateFileKind::Metadata)?,
            theme_file: typed_state_file_for(book_id, RustmixStateFileKind::Theme)?,
            bookmarks_index_file: BOOKMARKS_INDEX_FILE,
        })
    }

    pub fn for_path(path: &str) -> Option<Self> {
        Self::for_book_id(&RustmixBookId::from_path(path))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RustmixReadingProgressRecord {
    pub book_id: RustmixBookId,
    pub source_path: String,
    pub format: RustmixReaderFormat,
    pub chapter: u16,
    pub page: u32,
    pub byte_offset: u32,
    pub font_size_idx: u8,
}

impl RustmixReadingProgressRecord {
    pub fn new(path: &str, chapter: u16, page: u32, byte_offset: u32, font_size_idx: u8) -> Self {
        Self::from_identity(
            &RustmixBookIdentity::from_path(path),
            chapter,
            page,
            byte_offset,
            font_size_idx,
        )
    }

    pub fn from_identity(
        identity: &RustmixBookIdentity,
        chapter: u16,
        page: u32,
        byte_offset: u32,
        font_size_idx: u8,
    ) -> Self {
        Self {
            book_id: identity.book_id.clone(),
            source_path: identity.source_path.clone(),
            format: identity.format,
            chapter,
            page,
            byte_offset,
            font_size_idx,
        }
    }

    pub fn encode_line(&self) -> String {
        let mut line = String::new();
        push_field(&mut line, self.book_id.as_str());
        push_field(&mut line, &self.source_path);
        push_field(&mut line, self.format.as_str());
        push_field(&mut line, &u32::from(self.chapter).to_string());
        push_field(&mut line, &self.page.to_string());
        push_field(&mut line, &self.byte_offset.to_string());
        push_field(&mut line, &u32::from(self.font_size_idx).to_string());
        line
    }

    pub fn decode_line(line: &str) -> Option<Self> {
        let fields = split_fields(line);
        if fields.len() != 7 {
            return None;
        }
        let book_id = RustmixBookId::from_encoded(fields[0].clone());
        if book_id.is_empty() {
            return None;
        }
        Some(Self {
            book_id,
            source_path: fields[1].clone(),
            format: RustmixReaderFormat::parse(&fields[2]),
            chapter: fields[3].parse().ok()?,
            page: fields[4].parse().ok()?,
            byte_offset: fields[5].parse().ok()?,
            font_size_idx: fields[6].parse::<u16>().ok()? as u8,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RustmixBookMetaRecord {
    pub book_id: RustmixBookId,
    pub fingerprint_kind: String,
    pub source_path: String,
    pub display_title: String,
    pub format: RustmixReaderFormat,
}

impl RustmixBookMetaRecord {
    pub fn from_path(path: &str) -> Self {
        Self::from_identity(&RustmixBookIdentity::from_path(path))
    }

    pub fn from_identity(identity: &RustmixBookIdentity) -> Self {
        Self {
            book_id: identity.book_id.clone(),
            fingerprint_kind: identity.fingerprint_kind.to_string(),
            source_path: identity.source_path.clone(),
            display_title: identity.display_title.clone(),
            format: identity.format,
        }
    }

    pub fn encode_line(&self) -> String {
        let mut line = String::new();
        push_field(&mut line, self.book_id.as_str());
        push_field(&mut line, &self.fingerprint_kind);
        push_field(&mut line, &self.source_path);
        push_field(&mut line, &self.display_title);
        push_field(&mut line, self.format.as_str());
        line
    }

    pub fn decode_line(line: &str) -> Option<Self> {
        let fields = split_fields(line);
        if fields.len() != 5 {
            return None;
        }
        let book_id = RustmixBookId::from_encoded(fields[0].clone());
        if book_id.is_empty() {
            return None;
        }
        Some(Self {
            book_id,
            fingerprint_kind: fields[1].clone(),
            source_path: fields[2].clone(),
            display_title: fields[3].clone(),
            format: RustmixReaderFormat::parse(&fields[4]),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RustmixBookmarkRecord {
    pub book_id: RustmixBookId,
    pub source_path: String,
    pub chapter: u16,
    pub byte_offset: u32,
    pub label: String,
}

impl RustmixBookmarkRecord {
    pub fn new(
        identity: &RustmixBookIdentity,
        chapter: u16,
        byte_offset: u32,
        label: String,
    ) -> Self {
        Self {
            book_id: identity.book_id.clone(),
            source_path: identity.source_path.clone(),
            chapter,
            byte_offset,
            label,
        }
    }

    pub fn encode_line(&self) -> String {
        let mut line = String::new();
        push_field(&mut line, self.book_id.as_str());
        push_field(&mut line, &self.source_path);
        push_field(&mut line, &u32::from(self.chapter).to_string());
        push_field(&mut line, &self.byte_offset.to_string());
        push_field(&mut line, &self.label);
        line
    }

    pub fn decode_line(line: &str) -> Option<Self> {
        let fields = split_fields(line);
        if fields.len() != 5 {
            return None;
        }
        let book_id = RustmixBookId::from_encoded(fields[0].clone());
        if book_id.is_empty() {
            return None;
        }
        Some(Self {
            book_id,
            source_path: fields[1].clone(),
            chapter: fields[2].parse().ok()?,
            byte_offset: fields[3].parse().ok()?,
            label: fields[4].clone(),
        })
    }

    pub fn same_position(&self, chapter: u16, byte_offset: u32) -> bool {
        self.chapter == chapter && self.byte_offset == byte_offset
    }

    pub fn display_label(&self) -> String {
        let trimmed = self.label.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
        let mut out = String::from("Ch ");
        out.push_str(&(u32::from(self.chapter) + 1).to_string());
        out.push_str(" @ ");
        out.push_str(&self.byte_offset.to_string());
        out
    }
}

pub fn decode_bookmarks(payload: &str) -> Vec<RustmixBookmarkRecord> {
    payload
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                None
            } else {
                RustmixBookmarkRecord::decode_line(line)
            }
        })
        .collect()
}

pub fn encode_bookmarks(bookmarks: &[RustmixBookmarkRecord]) -> String {
    let mut out = String::new();
    for (idx, bookmark) in bookmarks.iter().enumerate() {
        if idx > 0 {
            out.push('\n');
        }
        out.push_str(&bookmark.encode_line());
    }
    out
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RustmixBookmarkIndexRecord {
    pub book_id: RustmixBookId,
    pub source_path: String,
    pub display_title: String,
    pub chapter: u16,
    pub byte_offset: u32,
    pub label: String,
}

impl RustmixBookmarkIndexRecord {
    pub fn from_bookmark(rec: &RustmixBookmarkRecord, display_title: impl Into<String>) -> Self {
        Self {
            book_id: rec.book_id.clone(),
            source_path: rec.source_path.clone(),
            display_title: display_title.into(),
            chapter: rec.chapter,
            byte_offset: rec.byte_offset,
            label: rec.label.clone(),
        }
    }

    pub fn encode_line(&self) -> String {
        let mut line = String::new();
        push_field(&mut line, self.book_id.as_str());
        push_field(&mut line, &self.source_path);
        push_field(&mut line, &self.display_title);
        push_field(&mut line, &u32::from(self.chapter).to_string());
        push_field(&mut line, &self.byte_offset.to_string());
        push_field(&mut line, &self.label);
        line
    }

    pub fn decode_line(line: &str) -> Option<Self> {
        let fields = split_fields(line);
        if fields.len() < 6 {
            return None;
        }
        let book_id = RustmixBookId::from_encoded(fields[0].clone());
        if book_id.is_empty() {
            return None;
        }
        Some(Self {
            book_id,
            source_path: fields[1].clone(),
            display_title: fields[2].clone(),
            chapter: fields[3].parse().ok()?,
            byte_offset: fields[4].parse().ok()?,
            label: fields[5].clone(),
        })
    }

    pub fn display_label(&self) -> String {
        let mut out = String::new();
        if !self.display_title.trim().is_empty() {
            out.push_str(self.display_title.trim());
        } else {
            out.push_str(&display_title(&self.source_path));
        }

        let detail = self.label.trim();
        if !detail.is_empty() {
            out.push_str(" - ");
            out.push_str(detail);
        } else {
            out.push_str(" - Ch ");
            out.push_str(&(u32::from(self.chapter) + 1).to_string());
            out.push_str(" - Off ");
            out.push_str(&self.byte_offset.to_string());
        }
        out
    }

    pub fn jump_message(&self) -> String {
        let mut line = String::new();
        push_field(&mut line, BOOKMARK_JUMP_PREFIX);
        push_field(&mut line, &self.source_path);
        push_field(&mut line, &u32::from(self.chapter).to_string());
        push_field(&mut line, &self.byte_offset.to_string());
        line
    }
}

pub fn decode_bookmarks_index(payload: &str) -> Vec<RustmixBookmarkIndexRecord> {
    payload
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                None
            } else {
                RustmixBookmarkIndexRecord::decode_line(line)
            }
        })
        .collect()
}

pub fn encode_bookmarks_index(entries: &[RustmixBookmarkIndexRecord]) -> String {
    let mut out = String::new();
    for (idx, entry) in entries.iter().enumerate() {
        if idx > 0 {
            out.push('\n');
        }
        out.push_str(&entry.encode_line());
    }
    out
}

pub fn decode_bookmark_jump(msg: &str) -> Option<(String, u16, u32)> {
    let fields = split_fields(msg);
    if fields.len() != 4 || fields[0] != BOOKMARK_JUMP_PREFIX {
        return None;
    }
    Some((
        fields[1].clone(),
        fields[2].parse().ok()?,
        fields[3].parse().ok()?,
    ))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RustmixReaderThemePreset {
    pub font_size_idx: u8,
    pub margin_px: u16,
    pub line_spacing_pct: u8,
    pub alignment: String,
    pub theme_name: String,
}

impl Default for RustmixReaderThemePreset {
    fn default() -> Self {
        Self {
            font_size_idx: 4,
            margin_px: 8,
            line_spacing_pct: 100,
            alignment: "justify".into(),
            theme_name: "default".into(),
        }
    }
}

impl RustmixReaderThemePreset {
    pub fn encode_line(&self) -> String {
        let mut line = String::new();
        push_field(&mut line, &u32::from(self.font_size_idx).to_string());
        push_field(&mut line, &u32::from(self.margin_px).to_string());
        push_field(&mut line, &u32::from(self.line_spacing_pct).to_string());
        push_field(&mut line, &self.alignment);
        push_field(&mut line, &self.theme_name);
        line
    }

    pub fn decode_line(line: &str) -> Option<Self> {
        let fields = split_fields(line);
        if fields.len() != 5 {
            return None;
        }
        Some(Self {
            font_size_idx: fields[0].parse::<u16>().ok()? as u8,
            margin_px: fields[1].parse().ok()?,
            line_spacing_pct: fields[2].parse::<u16>().ok()? as u8,
            alignment: fields[3].clone(),
            theme_name: fields[4].clone(),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RustmixReaderThemeRecord {
    pub book_id: RustmixBookId,
    pub source_path: String,
    pub format: RustmixReaderFormat,
    pub preset: RustmixReaderThemePreset,
}

impl RustmixReaderThemeRecord {
    pub fn new(path: &str, preset: RustmixReaderThemePreset) -> Self {
        Self::from_identity(&RustmixBookIdentity::from_path(path), preset)
    }

    pub fn from_identity(identity: &RustmixBookIdentity, preset: RustmixReaderThemePreset) -> Self {
        Self {
            book_id: identity.book_id.clone(),
            source_path: identity.source_path.clone(),
            format: identity.format,
            preset,
        }
    }

    pub fn encode_line(&self) -> String {
        let mut line = String::new();
        push_field(&mut line, self.book_id.as_str());
        push_field(&mut line, &self.source_path);
        push_field(&mut line, self.format.as_str());
        push_field(&mut line, &u32::from(self.preset.font_size_idx).to_string());
        push_field(&mut line, &u32::from(self.preset.margin_px).to_string());
        push_field(
            &mut line,
            &u32::from(self.preset.line_spacing_pct).to_string(),
        );
        push_field(&mut line, &self.preset.alignment);
        push_field(&mut line, &self.preset.theme_name);
        line
    }

    pub fn decode_line(line: &str) -> Option<Self> {
        let fields = split_fields(line);
        if fields.len() != 8 {
            return None;
        }
        let book_id = RustmixBookId::from_encoded(fields[0].clone());
        if book_id.is_empty() {
            return None;
        }
        Some(Self {
            book_id,
            source_path: fields[1].clone(),
            format: RustmixReaderFormat::parse(&fields[2]),
            preset: RustmixReaderThemePreset {
                font_size_idx: fields[3].parse::<u16>().ok()? as u8,
                margin_px: fields[4].parse().ok()?,
                line_spacing_pct: fields[5].parse::<u16>().ok()? as u8,
                alignment: fields[6].clone(),
                theme_name: fields[7].clone(),
            },
        })
    }
}

pub const READER_STATE_FACADE_OWNER: &str = "Rustmix-owned reader state facade";
pub const ACTIVE_READER_STATE_IO_OWNER: &str = "Rustmix-owned X4 runtime";
pub const ACTIVE_READER_STATE_IO_MOVED_TO_IMPORTED_RUNTIME: bool = false;
pub const ACTIVE_PROGRESS_BOOKMARK_IO_MOVED_TO_IMPORTED_RUNTIME: bool = false;
pub const BOOK_ID_MODEL: &str = "path-fnv1a32-v2";
pub const STATE_DIR: &str = RustmixStoragePathHelpers::STATE_DIR_STR;
pub const BOOKMARKS_INDEX_FILE: &str = RustmixStoragePathHelpers::BOOKMARK_INDEX_FILE;
pub const BOOKMARK_JUMP_PREFIX: &str = "BMJ";
pub const FINGERPRINT_KIND: &str = "path-v2";
pub const THEME_NAMES: &[&str] = &["Default", "Classic", "Serif"];

pub fn theme_idx_from_name(name: &str) -> u8 {
    for (idx, candidate) in THEME_NAMES.iter().enumerate() {
        if name.eq_ignore_ascii_case(candidate) {
            return idx as u8;
        }
    }
    0
}

pub fn progress_record_file_for(book_id: &RustmixBookId) -> Option<String> {
    typed_state_file_for(book_id, RustmixStateFileKind::Progress)
}

pub fn bookmark_record_file_for(book_id: &RustmixBookId) -> Option<String> {
    typed_state_file_for(book_id, RustmixStateFileKind::Bookmark)
}

pub fn theme_record_file_for(book_id: &RustmixBookId) -> Option<String> {
    typed_state_file_for(book_id, RustmixStateFileKind::Theme)
}

pub fn meta_record_file_for(book_id: &RustmixBookId) -> Option<String> {
    typed_state_file_for(book_id, RustmixStateFileKind::Metadata)
}

pub const fn bookmarks_index_file() -> &'static str {
    BOOKMARKS_INDEX_FILE
}

pub fn fingerprint_path(path: &str) -> String {
    let normalized = normalized_path_key(path);
    let mut hash: u32 = 0x811C9DC5;
    for &byte in normalized.as_bytes() {
        hash ^= u32::from(byte);
        hash = hash.wrapping_mul(0x01000193);
    }

    let mut out = String::from("bk-");
    append_hex_u32(&mut out, hash);
    out
}

pub fn normalized_path_key(path: &str) -> String {
    let mut out = String::new();
    for ch in path.chars() {
        let normalized = match ch {
            '\\' => '/',
            other => other,
        };
        out.push(normalized.to_ascii_lowercase());
    }
    out
}

pub fn display_title(path: &str) -> String {
    if let Some((_, tail)) = path.rsplit_once('/')
        && !tail.is_empty()
    {
        return tail.to_string();
    }
    path.to_string()
}

fn typed_state_file_for(book_id: &RustmixBookId, kind: RustmixStateFileKind) -> Option<String> {
    let hex = book_id.hex8()?;
    let path = RustmixStoragePathHelpers::state_file_name(hex, kind);
    Some(core::str::from_utf8(path.as_bytes()).ok()?.to_string())
}

fn push_field(out: &mut String, field: &str) {
    if !out.is_empty() {
        out.push('|');
    }
    for ch in field.chars() {
        match ch {
            '|' => out.push_str("%7C"),
            '\n' => out.push_str("%0A"),
            '\r' => out.push_str("%0D"),
            _ => out.push(ch),
        }
    }
}

fn split_fields(line: &str) -> Vec<String> {
    line.split('|').map(percent_decode).collect()
}

fn percent_decode(value: &str) -> String {
    value
        .replace("%7C", "|")
        .replace("%0A", "\n")
        .replace("%0D", "\r")
}

fn append_hex_u32(out: &mut String, value: u32) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for shift in (0..=28).rev().step_by(4) {
        let idx = ((value >> shift) & 0x0f) as usize;
        out.push(char::from(HEX[idx]));
    }
}

#[cfg(test)]
mod tests {
    use super::{
        RustmixBookId, RustmixBookIdentity, RustmixBookMetaRecord, RustmixBookmarkIndexRecord,
        RustmixBookmarkRecord, RustmixReaderStateLayout, RustmixReaderThemePreset,
        RustmixReaderThemeRecord, RustmixReadingProgressRecord, decode_bookmark_jump,
        decode_bookmarks, decode_bookmarks_index, encode_bookmarks, encode_bookmarks_index,
        theme_idx_from_name,
    };

    #[test]
    fn builds_helper_backed_theme_and_metadata_filenames() {
        let book_id = RustmixBookId::from_encoded("bk-8a79a61f".into());
        let layout = RustmixReaderStateLayout::for_book_id(&book_id).unwrap();

        assert_eq!(layout.theme_file, "8A79A61F.THM");
        assert_eq!(layout.meta_file, "8A79A61F.MTA");
        assert_eq!(layout.progress_file, "8A79A61F.PRG");
        assert_eq!(layout.bookmark_file, "8A79A61F.BKM");
        assert_eq!(layout.bookmarks_index_file, "BMIDX.TXT");
    }

    #[test]
    fn round_trips_progress_record_format() {
        let record = RustmixReadingProgressRecord::new("/books/Example.epub", 2, 17, 4096, 5);
        let encoded = record.encode_line();
        let decoded = RustmixReadingProgressRecord::decode_line(&encoded).unwrap();

        assert_eq!(decoded, record);
    }

    #[test]
    fn round_trips_bookmark_records_format() {
        let identity = RustmixBookIdentity::from_path("/books/Example.epub");
        let first = RustmixBookmarkRecord::new(&identity, 1, 128, "first|mark".into());
        let second = RustmixBookmarkRecord::new(&identity, 2, 256, String::new());
        let encoded = encode_bookmarks(&[first.clone(), second.clone()]);
        let decoded = decode_bookmarks(&encoded);

        assert_eq!(decoded, alloc::vec![first, second]);
        assert!(decoded[0].same_position(1, 128));
        assert_eq!(decoded[1].display_label(), "Ch 3 @ 256");
    }

    #[test]
    fn round_trips_bookmark_index_and_jump_format() {
        let identity = RustmixBookIdentity::from_path("/books/Example.epub");
        let bookmark = RustmixBookmarkRecord::new(&identity, 3, 2048, "chapter start".into());
        let entry = RustmixBookmarkIndexRecord::from_bookmark(&bookmark, "Example");
        let encoded = encode_bookmarks_index(core::slice::from_ref(&entry));
        let decoded = decode_bookmarks_index(&encoded);

        assert_eq!(decoded, alloc::vec![entry.clone()]);
        assert_eq!(decoded[0].display_label(), "Example - chapter start");
        assert_eq!(
            decode_bookmark_jump(&entry.jump_message()).unwrap(),
            ("/books/Example.epub".into(), 3, 2048)
        );
    }

    #[test]
    fn round_trips_metadata_record_format() {
        let record = RustmixBookMetaRecord::from_path("/books/Example.epub");
        let encoded = record.encode_line();
        let decoded = RustmixBookMetaRecord::decode_line(&encoded).unwrap();

        assert_eq!(decoded, record);
    }

    #[test]
    fn round_trips_theme_record_format() {
        let preset = RustmixReaderThemePreset {
            font_size_idx: 5,
            margin_px: 8,
            line_spacing_pct: 100,
            alignment: "justify".into(),
            theme_name: "classic".into(),
        };
        let record = RustmixReaderThemeRecord::new("/books/Example.epub", preset);
        let encoded = record.encode_line();
        let decoded = RustmixReaderThemeRecord::decode_line(&encoded).unwrap();

        assert_eq!(decoded, record);
    }

    #[test]
    fn resolves_theme_name_to_current_indices() {
        assert_eq!(theme_idx_from_name("Default"), 0);
        assert_eq!(theme_idx_from_name("classic"), 1);
        assert_eq!(theme_idx_from_name("SERIF"), 2);
        assert_eq!(theme_idx_from_name("unknown"), 0);
    }
}
