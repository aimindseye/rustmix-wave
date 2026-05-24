//! Reader-facing file and library entry models.
//!
//! These are intentionally pure, host-testable models. They do not know about
//! embedded-sdmmc, X4 pins, the target runtime, or the current display smoke
//! implementation.

use heapless::String;

pub const LIBRARY_PATH_MAX: usize = 96;
pub const LIBRARY_DISPLAY_NAME_MAX: usize = 40;
pub const LIBRARY_KIND_LABEL_MAX: usize = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReaderFileKind {
    Txt,
    Markdown,
    Epu,
    Epub,
    Unsupported,
}

impl ReaderFileKind {
    pub fn from_path(path: &str) -> Self {
        if ends_with_ascii_case_insensitive(path.as_bytes(), b".TXT") {
            Self::Txt
        } else if ends_with_ascii_case_insensitive(path.as_bytes(), b".MD") {
            Self::Markdown
        } else if ends_with_ascii_case_insensitive(path.as_bytes(), b".EPU") {
            Self::Epu
        } else if ends_with_ascii_case_insensitive(path.as_bytes(), b".EPUB") {
            Self::Epub
        } else {
            Self::Unsupported
        }
    }

    pub const fn is_supported(self) -> bool {
        !matches!(self, Self::Unsupported)
    }

    pub const fn is_text(self) -> bool {
        matches!(self, Self::Txt | Self::Markdown)
    }

    pub const fn is_epub_like(self) -> bool {
        matches!(self, Self::Epu | Self::Epub)
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Txt => "TXT",
            Self::Markdown => "MD",
            Self::Epu => "EPU",
            Self::Epub => "EPUB",
            Self::Unsupported => "?",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryEntry {
    pub path: String<LIBRARY_PATH_MAX>,
    pub display_name: String<LIBRARY_DISPLAY_NAME_MAX>,
    pub kind: ReaderFileKind,
    pub size_bytes: u32,
}

impl LibraryEntry {
    pub fn new(path: &str, size_bytes: u32) -> Self {
        let kind = ReaderFileKind::from_path(path);
        let mut entry = Self {
            path: String::new(),
            display_name: String::new(),
            kind,
            size_bytes,
        };
        let _ = entry.path.push_str(path);
        let _ = entry.display_name.push_str(basename(path));
        entry
    }

    pub fn empty() -> Self {
        Self {
            path: String::new(),
            display_name: String::new(),
            kind: ReaderFileKind::Unsupported,
            size_bytes: 0,
        }
    }

    pub fn is_openable_now(&self) -> bool {
        self.kind.is_text()
    }

    pub fn is_pending_reader(&self) -> bool {
        self.kind.is_epub_like()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LibraryScanPolicy {
    pub include_root: bool,
    pub max_depth: u8,
    pub include_txt: bool,
    pub include_md: bool,
    pub include_epu: bool,
    pub include_epub: bool,
}

impl LibraryScanPolicy {
    pub const fn recursive_all_files() -> Self {
        Self {
            include_root: true,
            max_depth: 4,
            include_txt: true,
            include_md: true,
            include_epu: true,
            include_epub: true,
        }
    }

    pub const fn accepts(self, kind: ReaderFileKind) -> bool {
        match kind {
            ReaderFileKind::Txt => self.include_txt,
            ReaderFileKind::Markdown => self.include_md,
            ReaderFileKind::Epu => self.include_epu,
            ReaderFileKind::Epub => self.include_epub,
            ReaderFileKind::Unsupported => false,
        }
    }
}

fn basename(path: &str) -> &str {
    match path.rsplit_once('/') {
        Some((_, name)) => name,
        None => path,
    }
}

fn ends_with_ascii_case_insensitive(haystack: &[u8], suffix: &[u8]) -> bool {
    if haystack.len() < suffix.len() {
        return false;
    }

    let offset = haystack.len() - suffix.len();
    let mut i = 0;
    while i < suffix.len() {
        if !haystack[offset + i].eq_ignore_ascii_case(&suffix[i]) {
            return false;
        }
        i += 1;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_kind_detects_reader_extensions_case_insensitively() {
        assert_eq!(
            ReaderFileKind::from_path("BOOKS/LONG.TXT"),
            ReaderFileKind::Txt
        );
        assert_eq!(
            ReaderFileKind::from_path("notes.md"),
            ReaderFileKind::Markdown
        );
        assert_eq!(
            ReaderFileKind::from_path("Fiction/Dracula.epub"),
            ReaderFileKind::Epub
        );
        assert_eq!(ReaderFileKind::from_path("LEGACY.EPU"), ReaderFileKind::Epu);
        assert_eq!(
            ReaderFileKind::from_path("image.png"),
            ReaderFileKind::Unsupported
        );
    }

    #[test]
    fn library_entry_keeps_path_and_basename() {
        let entry = LibraryEntry::new("BOOKS/LONG.TXT", 18_984);
        assert_eq!(entry.path.as_str(), "BOOKS/LONG.TXT");
        assert_eq!(entry.display_name.as_str(), "LONG.TXT");
        assert_eq!(entry.kind, ReaderFileKind::Txt);
        assert!(entry.is_openable_now());
        assert!(!entry.is_pending_reader());
    }

    #[test]
    fn recursive_policy_accepts_all_reader_files() {
        let policy = LibraryScanPolicy::recursive_all_files();
        assert!(policy.accepts(ReaderFileKind::Txt));
        assert!(policy.accepts(ReaderFileKind::Markdown));
        assert!(policy.accepts(ReaderFileKind::Epu));
        assert!(policy.accepts(ReaderFileKind::Epub));
        assert!(!policy.accepts(ReaderFileKind::Unsupported));
    }
}
