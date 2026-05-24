// reusable on-device text keyboard for Xteink X4 button navigation
//
// This widget is intentionally app-agnostic: apps own their text buffers,
// while this module owns cursor movement, layouts, key actions, and drawing.

use core::fmt::Write as _;

use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::PrimitiveStyle;

use crate::rustmix_x4::x4_apps::fonts::bitmap::BitmapFont;
use crate::rustmix_x4::x4_apps::ui::{Alignment, BitmapDynLabel, BitmapLabel, Region};
use crate::rustmix_x4::x4_kernel::drivers::strip::StripBuffer;

pub const TEXT_KEYBOARD_MARKER: &str = "x4-shared-text-keyboard-ok";
pub const TEXT_KEYBOARD_LARGE_HEIGHT: u16 = 236;

pub const LAYOUT_LOWER: u8 = 0;
pub const LAYOUT_UPPER: u8 = 1;
pub const LAYOUT_SYMBOLS: u8 = 2;

const KEYBOARD_ROW_COUNT: u8 = 4;
const ACTION_ROW: u8 = 3;

const LOWER_ROWS: [&[u8]; 3] = [b"qwertyuiop", b"asdfghjkl", b"zxcvbnm"];
const UPPER_ROWS: [&[u8]; 3] = [b"QWERTYUIOP", b"ASDFGHJKL", b"ZXCVBNM"];
const SYMBOL_ROWS: [&[u8]; 3] = [b"1234567890", b"-_@./:#$%", b"&*+=!?',\""];

const ACTION_COUNT: u8 = 6;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextKeyboardAction {
    None,
    Insert(u8),
    Space,
    Delete,
    Clear,
    ToggleCase,
    ToggleSymbols,
    Done,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct KeyboardPos {
    row: u8,
    col: u8,
}

#[inline]
pub fn normalize_layout(layout: u8) -> u8 {
    match layout {
        LAYOUT_UPPER => LAYOUT_UPPER,
        LAYOUT_SYMBOLS => LAYOUT_SYMBOLS,
        _ => LAYOUT_LOWER,
    }
}

#[inline]
pub fn layout_name(layout: u8) -> &'static str {
    match normalize_layout(layout) {
        LAYOUT_UPPER => "ABC",
        LAYOUT_SYMBOLS => "123",
        _ => "abc",
    }
}

#[inline]
pub fn default_index() -> u8 {
    0
}

fn letter_rows(layout: u8) -> [&'static [u8]; 3] {
    match normalize_layout(layout) {
        LAYOUT_UPPER => UPPER_ROWS,
        LAYOUT_SYMBOLS => SYMBOL_ROWS,
        _ => LOWER_ROWS,
    }
}

fn row_len(layout: u8, row: u8) -> u8 {
    if row == ACTION_ROW {
        ACTION_COUNT
    } else {
        letter_rows(layout)[row as usize].len() as u8
    }
}

fn total_keys(layout: u8) -> u8 {
    row_len(layout, 0) + row_len(layout, 1) + row_len(layout, 2) + row_len(layout, ACTION_ROW)
}

fn index_to_pos(layout: u8, index: u8) -> KeyboardPos {
    let mut remaining = index.min(total_keys(layout).saturating_sub(1));
    let mut row = 0u8;
    while row < KEYBOARD_ROW_COUNT {
        let len = row_len(layout, row);
        if remaining < len {
            return KeyboardPos {
                row,
                col: remaining,
            };
        }
        remaining = remaining.saturating_sub(len);
        row += 1;
    }
    KeyboardPos { row: 0, col: 0 }
}

fn pos_to_index(layout: u8, pos: KeyboardPos) -> u8 {
    let row = pos.row.min(KEYBOARD_ROW_COUNT - 1);
    let col = pos.col.min(row_len(layout, row).saturating_sub(1));
    let mut idx = 0u8;
    let mut r = 0u8;
    while r < row {
        idx = idx.saturating_add(row_len(layout, r));
        r += 1;
    }
    idx.saturating_add(col)
}

pub fn move_horizontal(layout: u8, index: u8, delta: isize) -> u8 {
    let layout = normalize_layout(layout);
    let mut pos = index_to_pos(layout, index);
    let len = row_len(layout, pos.row) as isize;
    pos.col = (pos.col as isize + delta).rem_euclid(len) as u8;
    pos_to_index(layout, pos)
}

pub fn move_vertical(layout: u8, index: u8, delta: isize) -> u8 {
    let layout = normalize_layout(layout);
    let mut pos = index_to_pos(layout, index);
    pos.row = (pos.row as isize + delta).rem_euclid(KEYBOARD_ROW_COUNT as isize) as u8;
    pos.col = pos.col.min(row_len(layout, pos.row).saturating_sub(1));
    pos_to_index(layout, pos)
}

pub fn switch_layout(current: u8, next: u8, index: u8) -> (u8, u8) {
    let old_layout = normalize_layout(current);
    let new_layout = normalize_layout(next);
    let old_pos = index_to_pos(old_layout, index);
    let new_pos = KeyboardPos {
        row: old_pos.row,
        col: old_pos
            .col
            .min(row_len(new_layout, old_pos.row).saturating_sub(1)),
    };
    (new_layout, pos_to_index(new_layout, new_pos))
}

pub fn activate(layout: u8, index: u8) -> TextKeyboardAction {
    let layout = normalize_layout(layout);
    let pos = index_to_pos(layout, index);
    if pos.row < ACTION_ROW {
        let rows = letter_rows(layout);
        return rows[pos.row as usize]
            .get(pos.col as usize)
            .copied()
            .map(TextKeyboardAction::Insert)
            .unwrap_or(TextKeyboardAction::None);
    }

    match pos.col {
        0 => TextKeyboardAction::Space,
        1 => TextKeyboardAction::Delete,
        2 => TextKeyboardAction::Clear,
        3 => TextKeyboardAction::ToggleCase,
        4 => TextKeyboardAction::ToggleSymbols,
        _ => TextKeyboardAction::Done,
    }
}

fn action_label(layout: u8, col: u8) -> &'static str {
    match col {
        0 => "space",
        1 => "del",
        2 => "clear",
        3 => {
            if normalize_layout(layout) == LAYOUT_UPPER {
                "abc"
            } else {
                "ABC"
            }
        }
        4 => {
            if normalize_layout(layout) == LAYOUT_SYMBOLS {
                "abc"
            } else {
                "123"
            }
        }
        _ => "done",
    }
}

fn key_region(area: Region, row: u8, col: u8, len: u8) -> Region {
    let row_gap = 8u16;
    let key_gap = 6u16;
    let row_h = (area
        .h
        .saturating_sub(row_gap * (KEYBOARD_ROW_COUNT as u16 - 1)))
        / KEYBOARD_ROW_COUNT as u16;
    let key_w = (area.w.saturating_sub(key_gap * (len as u16 - 1))) / len as u16;
    let used_w = key_w * len as u16 + key_gap * (len as u16 - 1);
    let x0 = area.x + area.w.saturating_sub(used_w) / 2;
    Region::new(
        x0 + col as u16 * (key_w + key_gap),
        area.y + row as u16 * (row_h + row_gap),
        key_w,
        row_h,
    )
}

pub fn draw(
    strip: &mut StripBuffer,
    area: Region,
    layout: u8,
    index: u8,
    key_font: &'static BitmapFont,
    hint_font: &'static BitmapFont,
) {
    let layout = normalize_layout(layout);
    let selected_pos = index_to_pos(layout, index);
    let mut row = 0u8;
    while row < KEYBOARD_ROW_COUNT {
        let len = row_len(layout, row);
        let mut col = 0u8;
        while col < len {
            let selected = selected_pos.row == row && selected_pos.col == col;
            let region = key_region(area, row, col, len);
            if selected {
                region
                    .to_rect()
                    .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                    .draw(strip)
                    .unwrap();
            } else {
                region
                    .to_rect()
                    .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
                    .draw(strip)
                    .unwrap();
            }

            if row < ACTION_ROW {
                let rows = letter_rows(layout);
                let key = rows[row as usize][col as usize] as char;
                let mut label = BitmapDynLabel::<8>::new(region, key_font)
                    .alignment(Alignment::Center)
                    .inverted(selected);
                let _ = write!(label, "{}", key);
                label.draw(strip).unwrap();
            } else {
                let label = action_label(layout, col);
                BitmapLabel::new(region, label, hint_font)
                    .alignment(Alignment::Center)
                    .inverted(selected)
                    .draw(strip)
                    .unwrap();
            }
            col += 1;
        }
        row += 1;
    }
}
