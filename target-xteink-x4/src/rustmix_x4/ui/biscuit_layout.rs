//! Biscuit-inspired layout primitives for the X4 target.
//!
//! The current implementation exposes layout descriptions only. Existing screen implementations
//! are not rewired in this implementation.

#![allow(dead_code)]

use super::biscuit_tokens::{
    BISCUIT_CHROME, BISCUIT_LIST, BISCUIT_SPACING, X4_BISCUIT_LOGICAL_HEIGHT,
    X4_BISCUIT_LOGICAL_WIDTH,
};

pub const BISCUIT_LAYOUT_PRIMITIVES_MARKER: &str = "x4-biscuit-ui-layout-primitives-ok";
pub const BISCUIT_LAYOUT_CLIPPY_REPAIR_MARKER: &str = "x4-biscuit-layout-clippy-checked-div-ok";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BiscuitRect {
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
}

impl BiscuitRect {
    pub const fn new(x: u16, y: u16, w: u16, h: u16) -> Self {
        Self { x, y, w, h }
    }

    pub const fn right(self) -> u16 {
        self.x.saturating_add(self.w)
    }

    pub const fn bottom(self) -> u16 {
        self.y.saturating_add(self.h)
    }

    pub const fn inset(self, by: u16) -> Self {
        let double = by.saturating_mul(2);
        Self {
            x: self.x.saturating_add(by),
            y: self.y.saturating_add(by),
            w: self.w.saturating_sub(double),
            h: self.h.saturating_sub(double),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BiscuitScreenLayout {
    pub screen: BiscuitRect,
    pub header: BiscuitRect,
    pub content: BiscuitRect,
    pub list: BiscuitRect,
    pub footer: BiscuitRect,
}

pub const fn x4_screen_layout() -> BiscuitScreenLayout {
    let screen = BiscuitRect::new(0, 0, X4_BISCUIT_LOGICAL_WIDTH, X4_BISCUIT_LOGICAL_HEIGHT);
    let header = BiscuitRect::new(0, 0, X4_BISCUIT_LOGICAL_WIDTH, BISCUIT_CHROME.header_height);
    let footer = BiscuitRect::new(
        0,
        X4_BISCUIT_LOGICAL_HEIGHT.saturating_sub(BISCUIT_CHROME.footer_height),
        X4_BISCUIT_LOGICAL_WIDTH,
        BISCUIT_CHROME.footer_height,
    );
    let content_y = header.bottom().saturating_add(BISCUIT_CHROME.status_gap);
    let content_bottom = footer.y.saturating_sub(BISCUIT_CHROME.status_gap);
    let content_h = content_bottom.saturating_sub(content_y);
    let content = BiscuitRect::new(
        BISCUIT_SPACING.screen_margin,
        content_y,
        X4_BISCUIT_LOGICAL_WIDTH.saturating_sub(BISCUIT_SPACING.screen_margin.saturating_mul(2)),
        content_h,
    );
    let list = BiscuitRect::new(content.x, content.y, content.w, content.h);

    BiscuitScreenLayout {
        screen,
        header,
        content,
        list,
        footer,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BiscuitListMetrics {
    pub row_height: u16,
    pub row_gap: u16,
    pub visible_rows: u16,
}

pub const fn list_metrics(layout: BiscuitScreenLayout) -> BiscuitListMetrics {
    let stride = BISCUIT_LIST
        .row_height
        .saturating_add(BISCUIT_SPACING.row_gap);
    let visible_rows = match layout.list.h.checked_div(stride) {
        Some(rows) => rows,
        None => 0,
    };

    BiscuitListMetrics {
        row_height: BISCUIT_LIST.row_height,
        row_gap: BISCUIT_SPACING.row_gap,
        visible_rows,
    }
}

pub const DEFAULT_SCREEN_LAYOUT: BiscuitScreenLayout = x4_screen_layout();
pub const DEFAULT_LIST_METRICS: BiscuitListMetrics = list_metrics(DEFAULT_SCREEN_LAYOUT);

pub const fn biscuit_layout_marker() -> &'static str {
    BISCUIT_LAYOUT_PRIMITIVES_MARKER
}
