//! CrossInk-style internal page renderer for Rustmix-owned app screens.
//!
//! This renderer is intentionally separate from the Biscuit-inspired Home
//! dashboard.  It provides the shared visual language for internal pages:
//! header, optional tab strip, large Inter rows, right-aligned values,
//! selected value pills, and X4 soft-key footer alignment.

#![allow(dead_code)]

use embedded_graphics::{pixelcolor::BinaryColor, prelude::*, primitives::PrimitiveStyle};

use crate::rustmix_x4::x4_apps::apps::widgets::bitmap_label::{
    BitmapDynLabel, BitmapLabel, BitmapTextWeight,
};
use crate::rustmix_x4::x4_apps::fonts;
use crate::rustmix_x4::x4_kernel::board::{SCREEN_H, SCREEN_W};
use crate::rustmix_x4::x4_kernel::drivers::strip::StripBuffer;
use crate::rustmix_x4::x4_kernel::ui::{Alignment, Region};

pub const CROSSINK_INTERNAL_UI_ROLLOUT_MARKER: &str = "crossink-internal-ui-rollout-rustmix-ok";
pub const CROSSINK_FILES_LIBRARY_VISUAL_PARITY_MARKER: &str =
    "crossink-files-library-visual-parity-rustmix-ok";
pub const CROSSINK_READER_UNIFIED_TABS_MARKER: &str = "crossink-reader-unified-tabs-rustmix-ok";

pub const HEADER_Y: u16 = 4;
pub const HEADER_H: u16 = 58;
pub const HEADER_TITLE_X: u16 = 24;
pub const HEADER_STATUS_W: u16 = 132;
pub const HEADER_RULE_Y: u16 = HEADER_Y + HEADER_H - 1;

pub const TAB_Y: u16 = HEADER_Y + HEADER_H;
pub const TAB_H: u16 = 34;
pub const TAB_TEXT_PAD_X: u16 = 10;

pub const CONTENT_X: u16 = 24;
pub const CONTENT_W: u16 = SCREEN_W - CONTENT_X * 2;
pub const LIST_TOP_WITH_TABS: u16 = TAB_Y + TAB_H + 14;
pub const LIST_TOP_NO_TABS: u16 = HEADER_Y + HEADER_H + 16;
pub const ROW_H: u16 = 42;
pub const ROW_STRIDE: u16 = 42;
pub const VALUE_W: u16 = 136;
pub const VALUE_PAD: u16 = 8;
pub const SELECTED_VALUE_PILL_W: u16 = 116;
pub const FOOTER_RESERVED_H: u16 = 44;
pub const CONTENT_BOTTOM: u16 = SCREEN_H - FOOTER_RESERVED_H - 8;

// Reader/library lists are allowed to use more of the vertical page than
// settings/configuration screens. This keeps the CrossInk-style header and
// tab strip, but lets Files and Bookmarks fill the area down to the footer.
// Medium reader-list text uses a taller row stride for legibility.
pub const READER_LIST_TOP_WITH_TABS: u16 = TAB_Y + TAB_H + 6;
pub const READER_ROW_H: u16 = 27;
pub const READER_ROW_STRIDE: u16 = 27;
pub const READER_CONTENT_BOTTOM: u16 = SCREEN_H - 28;

pub const READER_TABS: [&str; 4] = ["Recent", "Books", "Files", "Bookmarks"];
pub const NETWORK_TABS: [&str; 4] = ["Wi-Fi", "Transfer", "Time", "Status"];
pub const TOOLS_TABS: [&str; 3] = ["Tools", "Lua", "Status"];
pub const GAMES_TABS: [&str; 3] = ["Games", "Lua", "Status"];
pub const FONT_TABS: [&str; 3] = ["Installed", "SD", "Manage"];

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CrossInkRowTone {
    Normal,
    Selected,
    Section,
    Disabled,
}

pub fn header_title_region() -> Region {
    Region::new(
        HEADER_TITLE_X,
        HEADER_Y,
        SCREEN_W - HEADER_TITLE_X - HEADER_STATUS_W,
        HEADER_H,
    )
}

pub fn header_status_region() -> Region {
    Region::new(
        SCREEN_W - HEADER_STATUS_W - 20,
        HEADER_Y,
        HEADER_STATUS_W,
        HEADER_H,
    )
}

pub fn tab_region(index: usize, count: usize) -> Region {
    let safe_count = count.max(1) as u16;
    let tab_w = SCREEN_W / safe_count;
    let x = (index as u16).saturating_mul(tab_w);
    let w = if index + 1 == count {
        SCREEN_W.saturating_sub(x)
    } else {
        tab_w
    };
    Region::new(x, TAB_Y, w, TAB_H)
}

pub fn list_top(with_tabs: bool) -> u16 {
    if with_tabs {
        LIST_TOP_WITH_TABS
    } else {
        LIST_TOP_NO_TABS
    }
}

pub fn list_row_region_from(top: u16, index: usize) -> Region {
    Region::new(
        CONTENT_X,
        top + (index as u16).saturating_mul(ROW_STRIDE),
        CONTENT_W,
        ROW_H,
    )
}

pub fn list_row_region(index: usize) -> Region {
    list_row_region_from(LIST_TOP_WITH_TABS, index)
}

pub fn list_region_from(top: u16, rows: usize) -> Region {
    Region::new(
        CONTENT_X,
        top,
        CONTENT_W,
        ROW_STRIDE.saturating_mul(rows as u16),
    )
}

pub fn list_region(rows: usize) -> Region {
    list_region_from(LIST_TOP_WITH_TABS, rows)
}

pub fn visible_rows_from(top: u16) -> usize {
    let available = CONTENT_BOTTOM.saturating_sub(top);
    (available / ROW_STRIDE).max(1) as usize
}

pub fn visible_rows() -> usize {
    visible_rows_from(LIST_TOP_WITH_TABS)
}

pub fn reader_list_top_with_tabs() -> u16 {
    READER_LIST_TOP_WITH_TABS
}

pub fn reader_visible_rows_from(top: u16) -> usize {
    let available = READER_CONTENT_BOTTOM.saturating_sub(top);
    (available / READER_ROW_STRIDE).max(1) as usize
}

pub fn reader_visible_rows() -> usize {
    reader_visible_rows_from(READER_LIST_TOP_WITH_TABS)
}

pub fn reader_list_row_region_from(top: u16, index: usize) -> Region {
    Region::new(
        CONTENT_X,
        top + (index as u16).saturating_mul(READER_ROW_STRIDE),
        CONTENT_W,
        READER_ROW_H,
    )
}

pub fn reader_list_row_region(index: usize) -> Region {
    reader_list_row_region_from(READER_LIST_TOP_WITH_TABS, index)
}

pub fn reader_list_region_from(top: u16, rows: usize) -> Region {
    Region::new(
        CONTENT_X,
        top,
        CONTENT_W,
        READER_ROW_STRIDE.saturating_mul(rows as u16),
    )
}

pub fn reader_list_region(rows: usize) -> Region {
    reader_list_region_from(READER_LIST_TOP_WITH_TABS, rows)
}

pub fn draw_header(strip: &mut StripBuffer, title: &str, status: &str) {
    let header = Region::new(0, HEADER_Y, SCREEN_W, HEADER_H);
    if header.intersects(strip.logical_window()) {
        header
            .to_rect()
            .into_styled(PrimitiveStyle::with_fill(BinaryColor::Off))
            .draw(strip)
            .unwrap();
    }

    BitmapLabel::new(header_title_region(), title, fonts::ui_heading_font_fixed())
        .alignment(Alignment::CenterLeft)
        .weight(BitmapTextWeight::SemiBold)
        .draw(strip)
        .unwrap();

    if !status.is_empty() {
        BitmapLabel::new(header_status_region(), status, fonts::chrome_font())
            .alignment(Alignment::CenterRight)
            .weight(BitmapTextWeight::Medium)
            .draw(strip)
            .unwrap();
    }

    let rule = Region::new(0, HEADER_RULE_Y, SCREEN_W, 1);
    if rule.intersects(strip.logical_window()) {
        rule.to_rect()
            .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
            .draw(strip)
            .unwrap();
    }
}

pub fn draw_header_with_dynamic_status<const N: usize>(
    strip: &mut StripBuffer,
    title: &str,
    status: &BitmapDynLabel<N>,
) {
    draw_header(strip, title, "");
    status.draw(strip).unwrap();
}

pub fn draw_tab_strip(strip: &mut StripBuffer, tabs: &[&str], selected: usize) {
    if tabs.is_empty() {
        return;
    }

    let strip_region = Region::new(0, TAB_Y, SCREEN_W, TAB_H);
    if strip_region.intersects(strip.logical_window()) {
        strip_region
            .to_rect()
            .into_styled(PrimitiveStyle::with_fill(BinaryColor::Off))
            .draw(strip)
            .unwrap();
        Region::new(0, TAB_Y + TAB_H - 1, SCREEN_W, 1)
            .to_rect()
            .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
            .draw(strip)
            .unwrap();
    }

    for (idx, tab) in tabs.iter().enumerate() {
        let r = tab_region(idx, tabs.len());
        let active = idx == selected.min(tabs.len().saturating_sub(1));
        if active {
            let active_r = Region::new(
                r.x.saturating_add(TAB_TEXT_PAD_X / 2),
                r.y + 4,
                r.w.saturating_sub(TAB_TEXT_PAD_X),
                r.h.saturating_sub(8),
            );
            active_r
                .to_rect()
                .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                .draw(strip)
                .unwrap();
        }

        BitmapLabel::new(r, tab, fonts::ui_list_font_fixed())
            .alignment(Alignment::Center)
            .inverted(active)
            .weight(if active {
                BitmapTextWeight::SemiBold
            } else {
                BitmapTextWeight::Medium
            })
            .draw(strip)
            .unwrap();
    }
}

pub fn draw_row(
    strip: &mut StripBuffer,
    row: Region,
    label: &str,
    value: &str,
    tone: CrossInkRowTone,
) {
    let selected = tone == CrossInkRowTone::Selected;
    let section = tone == CrossInkRowTone::Section;
    let label_font = if section {
        fonts::ui_list_section_font_fixed()
    } else {
        fonts::ui_list_font_fixed()
    };
    let value_font = fonts::ui_list_font_fixed();

    if row.intersects(strip.logical_window()) {
        row.to_rect()
            .into_styled(PrimitiveStyle::with_fill(BinaryColor::Off))
            .draw(strip)
            .unwrap();
    }

    let label_region = Region::new(
        row.x,
        row.y,
        row.w.saturating_sub(VALUE_W + VALUE_PAD),
        row.h,
    );
    BitmapLabel::new(label_region, label, label_font)
        .alignment(Alignment::CenterLeft)
        .weight(if section {
            BitmapTextWeight::SemiBold
        } else {
            BitmapTextWeight::Medium
        })
        .draw(strip)
        .unwrap();

    if value.is_empty() {
        return;
    }

    let value_region = Region::new(row.x + row.w.saturating_sub(VALUE_W), row.y, VALUE_W, row.h);

    if selected {
        let pill = Region::new(
            value_region.x + value_region.w.saturating_sub(SELECTED_VALUE_PILL_W),
            value_region.y + 5,
            SELECTED_VALUE_PILL_W,
            value_region.h.saturating_sub(10),
        );
        pill.to_rect()
            .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
            .draw(strip)
            .unwrap();
        BitmapLabel::new(pill, value, value_font)
            .alignment(Alignment::CenterRight)
            .inverted(true)
            .weight(BitmapTextWeight::SemiBold)
            .draw(strip)
            .unwrap();
    } else {
        BitmapLabel::new(value_region, value, value_font)
            .alignment(Alignment::CenterRight)
            .weight(BitmapTextWeight::Medium)
            .draw(strip)
            .unwrap();
    }
}

pub fn clear_row(strip: &mut StripBuffer, row: Region) {
    if row.intersects(strip.logical_window()) {
        row.to_rect()
            .into_styled(PrimitiveStyle::with_fill(BinaryColor::Off))
            .draw(strip)
            .unwrap();
    }
}

pub fn draw_list_item(
    strip: &mut StripBuffer,
    row: Region,
    title: &str,
    detail: &str,
    selected: bool,
) {
    if row.intersects(strip.logical_window()) {
        row.to_rect()
            .into_styled(PrimitiveStyle::with_fill(if selected {
                BinaryColor::On
            } else {
                BinaryColor::Off
            }))
            .draw(strip)
            .unwrap();
    }

    let title_w = if detail.is_empty() {
        row.w
    } else {
        row.w.saturating_sub(VALUE_W + VALUE_PAD)
    };
    let title_region = Region::new(row.x, row.y, title_w, row.h);
    BitmapLabel::new(title_region, title, fonts::ui_list_font_fixed())
        .alignment(Alignment::CenterLeft)
        .inverted(selected)
        .weight(if selected {
            BitmapTextWeight::SemiBold
        } else {
            BitmapTextWeight::Medium
        })
        .draw(strip)
        .unwrap();

    if !detail.is_empty() {
        let detail_region =
            Region::new(row.x + row.w.saturating_sub(VALUE_W), row.y, VALUE_W, row.h);
        BitmapLabel::new(detail_region, detail, fonts::ui_list_font_fixed())
            .alignment(Alignment::CenterRight)
            .inverted(selected)
            .weight(if selected {
                BitmapTextWeight::SemiBold
            } else {
                BitmapTextWeight::Medium
            })
            .draw(strip)
            .unwrap();
    }
}

pub fn draw_reader_compact_item(
    strip: &mut StripBuffer,
    row: Region,
    title: &str,
    detail: &str,
    selected: bool,
) {
    if row.intersects(strip.logical_window()) {
        row.to_rect()
            .into_styled(PrimitiveStyle::with_fill(if selected {
                BinaryColor::On
            } else {
                BinaryColor::Off
            }))
            .draw(strip)
            .unwrap();
    }

    let detail_w = if detail.is_empty() { 0 } else { 88 };
    let title_w = row.w.saturating_sub(detail_w + VALUE_PAD);
    let title_region = Region::new(row.x, row.y, title_w, row.h);
    BitmapLabel::new(title_region, title, fonts::ui_reader_list_font_fixed())
        .alignment(Alignment::CenterLeft)
        .inverted(selected)
        .weight(if selected {
            BitmapTextWeight::SemiBold
        } else {
            BitmapTextWeight::Medium
        })
        .draw(strip)
        .unwrap();

    if !detail.is_empty() {
        let detail_region = Region::new(
            row.x + row.w.saturating_sub(detail_w),
            row.y,
            detail_w,
            row.h,
        );
        BitmapLabel::new(detail_region, detail, fonts::ui_reader_list_font_fixed())
            .alignment(Alignment::CenterRight)
            .inverted(selected)
            .weight(if selected {
                BitmapTextWeight::SemiBold
            } else {
                BitmapTextWeight::Medium
            })
            .draw(strip)
            .unwrap();
    }
}

pub fn draw_reader_tabs(strip: &mut StripBuffer, selected: usize) {
    draw_tab_strip(strip, &READER_TABS, selected);
}

pub fn draw_reader_tabs_focused(strip: &mut StripBuffer, selected: usize, focus_tabs: bool) {
    if focus_tabs {
        draw_tab_strip(strip, &READER_TABS, selected);
        return;
    }

    // When list rows own focus, keep the selected tab visible but do not
    // render it as the active inverted control. This matches Settings focus
    // behavior and makes Up/Down entry into the list obvious.
    if READER_TABS.is_empty() {
        return;
    }

    let strip_region = Region::new(0, TAB_Y, SCREEN_W, TAB_H);
    if strip_region.intersects(strip.logical_window()) {
        strip_region
            .to_rect()
            .into_styled(PrimitiveStyle::with_fill(BinaryColor::Off))
            .draw(strip)
            .unwrap();
        Region::new(0, TAB_Y + TAB_H - 1, SCREEN_W, 1)
            .to_rect()
            .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
            .draw(strip)
            .unwrap();
    }

    for (idx, tab) in READER_TABS.iter().enumerate() {
        let r = tab_region(idx, READER_TABS.len());
        let active = idx == selected.min(READER_TABS.len().saturating_sub(1));
        BitmapLabel::new(r, tab, fonts::ui_list_font_fixed())
            .alignment(Alignment::Center)
            .weight(if active {
                BitmapTextWeight::SemiBold
            } else {
                BitmapTextWeight::Medium
            })
            .draw(strip)
            .unwrap();
    }
}

pub fn draw_navigation_row(
    strip: &mut StripBuffer,
    visible_index: usize,
    title: &str,
    detail_or_value: &str,
    selected: bool,
    with_tabs: bool,
) {
    let row = list_row_region_from(list_top(with_tabs), visible_index);
    draw_row(
        strip,
        row,
        title,
        detail_or_value,
        if selected {
            CrossInkRowTone::Selected
        } else {
            CrossInkRowTone::Normal
        },
    );
}

pub fn draw_status_message(strip: &mut StripBuffer, line1: &str, line2: &str, with_tabs: bool) {
    let top = list_top(with_tabs);
    draw_row(
        strip,
        list_row_region_from(top, 0),
        line1,
        "",
        CrossInkRowTone::Normal,
    );
    if !line2.is_empty() {
        draw_row(
            strip,
            list_row_region_from(top, 1),
            line2,
            "",
            CrossInkRowTone::Normal,
        );
    }
}

pub fn draw_reader_status_message(strip: &mut StripBuffer, line1: &str, line2: &str) {
    draw_row(
        strip,
        reader_list_row_region_from(READER_LIST_TOP_WITH_TABS, 0),
        line1,
        "",
        CrossInkRowTone::Normal,
    );
    if !line2.is_empty() {
        draw_row(
            strip,
            reader_list_row_region_from(READER_LIST_TOP_WITH_TABS, 1),
            line2,
            "",
            CrossInkRowTone::Normal,
        );
    }
}
