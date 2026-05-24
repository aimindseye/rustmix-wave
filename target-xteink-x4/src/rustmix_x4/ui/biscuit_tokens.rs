//! Biscuit-inspired UI tokens for the X4 target.
//!
//! until later UI work adopts them in Home/Files/Reader rendering.

#![allow(dead_code)]

pub const BISCUIT_UI_ARCHITECTURE_MAP_MARKER: &str = "x4-biscuit-ui-architecture-map-ok";
pub const BISCUIT_UI_THEME_LAYOUT_TOKENS_MARKER: &str = "x4-biscuit-ui-theme-layout-tokens-ok";

pub const X4_BISCUIT_LOGICAL_WIDTH: u16 = 480;
pub const X4_BISCUIT_LOGICAL_HEIGHT: u16 = 800;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BiscuitMonoColor {
    Ink,
    Paper,
}

impl BiscuitMonoColor {
    pub const fn bit(self) -> bool {
        match self {
            Self::Ink => true,
            Self::Paper => false,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Ink => "ink",
            Self::Paper => "paper",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BiscuitSpacingTokens {
    pub hairline: u16,
    pub xs: u16,
    pub sm: u16,
    pub md: u16,
    pub lg: u16,
    pub xl: u16,
    pub screen_margin: u16,
    pub row_gap: u16,
}

pub const BISCUIT_SPACING: BiscuitSpacingTokens = BiscuitSpacingTokens {
    hairline: 1,
    xs: 2,
    sm: 4,
    md: 8,
    lg: 12,
    xl: 16,
    screen_margin: 16,
    row_gap: 6,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BiscuitTypographyTokens {
    pub title_line_height: u16,
    pub body_line_height: u16,
    pub small_line_height: u16,
    pub footer_line_height: u16,
}

pub const BISCUIT_TYPOGRAPHY: BiscuitTypographyTokens = BiscuitTypographyTokens {
    title_line_height: 24,
    body_line_height: 18,
    small_line_height: 14,
    footer_line_height: 16,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BiscuitListTokens {
    pub row_height: u16,
    pub compact_row_height: u16,
    pub selected_border: u16,
    pub title_max_lines: u16,
}

pub const BISCUIT_LIST: BiscuitListTokens = BiscuitListTokens {
    row_height: 42,
    compact_row_height: 34,
    selected_border: 2,
    title_max_lines: 2,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BiscuitChromeTokens {
    pub header_height: u16,
    pub footer_height: u16,
    pub status_gap: u16,
}

pub const BISCUIT_CHROME: BiscuitChromeTokens = BiscuitChromeTokens {
    header_height: 56,
    footer_height: 32,
    status_gap: 8,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BiscuitThemeTokens {
    pub foreground: BiscuitMonoColor,
    pub background: BiscuitMonoColor,
    pub spacing: BiscuitSpacingTokens,
    pub typography: BiscuitTypographyTokens,
    pub list: BiscuitListTokens,
    pub chrome: BiscuitChromeTokens,
}

pub const BISCUIT_X4_THEME: BiscuitThemeTokens = BiscuitThemeTokens {
    foreground: BiscuitMonoColor::Ink,
    background: BiscuitMonoColor::Paper,
    spacing: BISCUIT_SPACING,
    typography: BISCUIT_TYPOGRAPHY,
    list: BISCUIT_LIST,
    chrome: BISCUIT_CHROME,
};

pub const CHANGES_HOME_RENDERING: bool = false;
pub const CHANGES_FILES_RENDERING: bool = false;
pub const CHANGES_READER_RENDERING: bool = false;
pub const CHANGES_INPUT_MAPPING: bool = false;
pub const TOUCHES_WRITE_LANE: bool = false;
pub const TOUCHES_DISPLAY_GEOMETRY: bool = false;
pub const TOUCHES_READER_PAGINATION: bool = false;

pub const fn biscuit_theme() -> BiscuitThemeTokens {
    BISCUIT_X4_THEME
}

pub const fn biscuit_theme_marker() -> &'static str {
    BISCUIT_UI_THEME_LAYOUT_TOKENS_MARKER
}
