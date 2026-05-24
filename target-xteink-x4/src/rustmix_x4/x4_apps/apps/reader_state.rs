//! Reader state and book identity helpers for the X4 proving-ground repo.
//!
//! This module is intentionally simple and file-format conservative:
//! - no serde dependency
//! - no dynamic schema migration
//! - plain UTF-8 line encoding for debugging on SD
//!
//! The goal is to make the future Rustmix extraction easier without
//! disturbing the current X4 runtime path.
//!
//! This module makes the extraction surface explicit through BookIdentity,
//! BookStateLayout, and ReaderSliceDescriptor.

extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReaderFormat {
    Txt,
    Epub,
    Unknown,
}

impl ReaderFormat {
    pub fn from_path(path: &str) -> Self {
        let bytes = path.as_bytes();
        if bytes.len() >= 5 && bytes[bytes.len() - 5..] == *b".epub" {
            ReaderFormat::Epub
        } else if bytes.len() >= 4 && bytes[bytes.len() - 4..] == *b".txt" {
            ReaderFormat::Txt
        } else {
            ReaderFormat::Unknown
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            ReaderFormat::Txt => "txt",
            ReaderFormat::Epub => "epub",
            ReaderFormat::Unknown => "unknown",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "txt" => ReaderFormat::Txt,
            "epub" => ReaderFormat::Epub,
            _ => ReaderFormat::Unknown,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BookId(pub String);

impl BookId {
    pub fn from_path(path: &str) -> Self {
        Self(fingerprint_path(path))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.trim().is_empty()
    }

    pub fn matches_path(&self, path: &str) -> bool {
        *self == BookId::from_path(path)
    }
}

/// Reader-state boundary: stable, portable identity for one reader item.
///
/// This type is intentionally independent of X4 UI/kernel objects so the same
/// identity contract can be moved into Rustmix later. The current X4 build
/// still fingerprints paths, but all typed state should now move through this
/// identity model rather than raw UI strings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BookIdentity {
    pub book_id: BookId,
    pub source_path: String,
    pub display_title: String,
    pub format: ReaderFormat,
    pub fingerprint_kind: &'static str,
}

impl BookIdentity {
    pub fn from_path(path: &str) -> Self {
        Self {
            book_id: BookId::from_path(path),
            source_path: path.to_string(),
            display_title: display_title(path),
            format: ReaderFormat::from_path(path),
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

    pub fn open_path(&self) -> &str {
        self.source_path.as_str()
    }

    pub fn ui_title(&self) -> &str {
        if self.display_title.trim().is_empty() {
            self.source_path.as_str()
        } else {
            self.display_title.as_str()
        }
    }
}

/// Reader-state boundary: all per-book file locations needed by the
/// reader state slice. Paths remain 8.3-safe where the X4 SD stack requires it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BookStateLayout {
    pub book_id: BookId,
    pub state_dir: &'static str,
    pub cache_dir: String,
    pub meta_file: String,
    pub progress_file: String,
    pub theme_file: String,
    pub bookmarks_file: String,
    pub bookmarks_index_file: &'static str,
}

impl BookStateLayout {
    pub fn for_book_id(book_id: &BookId) -> Self {
        Self {
            book_id: book_id.clone(),
            state_dir: STATE_DIR,
            cache_dir: cache_dir_for(book_id),
            meta_file: meta_record_file_for(book_id),
            progress_file: progress_record_file_for(book_id),
            theme_file: theme_record_file_for(book_id),
            bookmarks_file: bookmark_record_file_for(book_id),
            bookmarks_index_file: BOOKMARKS_INDEX_FILE,
        }
    }

    pub fn for_path(path: &str) -> Self {
        Self::for_book_id(&BookId::from_path(path))
    }

    pub fn legacy_cache_dirs(&self) -> Vec<String> {
        candidate_cache_dirs_for(&self.book_id)
    }

    pub fn log_summary(&self) -> String {
        let mut out = String::new();
        out.push_str("meta=state/");
        out.push_str(&self.meta_file);
        out.push_str(" progress=state/");
        out.push_str(&self.progress_file);
        out.push_str(" theme=state/");
        out.push_str(&self.theme_file);
        out.push_str(" bookmarks=state/");
        out.push_str(&self.bookmarks_file);
        out.push_str(" index=state/");
        out.push_str(self.bookmarks_index_file);
        out.push_str(" cache=");
        out.push_str(&self.cache_dir);
        out
    }
}

/// Reader-state manifest for the Rustmix reader slice.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReaderSliceDescriptor {
    pub schema: &'static str,
    pub book_id_model: &'static str,
    pub identity: BookIdentity,
    pub layout: BookStateLayout,
}

impl ReaderSliceDescriptor {
    pub fn for_path(path: &str) -> Self {
        let identity = BookIdentity::from_path(path);
        let layout = BookStateLayout::for_book_id(&identity.book_id);
        Self {
            schema: READER_SLICE_SCHEMA,
            book_id_model: BOOK_ID_MODEL,
            identity,
            layout,
        }
    }

    pub fn with_display_title(mut self, title: &str) -> Self {
        self.identity = self.identity.with_display_title(title);
        self
    }

    pub fn log_summary(&self) -> String {
        let mut out = String::new();
        out.push_str(self.schema);
        out.push_str(" book_id=");
        out.push_str(self.identity.book_id.as_str());
        out.push_str(" format=");
        out.push_str(self.identity.format.as_str());
        out.push(' ');
        out.push_str(&self.layout.log_summary());
        out
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecentBookRecord {
    pub book_id: BookId,
    pub source_path: String,
    pub display_title: String,
    pub format: ReaderFormat,
    pub chapter: u16,
    pub page: u32,
    pub byte_offset: u32,
}

impl RecentBookRecord {
    pub fn from_path(path: &str) -> Self {
        Self::from_identity(&BookIdentity::from_path(path))
    }

    pub fn from_identity(identity: &BookIdentity) -> Self {
        Self {
            book_id: identity.book_id.clone(),
            source_path: identity.source_path.clone(),
            display_title: identity.display_title.clone(),
            format: identity.format,
            chapter: 0,
            page: 0,
            byte_offset: 0,
        }
    }

    pub fn open_path(&self) -> &str {
        self.source_path.as_str()
    }

    pub fn ui_title(&self) -> &str {
        let title = self.display_title.trim();
        if title.is_empty() {
            self.source_path.as_str()
        } else {
            self.display_title.as_str()
        }
    }

    pub fn encode_line(&self) -> String {
        let mut line = String::new();
        push_field(&mut line, self.book_id.as_str());
        push_field(&mut line, &self.source_path);
        push_field(&mut line, &self.display_title);
        push_field(&mut line, self.format.as_str());
        push_field(&mut line, &u32::from(self.chapter).to_string());
        push_field(&mut line, &self.page.to_string());
        push_field(&mut line, &self.byte_offset.to_string());
        line
    }

    pub fn decode_line(line: &str) -> Option<Self> {
        let fields = split_fields(line);
        if fields.len() != 7 {
            return None;
        }
        let book_id = BookId(fields[0].clone());
        if book_id.is_empty() {
            return None;
        }
        Some(Self {
            book_id,
            source_path: fields[1].clone(),
            display_title: fields[2].clone(),
            format: ReaderFormat::parse(&fields[3]),
            chapter: fields[4].parse().ok()?,
            page: fields[5].parse().ok()?,
            byte_offset: fields[6].parse().ok()?,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadingProgressRecord {
    pub book_id: BookId,
    pub source_path: String,
    pub format: ReaderFormat,
    pub chapter: u16,
    pub page: u32,
    pub byte_offset: u32,
    pub font_size_idx: u8,
}

impl ReadingProgressRecord {
    pub fn new(path: &str, chapter: u16, page: u32, byte_offset: u32, font_size_idx: u8) -> Self {
        Self::from_identity(
            &BookIdentity::from_path(path),
            chapter,
            page,
            byte_offset,
            font_size_idx,
        )
    }

    pub fn from_identity(
        identity: &BookIdentity,
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
        let book_id = BookId(fields[0].clone());
        if book_id.is_empty() {
            return None;
        }
        Some(Self {
            book_id,
            source_path: fields[1].clone(),
            format: ReaderFormat::parse(&fields[2]),
            chapter: fields[3].parse().ok()?,
            page: fields[4].parse().ok()?,
            byte_offset: fields[5].parse().ok()?,
            font_size_idx: fields[6].parse::<u16>().ok()? as u8,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BookMetaRecord {
    pub book_id: BookId,
    pub fingerprint_kind: String,
    pub source_path: String,
    pub display_title: String,
    pub format: ReaderFormat,
}

impl BookMetaRecord {
    pub fn from_path(path: &str) -> Self {
        Self::from_identity(&BookIdentity::from_path(path))
    }

    pub fn from_identity(identity: &BookIdentity) -> Self {
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
        let book_id = BookId(fields[0].clone());
        if book_id.is_empty() {
            return None;
        }
        Some(Self {
            book_id,
            fingerprint_kind: fields[1].clone(),
            source_path: fields[2].clone(),
            display_title: fields[3].clone(),
            format: ReaderFormat::parse(&fields[4]),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BookmarkRecord {
    pub book_id: BookId,
    pub source_path: String,
    pub chapter: u16,
    pub byte_offset: u32,
    pub label: String,
}

impl BookmarkRecord {
    pub fn new(identity: &BookIdentity, chapter: u16, byte_offset: u32, label: String) -> Self {
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
        let book_id = BookId(fields[0].clone());
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

pub fn decode_bookmarks(payload: &str) -> Vec<BookmarkRecord> {
    payload
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                None
            } else {
                BookmarkRecord::decode_line(line)
            }
        })
        .collect()
}

pub fn encode_bookmarks(bookmarks: &[BookmarkRecord]) -> String {
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
pub struct BookmarkIndexRecord {
    pub book_id: BookId,
    pub source_path: String,
    pub display_title: String,
    pub chapter: u16,
    pub byte_offset: u32,
    pub label: String,
}

impl BookmarkIndexRecord {
    pub fn from_bookmark(rec: &BookmarkRecord, display_title: impl Into<String>) -> Self {
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
        let book_id = BookId(fields[0].clone());
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
            out.push_str(" · ");
            out.push_str(detail);
        } else {
            out.push_str(" · Ch ");
            out.push_str(&(u32::from(self.chapter) + 1).to_string());
            out.push_str(" · Off ");
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

pub fn decode_bookmarks_index(payload: &str) -> Vec<BookmarkIndexRecord> {
    payload
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                None
            } else {
                BookmarkIndexRecord::decode_line(line)
            }
        })
        .collect()
}

pub fn encode_bookmarks_index(entries: &[BookmarkIndexRecord]) -> String {
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
pub struct ReaderThemePreset {
    pub font_size_idx: u8,
    pub margin_px: u16,
    pub line_spacing_pct: u8,
    pub alignment: String,
    pub theme_name: String,
}

pub const READER_SLICE_SCHEMA: &str = "rustmix-reader-slice-v1";
pub const BOOK_ID_MODEL: &str = "path-fnv1a32-v2";
pub const STATE_DIR: &str = "state";
pub const CACHE_DIR: &str = "cache";
pub const FINGERPRINT_KIND: &str = "path-v2";
pub const RECENT_RECORD_FILE: &str = "recent.txt";
// Legacy nested-cache filenames. Keep these constants so legacy readers can still
// read older SD cards, but new typed state is written flat under STATE_DIR with
// 8.3-safe names generated from the book id.
pub const PROGRESS_RECORD_FILE: &str = "progress.txt";
pub const BOOKMARKS_RECORD_FILE: &str = "BMARKS.TXT";
pub const THEME_RECORD_FILE: &str = "theme.txt";
pub const META_RECORD_FILE: &str = "meta.txt";

// Current flat typed-state filenames use per-book stems:
//   state/8A79A61F.PRG
//   state/8A79A61F.THM
//   state/8A79A61F.MTA
// Bookmarks intentionally stay on the already-working stable flat layout:
//   state/8A79A61F.BKM
//   state/BMIDX.TXT
pub const PROGRESS_RECORD_EXT: &str = ".PRG";
pub const THEME_RECORD_EXT: &str = ".THM";
pub const META_RECORD_EXT: &str = ".MTA";
pub const BOOKMARKS_INDEX_FILE: &str = "BMIDX.TXT";
pub const BOOKMARK_JUMP_PREFIX: &str = "BMJ";
pub const THEME_NAMES: &[&str] = &["Default", "Classic", "Serif"];

pub fn theme_idx_from_name(name: &str) -> u8 {
    for (idx, candidate) in THEME_NAMES.iter().enumerate() {
        if name.eq_ignore_ascii_case(candidate) {
            return idx as u8;
        }
    }
    0
}

impl Default for ReaderThemePreset {
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

impl ReaderThemePreset {
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
pub struct ReaderThemeRecord {
    pub book_id: BookId,
    pub source_path: String,
    pub format: ReaderFormat,
    pub preset: ReaderThemePreset,
}

impl ReaderThemeRecord {
    pub fn new(path: &str, preset: ReaderThemePreset) -> Self {
        Self::from_identity(&BookIdentity::from_path(path), preset)
    }

    pub fn from_identity(identity: &BookIdentity, preset: ReaderThemePreset) -> Self {
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
        let book_id = BookId(fields[0].clone());
        if book_id.is_empty() {
            return None;
        }
        Some(Self {
            book_id,
            source_path: fields[1].clone(),
            format: ReaderFormat::parse(&fields[2]),
            preset: ReaderThemePreset {
                font_size_idx: fields[3].parse::<u16>().ok()? as u8,
                margin_px: fields[4].parse().ok()?,
                line_spacing_pct: fields[5].parse::<u16>().ok()? as u8,
                alignment: fields[6].clone(),
                theme_name: fields[7].clone(),
            },
        })
    }
}

pub fn state_root() -> &'static str {
    STATE_DIR
}

pub fn cache_root() -> &'static str {
    CACHE_DIR
}

pub fn recent_record_file() -> &'static str {
    RECENT_RECORD_FILE
}

pub fn book_id_hex8(book_id: &BookId) -> String {
    let raw = book_id
        .as_str()
        .strip_prefix("bk-")
        .unwrap_or(book_id.as_str());
    let mut stem = String::new();
    for ch in raw.chars() {
        if ch.is_ascii_hexdigit() {
            stem.push(ch.to_ascii_uppercase());
            if stem.len() >= 8 {
                break;
            }
        }
    }
    while stem.len() < 8 {
        stem.push('0');
    }
    stem
}

pub fn cache_dir_for(book_id: &BookId) -> String {
    let mut out = String::from(cache_root());
    out.push('/');
    out.push_str(&book_id_hex8(book_id));
    out
}

pub fn legacy_cache_dir_for(book_id: &BookId) -> String {
    let mut out = String::from(cache_root());
    out.push('/');
    out.push_str(book_id.as_str());
    out
}

pub fn legacy_root_cache_dir_for(book_id: &BookId) -> String {
    String::from(book_id.as_str())
}

pub fn candidate_cache_dirs_for(book_id: &BookId) -> Vec<String> {
    vec![
        cache_dir_for(book_id),
        legacy_cache_dir_for(book_id),
        legacy_root_cache_dir_for(book_id),
    ]
}

fn typed_state_file_for(book_id: &BookId, ext: &str) -> String {
    let mut stem = book_id_hex8(book_id);
    stem.push_str(ext);
    stem
}

/// Current 8.3-safe flat progress record filename, relative to STATE_DIR.
pub fn progress_record_file_for(book_id: &BookId) -> String {
    typed_state_file_for(book_id, PROGRESS_RECORD_EXT)
}

/// Current 8.3-safe flat theme record filename, relative to STATE_DIR.
pub fn theme_record_file_for(book_id: &BookId) -> String {
    typed_state_file_for(book_id, THEME_RECORD_EXT)
}

/// Current 8.3-safe flat meta record filename, relative to STATE_DIR.
pub fn meta_record_file_for(book_id: &BookId) -> String {
    typed_state_file_for(book_id, META_RECORD_EXT)
}

/// Human-readable full progress path for logs/debug only.
pub fn progress_file_for(book_id: &BookId) -> String {
    let mut out = String::from(STATE_DIR);
    out.push('/');
    out.push_str(&progress_record_file_for(book_id));
    out
}

/// Human-readable full theme path for logs/debug only.
pub fn theme_file_for(book_id: &BookId) -> String {
    let mut out = String::from(STATE_DIR);
    out.push('/');
    out.push_str(&theme_record_file_for(book_id));
    out
}

/// Legacy nested-cache bookmark path helper. New bookmarks use
/// bookmark_record_file_for() under STATE_DIR and should remain unchanged.
pub fn bookmarks_file_for(book_id: &BookId) -> String {
    let mut out = cache_dir_for(book_id);
    out.push('/');
    out.push_str(BOOKMARKS_RECORD_FILE);
    out
}

/// Human-readable full meta path for logs/debug only.
pub fn meta_file_for(book_id: &BookId) -> String {
    let mut out = String::from(STATE_DIR);
    out.push('/');
    out.push_str(&meta_record_file_for(book_id));
    out
}

pub fn empty_bookmarks_payload() -> &'static [u8] {
    b""
}

/// FAT/embedded-sdmmc safe 8.3 bookmark record filename.
///
/// The app already uses book ids like `bk-8a79a61f`. The X4 SD write
/// path is happiest with short 8.3 names, so per-book bookmarks are stored
/// flat under STATE_DIR as `<8hex>.BKM`, for example `8A79A61F.BKM`.
pub fn bookmark_record_file_for(book_id: &BookId) -> String {
    let mut stem = book_id_hex8(book_id);
    stem.push_str(".BKM");
    stem
}

pub fn fingerprint_path(path: &str) -> String {
    let normalized = normalized_path_key(path);
    let mut hash: u32 = 0x811C9DC5;
    for &b in normalized.as_bytes() {
        hash ^= b as u32;
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
            c => c,
        };
        out.push(normalized.to_ascii_lowercase());
    }
    out
}

pub fn display_title(path: &str) -> String {
    if let Some((_, tail)) = path.rsplit_once('/') {
        if !tail.is_empty() {
            return tail.to_string();
        }
    }
    path.to_string()
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

fn percent_decode(s: &str) -> String {
    s.replace("%7C", "|")
        .replace("%0A", "\n")
        .replace("%0D", "\r")
}

fn append_hex_u32(out: &mut String, value: u32) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for shift in (0..=28).rev().step_by(4) {
        let idx = ((value >> shift) & 0x0f) as usize;
        out.push(HEX[idx] as char);
    }
}
