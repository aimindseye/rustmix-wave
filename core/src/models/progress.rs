use heapless::String;
use serde::{Deserialize, Serialize};

use super::BookId;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReaderProgress {
    pub book_id: BookId,
    pub source_path: String<128>,
    pub chapter: u16,
    pub page: u16,
    pub byte_offset: u32,
    pub percentage_x100: u16,
    pub font_size_idx: u8,
}

impl ReaderProgress {
    pub fn new(book_id: BookId, source_path: &str) -> Self {
        let mut path = String::new();
        let _ = path.push_str(source_path);
        Self {
            book_id,
            source_path: path,
            chapter: 0,
            page: 0,
            byte_offset: 0,
            percentage_x100: 0,
            font_size_idx: 0,
        }
    }

    pub fn with_position(
        mut self,
        chapter: u16,
        page: u16,
        byte_offset: u32,
        percentage_x100: u16,
    ) -> Self {
        self.chapter = chapter;
        self.page = page;
        self.byte_offset = byte_offset;
        self.percentage_x100 = percentage_x100;
        self
    }
}
