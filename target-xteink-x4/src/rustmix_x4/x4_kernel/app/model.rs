#![allow(dead_code)]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppScreen {
    Home,
    Browser,
    Reader,
    Settings,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppAction {
    Up,
    Down,
    Left,
    Right,
    Select,
    Back,
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HomeMenuItem {
    ContinueReading,
    FileBrowser,
    Bookmarks,
    Settings,
    Sync,
    Upload,
}

impl HomeMenuItem {
    pub const fn title(self) -> &'static str {
        match self {
            HomeMenuItem::ContinueReading => "Continue",
            HomeMenuItem::FileBrowser => "Files",
            HomeMenuItem::Bookmarks => "Bookmarks",
            HomeMenuItem::Settings => "Settings",
            HomeMenuItem::Sync => "Sync",
            HomeMenuItem::Upload => "Upload",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrowserEntryKind {
    File,
    Directory,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserEntry {
    pub name: String,
    pub kind: BrowserEntryKind,
}

impl BrowserEntry {
    pub fn file(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: BrowserEntryKind::File,
        }
    }

    pub fn directory(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: BrowserEntryKind::Directory,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReaderFormat {
    Txt,
    Epub,
    Unknown,
}

impl ReaderFormat {
    pub const fn from_is_epub(is_epub: bool) -> Self {
        if is_epub {
            ReaderFormat::Epub
        } else {
            ReaderFormat::Txt
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            ReaderFormat::Txt => "txt",
            ReaderFormat::Epub => "epub",
            ReaderFormat::Unknown => "unknown",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BookIdentity {
    pub book_id: String,
    pub source_path: String,
    pub display_title: String,
    pub format: ReaderFormat,
}

impl BookIdentity {
    pub fn from_path(path: impl Into<String>, is_epub: bool) -> Self {
        let source_path = path.into();
        let display_title = basename(&source_path);
        let format = ReaderFormat::from_is_epub(is_epub);
        let book_id = derive_book_id(&source_path, format);

        Self {
            book_id,
            source_path,
            display_title,
            format,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReaderSession {
    pub book_id: String,
    pub source_path: String,
    pub display_title: String,
    pub format: ReaderFormat,
    pub current_page: u32,
    pub chapter: u16,
    pub byte_offset: u32,
    pub handoff_pending: bool,
}

impl ReaderSession {
    pub fn new(
        book_path: impl Into<String>,
        current_page: u32,
        chapter: u16,
        is_epub: bool,
    ) -> Self {
        let identity = BookIdentity::from_path(book_path, is_epub);
        Self {
            book_id: identity.book_id,
            source_path: identity.source_path,
            display_title: identity.display_title,
            format: identity.format,
            current_page,
            chapter,
            byte_offset: 0,
            handoff_pending: false,
        }
    }

    pub fn pending(book_path: impl Into<String>, is_epub: bool) -> Self {
        let identity = BookIdentity::from_path(book_path, is_epub);
        Self {
            book_id: identity.book_id,
            source_path: identity.source_path,
            display_title: identity.display_title,
            format: identity.format,
            current_page: 0,
            chapter: 0,
            byte_offset: 0,
            handoff_pending: true,
        }
    }

    pub fn with_byte_offset(mut self, byte_offset: u32) -> Self {
        self.byte_offset = byte_offset;
        self
    }

    pub fn is_epub(&self) -> bool {
        self.format == ReaderFormat::Epub
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContinueTarget {
    pub book_id: String,
    pub source_path: String,
    pub display_title: String,
    pub format: ReaderFormat,
}

impl ContinueTarget {
    pub fn from_path(path: impl Into<String>, is_epub: bool) -> Self {
        let identity = BookIdentity::from_path(path, is_epub);
        Self {
            book_id: identity.book_id,
            source_path: identity.source_path,
            display_title: identity.display_title,
            format: identity.format,
        }
    }

    pub fn legacy_path(&self) -> &str {
        &self.source_path
    }
}

#[derive(Clone, Debug, Default)]
pub struct AppShell {
    screen: Option<AppScreen>,
    continue_target: Option<ContinueTarget>,
    home_items: Vec<HomeMenuItem>,
    home_selected: usize,
    browser_path: String,
    browser_scroll: usize,
    browser_selected: usize,
    browser_total: usize,
    browser_entries: Vec<BrowserEntry>,
    reader_session: Option<ReaderSession>,
}

impl AppShell {
    pub fn new() -> Self {
        Self {
            screen: Some(AppScreen::Home),
            continue_target: None,
            home_items: Vec::new(),
            home_selected: 0,
            browser_path: String::new(),
            browser_scroll: 0,
            browser_selected: 0,
            browser_total: 0,
            browser_entries: Vec::new(),
            reader_session: None,
        }
    }

    pub fn screen(&self) -> AppScreen {
        self.screen.unwrap_or(AppScreen::Home)
    }

    pub fn set_screen(&mut self, screen: AppScreen) {
        self.screen = Some(screen);
    }

    pub fn continue_target(&self) -> Option<&str> {
        self.continue_target
            .as_ref()
            .map(|t| t.source_path.as_str())
    }

    pub fn continue_target_ref(&self) -> Option<&ContinueTarget> {
        self.continue_target.as_ref()
    }

    pub fn set_continue_target(&mut self, path: impl Into<String>) {
        let path = path.into();
        let is_epub = path.as_bytes().ends_with(b".epub");
        self.continue_target = Some(ContinueTarget::from_path(path, is_epub));
    }

    pub fn set_continue_target_session(&mut self, session: &ReaderSession) {
        self.continue_target = Some(ContinueTarget {
            book_id: session.book_id.clone(),
            source_path: session.source_path.clone(),
            display_title: session.display_title.clone(),
            format: session.format,
        });
    }

    pub fn clear_continue_target(&mut self) {
        self.continue_target = None;
    }

    pub fn home_items(&self) -> &[HomeMenuItem] {
        &self.home_items
    }

    pub fn home_selected(&self) -> usize {
        self.home_selected
    }

    pub fn set_home(&mut self, items: Vec<HomeMenuItem>, selected: usize) {
        self.home_items = items;
        self.home_selected = if self.home_items.is_empty() {
            0
        } else {
            selected.min(self.home_items.len() - 1)
        };
    }

    pub fn browser_path(&self) -> &str {
        &self.browser_path
    }

    pub fn browser_scroll(&self) -> usize {
        self.browser_scroll
    }

    pub fn browser_selected(&self) -> usize {
        self.browser_selected
    }

    pub fn browser_total(&self) -> usize {
        self.browser_total
    }

    pub fn browser_entries(&self) -> &[BrowserEntry] {
        &self.browser_entries
    }

    pub fn set_browser_state(
        &mut self,
        path: impl Into<String>,
        scroll: usize,
        selected: usize,
        total: usize,
        entries: Vec<BrowserEntry>,
    ) {
        self.browser_path = path.into();
        self.browser_scroll = scroll;
        self.browser_total = total;
        self.browser_entries = entries;
        self.browser_selected = if self.browser_entries.is_empty() {
            0
        } else {
            selected.min(self.browser_entries.len() - 1)
        };
    }

    pub fn reader_session(&self) -> Option<&ReaderSession> {
        self.reader_session.as_ref()
    }

    pub fn set_reader_session(&mut self, session: ReaderSession) {
        self.continue_target = Some(ContinueTarget {
            book_id: session.book_id.clone(),
            source_path: session.source_path.clone(),
            display_title: session.display_title.clone(),
            format: session.format,
        });
        self.reader_session = Some(session);
    }

    pub fn clear_reader_session(&mut self) {
        self.reader_session = None;
    }

    pub fn begin_reader_handoff(&mut self, book_path: impl Into<String>, is_epub: bool) {
        let session = ReaderSession::pending(book_path, is_epub);
        self.set_continue_target_session(&session);
        self.reader_session = Some(session);
        self.screen = Some(AppScreen::Reader);
    }

    pub fn update_reader_progress(
        &mut self,
        book_path: impl Into<String>,
        current_page: u32,
        chapter: u16,
        is_epub: bool,
    ) {
        let session = ReaderSession::new(book_path, current_page, chapter, is_epub);
        self.set_continue_target_session(&session);
        self.reader_session = Some(session);
        self.screen = Some(AppScreen::Reader);
    }

    pub fn update_reader_progress_with_offset(
        &mut self,
        book_path: impl Into<String>,
        current_page: u32,
        chapter: u16,
        is_epub: bool,
        byte_offset: u32,
    ) {
        let session = ReaderSession::new(book_path, current_page, chapter, is_epub)
            .with_byte_offset(byte_offset);
        self.set_continue_target_session(&session);
        self.reader_session = Some(session);
        self.screen = Some(AppScreen::Reader);
    }
}

fn basename(path: &str) -> String {
    if let Some((_, tail)) = path.rsplit_once('/') {
        if !tail.is_empty() {
            return tail.into();
        }
    }
    path.into()
}

fn derive_book_id(path: &str, format: ReaderFormat) -> String {
    let mut hash: u32 = 0x811C9DC5;
    for &b in path.as_bytes() {
        hash ^= b as u32;
        hash = hash.wrapping_mul(0x01000193);
    }
    for &b in format.as_str().as_bytes() {
        hash ^= b as u32;
        hash = hash.wrapping_mul(0x01000193);
    }

    let mut out = String::from("bk-");
    append_hex_u32(&mut out, hash);
    out
}

fn append_hex_u32(out: &mut String, value: u32) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for shift in (0..=28).rev().step_by(4) {
        let idx = ((value >> shift) & 0x0f) as usize;
        out.push(HEX[idx] as char);
    }
}
