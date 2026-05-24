//! Reader runtime state models shared by target adapters and future core apps.
//!
//! Keeps X4 hardware code in `target-xteink-x4`; this module gives the
//! reader/library flow a stable, target-neutral vocabulary before EPUB parity.

use super::{BookId, LibraryEntry, ReaderFileKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReaderUiMode {
    Library,
    Reader,
    BookmarkList,
    EpubPending,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReaderNavAction {
    None,
    MovePrevious,
    MoveNext,
    OpenSelected,
    Back,
    ToggleBookmark,
    OpenBookmarkList,
    JumpToBookmark(u8),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReaderPageState {
    pub byte_offset: u32,
    pub read_len: u16,
    pub page_index: u16,
    pub total_pages: u16,
    pub bookmark_count: u8,
    pub current_page_marked: bool,
}

impl ReaderPageState {
    pub const fn first_page(total_pages: u16) -> Self {
        Self {
            byte_offset: 0,
            read_len: 0,
            page_index: 1,
            total_pages,
            bookmark_count: 0,
            current_page_marked: false,
        }
    }

    pub fn from_offset(byte_offset: u32, page_size: u32, read_len: u16, total_pages: u16) -> Self {
        let safe_page_size = if page_size == 0 { 1 } else { page_size };
        let page_index = (byte_offset / safe_page_size) as u16 + 1;
        Self {
            byte_offset,
            read_len,
            page_index,
            total_pages: total_pages.max(1),
            bookmark_count: 0,
            current_page_marked: false,
        }
    }

    pub const fn is_last_page(self) -> bool {
        self.page_index >= self.total_pages
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReaderSessionState {
    pub book_id: BookId,
    pub entry: LibraryEntry,
    pub mode: ReaderUiMode,
    pub page: ReaderPageState,
}

impl ReaderSessionState {
    pub fn open_text(book_id: BookId, entry: LibraryEntry, page: ReaderPageState) -> Self {
        Self {
            book_id,
            entry,
            mode: ReaderUiMode::Reader,
            page,
        }
    }

    pub fn epub_pending(book_id: BookId, entry: LibraryEntry) -> Self {
        Self {
            book_id,
            entry,
            mode: ReaderUiMode::EpubPending,
            page: ReaderPageState::first_page(1),
        }
    }

    pub fn file_kind(&self) -> ReaderFileKind {
        self.entry.kind
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::BookIdScheme;

    #[test]
    fn reader_page_state_maps_offset_to_one_based_page() {
        let page = ReaderPageState::from_offset(2048, 1024, 1024, 19);
        assert_eq!(page.page_index, 3);
        assert_eq!(page.total_pages, 19);
        assert!(!page.is_last_page());
    }

    #[test]
    fn text_session_preserves_entry_and_kind() {
        let book_id = BookId::new(BookIdScheme::ContentSampleFnv1a32V1, "b29c8c4f");
        let entry = LibraryEntry::new("BOOKS/LONG.TXT", 18_984);
        let session =
            ReaderSessionState::open_text(book_id, entry, ReaderPageState::first_page(19));
        assert_eq!(session.mode, ReaderUiMode::Reader);
        assert_eq!(session.file_kind(), ReaderFileKind::Txt);
        assert_eq!(session.entry.path.as_str(), "BOOKS/LONG.TXT");
    }
}
