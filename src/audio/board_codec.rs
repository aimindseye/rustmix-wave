//! Waveshare-specific ES8311 codec-profile overlay.
//!
//! The generic `es8311` crate configures the clock tree and PCM interface.
//! The Waveshare BSP uses Espressif's `esp_codec_dev` profile for an additional
//! board-specific analogue-path phase.  In particular, register `0x44` is
//! routed to `0x58` so the DAC reference path is active before the external
//! speaker amplifier is allowed to turn on.

use anyhow::{anyhow, Result};
use embedded_hal::{delay::DelayNs, i2c::I2c};
use es8311::{ClockConfig, Es8311, Resolution};

use super::{
    BSP_ES8311_ADC_REG15, BSP_ES8311_ADC_REG17, BSP_ES8311_DAC_REFERENCE_REG44,
    BSP_ES8311_GPIO_IDLE_REG44, BSP_ES8311_GP_REG45, BSP_ES8311_SYSTEM_REG14,
};

const SYSTEM_REG0B: u8 = 0x0B;
const SYSTEM_REG0C: u8 = 0x0C;
const SYSTEM_REG0D: u8 = 0x0D;
const SYSTEM_REG0E: u8 = 0x0E;
const SYSTEM_REG10: u8 = 0x10;
const SYSTEM_REG11: u8 = 0x11;
const SYSTEM_REG12: u8 = 0x12;
const SYSTEM_REG13: u8 = 0x13;
const SYSTEM_REG14: u8 = 0x14;
const ADC_REG15: u8 = 0x15;
const ADC_REG16: u8 = 0x16;
const ADC_REG17: u8 = 0x17;
const ADC_REG1B: u8 = 0x1B;
const ADC_REG1C: u8 = 0x1C;
const DAC_REG37: u8 = 0x37;
const GPIO_REG44: u8 = 0x44;
const GP_REG45: u8 = 0x45;

/// Read-back values used to prove that the board-specific DAC profile was
/// applied instead of merely accepting successful I2C writes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CodecProfileSnapshot {
    pub gpio44: u8,
    pub system14: u8,
    pub adc15: u8,
    pub adc17: u8,
    pub gp45: u8,
}

impl CodecProfileSnapshot {
    fn validate(self) -> Result<Self> {
        let expected = CodecProfileSnapshot {
            gpio44: BSP_ES8311_DAC_REFERENCE_REG44,
            system14: BSP_ES8311_SYSTEM_REG14,
            adc15: BSP_ES8311_ADC_REG15,
            adc17: BSP_ES8311_ADC_REG17,
            gp45: BSP_ES8311_GP_REG45,
        };
        if self != expected {
            return Err(anyhow!(
                "ES8311 Waveshare profile read-back mismatch: expected {expected:?}, received {self:?}"
            ));
        }
        Ok(self)
    }
}

/// Compose the generic codec driver with the Waveshare-specific analogue-path
/// profile while keeping all I2C bus ownership in the existing audio runtime.
pub struct BoardEs8311 {
    codec: Es8311,
    address: u8,
}

impl BoardEs8311 {
    #[must_use]
    pub fn new(address: u8) -> Self {
        Self {
            codec: Es8311::new(address),
            address,
        }
    }

    pub fn init<I2C, D>(
        &self,
        i2c: &mut I2C,
        clock: &ClockConfig,
        resolution_in: Resolution,
        resolution_out: Resolution,
        delay: &mut D,
    ) -> Result<CodecProfileSnapshot>
    where
        I2C: I2c,
        I2C::Error: core::fmt::Debug,
        D: DelayNs,
    {
        // Match Espressif's reliable two-write preamble before the generic
        // clock and PCM-format setup runs.
        self.write_reg(i2c, GPIO_REG44, BSP_ES8311_GPIO_IDLE_REG44)?;
        self.write_reg(i2c, GPIO_REG44, BSP_ES8311_GPIO_IDLE_REG44)?;
        self.codec
            .init(i2c, clock, resolution_in, resolution_out, delay)
            .map_err(|error| anyhow!("generic ES8311 initialization failed: {error:?}"))?;

        self.apply_waveshare_profile(i2c)?;
        delay.delay_ms(10);
        self.read_profile(i2c)?.validate()
    }

    pub fn volume_set<I2C>(
        &self,
        i2c: &mut I2C,
        volume: u8,
        volume_set: Option<&mut u8>,
    ) -> Result<()>
    where
        I2C: I2c,
        I2C::Error: core::fmt::Debug,
    {
        self.codec
            .volume_set(i2c, volume, volume_set)
            .map_err(|error| anyhow!("ES8311 volume update failed: {error:?}"))
    }

    pub fn mute<I2C>(&self, i2c: &mut I2C, muted: bool) -> Result<()>
    where
        I2C: I2c,
        I2C::Error: core::fmt::Debug,
    {
        self.codec
            .mute(i2c, muted)
            .map_err(|error| anyhow!("ES8311 mute update failed: {error:?}"))
    }

    fn apply_waveshare_profile<I2C>(&self, i2c: &mut I2C) -> Result<()>
    where
        I2C: I2c,
        I2C::Error: core::fmt::Debug,
    {
        // Keep this table local and explicit.  It mirrors the additional
        // analogue and DAC-reference writes performed by Espressif's ES8311
        // open/start sequence used by the Waveshare sample application.
        for (register, value) in [
            (SYSTEM_REG0B, 0x00),
            (SYSTEM_REG0C, 0x00),
            (SYSTEM_REG10, 0x1F),
            (SYSTEM_REG11, 0x7F),
            (SYSTEM_REG13, 0x10),
            (ADC_REG16, 0x24),
            (ADC_REG1B, 0x0A),
            (ADC_REG1C, 0x6A),
            (GPIO_REG44, BSP_ES8311_DAC_REFERENCE_REG44),
            (ADC_REG17, BSP_ES8311_ADC_REG17),
            (SYSTEM_REG0E, 0x02),
            (SYSTEM_REG12, 0x00),
            (SYSTEM_REG14, BSP_ES8311_SYSTEM_REG14),
            (SYSTEM_REG0D, 0x01),
            (ADC_REG15, BSP_ES8311_ADC_REG15),
            (DAC_REG37, 0x08),
            (GP_REG45, BSP_ES8311_GP_REG45),
        ] {
            self.write_reg(i2c, register, value)?;
        }
        Ok(())
    }

    fn read_profile<I2C>(&self, i2c: &mut I2C) -> Result<CodecProfileSnapshot>
    where
        I2C: I2c,
        I2C::Error: core::fmt::Debug,
    {
        Ok(CodecProfileSnapshot {
            gpio44: self.read_reg(i2c, GPIO_REG44)?,
            system14: self.read_reg(i2c, SYSTEM_REG14)?,
            adc15: self.read_reg(i2c, ADC_REG15)?,
            adc17: self.read_reg(i2c, ADC_REG17)?,
            gp45: self.read_reg(i2c, GP_REG45)?,
        })
    }

    fn write_reg<I2C>(&self, i2c: &mut I2C, register: u8, value: u8) -> Result<()>
    where
        I2C: I2c,
        I2C::Error: core::fmt::Debug,
    {
        i2c.write(self.address, &[register, value])
            .map_err(|error| anyhow!("ES8311 register 0x{register:02X} write failed: {error:?}"))
    }

    fn read_reg<I2C>(&self, i2c: &mut I2C, register: u8) -> Result<u8>
    where
        I2C: I2c,
        I2C::Error: core::fmt::Debug,
    {
        let mut value = [0_u8];
        i2c.write_read(self.address, &[register], &mut value)
            .map_err(|error| anyhow!("ES8311 register 0x{register:02X} read failed: {error:?}"))?;
        Ok(value[0])
    }
}
