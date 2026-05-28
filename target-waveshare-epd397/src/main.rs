use anyhow::Context;
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
