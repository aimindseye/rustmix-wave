use heapless::String;
use serde::{Deserialize, Serialize};

use super::{BookFormat, BookId};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookIdentity {
    pub book_id: BookId,
    pub source_path: String<128>,
    pub display_title: String<96>,
    pub author: String<96>,
    pub format: BookFormat,
    pub file_size_bytes: u64,
}

impl BookIdentity {
    pub fn new(book_id: BookId, source_path: &str, format: BookFormat) -> Self {
        let mut source = String::new();
        let _ = source.push_str(source_path);

        Self {
            book_id,
            source_path: source,
            display_title: String::new(),
            author: String::new(),
            format,
            file_size_bytes: 0,
        }
    }

    pub fn with_title(mut self, title: &str) -> Self {
        self.display_title.clear();
        let _ = self.display_title.push_str(title);
        self
    }

    pub fn with_author(mut self, author: &str) -> Self {
        self.author.clear();
        let _ = self.author.push_str(author);
        self
    }
}
