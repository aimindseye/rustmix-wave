use embedded_graphics::{pixelcolor::BinaryColor, prelude::*, primitives::PrimitiveStyle};

use crate::rustmix_x4::x4_apps::fonts::bitmap::BitmapFont;
use crate::rustmix_x4::x4_apps::ui::stack_fmt::StackFmt;
use crate::rustmix_x4::x4_apps::ui::{Alignment, Region, wrap_next, wrap_prev};
use crate::rustmix_x4::x4_kernel::board::SCREEN_W;
use crate::rustmix_x4::x4_kernel::board::action::Action;
use crate::rustmix_x4::x4_kernel::drivers::strip::StripBuffer;
pub use crate::rustmix_x4::x4_kernel::kernel::app::{
    MAX_APP_ACTIONS, QuickAction, QuickActionKind,
};

use super::bitmap_label::{BitmapLabel, BitmapTextWeight};
use super::selectable_row::draw_selection_if_visible;

const OVERLAY_W: u16 = 400;
const OVERLAY_X: u16 = (SCREEN_W - OVERLAY_W) / 2;
const OVERLAY_BOTTOM: u16 = 760;
const ITEM_H: u16 = 40;
const ITEM_GAP: u16 = 4;
const ITEM_STRIDE: u16 = ITEM_H + ITEM_GAP;
const PAD_TOP: u16 = 10;
const PAD_BOTTOM: u16 = 8;
const LABEL_X: u16 = OVERLAY_X + 16;
const LABEL_W: u16 = 150;
const VALUE_X: u16 = LABEL_X + LABEL_W + 8;
const VALUE_W: u16 = OVERLAY_W - 16 - LABEL_W - 8 - 16;
const HELP_H: u16 = 20;

const NUM_CORE: usize = 2; // Refresh + Go Home
const MAX_ITEMS: usize = MAX_APP_ACTIONS + NUM_CORE;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuickMenuResult {
    Consumed,
    Close,
    RefreshScreen,
    GoHome,
    AppTrigger(u8),
    AppCycleChanged(u8),
}

#[derive(Clone, Copy)]
enum MenuItemKind {
    AppCycle {
        id: u8,
        value: u8,
        options: &'static [&'static str],
    },
    AppTrigger {
        id: u8,
        display: &'static str,
    },
    CoreRefresh,
    CoreHome,
}

#[derive(Clone, Copy)]
struct MenuItem {
    label: &'static str,
    kind: MenuItemKind,
}

impl MenuItem {
    const EMPTY: Self = Self {
        label: "",
        kind: MenuItemKind::CoreRefresh,
    };
}

pub struct QuickMenu {
    pub open: bool,
    items: [MenuItem; MAX_ITEMS],
    count: usize,
    app_count: usize,
    selected: usize,
    pub dirty: bool,
    overlay_region: Region,
    font: Option<&'static BitmapFont>,
}

impl Default for QuickMenu {
    fn default() -> Self {
        Self::new()
    }
}

impl QuickMenu {
    pub const fn new() -> Self {
        Self {
            open: false,
            items: [MenuItem::EMPTY; MAX_ITEMS],
            count: 0,
            app_count: 0,
            selected: 0,
            dirty: false,
            overlay_region: Region::new(0, 0, 0, 0),
            font: None,
        }
    }

    pub fn set_chrome_font(&mut self, font: &'static BitmapFont) {
        self.font = Some(font);
    }

    pub fn show(&mut self, app_actions: &[QuickAction]) {
        let n_app = app_actions.len().min(MAX_APP_ACTIONS);
        self.app_count = n_app;

        for (i, a) in app_actions.iter().enumerate().take(n_app) {
            self.items[i] = MenuItem {
                label: a.label,
                kind: match a.kind {
                    QuickActionKind::Cycle { value, options } => MenuItemKind::AppCycle {
                        id: a.id,
                        value,
                        options,
                    },
                    QuickActionKind::Trigger { display } => {
                        MenuItemKind::AppTrigger { id: a.id, display }
                    }
                },
            };
        }

        self.items[n_app] = MenuItem {
            label: "Refresh",
            kind: MenuItemKind::CoreRefresh,
        };
        self.items[n_app + 1] = MenuItem {
            label: "Go Home",
            kind: MenuItemKind::CoreHome,
        };

        self.count = n_app + NUM_CORE;
        self.selected = 0;
        self.open = true;
        self.dirty = true;
        self.overlay_region = Self::compute_region(self.count);
    }

    pub fn hide(&mut self) {
        self.open = false;
        self.dirty = true;
    }

    pub fn region(&self) -> Region {
        self.overlay_region
    }

    pub fn app_cycle_value(&self, id: u8) -> Option<u8> {
        for i in 0..self.app_count {
            if let MenuItemKind::AppCycle {
                id: item_id, value, ..
            } = self.items[i].kind
                && item_id == id
            {
                return Some(value);
            }
        }
        None
    }

    pub fn on_action(&mut self, action: Action) -> QuickMenuResult {
        match action {
            Action::Menu | Action::Back => {
                self.hide();
                QuickMenuResult::Close
            }

            Action::Next => {
                let new = wrap_next(self.selected, self.count);
                if new != self.selected {
                    self.selected = new;
                    self.dirty = true;
                }
                QuickMenuResult::Consumed
            }

            Action::Prev => {
                let new = wrap_prev(self.selected, self.count);
                if new != self.selected {
                    self.selected = new;
                    self.dirty = true;
                }
                QuickMenuResult::Consumed
            }

            Action::NextJump => self.adjust_selected(1),

            Action::PrevJump => self.adjust_selected(-1),

            Action::Select => self.activate_selected(),
        }
    }

    fn adjust_selected(&mut self, delta: i8) -> QuickMenuResult {
        let item = &mut self.items[self.selected];
        if let MenuItemKind::AppCycle {
            id,
            ref mut value,
            options,
        } = item.kind
        {
            let max = options.len().saturating_sub(1) as u8;
            if delta > 0 && *value < max {
                *value += 1;
                self.dirty = true;
                return QuickMenuResult::AppCycleChanged(id);
            } else if delta < 0 && *value > 0 {
                *value -= 1;
                self.dirty = true;
                return QuickMenuResult::AppCycleChanged(id);
            }
        }
        QuickMenuResult::Consumed
    }

    fn activate_selected(&mut self) -> QuickMenuResult {
        match &mut self.items[self.selected].kind {
            MenuItemKind::AppCycle { id, value, options } => {
                let len = options.len() as u8;
                if len > 0 {
                    *value = (*value + 1) % len;
                    self.dirty = true;
                    QuickMenuResult::AppCycleChanged(*id)
                } else {
                    QuickMenuResult::Consumed
                }
            }
            MenuItemKind::AppTrigger { id, .. } => {
                let id = *id;
                self.hide();
                QuickMenuResult::AppTrigger(id)
            }
            MenuItemKind::CoreRefresh => {
                self.hide();
                QuickMenuResult::RefreshScreen
            }
            MenuItemKind::CoreHome => {
                self.hide();
                QuickMenuResult::GoHome
            }
        }
    }

    fn compute_region(total_items: usize) -> Region {
        let content_h = PAD_TOP + (ITEM_STRIDE * total_items as u16) + HELP_H + PAD_BOTTOM;
        let y = OVERLAY_BOTTOM - content_h;
        Region::new(OVERLAY_X, y, OVERLAY_W, content_h)
    }

    fn item_y(&self, i: usize) -> u16 {
        self.overlay_region.y + PAD_TOP + i as u16 * ITEM_STRIDE
    }

    fn item_label_region(&self, i: usize) -> Region {
        Region::new(LABEL_X, self.item_y(i), LABEL_W, ITEM_H)
    }

    fn item_value_region(&self, i: usize) -> Region {
        Region::new(VALUE_X, self.item_y(i), VALUE_W, ITEM_H)
    }

    fn help_region(&self) -> Region {
        let last = self.count.saturating_sub(1);
        let below_last = self.item_y(last) + ITEM_STRIDE + 2;
        Region::new(OVERLAY_X + 12, below_last, OVERLAY_W - 24, HELP_H)
    }

    fn format_value(&self, i: usize, buf: &mut StackFmt<20>) {
        buf.clear();
        match &self.items[i].kind {
            MenuItemKind::AppCycle { value, options, .. } => {
                let idx = *value as usize;
                let text = if idx < options.len() {
                    options[idx]
                } else {
                    "?"
                };
                let _ = core::fmt::Write::write_str(buf, text);
            }
            MenuItemKind::AppTrigger { display, .. } => {
                let _ = core::fmt::Write::write_str(buf, display);
            }
            MenuItemKind::CoreRefresh => {
                let _ = core::fmt::Write::write_str(buf, "Clear ghost");
            }
            MenuItemKind::CoreHome => {
                let _ = core::fmt::Write::write_str(buf, ">>>");
            }
        }
    }

    pub fn draw(&self, strip: &mut StripBuffer) {
        if !self.open {
            return;
        }

        let font = self
            .font
            .unwrap_or_else(crate::rustmix_x4::x4_apps::fonts::ui_list_font_fixed);

        let outer = self.overlay_region;
        if outer.intersects(strip.logical_window()) {
            outer
                .to_rect()
                .into_styled(PrimitiveStyle::with_fill(BinaryColor::Off))
                .draw(strip)
                .unwrap();
        }

        let mut val_buf = StackFmt::<20>::new();

        for i in 0..self.count {
            let selected = i == self.selected;
            let row_region = Region::new(OVERLAY_X, self.item_y(i), OVERLAY_W, ITEM_H);
            let _fg = draw_selection_if_visible(strip, row_region, selected);

            let label_region = self.item_label_region(i);
            let value_region = self.item_value_region(i);

            if label_region.intersects(strip.logical_window()) {
                BitmapLabel::new(label_region, self.items[i].label, font)
                    .alignment(Alignment::CenterLeft)
                    .inverted(selected)
                    .weight(BitmapTextWeight::SemiBold)
                    .draw(strip)
                    .unwrap();
            }

            if value_region.intersects(strip.logical_window()) {
                self.format_value(i, &mut val_buf);
                BitmapLabel::new(value_region, val_buf.as_str(), font)
                    .alignment(Alignment::Center)
                    .inverted(selected)
                    .weight(BitmapTextWeight::SemiBold)
                    .draw(strip)
                    .unwrap();
            }
        }

        let help = match &self.items[self.selected].kind {
            MenuItemKind::AppCycle { .. } => {
                "Up/Down: move  Ch-/Ch+: adjust  OK: toggle  Back: close"
            }
            _ => "Up/Down: move  OK: activate  Back: close",
        };

        let help_region = self.help_region();
        if help_region.intersects(strip.logical_window()) {
            BitmapLabel::new(help_region, help, font)
                .alignment(Alignment::Center)
                .weight(BitmapTextWeight::SemiBold)
                .draw(strip)
                .unwrap();
        }
    }
}
