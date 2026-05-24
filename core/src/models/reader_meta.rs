use heapless::String;
use serde::{Deserialize, Serialize};

use super::BookId;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BookFormat {
    Unknown,
    Txt,
    Epub,
    Xtc,
    Vchk,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReaderMeta {
    pub book_id: BookId,
    pub source_path: String<128>,
    pub display_title: String<96>,
    pub author: String<96>,
    pub format: BookFormat,
    pub chapter_count: u16,
    pub file_size_bytes: u64,
}

impl ReaderMeta {
    pub fn new(book_id: BookId, source_path: &str, format: BookFormat) -> Self {
        let mut path = String::new();
        let _ = path.push_str(source_path);

        Self {
            book_id,
            source_path: path,
            display_title: String::new(),
            author: String::new(),
            format,
            chapter_count: 0,
            file_size_bytes: 0,
        }
    }
}
