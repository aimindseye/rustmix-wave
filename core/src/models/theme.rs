use heapless::String;
use serde::{Deserialize, Serialize};

use super::BookId;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeKind {
    Classic,
    Warm,
    HighContrast,
    Custom,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeContrast {
    Normal,
    Strong,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReaderThemePreset {
    pub book_id: BookId,
    pub name: String<32>,
    pub kind: ThemeKind,
    pub font_size_idx: u8,
    pub line_spacing_pct: u16,
    pub margin_px: u16,
    pub contrast: ThemeContrast,
}

impl ReaderThemePreset {
    pub fn classic(book_id: BookId) -> Self {
        let mut name = String::new();
        let _ = name.push_str("classic");
        Self {
            book_id,
            name,
            kind: ThemeKind::Classic,
            font_size_idx: 2,
            line_spacing_pct: 120,
            margin_px: 16,
            contrast: ThemeContrast::Normal,
        }
    }
}
