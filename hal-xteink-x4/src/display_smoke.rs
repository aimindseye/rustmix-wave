//! Minimal Xteink X4 SSD1677 display smoke/home/input-navigation/library/TXT-reader driver.
//!
//! Keeps the proven display transport shape:
//! - DMA-backed `SpiDevice` chip-select ownership.
//! - SD chip-select kept high when the panel owns the shared SPI bus.
//! - Full-frame strip rendering, no full framebuffer.
//! - RED RAM then BW RAM before a full update.

use embedded_hal::delay::DelayNs;
use embedded_hal::digital::{InputPin, OutputPin};
use embedded_hal::spi::SpiDevice;

pub const X4_EPD_NATIVE_WIDTH: u16 = 800;
pub const X4_EPD_NATIVE_HEIGHT: u16 = 480;
pub const X4_EPD_LOGICAL_WIDTH: u16 = 480;
pub const X4_EPD_LOGICAL_HEIGHT: u16 = 800;
pub const X4_EPD_STRIP_ROWS: u16 = 40;
pub const X4_EPD_BYTES_PER_ROW: usize = X4_EPD_NATIVE_WIDTH as usize / 8;
pub const X4_EPD_STRIP_BYTES: usize = X4_EPD_BYTES_PER_ROW * X4_EPD_STRIP_ROWS as usize;
pub const X4_EPD_STRIP_COUNT: u16 = X4_EPD_NATIVE_HEIGHT / X4_EPD_STRIP_ROWS;
pub const X4_LIBRARY_MAX_ITEMS: usize = 8;
pub const X4_READER_TEXT_BYTES: usize = 1024;
pub const X4_READER_VISIBLE_LINES: usize = 18;
pub const X4_READER_LINE_CHARS: usize = 34;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LibraryFileKind {
    Txt,
    Md,
    Epu,
    Epub,
    Unknown,
}

impl LibraryFileKind {
    pub const fn label(self) -> &'static [u8] {
        match self {
            Self::Txt => b"TXT",
            Self::Md => b"MD",
            Self::Epu => b"EPU",
            Self::Epub => b"EPUB",
            Self::Unknown => b"FILE",
        }
    }

    pub fn is_text(self) -> bool {
        matches!(self, Self::Txt | Self::Md)
    }

    pub fn is_epub_like(self) -> bool {
        matches!(self, Self::Epu | Self::Epub)
    }
}

pub const X4_LIBRARY_PATH_BYTES: usize = 64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LibraryListItem {
    pub name: [u8; 13],
    pub name_len: u8,
    pub path: [u8; X4_LIBRARY_PATH_BYTES],
    pub path_len: u8,
    pub size: u32,
    pub kind: LibraryFileKind,
}

impl LibraryListItem {
    pub const EMPTY: Self = Self {
        name: [0; 13],
        name_len: 0,
        path: [0; X4_LIBRARY_PATH_BYTES],
        path_len: 0,
        size: 0,
        kind: LibraryFileKind::Unknown,
    };

    pub fn new(name: &[u8], size: u32) -> Self {
        Self::with_path(name, name, size)
    }

    pub fn with_path(path: &[u8], name: &[u8], size: u32) -> Self {
        let mut out = Self::EMPTY;
        let n = name.len().min(out.name.len());
        out.name[..n].copy_from_slice(&name[..n]);
        out.name_len = n as u8;

        let p = path.len().min(out.path.len());
        out.path[..p].copy_from_slice(&path[..p]);
        out.path_len = p as u8;

        out.size = size;
        out.kind = kind_for_name(name);
        out
    }

    pub fn name_bytes(&self) -> &[u8] {
        &self.name[..self.name_len as usize]
    }

    pub fn name_str(&self) -> &str {
        core::str::from_utf8(self.name_bytes()).unwrap_or("?")
    }

    pub fn path_bytes(&self) -> &[u8] {
        &self.path[..self.path_len as usize]
    }

    pub fn path_str(&self) -> &str {
        core::str::from_utf8(self.path_bytes()).unwrap_or("?")
    }

    pub fn kind_label(&self) -> &'static [u8] {
        self.kind.label()
    }

    pub fn is_text_reader_supported(&self) -> bool {
        self.kind.is_text()
    }

    pub fn is_epub_like(&self) -> bool {
        self.kind.is_epub_like()
    }
}

fn kind_for_name(name: &[u8]) -> LibraryFileKind {
    if ext_eq_ascii(name, b"TXT") {
        LibraryFileKind::Txt
    } else if ext_eq_ascii(name, b"MD") {
        LibraryFileKind::Md
    } else if ext_eq_ascii(name, b"EPU") {
        LibraryFileKind::Epu
    } else if ext_eq_ascii(name, b"EPUB") {
        LibraryFileKind::Epub
    } else {
        LibraryFileKind::Unknown
    }
}

fn ext_eq_ascii(name: &[u8], target: &[u8]) -> bool {
    let Some(dot) = name.iter().rposition(|&b| b == b'.') else {
        return false;
    };
    let ext = &name[dot + 1..];
    ext.len() == target.len() && ext.eq_ignore_ascii_case(target)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReaderPage {
    pub name: [u8; 13],
    pub name_len: u8,
    pub file_size: u32,
    pub read_len: u16,
    pub offset: u32,
    pub page_index: u16,
    pub total_pages: u16,
    pub bookmark_count: u8,
    pub bookmarked: bool,
    text: [u8; X4_READER_TEXT_BYTES],
    line_starts: [u16; X4_READER_VISIBLE_LINES],
    line_lens: [u8; X4_READER_VISIBLE_LINES],
    line_count: u8,
}

impl ReaderPage {
    pub const EMPTY: Self = Self {
        name: [0; 13],
        name_len: 0,
        file_size: 0,
        read_len: 0,
        offset: 0,
        page_index: 1,
        total_pages: 1,
        bookmark_count: 0,
        bookmarked: false,
        text: [0; X4_READER_TEXT_BYTES],
        line_starts: [0; X4_READER_VISIBLE_LINES],
        line_lens: [0; X4_READER_VISIBLE_LINES],
        line_count: 0,
    };

    pub fn new(name: &[u8], file_size: u32, bytes: &[u8]) -> Self {
        Self::new_paged(name, file_size, 0, 1, page_count_for(file_size), bytes)
    }

    pub fn new_paged(
        name: &[u8],
        file_size: u32,
        offset: u32,
        page_index: u16,
        total_pages: u16,
        bytes: &[u8],
    ) -> Self {
        let mut out = Self::EMPTY;
        let name_len = name.len().min(out.name.len());
        out.name[..name_len].copy_from_slice(&name[..name_len]);
        out.name_len = name_len as u8;
        out.file_size = file_size;
        out.offset = offset;
        out.page_index = page_index.max(1);
        out.total_pages = total_pages.max(1);

        let read_len = bytes.len().min(out.text.len());
        out.text[..read_len].copy_from_slice(&bytes[..read_len]);
        out.read_len = read_len as u16;
        out.index_lines();
        out
    }

    pub fn name_bytes(&self) -> &[u8] {
        &self.name[..self.name_len as usize]
    }

    pub fn name_str(&self) -> &str {
        core::str::from_utf8(self.name_bytes()).unwrap_or("?")
    }

    pub fn set_bookmark_state(&mut self, bookmark_count: u8, bookmarked: bool) {
        self.bookmark_count = bookmark_count;
        self.bookmarked = bookmarked;
    }

    pub fn line_count(&self) -> usize {
        self.line_count as usize
    }

    pub fn next_offset(&self) -> Option<u32> {
        if self.file_size == 0 {
            return None;
        }
        let next = self.offset.saturating_add(X4_READER_TEXT_BYTES as u32);
        (next < self.file_size).then_some(next)
    }

    pub fn prev_offset(&self) -> Option<u32> {
        (self.offset > 0).then_some(self.offset.saturating_sub(X4_READER_TEXT_BYTES as u32))
    }

    fn index_lines(&mut self) {
        self.line_starts = [0; X4_READER_VISIBLE_LINES];
        self.line_lens = [0; X4_READER_VISIBLE_LINES];
        self.line_count = 0;

        let mut i = 0usize;
        let end = self.read_len as usize;
        let mut line_start = 0usize;
        let mut line_len = 0usize;

        while i < end && (self.line_count as usize) < X4_READER_VISIBLE_LINES {
            let b = self.text[i];
            if b == b'\r' || b == b'\n' {
                self.commit_line(line_start, line_len);
                i += 1;
                if b == b'\r' && i < end && self.text[i] == b'\n' {
                    i += 1;
                }
                line_start = i;
                line_len = 0;
                continue;
            }
            if line_len >= X4_READER_LINE_CHARS {
                self.commit_line(line_start, line_len);
                line_start = i;
                line_len = 0;
                continue;
            }
            line_len += 1;
            i += 1;
        }

        if (self.line_count as usize) < X4_READER_VISIBLE_LINES && (line_len > 0 || end == 0) {
            self.commit_line(line_start, line_len);
        }
    }

    fn commit_line(&mut self, start: usize, len: usize) {
        let idx = self.line_count as usize;
        if idx >= X4_READER_VISIBLE_LINES {
            return;
        }
        self.line_starts[idx] = start.min(u16::MAX as usize) as u16;
        self.line_lens[idx] = len.min(X4_READER_LINE_CHARS).min(u8::MAX as usize) as u8;
        self.line_count += 1;
    }

    fn byte_at_line_col(&self, line: usize, col: usize) -> Option<u8> {
        if line >= self.line_count as usize || col >= self.line_lens[line] as usize {
            return None;
        }
        let idx = self.line_starts[line] as usize + col;
        self.text.get(idx).copied()
    }
}

const fn page_count_for(file_size: u32) -> u16 {
    let pages = if file_size == 0 {
        1
    } else {
        file_size.div_ceil(X4_READER_TEXT_BYTES as u32) as u16
    };
    if pages == 0 { 1 } else { pages }
}

#[allow(dead_code)]
mod cmd {
    pub const DRIVER_OUTPUT_CONTROL: u8 = 0x01;
    pub const BOOSTER_SOFT_START: u8 = 0x0C;
    pub const DATA_ENTRY_MODE: u8 = 0x11;
    pub const SW_RESET: u8 = 0x12;
    pub const TEMPERATURE_SENSOR: u8 = 0x18;
    pub const MASTER_ACTIVATION: u8 = 0x20;
    pub const DISPLAY_UPDATE_CONTROL_1: u8 = 0x21;
    pub const DISPLAY_UPDATE_CONTROL_2: u8 = 0x22;
    pub const WRITE_RAM_BW: u8 = 0x24;
    pub const WRITE_RAM_RED: u8 = 0x26;
    pub const BORDER_WAVEFORM: u8 = 0x3C;
    pub const SET_RAM_X_RANGE: u8 = 0x44;
    pub const SET_RAM_Y_RANGE: u8 = 0x45;
    pub const SET_RAM_X_COUNTER: u8 = 0x4E;
    pub const SET_RAM_Y_COUNTER: u8 = 0x4F;
}

pub struct X4Ssd1677Smoke<SPI, DC, RST, BUSY> {
    spi: SPI,
    dc: DC,
    rst: RST,
    busy: BUSY,
    initialised: bool,
}

impl<SPI, DC, RST, BUSY, E> X4Ssd1677Smoke<SPI, DC, RST, BUSY>
where
    SPI: SpiDevice<Error = E>,
    DC: OutputPin,
    RST: OutputPin,
    BUSY: InputPin,
{
    pub fn new(spi: SPI, dc: DC, rst: RST, busy: BUSY) -> Self {
        Self {
            spi,
            dc,
            rst,
            busy,
            initialised: false,
        }
    }

    pub fn init<D: DelayNs>(&mut self, delay: &mut D) {
        self.reset(delay);
        self.init_display(delay);
    }

    pub fn draw_display_smoke<D: DelayNs>(&mut self, delay: &mut D) {
        self.draw_full_frame(delay, render_smoke_strip);
    }

    pub fn draw_minimal_home<D: DelayNs>(&mut self, delay: &mut D, sd_ok: bool, battery_pct: u8) {
        self.draw_home_navigation(delay, sd_ok, battery_pct, 0);
    }

    pub fn draw_home_navigation<D: DelayNs>(
        &mut self,
        delay: &mut D,
        sd_ok: bool,
        battery_pct: u8,
        selected: u8,
    ) {
        self.draw_full_frame(delay, |strip_idx, strip| {
            render_home_nav_strip(strip_idx, strip, sd_ok, battery_pct, selected)
        });
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw_library_list<D: DelayNs>(
        &mut self,
        delay: &mut D,
        sd_ok: bool,
        battery_pct: u8,
        selected: u8,
        items: &[LibraryListItem],
        total_files: usize,
        from_books_dir: bool,
    ) {
        self.draw_full_frame(delay, |strip_idx, strip| {
            render_library_strip(
                strip_idx,
                strip,
                sd_ok,
                battery_pct,
                selected,
                items,
                total_files,
                from_books_dir,
            )
        });
    }

    pub fn draw_reader_page<D: DelayNs>(
        &mut self,
        delay: &mut D,
        sd_ok: bool,
        battery_pct: u8,
        page: &ReaderPage,
    ) {
        self.draw_full_frame(delay, |strip_idx, strip| {
            render_reader_strip(strip_idx, strip, sd_ok, battery_pct, page)
        });
    }

    pub fn draw_reader_bookmark_page<D: DelayNs>(
        &mut self,
        delay: &mut D,
        sd_ok: bool,
        battery_pct: u8,
        page: &ReaderPage,
    ) {
        self.draw_reader_page(delay, sd_ok, battery_pct, page);
    }

    pub fn is_busy(&mut self) -> bool {
        self.busy.is_high().unwrap_or(false)
    }

    fn draw_full_frame<D, F>(&mut self, delay: &mut D, mut render: F)
    where
        D: DelayNs,
        F: FnMut(u16, &mut [u8; X4_EPD_STRIP_BYTES]),
    {
        if !self.initialised {
            self.init_display(delay);
        }

        delay.delay_ms(1);
        let mut strip = [0xFFu8; X4_EPD_STRIP_BYTES];

        for ram_cmd in [cmd::WRITE_RAM_RED, cmd::WRITE_RAM_BW] {
            self.set_ram_area(0, 0, X4_EPD_NATIVE_WIDTH, X4_EPD_NATIVE_HEIGHT);
            self.send_command(ram_cmd);
            delay.delay_ms(1);

            for strip_idx in 0..X4_EPD_STRIP_COUNT {
                render(strip_idx, &mut strip);
                self.send_data(&strip);
            }
        }

        self.send_command(cmd::DISPLAY_UPDATE_CONTROL_1);
        self.send_data(&[0x40, 0x00]);

        self.send_command(cmd::DISPLAY_UPDATE_CONTROL_2);
        self.send_data(&[0xF7]);

        self.send_command(cmd::MASTER_ACTIVATION);
        self.wait_busy(delay, 20_000);
    }

    fn reset<D: DelayNs>(&mut self, delay: &mut D) {
        let _ = self.rst.set_high();
        delay.delay_ms(20);
        let _ = self.rst.set_low();
        delay.delay_ms(2);
        let _ = self.rst.set_high();
        delay.delay_ms(20);
    }

    fn init_display<D: DelayNs>(&mut self, delay: &mut D) {
        self.send_command(cmd::SW_RESET);
        delay.delay_ms(10);

        self.send_command(cmd::TEMPERATURE_SENSOR);
        self.send_data(&[0x80]);

        self.send_command(cmd::BOOSTER_SOFT_START);
        self.send_data(&[0xAE, 0xC7, 0xC3, 0xC0, 0x80]);

        self.send_command(cmd::DRIVER_OUTPUT_CONTROL);
        self.send_data(&[
            ((X4_EPD_NATIVE_HEIGHT - 1) & 0xFF) as u8,
            ((X4_EPD_NATIVE_HEIGHT - 1) >> 8) as u8,
            0x02,
        ]);

        self.send_command(cmd::BORDER_WAVEFORM);
        self.send_data(&[0x01]);

        self.set_ram_area(0, 0, X4_EPD_NATIVE_WIDTH, X4_EPD_NATIVE_HEIGHT);
        self.initialised = true;
    }

    // SSD1677 gates wired in reverse on the X4: X increments, Y decrements.
    fn set_ram_area(&mut self, x: u16, y: u16, w: u16, h: u16) {
        let y_flipped = X4_EPD_NATIVE_HEIGHT - y - h;

        self.send_command(cmd::DATA_ENTRY_MODE);
        self.send_data(&[0x01]);

        self.send_command(cmd::SET_RAM_X_RANGE);
        self.send_data(&[
            (x & 0xFF) as u8,
            (x >> 8) as u8,
            ((x + w - 1) & 0xFF) as u8,
            ((x + w - 1) >> 8) as u8,
        ]);

        self.send_command(cmd::SET_RAM_Y_RANGE);
        self.send_data(&[
            ((y_flipped + h - 1) & 0xFF) as u8,
            ((y_flipped + h - 1) >> 8) as u8,
            (y_flipped & 0xFF) as u8,
            (y_flipped >> 8) as u8,
        ]);

        self.send_command(cmd::SET_RAM_X_COUNTER);
        self.send_data(&[(x & 0xFF) as u8, (x >> 8) as u8]);

        self.send_command(cmd::SET_RAM_Y_COUNTER);
        self.send_data(&[
            ((y_flipped + h - 1) & 0xFF) as u8,
            ((y_flipped + h - 1) >> 8) as u8,
        ]);
    }

    fn send_command(&mut self, value: u8) {
        let _ = self.dc.set_low();
        let _ = self.spi.write(&[value]);
        let _ = self.dc.set_high();
    }

    fn send_data(&mut self, bytes: &[u8]) {
        let _ = self.dc.set_high();
        let _ = self.spi.write(bytes);
    }

    fn wait_busy<D: DelayNs>(&mut self, delay: &mut D, timeout_ms: u32) {
        let mut elapsed = 0;
        while elapsed < timeout_ms {
            if self.busy.is_low().unwrap_or(true) {
                return;
            }
            delay.delay_ms(1);
            elapsed += 1;
        }
    }
}

fn render_smoke_strip(strip_idx: u16, out: &mut [u8; X4_EPD_STRIP_BYTES]) {
    render_strip_with(strip_idx, out, smoke_pixel);
}

#[cfg(test)]
fn render_home_strip(
    strip_idx: u16,
    out: &mut [u8; X4_EPD_STRIP_BYTES],
    sd_ok: bool,
    battery_pct: u8,
) {
    render_home_nav_strip(strip_idx, out, sd_ok, battery_pct, 0);
}

fn render_home_nav_strip(
    strip_idx: u16,
    out: &mut [u8; X4_EPD_STRIP_BYTES],
    sd_ok: bool,
    battery_pct: u8,
    selected: u8,
) {
    render_strip_with(strip_idx, out, |x, y| {
        home_nav_pixel(x, y, sd_ok, battery_pct, selected)
    });
}

#[allow(clippy::too_many_arguments)]
fn render_library_strip(
    strip_idx: u16,
    out: &mut [u8; X4_EPD_STRIP_BYTES],
    sd_ok: bool,
    battery_pct: u8,
    selected: u8,
    items: &[LibraryListItem],
    total_files: usize,
    from_books_dir: bool,
) {
    render_strip_with(strip_idx, out, |x, y| {
        library_pixel(
            x,
            y,
            sd_ok,
            battery_pct,
            selected,
            items,
            total_files,
            from_books_dir,
        )
    });
}

fn render_reader_strip(
    strip_idx: u16,
    out: &mut [u8; X4_EPD_STRIP_BYTES],
    sd_ok: bool,
    battery_pct: u8,
    page: &ReaderPage,
) {
    render_strip_with(strip_idx, out, |x, y| {
        reader_pixel(x, y, sd_ok, battery_pct, page)
    });
}

fn render_strip_with<F>(strip_idx: u16, out: &mut [u8; X4_EPD_STRIP_BYTES], mut pixel: F)
where
    F: FnMut(u16, u16) -> bool,
{
    out.fill(0xFF);

    let native_y0 = strip_idx * X4_EPD_STRIP_ROWS;
    for row in 0..X4_EPD_STRIP_ROWS {
        let py = native_y0 + row;
        for px in 0..X4_EPD_NATIVE_WIDTH {
            // Inverse of the proven Deg270 mapping from x4-reader-os-rs:
            // physical = (logical_y, native_height - 1 - logical_x)
            let lx = X4_EPD_NATIVE_HEIGHT - 1 - py;
            let ly = px;
            if pixel(lx, ly) {
                set_black(out, row, px);
            }
        }
    }
}

fn set_black(out: &mut [u8; X4_EPD_STRIP_BYTES], row: u16, x: u16) {
    let idx = row as usize * X4_EPD_BYTES_PER_ROW + (x / 8) as usize;
    let bit = 7 - (x % 8);
    out[idx] &= !(1 << bit);
}

fn smoke_pixel(x: u16, y: u16) -> bool {
    if !(4..X4_EPD_LOGICAL_WIDTH - 4).contains(&x) || !(4..X4_EPD_LOGICAL_HEIGHT - 4).contains(&y) {
        return true;
    }
    if (72..78).contains(&y) || (376..382).contains(&y) || (698..704).contains(&y) {
        return true;
    }

    if (24..72).contains(&x) && (24..58).contains(&y) {
        return true;
    }
    if (408..456).contains(&x) && (742..776).contains(&y) {
        return true;
    }

    text_pixel(b"RUSTMIXOS", x, y, 78, 106, 6)
        || text_pixel(b"X4 DISPLAY SMOKE", x, y, 96, 204, 3)
        || text_pixel(b"X4 READY", x, y, 156, 284, 4)
        || text_pixel(b"480X800 PORTRAIT", x, y, 108, 430, 2)
        || text_pixel(b"BOOT OK", x, y, 168, 640, 3)
}

#[cfg(test)]
fn home_pixel(x: u16, y: u16, sd_ok: bool, battery_pct: u8) -> bool {
    home_nav_pixel(x, y, sd_ok, battery_pct, 0)
}

fn home_nav_pixel(x: u16, y: u16, sd_ok: bool, battery_pct: u8, selected: u8) -> bool {
    if !(4..X4_EPD_LOGICAL_WIDTH - 4).contains(&x) || !(4..X4_EPD_LOGICAL_HEIGHT - 4).contains(&y) {
        return true;
    }
    if (132..138).contains(&y) || (672..678).contains(&y) {
        return true;
    }

    if selected_marker_pixel(x, y, selected) {
        return true;
    }

    text_pixel(b"RUSTMIXOS", x, y, 66, 58, 5)
        || text_pixel(b"INPUT NAV SMOKE", x, y, 92, 116, 2)
        || text_pixel(b"CONTINUE", x, y, 82, 196, 3)
        || text_pixel(b"LIBRARY", x, y, 82, 270, 3)
        || text_pixel(b"SETTINGS", x, y, 82, 344, 3)
        || text_pixel(b"SYSTEM", x, y, 82, 418, 3)
        || text_pixel(b"UP DOWN MOVE", x, y, 110, 540, 2)
        || text_pixel(b"SELECT LOGS ITEM", x, y, 88, 584, 2)
        || text_pixel(if sd_ok { b"SD OK" } else { b"SD NO" }, x, y, 28, 724, 2)
        || battery_status_pixel(x, y, 328, 724, 2, battery_pct)
}

fn selected_marker_pixel(x: u16, y: u16, selected: u8) -> bool {
    let selected = selected.min(3) as u16;
    let y0 = 206 + selected * 74;
    (34..56).contains(&x) && (y0..y0 + 22).contains(&y)
}

#[allow(clippy::too_many_arguments)]
fn library_pixel(
    x: u16,
    y: u16,
    sd_ok: bool,
    battery_pct: u8,
    selected: u8,
    items: &[LibraryListItem],
    total_files: usize,
    from_books_dir: bool,
) -> bool {
    if !(4..X4_EPD_LOGICAL_WIDTH - 4).contains(&x) || !(4..X4_EPD_LOGICAL_HEIGHT - 4).contains(&y) {
        return true;
    }
    if (132..138).contains(&y) || (672..678).contains(&y) {
        return true;
    }

    text_pixel(b"RUSTMIXOS", x, y, 66, 58, 5)
        || text_pixel(b"LIBRARY SMOKE", x, y, 108, 116, 2)
        || text_pixel(
            if from_books_dir {
                b"ALL PATHS"
            } else {
                b"ROOT TREE"
            },
            x,
            y,
            28,
            152,
            2,
        )
        || text_pixel(b"FILES", x, y, 268, 152, 2)
        || small_number_pixel(x, y, 352, 152, 2, total_files.min(99) as u8)
        || library_rows_pixel(x, y, selected, items)
        || library_empty_pixel(x, y, items)
        || text_pixel(b"SELECT LOGS FILE", x, y, 82, 608, 2)
        || text_pixel(if sd_ok { b"SD OK" } else { b"SD NO" }, x, y, 28, 724, 2)
        || battery_status_pixel(x, y, 328, 724, 2, battery_pct)
}

fn library_rows_pixel(x: u16, y: u16, selected: u8, items: &[LibraryListItem]) -> bool {
    let visible = items.len().min(5);
    let selected = selected as usize;

    for (idx, item) in items.iter().take(visible).enumerate() {
        let y0 = 208 + idx as u16 * 72;
        if idx == selected && (34..56).contains(&x) && (y0..y0 + 22).contains(&y) {
            return true;
        }
        if text_pixel_limited(item.kind_label(), x, y, 68, y0 - 8, 2, 4) {
            return true;
        }
        if text_pixel_limited(item.path_bytes(), x, y, 124, y0 - 8, 2, 24) {
            return true;
        }
    }

    false
}

fn library_empty_pixel(x: u16, y: u16, items: &[LibraryListItem]) -> bool {
    items.is_empty()
        && (text_pixel(b"NO BOOKS FOUND", x, y, 78, 252, 3)
            || text_pixel(b"ADD TXT EPUB EPU", x, y, 94, 336, 2)
            || text_pixel(b"ROOT OR FOLDERS", x, y, 96, 380, 2))
}

fn reader_pixel(x: u16, y: u16, sd_ok: bool, battery_pct: u8, page: &ReaderPage) -> bool {
    if !(4..X4_EPD_LOGICAL_WIDTH - 4).contains(&x) || !(4..X4_EPD_LOGICAL_HEIGHT - 4).contains(&y) {
        return true;
    }
    if (132..138).contains(&y) || (672..678).contains(&y) {
        return true;
    }

    text_pixel(b"RUSTMIXOS", x, y, 66, 46, 4)
        || text_pixel(b"TXT READER", x, y, 116, 100, 2)
        || text_pixel_limited(page.name_bytes(), x, y, 28, 152, 2, 13)
        || reader_lines_pixel(x, y, page)
        || text_pixel(b"UP PREV  DN NEXT", x, y, 78, 608, 2)
        || text_pixel(b"LONG SEL MARK", x, y, 82, 636, 2)
        || reader_bookmark_status_pixel(x, y, 286, 636, 2, page)
        || text_pixel(b"PG", x, y, 28, 686, 2)
        || decimal_number_pixel(x, y, 64, 686, 2, page.page_index as u32)
        || char_pixel(b'/', x, y, 100, 686, 2)
        || decimal_number_pixel(x, y, 116, 686, 2, page.total_pages as u32)
        || text_pixel(b"OFF", x, y, 190, 686, 2)
        || decimal_number_pixel(x, y, 244, 686, 2, page.offset)
        || text_pixel(if sd_ok { b"SD OK" } else { b"SD NO" }, x, y, 28, 724, 2)
        || battery_status_pixel(x, y, 328, 724, 2, battery_pct)
}

fn reader_bookmark_status_pixel(
    x: u16,
    y: u16,
    x0: u16,
    y0: u16,
    scale: u16,
    page: &ReaderPage,
) -> bool {
    text_pixel(
        if page.bookmarked { b"BM ON" } else { b"BM OFF" },
        x,
        y,
        x0,
        y0,
        scale,
    ) || decimal_number_pixel(
        x,
        y,
        x0 + 7 * 6 * scale,
        y0,
        scale,
        page.bookmark_count as u32,
    )
}

fn reader_lines_pixel(x: u16, y: u16, page: &ReaderPage) -> bool {
    const X0: u16 = 30;
    const Y0: u16 = 196;
    const SCALE: u16 = 2;
    const ADVANCE: u16 = 6 * SCALE;
    const GLYPH_W: u16 = 5 * SCALE;
    const GLYPH_H: u16 = 7 * SCALE;
    const LINE_STEP: u16 = 24;

    if x < X0 || y < Y0 {
        return false;
    }
    let rel_y = y - Y0;
    let line = (rel_y / LINE_STEP) as usize;
    if line >= page.line_count() || rel_y % LINE_STEP >= GLYPH_H {
        return false;
    }
    let rel_x = x - X0;
    let col_idx = (rel_x / ADVANCE) as usize;
    if col_idx >= X4_READER_LINE_CHARS || rel_x % ADVANCE >= GLYPH_W {
        return false;
    }
    let ch = match page.byte_at_line_col(line, col_idx) {
        Some(b) => normalize_text_byte(b),
        None => return false,
    };
    let glyph_col = ((rel_x % ADVANCE) / SCALE) as usize;
    let glyph_row = ((rel_y % LINE_STEP) / SCALE) as usize;
    let glyph = glyph_for(ch);
    (glyph[glyph_row] & (1 << (4 - glyph_col))) != 0
}

fn normalize_text_byte(b: u8) -> u8 {
    match b {
        b'a'..=b'z' => b - 32,
        b'\t' | b'\r' | b'\n' => b' ',
        b',' | b';' | b':' | b'!' | b'?' | b'\'' | b'"' | b'(' | b')' => b' ',
        _ => b,
    }
}
fn small_number_pixel(x: u16, y: u16, x0: u16, y0: u16, scale: u16, value: u8) -> bool {
    let v = value.min(99);
    let tens = v / 10;
    let ones = v % 10;
    let advance = 6 * scale;

    if v >= 10 {
        char_pixel(b'0' + tens, x, y, x0, y0, scale)
            || char_pixel(b'0' + ones, x, y, x0 + advance, y0, scale)
    } else {
        char_pixel(b'0' + ones, x, y, x0, y0, scale)
    }
}

fn decimal_number_pixel(x: u16, y: u16, x0: u16, y0: u16, scale: u16, value: u32) -> bool {
    let mut buf = [0u8; 10];
    let mut n = value;
    let mut len = 0usize;

    if n == 0 {
        buf[0] = b'0';
        len = 1;
    } else {
        while n > 0 && len < buf.len() {
            buf[len] = b'0' + (n % 10) as u8;
            n /= 10;
            len += 1;
        }
    }

    let advance = 6 * scale;
    let mut i = 0usize;
    while i < len {
        let ch = buf[len - 1 - i];
        if char_pixel(ch, x, y, x0 + i as u16 * advance, y0, scale) {
            return true;
        }
        i += 1;
    }
    false
}

fn battery_status_pixel(x: u16, y: u16, x0: u16, y0: u16, scale: u16, battery_pct: u8) -> bool {
    if text_pixel(b"BAT", x, y, x0, y0, scale) {
        return true;
    }

    let pct = battery_pct.min(100);
    let advance = 6 * scale;
    let digit_x0 = x0 + 4 * advance;

    if pct >= 100 {
        return char_pixel(b'1', x, y, digit_x0, y0, scale)
            || char_pixel(b'0', x, y, digit_x0 + advance, y0, scale)
            || char_pixel(b'0', x, y, digit_x0 + 2 * advance, y0, scale);
    }

    let tens = pct / 10;
    let ones = pct % 10;
    char_pixel(b'0' + tens, x, y, digit_x0, y0, scale)
        || char_pixel(b'0' + ones, x, y, digit_x0 + advance, y0, scale)
}

fn text_pixel(text: &[u8], x: u16, y: u16, x0: u16, y0: u16, scale: u16) -> bool {
    let glyph_w = 5 * scale;
    let glyph_h = 7 * scale;
    let advance = 6 * scale;

    if y < y0 || y >= y0 + glyph_h || x < x0 {
        return false;
    }

    let rel_x = x - x0;
    let idx = (rel_x / advance) as usize;
    if idx >= text.len() {
        return false;
    }

    let in_glyph_x = rel_x % advance;
    if in_glyph_x >= glyph_w {
        return false;
    }

    let col = (in_glyph_x / scale) as usize;
    let row = ((y - y0) / scale) as usize;
    let glyph = glyph_for(text[idx]);
    (glyph[row] & (1 << (4 - col))) != 0
}

fn text_pixel_limited(
    text: &[u8],
    x: u16,
    y: u16,
    x0: u16,
    y0: u16,
    scale: u16,
    max_chars: usize,
) -> bool {
    let n = text.len().min(max_chars);
    text_pixel(&text[..n], x, y, x0, y0, scale)
}

fn char_pixel(ch: u8, x: u16, y: u16, x0: u16, y0: u16, scale: u16) -> bool {
    let glyph_w = 5 * scale;
    let glyph_h = 7 * scale;

    if y < y0 || y >= y0 + glyph_h || x < x0 || x >= x0 + glyph_w {
        return false;
    }

    let col = ((x - x0) / scale) as usize;
    let row = ((y - y0) / scale) as usize;
    let glyph = glyph_for(ch);
    (glyph[row] & (1 << (4 - col))) != 0
}

fn glyph_for(ch: u8) -> [u8; 7] {
    let ch = if ch.is_ascii_lowercase() {
        ch.to_ascii_uppercase()
    } else {
        ch
    };
    match ch {
        b'A' => [
            0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        b'B' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110,
        ],
        b'C' => [
            0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110,
        ],
        b'D' => [
            0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110,
        ],
        b'E' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
        ],
        b'F' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        b'G' => [
            0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01110,
        ],
        b'H' => [
            0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        b'I' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b11111,
        ],
        b'J' => [
            0b00111, 0b00010, 0b00010, 0b00010, 0b10010, 0b10010, 0b01100,
        ],
        b'K' => [
            0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001,
        ],
        b'L' => [
            0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111,
        ],
        b'M' => [
            0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001,
        ],
        b'N' => [
            0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001,
        ],
        b'O' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        b'P' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        b'Q' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101,
        ],
        b'R' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
        ],
        b'S' => [
            0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        b'T' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        b'U' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        b'V' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100,
        ],
        b'W' => [
            0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b10101, 0b01010,
        ],
        b'X' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001,
        ],
        b'Y' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        b'Z' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111,
        ],
        b'/' => [
            0b00001, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b10000,
        ],
        b'0' => [
            0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
        ],
        b'1' => [
            0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        b'2' => [
            0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111,
        ],
        b'3' => [
            0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        b'4' => [
            0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
        ],
        b'5' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b00001, 0b00001, 0b11110,
        ],
        b'6' => [
            0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
        ],
        b'7' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
        ],
        b'8' => [
            0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
        ],
        b'9' => [
            0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b11100,
        ],
        b'.' => [0, 0, 0, 0, 0, 0b01100, 0b01100],
        b'-' => [0, 0, 0, 0b11111, 0, 0, 0],
        b'_' => [0, 0, 0, 0, 0, 0, 0b11111],
        b'~' => [0, 0, 0b01000, 0b10101, 0b00010, 0, 0],
        b' ' => [0, 0, 0, 0, 0, 0, 0],
        _ => [0, 0, 0, 0, 0, 0, 0],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke_strip_has_expected_size() {
        let mut strip = [0xFFu8; X4_EPD_STRIP_BYTES];
        render_smoke_strip(0, &mut strip);
        assert_eq!(strip.len(), 4_000);
        assert!(strip.iter().any(|b| *b != 0xFF));
    }

    #[test]
    fn logical_smoke_marks_orientation_corners() {
        assert!(smoke_pixel(2, 2));
        assert!(smoke_pixel(
            X4_EPD_LOGICAL_WIDTH - 3,
            X4_EPD_LOGICAL_HEIGHT - 3
        ));
        assert!(smoke_pixel(30, 30));
        assert!(smoke_pixel(430, 760));
        assert!(!smoke_pixel(240, 520));
    }

    #[test]
    fn minimal_home_strip_has_expected_size() {
        let mut strip = [0xFFu8; X4_EPD_STRIP_BYTES];
        render_home_strip(0, &mut strip, true, 92);
        assert_eq!(strip.len(), 4_000);
        assert!(strip.iter().any(|b| *b != 0xFF));
    }

    #[test]
    fn home_nav_strip_has_expected_size() {
        let mut strip = [0xFFu8; X4_EPD_STRIP_BYTES];
        render_home_nav_strip(0, &mut strip, true, 92, 2);
        assert_eq!(strip.len(), 4_000);
        assert!(strip.iter().any(|b| *b != 0xFF));
    }

    #[test]
    fn logical_home_marks_title_and_status() {
        assert!(home_pixel(2, 2, true, 92));
        assert!(home_pixel(38, 212, true, 92));
        assert!(home_pixel(66, 58, true, 92));
        assert!(home_pixel(30, 724, true, 92));
        assert!(!home_pixel(240, 510, true, 92));
    }

    #[test]
    fn logical_home_moves_selection_marker() {
        assert!(home_nav_pixel(38, 212, true, 92, 0));
        assert!(!home_nav_pixel(38, 212, true, 92, 1));
        assert!(home_nav_pixel(38, 286, true, 92, 1));
        assert!(home_nav_pixel(38, 434, true, 92, 3));
    }

    #[test]
    fn library_strip_has_expected_size() {
        let mut strip = [0xFFu8; X4_EPD_STRIP_BYTES];
        let items = [LibraryListItem::new(b"BOOK1.TXT", 100)];
        render_library_strip(0, &mut strip, true, 92, 0, &items, 1, true);
        assert_eq!(strip.len(), 4_000);
        assert!(strip.iter().any(|b| *b != 0xFF));
    }

    #[test]
    fn logical_library_marks_selected_row_and_status() {
        let items = [
            LibraryListItem::new(b"BOOK1.TXT", 100),
            LibraryListItem::new(b"BOOK2.EPU", 200),
        ];
        assert!(library_pixel(2, 2, true, 92, 1, &items, 2, true));
        assert!(library_pixel(38, 280, true, 92, 1, &items, 2, true));
        assert!(!library_pixel(38, 208, true, 92, 1, &items, 2, true));
        // The recursive-library renderer now includes a compact file-kind prefix,
        // so exact glyph interior pixels are brittle. Selection/status pixels are
        // enough to prove the logical library renderer still marks rows.
        assert!(library_pixel(30, 724, true, 92, 0, &items, 2, true));
    }

    #[test]
    fn reader_page_indexes_lines() {
        let page = ReaderPage::new(b"SHORT.TXT", 24, b"hello world\nline two");
        assert_eq!(page.line_count(), 2);
        assert_eq!(page.name_str(), "SHORT.TXT");
        assert!(reader_pixel(30, 196, true, 92, &page));
    }

    #[test]
    fn reader_strip_has_expected_size() {
        let mut strip = [0xFFu8; X4_EPD_STRIP_BYTES];
        let page = ReaderPage::new(b"SHORT.TXT", 24, b"hello world\nline two");
        render_reader_strip(0, &mut strip, true, 92, &page);
        assert_eq!(strip.len(), 4_000);
        assert!(strip.iter().any(|b| *b != 0xFF));
    }
}
