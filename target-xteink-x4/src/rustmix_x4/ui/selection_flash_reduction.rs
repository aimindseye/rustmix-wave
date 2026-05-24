//! UI selection redraw policy descriptors for low-flash navigation.
//!
//! This module documents the Rustmix-owned UI policy used by the active
//! Xteink X4 runtime. It intentionally does not move SSD1677 refresh,
//! ghost-clear, EPUB loading, or reader page-turn behavior.

#![allow(dead_code)]

pub const UI_SELECTION_FLASH_REDUCTION_MARKER: &str = "ui-selection-flash-reduction-rustmix-ok";

pub const REDUCE_DASHBOARD_SELECTION_FLASH: bool = true;
pub const REDUCE_LIBRARY_SELECTION_FLASH: bool = true;
pub const REDUCE_BOOKMARK_SELECTION_FLASH: bool = true;

pub const TOUCHES_DISPLAY_REFRESH_SCHEDULER: bool = false;
pub const TOUCHES_SSD1677_DRIVER: bool = false;
pub const TOUCHES_EPUB_LOADING_POLICY: bool = false;
pub const TOUCHES_READER_PAGE_TURN_POLICY: bool = false;
pub const TOUCHES_SD_DRIVER: bool = false;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectionVisualStyle {
    InvertedBlock,
    Rail,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SelectionFlashReductionPolicy {
    pub dashboard_style: SelectionVisualStyle,
    pub library_style: SelectionVisualStyle,
    pub bookmark_style: SelectionVisualStyle,
    pub keep_full_refresh_on_entry_exit: bool,
    pub keep_reader_page_turn_refresh: bool,
}

impl SelectionFlashReductionPolicy {
    pub const fn active() -> Self {
        Self {
            dashboard_style: SelectionVisualStyle::Rail,
            library_style: SelectionVisualStyle::Rail,
            bookmark_style: SelectionVisualStyle::Rail,
            keep_full_refresh_on_entry_exit: true,
            keep_reader_page_turn_refresh: true,
        }
    }
}

pub const fn marker() -> &'static str {
    UI_SELECTION_FLASH_REDUCTION_MARKER
}
