//! Reader app-level contracts.
//!
//! This module moves stable reader/library vocabulary into `crate::models` while
//! leaving the current X4 target smoke runtime untouched. Target code can now
//! progressively replace local structs with these core models without pulling
//! hardware dependencies into `core`.

pub use crate::models::{
    BookFormat, BookId, BookIdScheme, BookmarkIndexRecord, LibraryEntry, LibraryScanPolicy,
    ReaderBookmark, ReaderFileKind, ReaderMeta, ReaderNavAction, ReaderPageState, ReaderProgress,
    ReaderSessionState, ReaderStoragePaths, ReaderThemePreset, ReaderUiMode, StorageLayout,
    StorageLayoutKind,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReaderOpenMode {
    Continue,
    FromLibrary,
    FromBookmark,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReaderOpenRequest {
    pub book_id: BookId,
    pub mode: ReaderOpenMode,
    pub progress: Option<ReaderProgress>,
}

impl ReaderOpenRequest {
    pub fn continue_from(progress: ReaderProgress) -> Self {
        Self {
            book_id: progress.book_id.clone(),
            mode: ReaderOpenMode::Continue,
            progress: Some(progress),
        }
    }

    pub fn from_library(book_id: BookId) -> Self {
        Self {
            book_id,
            mode: ReaderOpenMode::FromLibrary,
            progress: None,
        }
    }

    pub fn from_bookmark(book_id: BookId, progress: ReaderProgress) -> Self {
        Self {
            book_id,
            mode: ReaderOpenMode::FromBookmark,
            progress: Some(progress),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReaderLibrarySelection {
    pub selected_index: u8,
    pub entry: LibraryEntry,
}

impl ReaderLibrarySelection {
    pub fn new(selected_index: u8, entry: LibraryEntry) -> Self {
        Self {
            selected_index,
            entry,
        }
    }

    pub fn can_open_now(&self) -> bool {
        self.entry.kind.is_text()
    }

    pub fn is_deferred_reader(&self) -> bool {
        self.entry.kind.is_epub_like()
    }
}
