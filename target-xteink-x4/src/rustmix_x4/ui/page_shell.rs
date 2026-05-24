//! Rustmix internal page shell descriptors for tabbed, large-row X4 screens.
//!
//! The shell is a pure layout/contract layer. It is intended for Settings,
//! Files, Library, Fonts, Network, Tools, and other internal pages that need
//! CrossInk-style tabs, Inter UI typography, larger rows, section headers,
//! right-aligned values, value pills, scrollbars, and soft-key hints.
//!
//! This module does not draw pixels, change the Biscuit-inspired Home
//! dashboard, change reader pagination, change input mapping, change storage,
//! or touch the display refresh scheduler.

#![allow(dead_code)]

use super::biscuit_layout::{BiscuitRect, BiscuitScreenLayout, DEFAULT_SCREEN_LAYOUT};
use super::biscuit_tokens::{BISCUIT_SPACING, BISCUIT_TYPOGRAPHY};

pub const UI_SHELL_FOUNDATION_MARKER: &str = "ui-shell-foundation-rustmix-ok";
pub const SETTINGS_TABBED_UI_MIGRATION_MARKER: &str = "settings-tabbed-ui-migration-rustmix-ok";
pub const UI_SHELL_CONTRACT_VERSION: u8 = 4;

pub const CHANGES_HOME_DASHBOARD_RENDERING: bool = false;
pub const CHANGES_READER_PAGE_RENDERING: bool = false;
pub const CHANGES_READER_PAGINATION: bool = false;
pub const CHANGES_INPUT_MAPPING: bool = false;
pub const CHANGES_SETTINGS_PERSISTENCE: bool = false;
pub const CHANGES_STORAGE_BEHAVIOR: bool = false;
pub const CHANGES_WIFI_BEHAVIOR: bool = false;
pub const TOUCHES_VENDOR_PULP_OS: bool = false;
pub const TOUCHES_DISPLAY_REFRESH_SCHEDULER: bool = false;
pub const TOUCHES_SSD1677_DRIVER: bool = false;

pub const DEFAULT_SETTINGS_TABS: [&str; 4] = ["Display", "Reader", "Controls", "System"];
pub const DEFAULT_READER_TABS: [&str; 4] = ["Recent", "Books", "Files", "Bookmarks"];
pub const DEFAULT_NETWORK_TABS: [&str; 4] = ["Wi-Fi", "Transfer", "Time", "Status"];
pub const DEFAULT_FOOTER_LABELS: [&str; 4] = ["Back", "Select", "Up", "Down"];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiShellPageKind {
    Settings,
    ReaderLibrary,
    Files,
    Fonts,
    Network,
    Tools,
    System,
    Custom,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiShellRowKind {
    Setting,
    Navigation,
    Section,
    Status,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiShellRowState {
    Normal,
    Selected,
    Disabled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiShellValueStyle {
    Plain,
    SelectedPill,
    Muted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiShellRefreshScope {
    FullPageOnOpen,
    ContentOnTabChange,
    RowsOnNavigation,
    SelectedRowOnToggle,
    FooterOnly,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UiShellTokens {
    pub outer_margin: u16,
    pub hairline: u16,
    pub header_height: u16,
    pub tab_bar_height: u16,
    pub footer_height: u16,
    pub row_height: u16,
    pub section_height: u16,
    pub row_gap: u16,
    pub content_top_gap: u16,
    pub value_column_width: u16,
    pub value_pill_min_width: u16,
    pub scrollbar_width: u16,
}

pub const UI_SHELL_TOKENS: UiShellTokens = UiShellTokens {
    outer_margin: 0,
    hairline: 1,
    header_height: 88,
    tab_bar_height: 34,
    footer_height: 48,
    row_height: 42,
    section_height: 30,
    row_gap: 0,
    content_top_gap: 16,
    value_column_width: 142,
    value_pill_min_width: 76,
    scrollbar_width: 3,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UiShellLayout {
    pub screen: BiscuitRect,
    pub header: BiscuitRect,
    pub title: BiscuitRect,
    pub status: BiscuitRect,
    pub tab_bar: BiscuitRect,
    pub content: BiscuitRect,
    pub list: BiscuitRect,
    pub footer: BiscuitRect,
    pub scrollbar: BiscuitRect,
    pub row_height: u16,
    pub section_height: u16,
    pub visible_rows: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UiShellTabMetrics {
    pub tab_count: u8,
    pub selected_index: u8,
    pub tab_width: u16,
    pub selected: BiscuitRect,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UiShellRowLayout {
    pub row: BiscuitRect,
    pub label: BiscuitRect,
    pub value: BiscuitRect,
    pub value_pill: BiscuitRect,
    pub kind: UiShellRowKind,
    pub state: UiShellRowState,
    pub value_style: UiShellValueStyle,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UiShellFooterMetrics {
    pub key_count: u8,
    pub key_width: u16,
    pub key_height: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UiShellPageContract {
    pub kind: UiShellPageKind,
    pub tab_count: u8,
    pub row_count_hint: u16,
    pub uses_section_headers: bool,
    pub uses_right_aligned_values: bool,
    pub uses_value_pills: bool,
    pub uses_scrollbar: bool,
    pub preserve_home_dashboard: bool,
    pub preserve_reader_pagination: bool,
}

impl UiShellPageContract {
    pub const fn settings() -> Self {
        Self {
            kind: UiShellPageKind::Settings,
            tab_count: 4,
            row_count_hint: 8,
            uses_section_headers: false,
            uses_right_aligned_values: true,
            uses_value_pills: true,
            uses_scrollbar: true,
            preserve_home_dashboard: true,
            preserve_reader_pagination: true,
        }
    }

    pub const fn reader_library() -> Self {
        Self {
            kind: UiShellPageKind::ReaderLibrary,
            tab_count: 4,
            row_count_hint: 10,
            uses_section_headers: false,
            uses_right_aligned_values: false,
            uses_value_pills: false,
            uses_scrollbar: true,
            preserve_home_dashboard: true,
            preserve_reader_pagination: true,
        }
    }

    pub const fn network() -> Self {
        Self {
            kind: UiShellPageKind::Network,
            tab_count: 4,
            row_count_hint: 8,
            uses_section_headers: true,
            uses_right_aligned_values: true,
            uses_value_pills: true,
            uses_scrollbar: true,
            preserve_home_dashboard: true,
            preserve_reader_pagination: true,
        }
    }
}

impl UiShellLayout {
    pub const fn from_screen(screen: BiscuitScreenLayout) -> Self {
        let outer = screen.screen.inset(UI_SHELL_TOKENS.outer_margin);
        let header = BiscuitRect::new(outer.x, outer.y, outer.w, UI_SHELL_TOKENS.header_height);
        let status_w = UI_SHELL_TOKENS.value_column_width;
        let title = BiscuitRect::new(
            header.x,
            header.y,
            header.w.saturating_sub(status_w),
            BISCUIT_TYPOGRAPHY.title_line_height,
        );
        let status = BiscuitRect::new(
            header.right().saturating_sub(status_w),
            header.y,
            status_w,
            BISCUIT_TYPOGRAPHY.body_line_height,
        );
        let tab_y = header.bottom();
        let tab_bar = BiscuitRect::new(outer.x, tab_y, outer.w, UI_SHELL_TOKENS.tab_bar_height);
        let footer = BiscuitRect::new(
            outer.x,
            outer.bottom().saturating_sub(UI_SHELL_TOKENS.footer_height),
            outer.w,
            UI_SHELL_TOKENS.footer_height,
        );
        let content_y = tab_bar
            .bottom()
            .saturating_add(UI_SHELL_TOKENS.content_top_gap);
        let content_bottom = footer.y.saturating_sub(BISCUIT_SPACING.md);
        let content = BiscuitRect::new(
            outer.x,
            content_y,
            outer.w,
            content_bottom.saturating_sub(content_y),
        );
        let scrollbar = BiscuitRect::new(
            content
                .right()
                .saturating_sub(UI_SHELL_TOKENS.scrollbar_width),
            content.y,
            UI_SHELL_TOKENS.scrollbar_width,
            content.h,
        );
        let list = BiscuitRect::new(
            content.x,
            content.y,
            content
                .w
                .saturating_sub(UI_SHELL_TOKENS.scrollbar_width)
                .saturating_sub(BISCUIT_SPACING.sm),
            content.h,
        );
        let stride = UI_SHELL_TOKENS
            .row_height
            .saturating_add(UI_SHELL_TOKENS.row_gap);
        let visible_rows = match list.h.checked_div(stride) {
            Some(rows) => rows,
            None => 0,
        };

        Self {
            screen: screen.screen,
            header,
            title,
            status,
            tab_bar,
            content,
            list,
            footer,
            scrollbar,
            row_height: UI_SHELL_TOKENS.row_height,
            section_height: UI_SHELL_TOKENS.section_height,
            visible_rows,
        }
    }

    pub const fn tab_metrics(self, tab_count: u8, selected_index: u8) -> UiShellTabMetrics {
        let safe_count = if tab_count == 0 { 1 } else { tab_count };
        let selected = if selected_index >= safe_count {
            safe_count.saturating_sub(1)
        } else {
            selected_index
        };
        let tab_width = match self.tab_bar.w.checked_div(safe_count as u16) {
            Some(width) => width,
            None => self.tab_bar.w,
        };
        let selected_rect = BiscuitRect::new(
            self.tab_bar
                .x
                .saturating_add(tab_width.saturating_mul(selected as u16)),
            self.tab_bar.y,
            tab_width,
            self.tab_bar.h,
        );

        UiShellTabMetrics {
            tab_count: safe_count,
            selected_index: selected,
            tab_width,
            selected: selected_rect,
        }
    }

    pub const fn row_layout(
        self,
        visible_index: u16,
        kind: UiShellRowKind,
        state: UiShellRowState,
        value_style: UiShellValueStyle,
    ) -> UiShellRowLayout {
        let row_y = self.list.y.saturating_add(
            visible_index.saturating_mul(
                UI_SHELL_TOKENS
                    .row_height
                    .saturating_add(UI_SHELL_TOKENS.row_gap),
            ),
        );
        let row_h = match kind {
            UiShellRowKind::Section => UI_SHELL_TOKENS.section_height,
            _ => UI_SHELL_TOKENS.row_height,
        };
        let row = BiscuitRect::new(self.list.x, row_y, self.list.w, row_h);
        let value = BiscuitRect::new(
            row.right()
                .saturating_sub(UI_SHELL_TOKENS.value_column_width),
            row.y,
            UI_SHELL_TOKENS.value_column_width,
            row.h,
        );
        let label = BiscuitRect::new(
            row.x.saturating_add(BISCUIT_SPACING.sm),
            row.y,
            row.w
                .saturating_sub(UI_SHELL_TOKENS.value_column_width)
                .saturating_sub(BISCUIT_SPACING.md),
            row.h,
        );
        let pill_w = if value.w < UI_SHELL_TOKENS.value_pill_min_width {
            value.w
        } else {
            UI_SHELL_TOKENS.value_pill_min_width
        };
        let value_pill = BiscuitRect::new(
            value.right().saturating_sub(pill_w),
            value.y.saturating_add(BISCUIT_SPACING.xs),
            pill_w,
            value.h.saturating_sub(BISCUIT_SPACING.sm),
        );

        UiShellRowLayout {
            row,
            label,
            value,
            value_pill,
            kind,
            state,
            value_style,
        }
    }

    pub const fn footer_metrics(self, key_count: u8) -> UiShellFooterMetrics {
        let safe_count = if key_count == 0 { 1 } else { key_count };
        let key_width = match self.footer.w.checked_div(safe_count as u16) {
            Some(width) => width,
            None => self.footer.w,
        };

        UiShellFooterMetrics {
            key_count: safe_count,
            key_width,
            key_height: self.footer.h,
        }
    }
}

pub const DEFAULT_UI_SHELL_LAYOUT: UiShellLayout =
    UiShellLayout::from_screen(DEFAULT_SCREEN_LAYOUT);
pub const SETTINGS_SHELL_CONTRACT: UiShellPageContract = UiShellPageContract::settings();
pub const READER_LIBRARY_SHELL_CONTRACT: UiShellPageContract =
    UiShellPageContract::reader_library();
pub const NETWORK_SHELL_CONTRACT: UiShellPageContract = UiShellPageContract::network();

pub const fn ui_shell_foundation_marker() -> &'static str {
    UI_SHELL_FOUNDATION_MARKER
}

pub const fn default_ui_shell_layout() -> UiShellLayout {
    DEFAULT_UI_SHELL_LAYOUT
}

pub const fn settings_tabbed_ui_migration_marker() -> &'static str {
    SETTINGS_TABBED_UI_MIGRATION_MARKER
}
