//! Biscuit-inspired Files/Library layout descriptors.
//!
//! The current implementation changes only the Files/Library visual row style. It does not change
//! the title source, input mapping, write lane, display geometry, or reader handoff.

#![allow(dead_code)]

use super::biscuit_layout::{BiscuitRect, BiscuitScreenLayout};
use super::biscuit_tokens::{BISCUIT_LIST, BISCUIT_SPACING, BISCUIT_TYPOGRAPHY};

pub const FILES_BISCUIT_LIST_MARKER: &str = "x4-files-biscuit-list-patch-ok";

pub const CHANGES_HOME_RENDERING: bool = false;
pub const CHANGES_FILES_RENDERING: bool = true;
pub const CHANGES_READER_RENDERING: bool = false;
pub const CHANGES_TITLE_WORKFLOW: bool = false;
pub const CHANGES_FOOTER_LABELS: bool = false;
pub const CHANGES_INPUT_MAPPING: bool = false;
pub const TOUCHES_WRITE_LANE: bool = false;
pub const TOUCHES_DISPLAY_GEOMETRY: bool = false;
pub const TOUCHES_READER_PAGINATION: bool = false;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BiscuitFilesLayout {
    pub header: BiscuitRect,
    pub status: BiscuitRect,
    pub list: BiscuitRect,
    pub row_height: u16,
    pub row_gap: u16,
}

impl BiscuitFilesLayout {
    pub const fn from_screen(screen: BiscuitScreenLayout) -> Self {
        let header = BiscuitRect::new(
            screen.content.x,
            screen.content.y,
            screen.content.w.saturating_sub(96),
            BISCUIT_TYPOGRAPHY.title_line_height,
        );

        let status = BiscuitRect::new(
            screen.content.right().saturating_sub(96),
            screen.content.y,
            96,
            BISCUIT_TYPOGRAPHY.body_line_height,
        );

        let list_y = header.bottom().saturating_add(BISCUIT_SPACING.lg);

        let list = BiscuitRect::new(
            screen.content.x,
            list_y,
            screen.content.w,
            screen
                .footer
                .y
                .saturating_sub(BISCUIT_SPACING.md)
                .saturating_sub(list_y),
        );

        Self {
            header,
            status,
            list,
            row_height: BISCUIT_LIST.row_height,
            row_gap: BISCUIT_SPACING.row_gap,
        }
    }
}

pub const fn files_biscuit_list_marker() -> &'static str {
    FILES_BISCUIT_LIST_MARKER
}
