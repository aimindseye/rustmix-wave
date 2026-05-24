use heapless::String;
use serde::{Deserialize, Serialize};

use super::BookId;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReaderBookmark {
    pub book_id: BookId,
    pub source_path: String<128>,
    pub display_title: String<96>,
    pub chapter: u16,
    pub page: u16,
    pub byte_offset: u32,
    pub label: String<96>,
}

impl ReaderBookmark {
    pub fn new(
        book_id: BookId,
        source_path: &str,
        chapter: u16,
        page: u16,
        byte_offset: u32,
    ) -> Self {
        let mut path = String::new();
        let _ = path.push_str(source_path);

        let mut label = String::new();
        let _ = core::fmt::write(
            &mut label,
            format_args!(
                "Ch {} · Pg {} · Off {}",
                chapter as u32 + 1,
                page as u32 + 1,
                byte_offset
            ),
        );

        Self {
            book_id,
            source_path: path,
            display_title: String::new(),
            chapter,
            page,
            byte_offset,
            label,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookmarkIndexRecord {
    pub book_id: BookId,
    pub source_path: String<128>,
    pub display_title: String<96>,
    pub chapter: u16,
    pub page: u16,
    pub byte_offset: u32,
    pub label: String<96>,
}

impl From<ReaderBookmark> for BookmarkIndexRecord {
    fn from(value: ReaderBookmark) -> Self {
        Self {
            book_id: value.book_id,
            source_path: value.source_path,
            display_title: value.display_title,
            chapter: value.chapter,
            page: value.page,
            byte_offset: value.byte_offset,
            label: value.label,
        }
    }
}
