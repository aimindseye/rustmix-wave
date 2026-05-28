//! Waveshare ESP32-S3 e-Paper 3.97 HAL for Rustmix-Wave.
//!
//! Display Backend Import v0 imports the accepted Focus Hub Waveshare display
//! backend path into Rustmix-Wave.
//!
//! Accepted display pin map:
//! - EPD_SCLK GPIO11
//! - EPD_MOSI GPIO12
//! - EPD_CS GPIO10
//! - EPD_DC GPIO9
//! - EPD_RST GPIO46
//! - EPD_BUSY GPIO3
//!
//! Important: GPIO3 is EPD_BUSY and must not be reused for rotary/input.

pub mod board {
    pub const TARGET_NAME: &str = "waveshare-esp32-s3-epaper-3.97";

    pub const DISPLAY_WIDTH_NATIVE: usize = 800;
    pub const DISPLAY_HEIGHT_NATIVE: usize = 480;
    pub const DISPLAY_WIDTH_PORTRAIT: usize = 480;
    pub const DISPLAY_HEIGHT_PORTRAIT: usize = 800;

    pub const EPD_SCLK: i32 = 11;
    pub const EPD_MOSI: i32 = 12;
    pub const EPD_CS: i32 = 10;
    pub const EPD_DC: i32 = 9;
    pub const EPD_RST: i32 = 46;
    pub const EPD_BUSY: i32 = 3;
}

pub fn raw_marker(msg: &'static [u8]) {
    unsafe {
        esp_idf_sys::esp_rom_printf(msg.as_ptr());
    }
}

pub mod display {
    use anyhow::{anyhow, Result};
    use embedded_graphics::pixelcolor::BinaryColor;
    use esp_idf_hal::delay::FreeRtos;
    use esp_idf_hal::gpio::{Input, Output, PinDriver};
    use esp_idf_hal::spi::{SpiDeviceDriver, SpiDriver};

    use crate::raw_marker;

    pub const EPD_WIDTH: usize = 800;
    pub const EPD_HEIGHT: usize = 480;
    pub const BYTES_PER_ROW: usize = EPD_WIDTH / 8;
    pub const FB_SIZE: usize = BYTES_PER_ROW * EPD_HEIGHT;

    const SHELL_LOGICAL_WIDTH: usize = 480;
    const SHELL_LOGICAL_HEIGHT: usize = 800;

    fn shell_logical_to_native(x: u32, y: u32) -> Option<(usize, usize)> {
        if x >= SHELL_LOGICAL_WIDTH as u32 || y >= SHELL_LOGICAL_HEIGHT as u32 {
            return None;
        }

        let native_x = y as usize;
        let native_y = SHELL_LOGICAL_WIDTH - 1 - x as usize;

        if native_x >= EPD_WIDTH || native_y >= EPD_HEIGHT {
            return None;
        }

        Some((native_x, native_y))
    }

    fn epd_cmd<'d>(
        spi: &mut SpiDeviceDriver<'d, SpiDriver<'d>>,
        dc: &mut PinDriver<'d, Output>,
        byte: u8,
    ) -> Result<()> {
        dc.set_low()?;
        spi.write(&[byte])
            .map_err(|_e| anyhow!("waveshare display command write failed"))?;
        Ok(())
    }

    fn epd_data<'d>(
        spi: &mut SpiDeviceDriver<'d, SpiDriver<'d>>,
        dc: &mut PinDriver<'d, Output>,
        bytes: &[u8],
    ) -> Result<()> {
        dc.set_high()?;
        spi.write(bytes)
            .map_err(|_e| anyhow!("waveshare display data write failed"))?;
        Ok(())
    }

    fn epd_wait_ready<'d>(
        busy: &PinDriver<'d, Input>,
        stage: &'static [u8],
        loops_max: u32,
    ) -> Result<()> {
        raw_marker(stage);

        // Mirrors the accepted Focus Hub / Waveshare 08-style wait behavior.
        FreeRtos::delay_ms(100);

        let mut loops = 0u32;
        while busy.is_high() {
            if loops >= loops_max {
                raw_marker(b"RAW-RUSTMIX-WAVE-DISPLAY-BUSY-TIMEOUT\n\0");
                return Ok(());
            }

            FreeRtos::delay_ms(20);
            loops += 1;
        }

        raw_marker(b"RAW-RUSTMIX-WAVE-DISPLAY-BUSY-READY\n\0");
        Ok(())
    }

    fn epd_set_ram_cursor<'d>(
        spi: &mut SpiDeviceDriver<'d, SpiDriver<'d>>,
        dc: &mut PinDriver<'d, Output>,
        busy: &PinDriver<'d, Input>,
    ) -> Result<()> {
        epd_cmd(spi, dc, 0x4E)?;
        epd_data(spi, dc, &[0x00, 0x00])?;

        epd_cmd(spi, dc, 0x4F)?;
        epd_data(spi, dc, &[0x00, 0x00])?;

        epd_wait_ready(busy, b"RAW-RUSTMIX-WAVE-DISPLAY-WAIT-CURSOR\n\0", 100)?;
        Ok(())
    }

    fn epd_write_ram_fill<'d>(
        spi: &mut SpiDeviceDriver<'d, SpiDriver<'d>>,
        dc: &mut PinDriver<'d, Output>,
        busy: &PinDriver<'d, Input>,
        ram_cmd: u8,
        fill_byte: u8,
    ) -> Result<()> {
        epd_set_ram_cursor(spi, dc, busy)?;

        epd_cmd(spi, dc, ram_cmd)?;
        dc.set_high()?;

        let fill = [fill_byte; 512];
        let mut remaining = FB_SIZE;

        while remaining > 0 {
            let n = core::cmp::min(remaining, fill.len());
            spi.write(&fill[..n])
                .map_err(|_e| anyhow!("waveshare display RAM fill write failed"))?;
            remaining -= n;
        }

        Ok(())
    }

    fn epd_write_ram_frame<'d>(
        spi: &mut SpiDeviceDriver<'d, SpiDriver<'d>>,
        dc: &mut PinDriver<'d, Output>,
        busy: &PinDriver<'d, Input>,
        ram_cmd: u8,
        frame: &[u8],
    ) -> Result<()> {
        if frame.len() != FB_SIZE {
            return Err(anyhow!("waveshare display frame length mismatch"));
        }

        epd_set_ram_cursor(spi, dc, busy)?;

        epd_cmd(spi, dc, ram_cmd)?;
        dc.set_high()?;

        for chunk in frame.chunks(512) {
            spi.write(chunk)
                .map_err(|_e| anyhow!("waveshare display RAM frame write failed"))?;
        }

        Ok(())
    }

    pub fn init_display_free<'d>(
        spi: &mut SpiDeviceDriver<'d, SpiDriver<'d>>,
        dc: &mut PinDriver<'d, Output>,
        rst: &mut PinDriver<'d, Output>,
        busy: &PinDriver<'d, Input>,
    ) -> Result<()> {
        raw_marker(b"RAW-RUSTMIX-WAVE-DISPLAY-INIT-START\n\0");

        rst.set_high()?;
        FreeRtos::delay_ms(50);
        rst.set_low()?;
        FreeRtos::delay_ms(2);
        rst.set_high()?;
        FreeRtos::delay_ms(50);

        raw_marker(b"RAW-RUSTMIX-WAVE-DISPLAY-RESET-OK\n\0");

        epd_wait_ready(busy, b"RAW-RUSTMIX-WAVE-DISPLAY-WAIT-RESET\n\0", 300)?;

        epd_cmd(spi, dc, 0x12)?;
        raw_marker(b"RAW-RUSTMIX-WAVE-DISPLAY-SWRESET-OK\n\0");
        epd_wait_ready(busy, b"RAW-RUSTMIX-WAVE-DISPLAY-WAIT-SWRESET\n\0", 300)?;

        // Native Waveshare 08-compatible 800x480 SSD1677 init sequence.
        epd_cmd(spi, dc, 0x18)?;
        epd_data(spi, dc, &[0x80])?;

        epd_cmd(spi, dc, 0x0C)?;
        epd_data(spi, dc, &[0xAE, 0xC7, 0xC3, 0xC0, 0x80])?;

        epd_cmd(spi, dc, 0x01)?;
        epd_data(spi, dc, &[0xDF, 0x01, 0x02])?;

        epd_cmd(spi, dc, 0x3C)?;
        epd_data(spi, dc, &[0x01])?;

        epd_cmd(spi, dc, 0x11)?;
        epd_data(spi, dc, &[0x01])?;

        epd_cmd(spi, dc, 0x44)?;
        epd_data(spi, dc, &[0x00, 0x00, 0x1F, 0x03])?;

        epd_cmd(spi, dc, 0x45)?;
        epd_data(spi, dc, &[0xDF, 0x01, 0x00, 0x00])?;

        epd_set_ram_cursor(spi, dc, busy)?;

        raw_marker(b"RAW-RUSTMIX-WAVE-DISPLAY-INIT-OK\n\0");
        Ok(())
    }

    pub fn write_frame_free<'d>(
        spi: &mut SpiDeviceDriver<'d, SpiDriver<'d>>,
        dc: &mut PinDriver<'d, Output>,
        busy: &PinDriver<'d, Input>,
        frame: &[u8],
    ) -> Result<()> {
        epd_write_ram_frame(spi, dc, busy, 0x24, frame)?;
        epd_write_ram_frame(spi, dc, busy, 0x26, frame)?;

        raw_marker(b"RAW-RUSTMIX-WAVE-DISPLAY-WRITE-FRAME-OK\n\0");
        Ok(())
    }

    pub fn clear_display_free<'d>(
        spi: &mut SpiDeviceDriver<'d, SpiDriver<'d>>,
        dc: &mut PinDriver<'d, Output>,
        busy: &PinDriver<'d, Input>,
        color: BinaryColor,
    ) -> Result<()> {
        let fill_byte = match color {
            BinaryColor::On => 0x00,
            BinaryColor::Off => 0xFF,
        };

        epd_write_ram_fill(spi, dc, busy, 0x24, fill_byte)?;
        raw_marker(b"RAW-RUSTMIX-WAVE-DISPLAY-RAM24-FILL-OK\n\0");

        epd_write_ram_fill(spi, dc, busy, 0x26, fill_byte)?;
        raw_marker(b"RAW-RUSTMIX-WAVE-DISPLAY-RAM26-FILL-OK\n\0");

        match color {
            BinaryColor::On => raw_marker(b"RAW-RUSTMIX-WAVE-DISPLAY-CLEAR-BLACK-OK\n\0"),
            BinaryColor::Off => raw_marker(b"RAW-RUSTMIX-WAVE-DISPLAY-CLEAR-WHITE-OK\n\0"),
        }

        Ok(())
    }

    pub fn refresh_display_free<'d>(
        spi: &mut SpiDeviceDriver<'d, SpiDriver<'d>>,
        dc: &mut PinDriver<'d, Output>,
        busy: &PinDriver<'d, Input>,
    ) -> Result<()> {
        raw_marker(b"RAW-RUSTMIX-WAVE-DISPLAY-REFRESH-START\n\0");

        epd_cmd(spi, dc, 0x22)?;
        epd_data(spi, dc, &[0xF7])?;
        epd_cmd(spi, dc, 0x20)?;

        epd_wait_ready(busy, b"RAW-RUSTMIX-WAVE-DISPLAY-WAIT-REFRESH\n\0", 1000)?;

        raw_marker(b"RAW-RUSTMIX-WAVE-DISPLAY-REFRESH-OK\n\0");
        Ok(())
    }

    pub fn sleep_display_free<'d>(
        spi: &mut SpiDeviceDriver<'d, SpiDriver<'d>>,
        dc: &mut PinDriver<'d, Output>,
    ) -> Result<()> {
        raw_marker(b"RAW-RUSTMIX-WAVE-DISPLAY-SLEEP-START\n\0");

        epd_cmd(spi, dc, 0x10)?;
        epd_data(spi, dc, &[0x01])?;

        FreeRtos::delay_ms(100);
        raw_marker(b"RAW-RUSTMIX-WAVE-DISPLAY-SLEEP-OK\n\0");
        Ok(())
    }

    pub struct DisplayBackendAdapter<'d> {
        spi: SpiDeviceDriver<'d, SpiDriver<'d>>,
        dc: PinDriver<'d, Output>,
        rst: PinDriver<'d, Output>,
        busy: PinDriver<'d, Input>,
    }

    impl<'d> DisplayBackendAdapter<'d> {
        pub fn new(
            spi: SpiDeviceDriver<'d, SpiDriver<'d>>,
            dc: PinDriver<'d, Output>,
            rst: PinDriver<'d, Output>,
            busy: PinDriver<'d, Input>,
        ) -> Self {
            raw_marker(b"RAW-RUSTMIX-WAVE-DISPLAY-BACKEND-NEW-ENTER\n\0");
            let adapter = Self { spi, dc, rst, busy };
            raw_marker(b"RAW-RUSTMIX-WAVE-DISPLAY-BACKEND-NEW-OK\n\0");
            adapter
        }

        pub fn init(&mut self) -> Result<()> {
            raw_marker(b"RAW-RUSTMIX-WAVE-DISPLAY-BACKEND-INIT-START\n\0");
            init_display_free(&mut self.spi, &mut self.dc, &mut self.rst, &self.busy)?;
            raw_marker(b"RAW-RUSTMIX-WAVE-DISPLAY-BACKEND-INIT-OK\n\0");
            Ok(())
        }

        pub fn clear(&mut self, color: BinaryColor) -> Result<()> {
            clear_display_free(&mut self.spi, &mut self.dc, &self.busy, color)
        }

        pub fn write_frame(&mut self, frame: &[u8]) -> Result<()> {
            raw_marker(b"RAW-RUSTMIX-WAVE-DISPLAY-BACKEND-WRITE-FRAME-START\n\0");
            write_frame_free(&mut self.spi, &mut self.dc, &self.busy, frame)?;
            raw_marker(b"RAW-RUSTMIX-WAVE-DISPLAY-BACKEND-WRITE-FRAME-OK\n\0");
            Ok(())
        }

        pub fn refresh(&mut self) -> Result<()> {
            raw_marker(b"RAW-RUSTMIX-WAVE-DISPLAY-BACKEND-REFRESH-START\n\0");
            refresh_display_free(&mut self.spi, &mut self.dc, &self.busy)?;
            raw_marker(b"RAW-RUSTMIX-WAVE-DISPLAY-BACKEND-REFRESH-OK\n\0");
            Ok(())
        }

        pub fn sleep(&mut self) -> Result<()> {
            raw_marker(b"RAW-RUSTMIX-WAVE-DISPLAY-BACKEND-SLEEP-START\n\0");
            sleep_display_free(&mut self.spi, &mut self.dc)?;
            raw_marker(b"RAW-RUSTMIX-WAVE-DISPLAY-BACKEND-SLEEP-OK\n\0");
            Ok(())
        }
    }

    pub struct ShellDisplayBridge<'d> {
        backend: DisplayBackendAdapter<'d>,
        fb: Vec<u8>,
    }

    impl<'d> ShellDisplayBridge<'d> {
        pub fn new(backend: DisplayBackendAdapter<'d>) -> Self {
            raw_marker(b"RAW-RUSTMIX-WAVE-SHELL-BRIDGE-NEW-ENTER\n\0");

            let bridge = Self {
                backend,
                fb: vec![0xFFu8; FB_SIZE],
            };

            raw_marker(b"RAW-RUSTMIX-WAVE-SHELL-BRIDGE-NEW-OK\n\0");
            bridge
        }

        pub fn init(&mut self) -> Result<()> {
            raw_marker(b"RAW-RUSTMIX-WAVE-SHELL-BRIDGE-INIT-START\n\0");
            self.backend.init()?;
            raw_marker(b"RAW-RUSTMIX-WAVE-SHELL-BRIDGE-INIT-OK\n\0");
            Ok(())
        }

        pub fn clear_fb(&mut self, color: BinaryColor) {
            let fill = match color {
                BinaryColor::On => 0x00u8,
                BinaryColor::Off => 0xFFu8,
            };

            self.fb.fill(fill);
        }

        pub fn set_pixel(&mut self, x: u32, y: u32, color: BinaryColor) {
            let (native_x, native_y) = match shell_logical_to_native(x, y) {
                Some(mapped) => mapped,
                None => return,
            };

            let byte_idx = native_y * BYTES_PER_ROW + native_x / 8;
            let bit = 7 - (native_x % 8);

            match color {
                BinaryColor::On => self.fb[byte_idx] &= !(1u8 << bit),
                BinaryColor::Off => self.fb[byte_idx] |= 1u8 << bit,
            }
        }

        pub fn fill_rect(&mut self, x: u32, y: u32, w: u32, h: u32, color: BinaryColor) {
            let x_end = core::cmp::min(x.saturating_add(w), SHELL_LOGICAL_WIDTH as u32);
            let y_end = core::cmp::min(y.saturating_add(h), SHELL_LOGICAL_HEIGHT as u32);

            let mut yy = y;
            while yy < y_end {
                let mut xx = x;
                while xx < x_end {
                    self.set_pixel(xx, yy, color);
                    xx += 1;
                }
                yy += 1;
            }
        }

        pub fn stroke_rect(
            &mut self,
            x: u32,
            y: u32,
            w: u32,
            h: u32,
            stroke: u32,
            color: BinaryColor,
        ) {
            if w == 0 || h == 0 || stroke == 0 {
                return;
            }

            self.fill_rect(x, y, w, stroke, color);
            self.fill_rect(x, y.saturating_add(h.saturating_sub(stroke)), w, stroke, color);
            self.fill_rect(x, y, stroke, h, color);
            self.fill_rect(x.saturating_add(w.saturating_sub(stroke)), y, stroke, h, color);
        }

        pub fn write_frame(&mut self, frame: &[u8]) -> Result<()> {
            self.backend.write_frame(frame)
        }

        pub fn flush(&mut self) -> Result<()> {
            raw_marker(b"RAW-RUSTMIX-WAVE-SHELL-BRIDGE-FLUSH-START\n\0");
            self.backend.write_frame(self.fb.as_slice())?;
            self.backend.refresh()?;
            raw_marker(b"RAW-RUSTMIX-WAVE-SHELL-BRIDGE-FLUSH-OK\n\0");
            Ok(())
        }

        pub fn refresh(&mut self) -> Result<()> {
            self.backend.refresh()
        }

        pub fn sleep(&mut self) -> Result<()> {
            self.backend.sleep()
        }
    }
}

pub mod input {
    //! Placeholder for safe rotary input mapping.
    //! GPIO3 is reserved for EPD_BUSY and must not be used as input.
}

pub mod audio {
    //! Placeholder for future voice/audio codec bring-up.
}

pub mod storage {
    //! Placeholder for TF/SD storage adaptation.
}

pub mod power {
    //! Placeholder for PMU/battery services.
}

pub mod rtc {
    //! Placeholder for RTC/time services.
}

pub mod sensors {
    //! Placeholder for environment and IMU sensors.
}

pub mod wifi {
    //! Placeholder for Wi-Fi transfer and assistant connectivity.
}

// BEGIN RUSTMIX_WAVE_SHELL_BRIDGE_UI_IMPORT_V0
pub mod ui {
    use anyhow::Result;
    use embedded_graphics::pixelcolor::BinaryColor;
    use esp_idf_hal::delay::FreeRtos;

    use crate::{display::ShellDisplayBridge, raw_marker};

    pub trait RustmixWaveHomeDisplaySurface {
        fn clear(&mut self, color: BinaryColor);
        fn fill_rect(&mut self, x: u32, y: u32, w: u32, h: u32, color: BinaryColor);
        fn stroke_rect(
            &mut self,
            x: u32,
            y: u32,
            w: u32,
            h: u32,
            stroke: u32,
            color: BinaryColor,
        );
        fn flush(&mut self) -> Result<()>;
    }

    impl<'d> RustmixWaveHomeDisplaySurface for ShellDisplayBridge<'d> {
        fn clear(&mut self, color: BinaryColor) {
            self.clear_fb(color);
        }

        fn fill_rect(&mut self, x: u32, y: u32, w: u32, h: u32, color: BinaryColor) {
            self.fill_rect(x, y, w, h, color);
        }

        fn stroke_rect(
            &mut self,
            x: u32,
            y: u32,
            w: u32,
            h: u32,
            stroke: u32,
            color: BinaryColor,
        ) {
            ShellDisplayBridge::stroke_rect(self, x, y, w, h, stroke, color);
        }

        fn flush(&mut self) -> Result<()> {
            self.flush()
        }
    }

    pub struct RustmixWaveHomeItem {
        pub label: &'static str,
        pub status: &'static str,
        pub detail_title: &'static str,
        pub detail_text: &'static str,
    }

    pub struct RustmixWaveHomeState {
        pub selected_index: usize,
        pub items: &'static [RustmixWaveHomeItem],
        pub footer_hint: &'static str,
        pub voice_status: &'static str,
    }

    const RUSTMIX_WAVE_HOME_ITEMS: [RustmixWaveHomeItem; 6] = [
        RustmixWaveHomeItem {
            label: "READER",
            status: "BOOKS",
            detail_title: "READER",
            detail_text: "OPEN BOOKS AND RECENT READS",
        },
        RustmixWaveHomeItem {
            label: "NETWORK",
            status: "WIFI",
            detail_title: "NETWORK",
            detail_text: "WIFI TRANSFER AND SYNC",
        },
        RustmixWaveHomeItem {
            label: "PRODUCT",
            status: "TOOLS",
            detail_title: "PRODUCTIVITY",
            detail_text: "CALENDAR NOTES AND TASKS",
        },
        RustmixWaveHomeItem {
            label: "VOICE",
            status: "HOLD",
            detail_title: "VOICE",
            detail_text: "HOLD DIAL TO TALK LATER",
        },
        RustmixWaveHomeItem {
            label: "TOOLS",
            status: "APPS",
            detail_title: "TOOLS",
            detail_text: "FLASHCARDS DICTIONARY APPS",
        },
        RustmixWaveHomeItem {
            label: "SYSTEM",
            status: "SETUP",
            detail_title: "SYSTEM",
            detail_text: "SETTINGS POWER AND STATUS",
        },
    ];

    impl RustmixWaveHomeState {
        pub fn new(selected_index: usize) -> Self {
            let max_index = RUSTMIX_WAVE_HOME_ITEMS.len().saturating_sub(1);

            Self {
                selected_index: core::cmp::min(selected_index, max_index),
                items: &RUSTMIX_WAVE_HOME_ITEMS,
                footer_hint: "ROTATE SELECT  PRESS OPEN  HOLD TALK",
                voice_status: "VOICE IDLE",
            }
        }

        pub fn selected_item(&self) -> &'static RustmixWaveHomeItem {
            &self.items[self.selected_index]
        }
    }

    fn marker_for_selection(index: usize) {
        match index {
            0 => raw_marker(b"RAW-RUSTMIX-WAVE-UI-SELECT-READER\n\0"),
            1 => raw_marker(b"RAW-RUSTMIX-WAVE-UI-SELECT-NETWORK\n\0"),
            2 => raw_marker(b"RAW-RUSTMIX-WAVE-UI-SELECT-PRODUCT\n\0"),
            3 => raw_marker(b"RAW-RUSTMIX-WAVE-UI-SELECT-VOICE\n\0"),
            4 => raw_marker(b"RAW-RUSTMIX-WAVE-UI-SELECT-TOOLS\n\0"),
            5 => raw_marker(b"RAW-RUSTMIX-WAVE-UI-SELECT-SYSTEM\n\0"),
            _ => raw_marker(b"RAW-RUSTMIX-WAVE-UI-SELECT-UNKNOWN\n\0"),
        }
    }

    fn glyph_5x7(ch: char) -> [u8; 7] {
        match ch {
            'A' => [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
            'B' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110],
            'C' => [0b01111, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b01111],
            'D' => [0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110],
            'E' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111],
            'F' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000],
            'G' => [0b01111, 0b10000, 0b10000, 0b10011, 0b10001, 0b10001, 0b01111],
            'H' => [0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
            'I' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b11111],
            'J' => [0b00111, 0b00010, 0b00010, 0b00010, 0b10010, 0b10010, 0b01100],
            'K' => [0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001],
            'L' => [0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111],
            'M' => [0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001],
            'N' => [0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001],
            'O' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
            'P' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000],
            'Q' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101],
            'R' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001],
            'S' => [0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110],
            'T' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100],
            'U' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
            'V' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100],
            'W' => [0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b10101, 0b01010],
            'X' => [0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001],
            'Y' => [0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100],
            'Z' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111],
            '0' => [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110],
            '1' => [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
            '2' => [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111],
            '3' => [0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110],
            '4' => [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010],
            '5' => [0b11111, 0b10000, 0b10000, 0b11110, 0b00001, 0b00001, 0b11110],
            '6' => [0b01110, 0b10000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110],
            '7' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000],
            '8' => [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110],
            '9' => [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00001, 0b01110],
            ':' => [0b00000, 0b00100, 0b00100, 0b00000, 0b00100, 0b00100, 0b00000],
            '-' => [0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000],
            '/' => [0b00001, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b10000],
            ' ' => [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000],
            _ => [0b11111, 0b10001, 0b00110, 0b00100, 0b00110, 0b10001, 0b11111],
        }
    }

    fn draw_char<D>(
        display: &mut D,
        x: u32,
        y: u32,
        scale: u32,
        ch: char,
        color: BinaryColor,
    )
    where
        D: RustmixWaveHomeDisplaySurface,
    {
        let glyph = glyph_5x7(ch);

        for (row, bits) in glyph.iter().enumerate() {
            for col in 0..5u32 {
                if (*bits & (1u8 << (4 - col))) != 0 {
                    display.fill_rect(
                        x + col * scale,
                        y + row as u32 * scale,
                        scale,
                        scale,
                        color,
                    );
                }
            }
        }
    }

    fn draw_text<D>(
        display: &mut D,
        mut x: u32,
        y: u32,
        scale: u32,
        text: &str,
        color: BinaryColor,
    )
    where
        D: RustmixWaveHomeDisplaySurface,
    {
        for ch in text.chars() {
            draw_char(display, x, y, scale, ch, color);
            x = x.saturating_add(6 * scale);
        }
    }

    fn draw_menu_row<D>(
        display: &mut D,
        index: usize,
        item: &RustmixWaveHomeItem,
        selected: bool,
    )
    where
        D: RustmixWaveHomeDisplaySurface,
    {
        let y = 96 + index as u32 * 74;

        if selected {
            display.stroke_rect(76, y - 8, 368, 64, 4, BinaryColor::On);
            display.fill_rect(90, y + 5, 12, 38, BinaryColor::On);
            display.fill_rect(112, y + 4, 124, 32, BinaryColor::On);
            draw_text(display, 126, y + 10, 3, item.label, BinaryColor::Off);
        } else {
            display.stroke_rect(88, y - 4, 344, 58, 2, BinaryColor::On);
            draw_text(display, 116, y + 10, 3, item.label, BinaryColor::On);
        }

        draw_text(display, 304, y + 18, 2, item.status, BinaryColor::On);
    }

    pub fn render_rustmix_wave_home_v0<D>(
        display: &mut D,
        state: &RustmixWaveHomeState,
    ) -> Result<()>
    where
        D: RustmixWaveHomeDisplaySurface,
    {
        raw_marker(b"RAW-RUSTMIX-WAVE-UI-RENDER-START\n\0");

        display.clear(BinaryColor::Off);

        display.fill_rect(0, 0, 480, 64, BinaryColor::On);
        draw_text(display, 20, 18, 3, "RUSTMIX WAVE", BinaryColor::Off);
        draw_text(display, 366, 18, 3, "14:15", BinaryColor::Off);

        draw_text(display, 80, 74, 2, "ROTARY HOME", BinaryColor::On);

        for (index, item) in state.items.iter().enumerate() {
            draw_menu_row(display, index, item, index == state.selected_index);
        }

        let selected = state.selected_item();

        display.stroke_rect(76, 558, 368, 126, 3, BinaryColor::On);
        draw_text(display, 102, 584, 3, selected.detail_title, BinaryColor::On);
        draw_text(display, 102, 626, 2, selected.detail_text, BinaryColor::On);
        draw_text(display, 102, 656, 2, state.voice_status, BinaryColor::On);

        display.fill_rect(76, 728, 368, 6, BinaryColor::On);
        draw_text(display, 34, 752, 2, state.footer_hint, BinaryColor::On);

        display.flush()?;

        raw_marker(b"RAW-RUSTMIX-WAVE-UI-RENDER-OK\n\0");

        Ok(())
    }

    pub fn render_rustmix_wave_home_navigation_smoke<D>(display: &mut D) -> Result<()>
    where
        D: RustmixWaveHomeDisplaySurface,
    {
        raw_marker(b"RAW-RUSTMIX-WAVE-SHELL-UI-V0-START\n\0");

        for selected_index in 0..RUSTMIX_WAVE_HOME_ITEMS.len() {
            marker_for_selection(selected_index);

            let state = RustmixWaveHomeState::new(selected_index);
            render_rustmix_wave_home_v0(display, &state)?;

            FreeRtos::delay_ms(1200);
        }

        raw_marker(b"RAW-RUSTMIX-WAVE-SHELL-UI-V0-OK\n\0");

        Ok(())
    }
}
// END RUSTMIX_WAVE_SHELL_BRIDGE_UI_IMPORT_V0

// BEGIN RUSTMIX_WAVE_READER_DISPLAY_SURFACE_BOUNDARY_V0
pub mod reader_display {
    use anyhow::Result;
    use embedded_graphics::pixelcolor::BinaryColor;

    use crate::{display::ShellDisplayBridge, raw_marker};

    /// Reader-facing display surface.
    ///
    /// Reader code must target this trait instead of directly owning SPI pins,
    /// DisplayBackendAdapter, or native display RAM orientation.
    pub trait ReaderDisplaySurface {
        fn logical_width(&self) -> u32;
        fn logical_height(&self) -> u32;
        fn clear(&mut self);
        fn fill_rect(&mut self, x: u32, y: u32, w: u32, h: u32, black: bool);
        fn draw_mono_bitmap(&mut self, x: u32, y: u32, w: u32, h: u32, data: &[u8]);
        fn flush(&mut self) -> Result<()>;
    }

    impl<'d> ReaderDisplaySurface for ShellDisplayBridge<'d> {
        fn logical_width(&self) -> u32 {
            480
        }

        fn logical_height(&self) -> u32 {
            800
        }

        fn clear(&mut self) {
            ShellDisplayBridge::clear_fb(self, BinaryColor::Off);
        }

        fn fill_rect(&mut self, x: u32, y: u32, w: u32, h: u32, black: bool) {
            let color = if black {
                BinaryColor::On
            } else {
                BinaryColor::Off
            };

            ShellDisplayBridge::fill_rect(self, x, y, w, h, color);
        }

        fn draw_mono_bitmap(&mut self, x: u32, y: u32, w: u32, h: u32, data: &[u8]) {
            let bytes_per_row = ((w as usize) + 7) / 8;

            let mut yy = 0u32;
            while yy < h {
                let mut xx = 0u32;
                while xx < w {
                    let byte_idx = yy as usize * bytes_per_row + xx as usize / 8;
                    if byte_idx >= data.len() {
                        return;
                    }

                    let mask = 1u8 << (7 - (xx % 8));
                    if (data[byte_idx] & mask) != 0 {
                        ShellDisplayBridge::set_pixel(self, x + xx, y + yy, BinaryColor::On);
                    }

                    xx += 1;
                }

                yy += 1;
            }
        }

        fn flush(&mut self) -> Result<()> {
            ShellDisplayBridge::flush(self)
        }
    }

    fn glyph_5x7(ch: char) -> [u8; 7] {
        match ch {
            'A' => [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
            'B' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110],
            'C' => [0b01111, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b01111],
            'D' => [0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110],
            'E' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111],
            'F' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000],
            'G' => [0b01111, 0b10000, 0b10000, 0b10011, 0b10001, 0b10001, 0b01111],
            'H' => [0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
            'I' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b11111],
            'J' => [0b00111, 0b00010, 0b00010, 0b00010, 0b10010, 0b10010, 0b01100],
            'K' => [0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001],
            'L' => [0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111],
            'M' => [0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001],
            'N' => [0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001],
            'O' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
            'P' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000],
            'Q' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101],
            'R' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001],
            'S' => [0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110],
            'T' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100],
            'U' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
            'V' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100],
            'W' => [0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b10101, 0b01010],
            'X' => [0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001],
            'Y' => [0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100],
            'Z' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111],
            '0' => [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110],
            '1' => [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
            '2' => [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111],
            '3' => [0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110],
            '4' => [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010],
            '5' => [0b11111, 0b10000, 0b10000, 0b11110, 0b00001, 0b00001, 0b11110],
            '6' => [0b01110, 0b10000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110],
            '7' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000],
            '8' => [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110],
            '9' => [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00001, 0b01110],
            ':' => [0b00000, 0b00100, 0b00100, 0b00000, 0b00100, 0b00100, 0b00000],
            '-' => [0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000],
            '/' => [0b00001, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b10000],
            ' ' => [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000],
            _ => [0b11111, 0b10001, 0b00110, 0b00100, 0b00110, 0b10001, 0b11111],
        }
    }

    fn draw_char<D: ReaderDisplaySurface>(
        display: &mut D,
        x: u32,
        y: u32,
        scale: u32,
        ch: char,
        black: bool,
    ) {
        let glyph = glyph_5x7(ch);

        for (row, bits) in glyph.iter().enumerate() {
            for col in 0..5u32 {
                if (*bits & (1u8 << (4 - col))) != 0 {
                    display.fill_rect(
                        x + col * scale,
                        y + row as u32 * scale,
                        scale,
                        scale,
                        black,
                    );
                }
            }
        }
    }

    fn draw_text<D: ReaderDisplaySurface>(
        display: &mut D,
        mut x: u32,
        y: u32,
        scale: u32,
        text: &str,
        black: bool,
    ) {
        for ch in text.chars() {
            draw_char(display, x, y, scale, ch, black);
            x = x.saturating_add(6 * scale);
        }
    }

    pub fn render_reader_display_surface_placeholder_v0<D>(
        display: &mut D,
    ) -> Result<()>
    where
        D: ReaderDisplaySurface,
    {
        raw_marker(b"RAW-RUSTMIX-WAVE-READER-BOUNDARY-V0-START\n\0");

        let width = display.logical_width();
        let height = display.logical_height();

        display.clear();

        // Header.
        display.fill_rect(0, 0, width, 64, true);
        draw_text(display, 20, 18, 3, "READER BOUNDARY", false);

        // Static reader page placeholder.
        display.fill_rect(38, 88, width.saturating_sub(76), height.saturating_sub(176), false);
        display.fill_rect(38, 88, width.saturating_sub(76), 3, true);
        display.fill_rect(38, height.saturating_sub(91), width.saturating_sub(76), 3, true);
        display.fill_rect(38, 88, 3, height.saturating_sub(176), true);
        display.fill_rect(width.saturating_sub(41), 88, 3, height.saturating_sub(176), true);

        // Small 16x16 marker rendered through draw_mono_bitmap.
        const BOOK_ICON: [u8; 32] = [
            0b11111111, 0b11111110,
            0b10000000, 0b00000110,
            0b10111111, 0b11110110,
            0b10100000, 0b00010110,
            0b10101111, 0b11010110,
            0b10101000, 0b01010110,
            0b10101011, 0b01010110,
            0b10101010, 0b01010110,
            0b10101010, 0b01010110,
            0b10101011, 0b01010110,
            0b10101000, 0b01010110,
            0b10101111, 0b11010110,
            0b10100000, 0b00010110,
            0b10111111, 0b11110110,
            0b10000000, 0b00000110,
            0b11111111, 0b11111110,
        ];
        display.draw_mono_bitmap(60, 114, 16, 16, &BOOK_ICON);

        draw_text(display, 92, 112, 3, "DISPLAY SURFACE", true);
        draw_text(display, 60, 170, 3, "STATIC PAGE ONLY", true);
        draw_text(display, 60, 226, 2, "NO STORAGE YET", true);
        draw_text(display, 60, 256, 2, "NO READER PORT YET", true);
        draw_text(display, 60, 286, 2, "NO REAL INPUT YET", true);
        draw_text(display, 60, 316, 2, "GPIO3 EPD BUSY", true);
        draw_text(display, 60, 376, 2, "SURFACE FLUSH VIA SHELL BRIDGE", true);
        draw_text(display, 60, 406, 2, "FULL REFRESH ONLY", true);

        // Footer.
        display.fill_rect(0, height.saturating_sub(58), width, 58, true);
        draw_text(display, 22, height.saturating_sub(38), 2, "READERDISPLAY -> SHELLDISPLAY", false);

        display.flush()?;

        raw_marker(b"RAW-RUSTMIX-WAVE-READER-DISPLAY-PLACEHOLDER-OK\n\0");
        raw_marker(b"RAW-RUSTMIX-WAVE-READER-BOUNDARY-V0-OK\n\0");

        Ok(())
    }
}
// END RUSTMIX_WAVE_READER_DISPLAY_SURFACE_BOUNDARY_V0

// BEGIN RUSTMIX_WAVE_READER_FOUNDATION_V0
pub mod reader_foundation {
    use anyhow::{anyhow, Result};
    use esp_idf_hal::delay::FreeRtos;

    use crate::{raw_marker, reader_display::ReaderDisplaySurface};

    pub struct ReaderBook {
        pub id: &'static str,
        pub title: &'static str,
        pub path: &'static str,
    }

    pub trait ReaderStorage {
        fn list_books(&mut self) -> Result<&'static [ReaderBook]>;
        fn read_file_chunk(
            &mut self,
            path: &str,
            offset: usize,
            buf: &mut [u8],
        ) -> Result<usize>;
        fn read_state_file(&mut self, path: &str, buf: &mut [u8]) -> Result<usize>;
        fn write_state_file(&mut self, path: &str, data: &[u8]) -> Result<()>;
    }

    static MOCK_BOOKS: [ReaderBook; 1] = [ReaderBook {
        id: "SAMPLE",
        title: "SAMPLE READER",
        path: "/MOCK/SAMPLE.TXT",
    }];

    const MOCK_SAMPLE_TEXT: &str = "\
RUSTMIX WAVE READER FOUNDATION. THIS IS A MOCK BOOK USED TO PROVE THE READER DISPLAY SURFACE AND STORAGE CONTRACT BEFORE REAL SD CARD READING. \
THE DISPLAY PATH IS READERDISPLAY SURFACE TO SHELLDISPLAYBRIDGE TO DISPLAYBACKENDADAPTER. \
THE FIRST PAGE SHOWS A HEADER BODY TEXT FOOTER PAGE NUMBER AND PROGRESS BAR. \
THE SECOND PAGE IS A SIMULATED NEXT PAGE. THE THIRD RENDER RETURNS TO THE PREVIOUS PAGE. \
NO EPUB IS PORTED IN THIS SLICE. NO REAL ROTARY INPUT IS ENABLED. NO BOOKMARKS OR PROGRESS ARE WRITTEN TO STORAGE YET. \
GPIO3 REMAINS EPD BUSY AND IS NOT USED FOR INPUT. \
THIS FOUNDATION LETS THE NEXT SLICE REPLACE MOCK STORAGE WITH REAL SD TXT READING WITHOUT CHANGING THE DISPLAY BOUNDARY.";

    pub struct MockReaderStorage;

    impl MockReaderStorage {
        pub fn new() -> Self {
            Self
        }
    }

    impl ReaderStorage for MockReaderStorage {
        fn list_books(&mut self) -> Result<&'static [ReaderBook]> {
            raw_marker(b"RAW-RUSTMIX-WAVE-READER-MOCK-STORAGE-OK\n\0");
            Ok(&MOCK_BOOKS)
        }

        fn read_file_chunk(
            &mut self,
            path: &str,
            offset: usize,
            buf: &mut [u8],
        ) -> Result<usize> {
            if path != MOCK_BOOKS[0].path {
                return Err(anyhow!("mock reader storage unknown path"));
            }

            let bytes = MOCK_SAMPLE_TEXT.as_bytes();
            if offset >= bytes.len() {
                return Ok(0);
            }

            let available = bytes.len() - offset;
            let n = core::cmp::min(available, buf.len());
            buf[..n].copy_from_slice(&bytes[offset..offset + n]);
            Ok(n)
        }

        fn read_state_file(&mut self, _path: &str, _buf: &mut [u8]) -> Result<usize> {
            Ok(0)
        }

        fn write_state_file(&mut self, _path: &str, _data: &[u8]) -> Result<()> {
            Ok(())
        }
    }

    pub struct ReaderScreenState {
        pub selected_book_index: usize,
        pub page_index: usize,
        pub total_pages_placeholder: usize,
    }

    impl ReaderScreenState {
        pub fn new(selected_book_index: usize, page_index: usize) -> Self {
            Self {
                selected_book_index,
                page_index,
                total_pages_placeholder: 3,
            }
        }

        pub fn next_page(&mut self) {
            let max_page = self.total_pages_placeholder.saturating_sub(1);
            self.page_index = core::cmp::min(self.page_index.saturating_add(1), max_page);
        }

        pub fn previous_page(&mut self) {
            self.page_index = self.page_index.saturating_sub(1);
        }
    }

    fn glyph_5x7(ch: char) -> [u8; 7] {
        match ch {
            'A' => [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
            'B' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110],
            'C' => [0b01111, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b01111],
            'D' => [0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110],
            'E' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111],
            'F' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000],
            'G' => [0b01111, 0b10000, 0b10000, 0b10011, 0b10001, 0b10001, 0b01111],
            'H' => [0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
            'I' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b11111],
            'J' => [0b00111, 0b00010, 0b00010, 0b00010, 0b10010, 0b10010, 0b01100],
            'K' => [0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001],
            'L' => [0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111],
            'M' => [0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001],
            'N' => [0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001],
            'O' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
            'P' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000],
            'Q' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101],
            'R' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001],
            'S' => [0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110],
            'T' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100],
            'U' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
            'V' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100],
            'W' => [0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b10101, 0b01010],
            'X' => [0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001],
            'Y' => [0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100],
            'Z' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111],
            '0' => [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110],
            '1' => [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
            '2' => [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111],
            '3' => [0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110],
            '4' => [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010],
            '5' => [0b11111, 0b10000, 0b10000, 0b11110, 0b00001, 0b00001, 0b11110],
            '6' => [0b01110, 0b10000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110],
            '7' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000],
            '8' => [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110],
            '9' => [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00001, 0b01110],
            ':' => [0b00000, 0b00100, 0b00100, 0b00000, 0b00100, 0b00100, 0b00000],
            '-' => [0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000],
            '/' => [0b00001, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b10000],
            ' ' => [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000],
            _ => [0b11111, 0b10001, 0b00110, 0b00100, 0b00110, 0b10001, 0b11111],
        }
    }

    fn draw_char<D: ReaderDisplaySurface>(
        display: &mut D,
        x: u32,
        y: u32,
        scale: u32,
        ch: char,
        black: bool,
    ) {
        let glyph = glyph_5x7(ch);

        for (row, bits) in glyph.iter().enumerate() {
            for col in 0..5u32 {
                if (*bits & (1u8 << (4 - col))) != 0 {
                    display.fill_rect(
                        x + col * scale,
                        y + row as u32 * scale,
                        scale,
                        scale,
                        black,
                    );
                }
            }
        }
    }

    fn draw_text<D: ReaderDisplaySurface>(
        display: &mut D,
        mut x: u32,
        y: u32,
        scale: u32,
        text: &str,
        black: bool,
    ) {
        for ch in text.chars() {
            draw_char(display, x, y, scale, ch, black);
            x = x.saturating_add(6 * scale);
        }
    }

    fn draw_wrapped_text<D: ReaderDisplaySurface>(
        display: &mut D,
        x: u32,
        mut y: u32,
        max_width: u32,
        max_y: u32,
        scale: u32,
        text: &str,
    ) {
        let char_w = 6 * scale;
        let line_h = 10 * scale;
        let max_cols = core::cmp::max(1, (max_width / char_w) as usize);

        let mut line = [b' '; 42];
        let mut len = 0usize;

        for byte in text.bytes() {
            let b = if byte.is_ascii_lowercase() {
                byte.to_ascii_uppercase()
            } else if byte.is_ascii_graphic() || byte == b' ' {
                byte
            } else {
                b' '
            };

            line[len] = b;
            len += 1;

            if len >= max_cols || len >= line.len() || b == b'.' {
                if y + line_h > max_y {
                    return;
                }

                let s = core::str::from_utf8(&line[..len]).unwrap_or("");
                draw_text(display, x, y, scale, s, true);
                y = y.saturating_add(line_h);
                len = 0;
            }
        }

        if len > 0 && y + line_h <= max_y {
            let s = core::str::from_utf8(&line[..len]).unwrap_or("");
            draw_text(display, x, y, scale, s, true);
        }
    }

    fn draw_page_label<D: ReaderDisplaySurface>(display: &mut D, page_index: usize) {
        match page_index {
            0 => draw_text(display, 360, 744, 2, "PAGE 1 / 3", false),
            1 => draw_text(display, 360, 744, 2, "PAGE 2 / 3", false),
            _ => draw_text(display, 360, 744, 2, "PAGE 3 / 3", false),
        }
    }

    pub fn render_reader_page_v0<D, S>(
        display: &mut D,
        storage: &mut S,
        state: &ReaderScreenState,
    ) -> Result<()>
    where
        D: ReaderDisplaySurface,
        S: ReaderStorage,
    {
        raw_marker(b"RAW-RUSTMIX-WAVE-READER-PAGE-RENDER-START\n\0");

        let books = storage.list_books()?;
        if state.selected_book_index >= books.len() {
            return Err(anyhow!("mock reader selected book out of range"));
        }

        let book = &books[state.selected_book_index];
        let mut buf = [0u8; 768];
        let offset = state.page_index.saturating_mul(300);
        let n = storage.read_file_chunk(book.path, offset, &mut buf)?;
        let text = core::str::from_utf8(&buf[..n]).unwrap_or("");

        let width = display.logical_width();
        let height = display.logical_height();

        display.clear();

        display.fill_rect(0, 0, width, 64, true);
        draw_text(display, 18, 18, 3, book.title, false);
        draw_page_label(display, state.page_index);

        display.fill_rect(32, 88, width.saturating_sub(64), 3, true);
        display.fill_rect(32, height.saturating_sub(104), width.saturating_sub(64), 3, true);
        display.fill_rect(32, 88, 3, height.saturating_sub(192), true);
        display.fill_rect(width.saturating_sub(35), 88, 3, height.saturating_sub(192), true);

        draw_wrapped_text(display, 56, 124, width.saturating_sub(112), 640, 2, text);

        display.fill_rect(0, height.saturating_sub(64), width, 64, true);

        let progress_w = match state.page_index {
            0 => 120,
            1 => 240,
            _ => 360,
        };
        display.fill_rect(24, height.saturating_sub(38), 360, 8, false);
        display.fill_rect(24, height.saturating_sub(38), progress_w, 8, true);

        match state.page_index {
            0 => draw_text(display, 24, height.saturating_sub(58), 2, "MOCK FIRST PAGE", false),
            1 => draw_text(display, 24, height.saturating_sub(58), 2, "MOCK NEXT PAGE", false),
            _ => draw_text(display, 24, height.saturating_sub(58), 2, "MOCK PREV PAGE", false),
        }

        display.flush()?;

        raw_marker(b"RAW-RUSTMIX-WAVE-READER-PAGE-RENDER-OK\n\0");

        Ok(())
    }

    pub fn render_reader_foundation_flow_v0<D>(display: &mut D) -> Result<()>
    where
        D: ReaderDisplaySurface,
    {
        raw_marker(b"RAW-RUSTMIX-WAVE-READER-FOUNDATION-V0-START\n\0");

        let mut storage = MockReaderStorage::new();
        let mut state = ReaderScreenState::new(0, 0);

        render_reader_page_v0(display, &mut storage, &state)?;
        raw_marker(b"RAW-RUSTMIX-WAVE-READER-MOCK-FIRST-PAGE-OK\n\0");

        FreeRtos::delay_ms(1400);

        state.next_page();
        render_reader_page_v0(display, &mut storage, &state)?;
        raw_marker(b"RAW-RUSTMIX-WAVE-READER-MOCK-NAV-NEXT-OK\n\0");

        FreeRtos::delay_ms(1400);

        state.previous_page();
        render_reader_page_v0(display, &mut storage, &state)?;
        raw_marker(b"RAW-RUSTMIX-WAVE-READER-MOCK-NAV-PREV-OK\n\0");

        raw_marker(b"RAW-RUSTMIX-WAVE-READER-FOUNDATION-V0-OK\n\0");

        Ok(())
    }
}
// END RUSTMIX_WAVE_READER_FOUNDATION_V0
