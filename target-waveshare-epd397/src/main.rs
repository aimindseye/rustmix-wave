use anyhow::Context;
use embedded_graphics::pixelcolor::BinaryColor;
use esp_idf_hal::{
    gpio::{AnyIOPin, PinDriver, Pull},
    peripherals::Peripherals,
    spi::{config::Config as SpiConfig, Dma, SpiDeviceDriver, SpiDriver, SpiDriverConfig},
    units::Hertz,
};

use hal_waveshare_epd397::{
    board,
    display::{DisplayBackendAdapter, ShellDisplayBridge},
    raw_marker,
    ui::render_rustmix_wave_home_navigation_smoke,
    reader_foundation::render_reader_foundation_flow_v0,
};

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

    raw_marker(b"RAW-RUSTMIX-WAVE-SHELL-UI-SMOKE-START\n\0");

    let backend = DisplayBackendAdapter::new(spi, dc, rst, busy);
    let mut shell_display = ShellDisplayBridge::new(backend);

    shell_display
        .init()
        .context("Rustmix-Wave shell display init failed")?;
    raw_marker(b"RAW-RUSTMIX-WAVE-SHELL-UI-INIT-OK\n\0");

    render_rustmix_wave_home_navigation_smoke(&mut shell_display)
        .context("Rustmix-Wave shell UI navigation smoke failed")?;

    raw_marker(b"RAW-RUSTMIX-WAVE-SHELL-UI-SMOKE-OK\n\0");

    raw_marker(b"RAW-RUSTMIX-WAVE-READER-FOUNDATION-DEMO-START\n\0");
    render_reader_foundation_flow_v0(&mut shell_display)
        .context("Rustmix-Wave reader foundation flow failed")?;
    raw_marker(b"RAW-RUSTMIX-WAVE-READER-FOUNDATION-DEMO-OK\n\0");
    loop {
        esp_idf_hal::delay::FreeRtos::delay_ms(1000);
    }
}
