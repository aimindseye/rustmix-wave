//! Biscuit-inspired Home screen layout descriptors.
//!
//! provides a stable descriptor for the accepted Home polish patch.

#![allow(dead_code)]

use super::biscuit_layout::{BiscuitRect, BiscuitScreenLayout};
use super::biscuit_tokens::{BISCUIT_CHROME, BISCUIT_SPACING, BISCUIT_TYPOGRAPHY};

pub const HOME_BISCUIT_LAYOUT_MARKER: &str = "x4-home-biscuit-layout-patch-ok";

pub const CHANGES_HOME_RENDERING: bool = true;
pub const CHANGES_FILES_RENDERING: bool = false;
pub const CHANGES_READER_RENDERING: bool = false;
pub const CHANGES_TITLE_WORKFLOW: bool = false;
pub const CHANGES_FOOTER_LABELS: bool = false;
pub const CHANGES_INPUT_MAPPING: bool = false;
pub const TOUCHES_WRITE_LANE: bool = false;
pub const TOUCHES_DISPLAY_GEOMETRY: bool = false;
pub const TOUCHES_READER_PAGINATION: bool = false;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BiscuitHomeLayout {
    pub title: BiscuitRect,
    pub recent: BiscuitRect,
    pub menu: BiscuitRect,
    pub footer: BiscuitRect,
}

impl BiscuitHomeLayout {
    pub const fn from_screen(screen: BiscuitScreenLayout) -> Self {
        let content = screen.content;
        let title = BiscuitRect::new(
            content.x,
            content.y,
            content.w,
            BISCUIT_TYPOGRAPHY.title_line_height,
        );

        let recent_y = title.bottom().saturating_add(BISCUIT_SPACING.md);

        let recent = BiscuitRect::new(
            content.x,
            recent_y,
            content.w,
            BISCUIT_TYPOGRAPHY.body_line_height.saturating_mul(2),
        );

        let menu_y = recent.bottom().saturating_add(BISCUIT_SPACING.lg);

        let menu_h = screen
            .footer
            .y
            .saturating_sub(BISCUIT_CHROME.status_gap)
            .saturating_sub(menu_y);

        let menu = BiscuitRect::new(content.x, menu_y, content.w, menu_h);

        Self {
            title,
            recent,
            menu,
            footer: screen.footer,
        }
    }
}

pub const fn home_biscuit_layout_marker() -> &'static str {
    HOME_BISCUIT_LAYOUT_MARKER
}
