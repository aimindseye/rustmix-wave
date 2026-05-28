#!/usr/bin/env python3
from pathlib import Path
import re
import sys

ROOT = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else Path.cwd().resolve()

CARGO = ROOT / "Cargo.toml"
HAL = ROOT / "hal-waveshare-epd397"
TARGET = ROOT / "target-waveshare-epd397"
DOCS = ROOT / "docs" / "rustmix-wave"
SCRIPTS = ROOT / "scripts"

for path in [CARGO, HAL, TARGET, DOCS]:
    if not path.exists():
        raise SystemExit(f"missing bootstrap path: {path}")


def write_text(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)


def replace_marked_block(src: str, begin: str, end: str, block: str) -> str:
    if begin in src and end in src:
        a = src.find(begin)
        b = src.find(end, a)
        line_end = src.find("\n", b)
        if line_end < 0:
            line_end = len(src)
        else:
            line_end += 1
        return src[:a] + block + "\n" + src[line_end:]
    return src.rstrip() + "\n\n" + block + "\n"


def ensure_workspace_members(cargo_src: str, new_members: list[str]) -> str:
    if "[workspace]" not in cargo_src:
        members = ",\n    ".join(f'"{m}"' for m in new_members)
        return cargo_src.rstrip() + f'''

[workspace]
members = [
    {members},
]
resolver = "2"
'''

    pattern = re.compile(r"(?ms)^members\s*=\s*\[(.*?)\]")
    match = pattern.search(cargo_src)

    if not match:
        workspace_pos = cargo_src.find("[workspace]")
        next_section = cargo_src.find("\n[", workspace_pos + len("[workspace]"))
        insert_at = next_section if next_section >= 0 else len(cargo_src)
        members = ",\n    ".join(f'"{m}"' for m in new_members)
        block = f'''
members = [
    {members},
]
'''
        return cargo_src[:insert_at].rstrip() + "\n" + block + "\n" + cargo_src[insert_at:]

    body = match.group(1)
    existing = re.findall(r'"([^"]+)"', body)

    for member in new_members:
        if member not in existing:
            existing.append(member)

    members_block = "members = [\n" + "".join(f'    "{m}",\n' for m in existing) + "]"
    return cargo_src[:match.start()] + members_block + cargo_src[match.end():]


# ---------------------------------------------------------------------------
# Workspace wiring.
# ---------------------------------------------------------------------------
cargo_src = CARGO.read_text()
cargo_src = ensure_workspace_members(
    cargo_src,
    ["hal-waveshare-epd397", "target-waveshare-epd397"],
)
CARGO.write_text(cargo_src)


# ---------------------------------------------------------------------------
# HAL Cargo.toml.
# ---------------------------------------------------------------------------
write_text(
    HAL / "Cargo.toml",
    """[package]
name = "hal-waveshare-epd397"
version = "0.1.0"
edition = "2021"
description = "Waveshare ESP32-S3 e-Paper 3.97 hardware support for Rustmix-Wave"

[lib]
name = "hal_waveshare_epd397"
path = "src/lib.rs"

[dependencies]
anyhow = "1"
embedded-graphics = "0.8.1"
esp-idf-hal = "0.46.2"
esp-idf-sys = "0.36.1"
""",
)


# ---------------------------------------------------------------------------
# HAL implementation.
# ---------------------------------------------------------------------------
write_text(
    HAL / "src" / "lib.rs",
    r'''//! Waveshare ESP32-S3 e-Paper 3.97 HAL for Rustmix-Wave.
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
''',
)


# ---------------------------------------------------------------------------
# Target Cargo.toml and build.rs.
# ---------------------------------------------------------------------------
write_text(
    TARGET / "Cargo.toml",
    """[package]
name = "target-waveshare-epd397"
version = "0.1.0"
edition = "2021"
build = "build.rs"
description = "Rustmix-Wave Waveshare ESP32-S3 e-Paper 3.97 target"

[[bin]]
name = "target-waveshare-epd397"
path = "src/main.rs"

[dependencies]
anyhow = "1"
embedded-graphics = "0.8.1"
esp-idf-hal = "0.46.2"
esp-idf-sys = { version = "0.36.1", features = ["binstart"] }
hal-waveshare-epd397 = { path = "../hal-waveshare-epd397" }

[build-dependencies]
embuild = "0.33"
""",
)

write_text(
    TARGET / "build.rs",
    """fn main() {
    embuild::espidf::sysenv::output();
}
""",
)

write_text(
    TARGET / "sdkconfig.defaults",
    """CONFIG_ESPTOOLPY_FLASHSIZE_16MB=y
CONFIG_ESPTOOLPY_FLASHSIZE="16MB"
CONFIG_FREERTOS_HZ=1000
""",
)


# ---------------------------------------------------------------------------
# Target smoke binary.
# ---------------------------------------------------------------------------
write_text(
    TARGET / "src" / "main.rs",
    r'''use anyhow::Context;
use embedded_graphics::pixelcolor::BinaryColor;
use esp_idf_hal::{
    gpio::{AnyIOPin, PinDriver, Pull},
    peripherals::Peripherals,
    spi::{config::Config as SpiConfig, Dma, SpiDeviceDriver, SpiDriver, SpiDriverConfig},
    units::Hertz,
};

use hal_waveshare_epd397::{board, display::DisplayBackendAdapter, raw_marker};

fn main() {
    esp_idf_sys::link_patches();

    raw_marker(b"RAW-RUSTMIX-WAVE-MAIN-ENTER\n\0");

    if let Err(err) = try_main() {
        raw_marker(b"RAW-RUSTMIX-WAVE-BOOT-ERROR\n\0");
        println!("rustmix-wave boot error: {err:?}");

        loop {
            esp_idf_hal::delay::FreeRtos::delay_ms(1000);
        }
    }
}

fn try_main() -> anyhow::Result<()> {
    raw_marker(b"RAW-RUSTMIX-WAVE-TRY-MAIN-ENTER\n\0");

    let peripherals = Peripherals::take().context("peripherals take failed")?;
    let pins = peripherals.pins;
    raw_marker(b"RAW-RUSTMIX-WAVE-PERIPHERALS-OK\n\0");

    raw_marker(b"RAW-RUSTMIX-WAVE-DISPLAY-PINMAP-OK\n\0");

    let spi_driver = SpiDriver::new(
        peripherals.spi2,
        pins.gpio11,
        pins.gpio12,
        None::<AnyIOPin>,
        &SpiDriverConfig::new().dma(Dma::Auto(4096)),
    )
    .context("display SPI driver init failed")?;

    raw_marker(b"RAW-RUSTMIX-WAVE-DISPLAY-SPI-DRIVER-OK\n\0");

    let spi = SpiDeviceDriver::new(
        spi_driver,
        Some(pins.gpio10),
        &SpiConfig::new().baudrate(Hertz(500_000)),
    )
    .context("display SPI device init failed")?;

    raw_marker(b"RAW-RUSTMIX-WAVE-DISPLAY-SPI-DEVICE-OK\n\0");

    let dc = PinDriver::output(pins.gpio9).context("display DC pin init failed")?;
    let rst = PinDriver::output(pins.gpio46).context("display RST pin init failed")?;
    let busy = PinDriver::input(pins.gpio3, Pull::Floating).context("display BUSY pin init failed")?;

    raw_marker(b"RAW-RUSTMIX-WAVE-DISPLAY-PINS-OK\n\0");

    // Keep the accepted pin map visible in binary/log-review context.
    println!(
        "rustmix-wave display pins SCLK={} MOSI={} CS={} DC={} RST={} BUSY={}",
        board::EPD_SCLK,
        board::EPD_MOSI,
        board::EPD_CS,
        board::EPD_DC,
        board::EPD_RST,
        board::EPD_BUSY
    );

    raw_marker(b"RAW-RUSTMIX-WAVE-DISPLAY-SMOKE-START\n\0");

    let mut display = DisplayBackendAdapter::new(spi, dc, rst, busy);

    display.init().context("display backend init failed")?;
    raw_marker(b"RAW-RUSTMIX-WAVE-DISPLAY-SMOKE-INIT-OK\n\0");

    display
        .clear(BinaryColor::On)
        .context("display black clear failed")?;
    display.refresh().context("display black refresh failed")?;
    raw_marker(b"RAW-RUSTMIX-WAVE-DISPLAY-SMOKE-BLACK-OK\n\0");

    esp_idf_hal::delay::FreeRtos::delay_ms(2000);

    display
        .clear(BinaryColor::Off)
        .context("display white clear failed")?;
    display.refresh().context("display white refresh failed")?;
    raw_marker(b"RAW-RUSTMIX-WAVE-DISPLAY-SMOKE-WHITE-OK\n\0");

    raw_marker(b"RAW-RUSTMIX-WAVE-DISPLAY-SMOKE-OK\n\0");

    loop {
        esp_idf_hal::delay::FreeRtos::delay_ms(1000);
    }
}
''',
)


# ---------------------------------------------------------------------------
# Docs and README update.
# ---------------------------------------------------------------------------
write_text(
    DOCS / "display-backend-import-v0.md",
    """# Rustmix-Wave Display Backend Import v0

## Scope

This slice imports the accepted Focus Hub Waveshare display backend into
`hal-waveshare-epd397` and adds a minimal `target-waveshare-epd397` display
smoke binary.

## Accepted pin map

- EPD_SCLK GPIO11
- EPD_MOSI GPIO12
- EPD_CS GPIO10
- EPD_DC GPIO9
- EPD_RST GPIO46
- EPD_BUSY GPIO3

GPIO3 is EPD_BUSY and must not be used for input.

## Imported backend pieces

- `init_display_free`
- `clear_display_free`
- `write_frame_free`
- `refresh_display_free`
- `sleep_display_free`
- `DisplayBackendAdapter`
- `ShellDisplayBridge`

## What this slice does not do

- Does not port the Rustmix reader.
- Does not enable rotary input.
- Does not add audio/voice.
- Does not delete X4 code.

## Smoke test

Build:

```bash
source "$HOME/export-esp.sh"
cargo +esp build -p target-waveshare-epd397 --release --target xtensa-esp32s3-espidf
```

Flash:

```bash
espflash flash \\
  --chip esp32s3 \\
  --port "$PORT" \\
  --baud 921600 \\
  --monitor \\
  target/xtensa-esp32s3-espidf/release/target-waveshare-epd397 \\
  | rg 'RAW-|RUSTMIX-WAVE|DISPLAY|BUSY|panic|assertion|boot-error|rst:|Saved PC'
```

Expected physical result: clean black refresh, pause, clean white refresh.
""",
)

readme = ROOT / "README.md"
readme_src = readme.read_text()

readme_block = """<!-- BEGIN RUSTMIX_WAVE_DISPLAY_BACKEND_IMPORT_V0 -->
## Rustmix-Wave Display Backend Import v0

Rustmix-Wave now includes a Waveshare ESP32-S3 e-Paper 3.97 display backend
imported into `hal-waveshare-epd397`.

Accepted display pin map:

- EPD_SCLK GPIO11
- EPD_MOSI GPIO12
- EPD_CS GPIO10
- EPD_DC GPIO9
- EPD_RST GPIO46
- EPD_BUSY GPIO3

This slice adds:

- Free-function Waveshare display backend.
- `DisplayBackendAdapter`.
- `ShellDisplayBridge`.
- Minimal `target-waveshare-epd397` black/white display smoke.

This slice intentionally does not port the reader yet.
<!-- END RUSTMIX_WAVE_DISPLAY_BACKEND_IMPORT_V0 -->"""

readme_src = replace_marked_block(
    readme_src,
    "<!-- BEGIN RUSTMIX_WAVE_DISPLAY_BACKEND_IMPORT_V0 -->",
    "<!-- END RUSTMIX_WAVE_DISPLAY_BACKEND_IMPORT_V0 -->",
    readme_block,
)
readme.write_text(readme_src)


# ---------------------------------------------------------------------------
# Repo validator.
# ---------------------------------------------------------------------------
write_text(
    SCRIPTS / "validate_rustmix_wave_display_backend_import_v0.py",
    r'''#!/usr/bin/env python3
from pathlib import Path
import sys

ROOT = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else Path.cwd().resolve()

errors = []

def read(path: Path) -> str:
    try:
        return path.read_text()
    except FileNotFoundError:
        errors.append(f"missing file: {path.relative_to(ROOT)}")
        return ""

required_files = [
    "hal-waveshare-epd397/Cargo.toml",
    "hal-waveshare-epd397/src/lib.rs",
    "target-waveshare-epd397/Cargo.toml",
    "target-waveshare-epd397/build.rs",
    "target-waveshare-epd397/sdkconfig.defaults",
    "target-waveshare-epd397/src/main.rs",
    "docs/rustmix-wave/display-backend-import-v0.md",
    "README.md",
]

for file in required_files:
    if not (ROOT / file).is_file():
        errors.append(f"missing file: {file}")

cargo = read(ROOT / "Cargo.toml")
for member in ['"hal-waveshare-epd397"', '"target-waveshare-epd397"']:
    if member not in cargo:
        errors.append(f"workspace missing member: {member}")

hal = read(ROOT / "hal-waveshare-epd397/src/lib.rs")
target = read(ROOT / "target-waveshare-epd397/src/main.rs")
docs = read(ROOT / "docs/rustmix-wave/display-backend-import-v0.md")
readme = read(ROOT / "README.md")

required_hal = [
    "EPD_SCLK: i32 = 11",
    "EPD_MOSI: i32 = 12",
    "EPD_CS: i32 = 10",
    "EPD_DC: i32 = 9",
    "EPD_RST: i32 = 46",
    "EPD_BUSY: i32 = 3",
    "GPIO3 is EPD_BUSY",
    "pub fn init_display_free",
    "pub fn clear_display_free",
    "pub fn write_frame_free",
    "pub fn refresh_display_free",
    "pub fn sleep_display_free",
    "pub struct DisplayBackendAdapter",
    "pub struct ShellDisplayBridge",
    "SHELL_LOGICAL_WIDTH: usize = 480",
    "SHELL_LOGICAL_HEIGHT: usize = 800",
]

for item in required_hal:
    if item not in hal:
        errors.append(f"HAL missing {item}")

required_target = [
    "pins.gpio11",
    "pins.gpio12",
    "pins.gpio10",
    "pins.gpio9",
    "pins.gpio46",
    "pins.gpio3",
    "Pull::Floating",
    "DisplayBackendAdapter::new",
    "RAW-RUSTMIX-WAVE-DISPLAY-SMOKE-START",
    "RAW-RUSTMIX-WAVE-DISPLAY-SMOKE-BLACK-OK",
    "RAW-RUSTMIX-WAVE-DISPLAY-SMOKE-WHITE-OK",
    "RAW-RUSTMIX-WAVE-DISPLAY-SMOKE-OK",
]

for item in required_target:
    if item not in target:
        errors.append(f"target missing {item}")

combined_docs = docs + "\n" + readme
for item in [
    "Waveshare ESP32-S3 e-Paper 3.97",
    "DisplayBackendAdapter",
    "ShellDisplayBridge",
    "GPIO3",
    "Does not port the Rustmix reader",
]:
    if item.lower() not in combined_docs.lower():
        errors.append(f"docs/README missing {item}")

for forbidden in [
    "reader port",
    "Reader port complete",
    "InputRouter",
    "rotary input enabled",
]:
    if forbidden in target:
        errors.append(f"target should not include forbidden functionality: {forbidden}")

if errors:
    for error in errors:
        print("ERROR:", error)
    raise SystemExit(1)

print("rustmix-wave-display-backend-import-v0=ok")
''',
)

(SCRIPTS / "validate_rustmix_wave_display_backend_import_v0.py").chmod(0o755)

print("rustmix-wave-display-backend-import-v0-applied=ok")
