//! Biscuit-inspired Home app placeholder descriptors.
//!
//! The current implementation adds visual placeholders only. It does not implement app launching,
//! settings, sync, new input mapping, or any reader/files behavior changes.

#![allow(dead_code)]

use super::biscuit_layout::{BiscuitRect, BiscuitScreenLayout};
use super::biscuit_tokens::{BISCUIT_SPACING, BISCUIT_TYPOGRAPHY};

pub const HOME_APP_PLACEHOLDERS_MARKER: &str = "x4-home-app-placeholders-ok";

pub const HOME_APP_PLACEHOLDERS_VISUAL_ONLY: bool = true;
pub const HOME_PLACEHOLDER_APPS: [&str; 3] = ["Apps", "Sync", "Settings"];

pub const CHANGES_HOME_RENDERING: bool = true;
pub const CHANGES_APP_ROUTING: bool = false;
pub const CHANGES_FILES_RENDERING: bool = false;
pub const CHANGES_READER_RENDERING: bool = false;
pub const CHANGES_TITLE_WORKFLOW: bool = false;
pub const CHANGES_FOOTER_LABELS: bool = false;
pub const CHANGES_INPUT_MAPPING: bool = false;
pub const TOUCHES_WRITE_LANE: bool = false;
pub const TOUCHES_DISPLAY_GEOMETRY: bool = false;
pub const TOUCHES_READER_PAGINATION: bool = false;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BiscuitHomeAppShelfLayout {
    pub heading: BiscuitRect,
    pub shelf: BiscuitRect,
    pub visual_only: bool,
}

impl BiscuitHomeAppShelfLayout {
    pub const fn from_screen(screen: BiscuitScreenLayout) -> Self {
        let heading_y = screen
            .content
            .y
            .saturating_add(BISCUIT_TYPOGRAPHY.title_line_height)
            .saturating_add(BISCUIT_SPACING.lg)
            .saturating_add(BISCUIT_TYPOGRAPHY.body_line_height.saturating_mul(2));

        let heading = BiscuitRect::new(
            screen.content.x,
            heading_y,
            screen.content.w,
            BISCUIT_TYPOGRAPHY.body_line_height,
        );

        let shelf = BiscuitRect::new(
            screen.content.x,
            heading.bottom().saturating_add(BISCUIT_SPACING.sm),
            screen.content.w,
            BISCUIT_TYPOGRAPHY.body_line_height,
        );

        Self {
            heading,
            shelf,
            visual_only: HOME_APP_PLACEHOLDERS_VISUAL_ONLY,
        }
    }
}

pub const fn home_apps_marker() -> &'static str {
    HOME_APP_PLACEHOLDERS_MARKER
}

pub const HOME_APP_CONTAINER_COUNT: usize = 2;
pub const HOME_APP_CONTAINER_SUBTITLE: &str = "2 apps";
pub const fn home_app_container_count() -> usize {
    HOME_APP_CONTAINER_COUNT
}
pub const fn home_app_container_subtitle() -> &'static str {
    HOME_APP_CONTAINER_SUBTITLE
}
