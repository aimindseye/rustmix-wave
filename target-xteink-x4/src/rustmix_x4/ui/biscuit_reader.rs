//! Biscuit-inspired Reader shell descriptors.
//!
//! layout, pagination, restore, input mapping, title-cache, or write-lane behavior.

#![allow(dead_code)]

use super::biscuit_layout::{BiscuitRect, BiscuitScreenLayout};
use super::biscuit_tokens::{BISCUIT_CHROME, BISCUIT_SPACING, BISCUIT_TYPOGRAPHY};

pub const READER_BISCUIT_SHELL_MARKER: &str = "x4-reader-biscuit-shell-polish-ok";

pub const READER_STATUS_PREFIX: &str = "Read ";
pub const READER_LOADING_LABEL: &str = "Reading";

pub const CHANGES_HOME_RENDERING: bool = false;
pub const CHANGES_FILES_RENDERING: bool = false;
pub const CHANGES_READER_RENDERING: bool = true;
pub const CHANGES_TEXT_LAYOUT: bool = false;
pub const CHANGES_TITLE_WORKFLOW: bool = false;
pub const CHANGES_FOOTER_LABELS: bool = false;
pub const CHANGES_INPUT_MAPPING: bool = false;
pub const TOUCHES_WRITE_LANE: bool = false;
pub const TOUCHES_DISPLAY_GEOMETRY: bool = false;
pub const TOUCHES_READER_PAGINATION: bool = false;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BiscuitReaderShellLayout {
    pub header: BiscuitRect,
    pub status: BiscuitRect,
    pub text: BiscuitRect,
    pub footer: BiscuitRect,
}

impl BiscuitReaderShellLayout {
    pub const fn from_screen(screen: BiscuitScreenLayout) -> Self {
        let header = BiscuitRect::new(
            screen.content.x,
            screen.content.y,
            screen.content.w.saturating_sub(120),
            BISCUIT_TYPOGRAPHY.body_line_height,
        );

        let status = BiscuitRect::new(
            screen.content.right().saturating_sub(120),
            screen.content.y,
            120,
            BISCUIT_TYPOGRAPHY.body_line_height,
        );

        let text_y = header.bottom().saturating_add(BISCUIT_SPACING.sm);

        let text_bottom = screen.footer.y.saturating_sub(BISCUIT_CHROME.status_gap);

        let text = BiscuitRect::new(
            screen.content.x,
            text_y,
            screen.content.w,
            text_bottom.saturating_sub(text_y),
        );

        Self {
            header,
            status,
            text,
            footer: screen.footer,
        }
    }
}

pub const fn reader_biscuit_shell_marker() -> &'static str {
    READER_BISCUIT_SHELL_MARKER
}
