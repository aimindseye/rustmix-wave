// formatting helpers for common UI patterns
//
// small helpers for frequently-used format strings like position
// indicators (1/10), percentages, etc. reduces code duplication
// and ensures consistent formatting across apps.

use core::fmt::Write;

use crate::rustmix_x4::x4_apps::fonts::bitmap::BitmapFont;
use crate::rustmix_x4::x4_apps::ui::{Alignment, BitmapDynLabel, Region};

// format and draw a position indicator like "3/15" or "3/15 ..."
pub fn draw_position_indicator<const N: usize>(
    strip: &mut crate::rustmix_x4::x4_kernel::drivers::strip::StripBuffer,
    region: Region,
    current: usize,
    total: usize,
    font: &'static BitmapFont,
    suffix: Option<&str>,
) {
    if total == 0 {
        return;
    }
    let mut label = BitmapDynLabel::<N>::new(region, font).alignment(Alignment::CenterRight);
    let _ = write!(label, "{}/{}", current, total);
    if let Some(s) = suffix {
        let _ = write!(label, "{}", s);
    }
    label.draw(strip).unwrap();
}

// format position as "current/total" into a buffer
pub fn fmt_position(buf: &mut [u8], current: usize, total: usize) -> usize {
    let mut pos = 0;

    // format current
    if current >= 10000 {
        buf[pos] = b'0' + ((current / 10000) % 10) as u8;
        pos += 1;
    }
    if current >= 1000 {
        buf[pos] = b'0' + ((current / 1000) % 10) as u8;
        pos += 1;
    }
    if current >= 100 {
        buf[pos] = b'0' + ((current / 100) % 10) as u8;
        pos += 1;
    }
    if current >= 10 {
        buf[pos] = b'0' + ((current / 10) % 10) as u8;
        pos += 1;
    }
    buf[pos] = b'0' + (current % 10) as u8;
    pos += 1;

    buf[pos] = b'/';
    pos += 1;

    // format total
    if total >= 10000 {
        buf[pos] = b'0' + ((total / 10000) % 10) as u8;
        pos += 1;
    }
    if total >= 1000 {
        buf[pos] = b'0' + ((total / 1000) % 10) as u8;
        pos += 1;
    }
    if total >= 100 {
        buf[pos] = b'0' + ((total / 100) % 10) as u8;
        pos += 1;
    }
    if total >= 10 {
        buf[pos] = b'0' + ((total / 10) % 10) as u8;
        pos += 1;
    }
    buf[pos] = b'0' + (total % 10) as u8;
    pos += 1;

    pos
}

// format a percentage (0-100) into a buffer as "NN%"
pub fn fmt_percent(buf: &mut [u8], pct: u8) -> usize {
    let pct = pct.min(100);
    let mut pos = 0;

    if pct >= 100 {
        buf[pos] = b'1';
        pos += 1;
        buf[pos] = b'0';
        pos += 1;
        buf[pos] = b'0';
        pos += 1;
    } else if pct >= 10 {
        buf[pos] = b'0' + (pct / 10);
        pos += 1;
        buf[pos] = b'0' + (pct % 10);
        pos += 1;
    } else {
        buf[pos] = b'0' + pct;
        pos += 1;
    }

    buf[pos] = b'%';
    pos += 1;

    pos
}
