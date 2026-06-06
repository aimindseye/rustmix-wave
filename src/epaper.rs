//! Waveshare 3.97-inch 800 × 480 monochrome e-paper driver.
//!
//! The command sequence and refresh policy are ported from the uploaded
//! Waveshare ESP-IDF reference. The driver intentionally has no app/UI logic.

use core::fmt::Debug;

use anyhow::{anyhow, bail, Result};
use embedded_hal::{
    delay::DelayNs,
    digital::{InputPin, OutputPin},
    spi::SpiBus,
};
use log::{debug, info};

use crate::{
    framebuffer::{FRAMEBUFFER_SIZE, HEIGHT, WIDTH},
    power::PanelPower,
};

const BUSY_POLL_MS: u32 = 20;
const BUSY_TIMEOUT_MS: u32 = 15_000;

/// Controller driver with explicit ownership of the panel bus and pins.
pub struct Epaper397<SPI, DC, RST, CS, BUSY, DELAY, POWER> {
    spi: SPI,
    dc: DC,
    reset: RST,
    cs: CS,
    busy: BUSY,
    delay: DELAY,
    power: POWER,
}

impl<SPI, DC, RST, CS, BUSY, DELAY, POWER> Epaper397<SPI, DC, RST, CS, BUSY, DELAY, POWER>
where
    SPI: SpiBus<u8>,
    SPI::Error: Debug,
    DC: OutputPin,
    DC::Error: Debug,
    RST: OutputPin,
    RST::Error: Debug,
    CS: OutputPin,
    CS::Error: Debug,
    BUSY: InputPin,
    BUSY::Error: Debug,
    DELAY: DelayNs,
    POWER: PanelPower,
{
    /// Construct the driver and match the reference firmware's idle pin state.
    pub fn new(
        spi: SPI,
        mut dc: DC,
        mut reset: RST,
        mut cs: CS,
        busy: BUSY,
        delay: DELAY,
        power: POWER,
    ) -> Result<Self> {
        dc.set_low()
            .map_err(|error| anyhow!("EPD_DC low failed: {error:?}"))?;
        reset
            .set_high()
            .map_err(|error| anyhow!("EPD_RST high failed: {error:?}"))?;
        // The uploaded reference keeps the panel selected and uses GPIO10 as
        // a manually-owned CS line. Preserve that behavior.
        cs.set_low()
            .map_err(|error| anyhow!("EPD_CS low failed: {error:?}"))?;

        Ok(Self {
            spi,
            dc,
            reset,
            cs,
            busy,
            delay,
            power,
        })
    }

    /// Power the panel and configure the controller for global refresh.
    pub fn initialize(&mut self) -> Result<()> {
        info!("epd397: enable ALDO3 and initialize panel");
        self.power.enable_panel_rail()?;
        self.delay.delay_ms(10);
        self.hardware_reset()?;
        self.wait_until_idle()?;

        self.command(0x12)?; // SWRESET
        self.wait_until_idle()?;

        self.command_data(0x18, &[0x80])?;
        self.command_data(0x0C, &[0xAE, 0xC7, 0xC3, 0xC0, 0x80])?;
        self.command_data(
            0x01,
            &[((HEIGHT - 1) & 0xFF) as u8, ((HEIGHT - 1) >> 8) as u8, 0x02],
        )?;
        self.command_data(0x3C, &[0x01])?;
        self.command_data(0x11, &[0x01])?;
        self.command_data(
            0x44,
            &[
                0x00,
                0x00,
                ((WIDTH - 1) & 0xFF) as u8,
                ((WIDTH - 1) >> 8) as u8,
            ],
        )?;
        self.command_data(
            0x45,
            &[
                ((HEIGHT - 1) & 0xFF) as u8,
                ((HEIGHT - 1) >> 8) as u8,
                0x00,
                0x00,
            ],
        )?;
        self.command_data(0x4E, &[0x00, 0x00])?;
        self.command_data(0x4F, &[0x00, 0x00])?;
        self.wait_until_idle()
    }

    /// Transfer a base frame to both controller RAM planes and run a global
    /// refresh. Use this at boot and periodically after partial refreshes.
    pub fn show_base(&mut self, frame: &[u8]) -> Result<()> {
        validate_frame(frame)?;
        info!("epd397: global base refresh");
        self.command(0x24)?;
        self.data(frame)?;
        self.command(0x26)?;
        self.data(frame)?;
        self.turn_on_display(0xF7)
    }

    /// Apply a full-screen partial refresh. This intentionally mirrors the
    /// vendor UI behavior while keeping the API narrow for the first milestone.
    pub fn show_partial_fullscreen(&mut self, frame: &[u8]) -> Result<()> {
        validate_frame(frame)?;
        info!("epd397: partial full-screen refresh");
        self.hardware_reset()?;
        self.command_data(0x18, &[0x80])?;
        self.command_data(0x3C, &[0x80])?;
        self.command_data(0x44, &[0x00, 0x00, 0x18, 0x03])?; // 0 .. 792
        self.command_data(0x45, &[0xDF, 0x01, 0x00, 0x00])?; // 479 .. 0
        self.command_data(0x4E, &[0x00, 0x00])?;
        self.command_data(0x4F, &[0x00, 0x00])?;
        self.command(0x24)?;
        self.data(frame)?;
        self.turn_on_display(0xFF)
    }

    /// Put the panel controller into deep sleep and disable its PMIC rail.
    pub fn sleep(&mut self) -> Result<()> {
        info!("epd397: deep sleep and disable ALDO3");
        self.command_data(0x10, &[0x01])?;
        self.delay.delay_ms(10);
        self.reset
            .set_low()
            .map_err(|error| anyhow!("EPD_RST low failed: {error:?}"))?;
        self.cs
            .set_low()
            .map_err(|error| anyhow!("EPD_CS low failed: {error:?}"))?;
        self.dc
            .set_low()
            .map_err(|error| anyhow!("EPD_DC low failed: {error:?}"))?;
        self.power.disable_panel_rail()?;
        self.delay.delay_ms(10);
        Ok(())
    }

    fn hardware_reset(&mut self) -> Result<()> {
        self.reset
            .set_high()
            .map_err(|error| anyhow!("EPD_RST high failed: {error:?}"))?;
        self.delay.delay_ms(50);
        self.reset
            .set_low()
            .map_err(|error| anyhow!("EPD_RST low failed: {error:?}"))?;
        self.delay.delay_ms(2);
        self.reset
            .set_high()
            .map_err(|error| anyhow!("EPD_RST high failed: {error:?}"))?;
        self.delay.delay_ms(50);
        Ok(())
    }

    fn turn_on_display(&mut self, refresh_control: u8) -> Result<()> {
        self.command_data(0x22, &[refresh_control])?;
        self.command(0x20)?;
        self.wait_until_idle()
    }

    fn wait_until_idle(&mut self) -> Result<()> {
        self.delay.delay_ms(100);
        let mut elapsed_ms = 100;
        while self
            .busy
            .is_high()
            .map_err(|error| anyhow!("EPD_BUSY read failed: {error:?}"))?
        {
            if elapsed_ms >= BUSY_TIMEOUT_MS {
                bail!("EPD_BUSY remained high for {BUSY_TIMEOUT_MS} ms");
            }
            self.delay.delay_ms(BUSY_POLL_MS);
            elapsed_ms += BUSY_POLL_MS;
        }
        debug!("epd397: busy released after {elapsed_ms} ms");
        Ok(())
    }

    fn command_data(&mut self, command: u8, data: &[u8]) -> Result<()> {
        self.command(command)?;
        self.data(data)
    }

    fn command(&mut self, command: u8) -> Result<()> {
        self.dc
            .set_low()
            .map_err(|error| anyhow!("EPD_DC command mode failed: {error:?}"))?;
        self.spi
            .write(&[command])
            .map_err(|error| anyhow!("EPD command 0x{command:02X} failed: {error:?}"))
    }

    fn data(&mut self, data: &[u8]) -> Result<()> {
        self.dc
            .set_high()
            .map_err(|error| anyhow!("EPD_DC data mode failed: {error:?}"))?;
        self.spi.write(data).map_err(|error| {
            anyhow!(
                "EPD data transfer of {} bytes failed: {error:?}",
                data.len()
            )
        })
    }
}

fn validate_frame(frame: &[u8]) -> Result<()> {
    if frame.len() != FRAMEBUFFER_SIZE {
        bail!(
            "invalid panel frame size: got {}, expected {FRAMEBUFFER_SIZE}",
            frame.len()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_frame;
    use crate::framebuffer::FRAMEBUFFER_SIZE;

    #[test]
    fn accepts_native_frame_size() {
        let frame = vec![0_u8; FRAMEBUFFER_SIZE];
        assert!(validate_frame(&frame).is_ok());
    }

    #[test]
    fn rejects_wrong_frame_size() {
        assert!(validate_frame(&[]).is_err());
        assert!(validate_frame(&[0_u8; 1]).is_err());
    }
}
