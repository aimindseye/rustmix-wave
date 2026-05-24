// CrossInk-style footer labels for the X4 physical button row.
//
// Renders the bottom soft-key bar as four equal tab-like boxes using the
// same fixed Inter UI typography path as Home, Settings, and internal pages.
// The labels still flow through ButtonMapper so Settings button swaps and
// per-app label modes remain honored.

use embedded_graphics::{pixelcolor::BinaryColor, prelude::*, primitives::PrimitiveStyle};

use crate::rustmix_x4::ui::page_shell::DEFAULT_SETTINGS_TABS;
use crate::rustmix_x4::x4_apps::apps::widgets::bitmap_label::{BitmapLabel, BitmapTextWeight};
use crate::rustmix_x4::x4_apps::fonts;
use crate::rustmix_x4::x4_apps::fonts::bitmap::BitmapFont;
use crate::rustmix_x4::x4_apps::ui::{Alignment, Region};
use crate::rustmix_x4::x4_kernel::board::SCREEN_H;
use crate::rustmix_x4::x4_kernel::board::action::{Action, ButtonMapper};
use crate::rustmix_x4::x4_kernel::board::button::Button;
use crate::rustmix_x4::x4_kernel::drivers::strip::StripBuffer;

const FOOTER_TAB_COUNT: usize = 4;
const FOOTER_BUTTON_W: u16 = 106;
const TAB_H: u16 = 34;
const BOTTOM_INSET: u16 = 10;
const FOOTER_X4_POSITIONS: [u16; FOOTER_TAB_COUNT] = [25, 130, 245, 350];

pub const BUTTON_BAR_H: u16 = TAB_H + BOTTOM_INSET;

const FOOTER_BUTTONS: [Button; FOOTER_TAB_COUNT] =
    [Button::Back, Button::Confirm, Button::Left, Button::Right];

fn footer_tab_region(index: usize) -> Region {
    let x = FOOTER_X4_POSITIONS
        .get(index)
        .copied()
        .unwrap_or(FOOTER_X4_POSITIONS[0]);
    Region::new(
        x,
        SCREEN_H.saturating_sub(TAB_H).saturating_sub(BOTTOM_INSET),
        FOOTER_BUTTON_W,
        TAB_H,
    )
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LabelMode {
    Default,
    Reader,
    Settings,
    Games,
}

fn default_action_label(mode: LabelMode, action: Action) -> &'static str {
    match mode {
        LabelMode::Default => match action {
            Action::Next => "Next",
            Action::Prev => "Prev",
            Action::NextJump => ">>",
            Action::PrevJump => "<<",
            Action::Select => "OK",
            Action::Back => "Back",
            Action::Menu => "",
        },
        LabelMode::Reader => match action {
            Action::Next => "Next",
            Action::Prev => "Prev",
            Action::NextJump => "Ch+",
            Action::PrevJump => "Ch-",
            Action::Select => "Menu",
            Action::Back => "Back",
            Action::Menu => "",
        },
        LabelMode::Games => match action {
            Action::Next => "Down",
            Action::Prev => "Up",
            Action::NextJump => "Right",
            Action::PrevJump => "Left",
            Action::Select => "OK",
            Action::Back => "Back",
            Action::Menu => "",
        },
        LabelMode::Settings => match action {
            Action::Next => "Down",
            Action::Prev => "Up",
            Action::NextJump => "Tab+",
            Action::PrevJump => "Tab-",
            Action::Select => "Toggle",
            Action::Back => "Back",
            Action::Menu => "",
        },
    }
}

pub struct ButtonFeedback {
    swap: bool,
    mode: LabelMode,
    settings_tab: u8,
    settings_focus_tabs: bool,
    font: Option<&'static BitmapFont>,
}

impl Default for ButtonFeedback {
    fn default() -> Self {
        Self::new()
    }
}

impl ButtonFeedback {
    pub const fn new() -> Self {
        Self {
            swap: false,
            mode: LabelMode::Default,
            settings_tab: 0,
            settings_focus_tabs: false,
            font: None,
        }
    }

    pub fn set_chrome_font(&mut self, font: &'static BitmapFont) {
        self.font = Some(font);
    }

    pub fn set_label_mode(&mut self, mode: LabelMode) -> bool {
        if self.mode != mode {
            self.mode = mode;
            true
        } else {
            false
        }
    }

    pub fn set_swap(&mut self, swap: bool) -> bool {
        if self.swap != swap {
            self.swap = swap;
            true
        } else {
            false
        }
    }

    pub fn set_settings_state(&mut self, selected_tab: u8, focus_tabs: bool) -> bool {
        let selected_tab = selected_tab.min((DEFAULT_SETTINGS_TABS.len() - 1) as u8);
        if self.settings_tab != selected_tab || self.settings_focus_tabs != focus_tabs {
            self.settings_tab = selected_tab;
            self.settings_focus_tabs = focus_tabs;
            true
        } else {
            false
        }
    }

    fn label_for_action_with_quick_menu(
        &self,
        action: Action,
        reader_quick_menu_open: bool,
    ) -> &'static str {
        if self.mode == LabelMode::Settings
            && matches!(action, Action::Select)
            && self.settings_focus_tabs
        {
            let next = (self.settings_tab as usize + 1) % DEFAULT_SETTINGS_TABS.len();
            return DEFAULT_SETTINGS_TABS[next];
        }

        if self.mode == LabelMode::Reader
            && reader_quick_menu_open
            && matches!(action, Action::Select)
        {
            return "OK";
        }

        default_action_label(self.mode, action)
    }

    pub fn draw(&self, strip: &mut StripBuffer) {
        self.draw_with_reader_quick_menu(strip, false)
    }

    pub fn draw_with_reader_quick_menu(
        &self,
        strip: &mut StripBuffer,
        reader_quick_menu_open: bool,
    ) {
        let font = fonts::ui_body_font_fixed();
        let mapper = if self.swap {
            let mut m = ButtonMapper::new();
            m.set_swap(true);
            m
        } else {
            ButtonMapper::new()
        };

        for (idx, button) in FOOTER_BUTTONS.iter().enumerate() {
            let r = footer_tab_region(idx);

            if !r.intersects(strip.logical_window()) {
                continue;
            }

            r.to_rect()
                .into_styled(PrimitiveStyle::with_fill(BinaryColor::Off))
                .draw(strip)
                .unwrap();
            r.to_rect()
                .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
                .draw(strip)
                .unwrap();

            let action = mapper.map_button(*button);
            let label = self.label_for_action_with_quick_menu(action, reader_quick_menu_open);
            if label.is_empty() {
                continue;
            }

            BitmapLabel::new(r, label, font)
                .alignment(Alignment::Center)
                .weight(BitmapTextWeight::SemiBold)
                .draw(strip)
                .unwrap();
        }
    }
}
