// boot console: accumulates text lines during hardware init, rendered
// once to EPD before the app layer takes over
//
// uses the embedded-graphics built-in FONT_9X18 -- no TTF assets or
// build.rs font pipeline needed; the kernel can show boot progress
// on a bare display with nothing but this mono font

use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::mono_font::ascii::FONT_9X18;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::text::Text;

use crate::rustmix_x4::x4_kernel::drivers::strip::StripBuffer;

const MAX_LINES: usize = 40;
const MAX_LINE_LEN: usize = 76;
const LEFT_MARGIN: i32 = 8;
const TOP_MARGIN: i32 = 6;
const LINE_H: i32 = 20;

pub struct BootConsole {
    lines: [[u8; MAX_LINE_LEN]; MAX_LINES],
    lengths: [u8; MAX_LINES],
    count: usize,
}

impl Default for BootConsole {
    fn default() -> Self {
        Self::new()
    }
}

impl BootConsole {
    pub const fn new() -> Self {
        Self {
            lines: [[0u8; MAX_LINE_LEN]; MAX_LINES],
            lengths: [0u8; MAX_LINES],
            count: 0,
        }
    }

    pub fn push(&mut self, text: &str) {
        if self.count >= MAX_LINES {
            return;
        }
        let bytes = text.as_bytes();
        let len = bytes.len().min(MAX_LINE_LEN);
        self.lines[self.count][..len].copy_from_slice(&bytes[..len]);
        self.lengths[self.count] = len as u8;
        self.count += 1;
    }

    pub fn draw(&self, strip: &mut StripBuffer) {
        let style = MonoTextStyle::new(&FONT_9X18, BinaryColor::On);
        for i in 0..self.count {
            let len = self.lengths[i] as usize;
            let text = core::str::from_utf8(&self.lines[i][..len]).unwrap_or("");
            let y = TOP_MARGIN + (i as i32 + 1) * LINE_H;
            let _ = Text::new(text, Point::new(LEFT_MARGIN, y), style).draw(strip);
        }
    }
}
