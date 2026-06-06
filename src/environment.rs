//! SHTC3 temperature and humidity support ported from the Waveshare sample.

use core::fmt::Debug;

use anyhow::{anyhow, bail, Result};
use embedded_hal::{delay::DelayNs, i2c::I2c};

use crate::regional::TemperatureUnit;

/// SHTC3 7-bit I2C address used by the sample firmware.
pub const SHTC3_ADDRESS: u8 = 0x70;
const READ_ID: [u8; 2] = [0xEF, 0xC8];
const SOFT_RESET: [u8; 2] = [0x80, 0x5D];
const SLEEP: [u8; 2] = [0xB0, 0x98];
const WAKEUP: [u8; 2] = [0x35, 0x17];
const MEASURE_T_RH_POLLING: [u8; 2] = [0x78, 0x66];
const CRC_POLYNOMIAL: u8 = 0x31;
/// The sample compensates its onboard reading downward by 1.5 °C.
const SAMPLE_TEMPERATURE_OFFSET_TENTHS_C: i16 = -15;

/// Fixed-point environmental snapshot suitable for host tests and UI display.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct EnvironmentReading {
    /// Temperature in tenths of a degree Celsius.
    pub temperature_tenths_c: i16,
    /// Relative humidity in tenths of a percent.
    pub humidity_tenths_percent: u16,
}

impl EnvironmentReading {
    #[must_use]
    pub fn temperature_tenths_f(self) -> i16 {
        rounded_div(i32::from(self.temperature_tenths_c) * 9, 5) as i16 + 320
    }

    #[must_use]
    pub fn temperature_label(self, unit: TemperatureUnit) -> String {
        match unit {
            TemperatureUnit::Celsius => {
                format_signed_tenths(self.temperature_tenths_c, unit.suffix())
            }
            TemperatureUnit::Fahrenheit => {
                format_signed_tenths(self.temperature_tenths_f(), unit.suffix())
            }
        }
    }

    #[must_use]
    pub fn humidity_label(self) -> String {
        format!(
            "{}.{}%",
            self.humidity_tenths_percent / 10,
            self.humidity_tenths_percent % 10
        )
    }
}

/// Narrow command-level SHTC3 driver.
pub struct Shtc3<I2C> {
    i2c: I2C,
}

impl<I2C> Shtc3<I2C> {
    #[must_use]
    pub fn new(i2c: I2C) -> Self {
        Self { i2c }
    }
}

impl<I2C> Shtc3<I2C>
where
    I2C: I2c,
    I2C::Error: Debug,
{
    /// Wake, reset, verify the sensor ID CRC, and return the raw ID.
    pub fn initialize<D: DelayNs>(&mut self, delay: &mut D) -> Result<u16> {
        self.write_command(WAKEUP)?;
        delay.delay_us(300);
        self.write_command(SOFT_RESET)?;
        delay.delay_us(300);
        let id = self.read_id()?;
        self.write_command(SLEEP)?;
        Ok(id)
    }

    /// Capture one compensated onboard temperature and humidity reading.
    pub fn read_environment<D: DelayNs>(&mut self, delay: &mut D) -> Result<EnvironmentReading> {
        self.write_command(WAKEUP)?;
        delay.delay_us(300);
        self.write_command(MEASURE_T_RH_POLLING)?;
        delay.delay_ms(20);

        let mut bytes = [0_u8; 6];
        let read_result = self
            .i2c
            .read(SHTC3_ADDRESS, &mut bytes)
            .map_err(|error| anyhow!("SHTC3 measurement read failed: {error:?}"));
        let sleep_result = self.write_command(SLEEP);
        read_result?;
        sleep_result?;

        decode_measurement(bytes)
    }

    fn read_id(&mut self) -> Result<u16> {
        let mut bytes = [0_u8; 3];
        self.i2c
            .write_read(SHTC3_ADDRESS, &READ_ID, &mut bytes)
            .map_err(|error| anyhow!("SHTC3 ID read failed: {error:?}"))?;
        verify_crc(&bytes[..2], bytes[2])?;
        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    fn write_command(&mut self, command: [u8; 2]) -> Result<()> {
        self.i2c.write(SHTC3_ADDRESS, &command).map_err(|error| {
            anyhow!(
                "SHTC3 command 0x{:02X}{:02X} failed: {error:?}",
                command[0],
                command[1]
            )
        })
    }
}

fn decode_measurement(bytes: [u8; 6]) -> Result<EnvironmentReading> {
    verify_crc(&bytes[..2], bytes[2])?;
    verify_crc(&bytes[3..5], bytes[5])?;

    let raw_temperature = u16::from_be_bytes([bytes[0], bytes[1]]);
    let raw_humidity = u16::from_be_bytes([bytes[3], bytes[4]]);
    Ok(EnvironmentReading {
        temperature_tenths_c: temperature_tenths_c(raw_temperature),
        humidity_tenths_percent: humidity_tenths_percent(raw_humidity),
    })
}

fn verify_crc(bytes: &[u8], expected: u8) -> Result<()> {
    let actual = crc8(bytes);
    if actual != expected {
        bail!("SHTC3 CRC mismatch: expected 0x{expected:02X}, calculated 0x{actual:02X}");
    }
    Ok(())
}

fn crc8(bytes: &[u8]) -> u8 {
    let mut crc = 0xFF_u8;
    for byte in bytes {
        crc ^= *byte;
        for _ in 0..8 {
            crc = if crc & 0x80 != 0 {
                (crc << 1) ^ CRC_POLYNOMIAL
            } else {
                crc << 1
            };
        }
    }
    crc
}

fn rounded_div(numerator: i32, denominator: i32) -> i32 {
    if numerator < 0 {
        (numerator - denominator / 2) / denominator
    } else {
        (numerator + denominator / 2) / denominator
    }
}

fn temperature_tenths_c(raw: u16) -> i16 {
    let converted = (1750_i32 * i32::from(raw)) / 65_536 - 450;
    (converted + i32::from(SAMPLE_TEMPERATURE_OFFSET_TENTHS_C)) as i16
}

fn humidity_tenths_percent(raw: u16) -> u16 {
    ((1000_u32 * u32::from(raw)) / 65_536) as u16
}

fn format_signed_tenths(value: i16, suffix: &str) -> String {
    let magnitude = i32::from(value).abs();
    let sign = if value < 0 { "-" } else { "" };
    format!("{sign}{}.{:01}{suffix}", magnitude / 10, magnitude % 10)
}

#[cfg(test)]
mod tests {
    use crate::regional::TemperatureUnit;

    use super::{
        crc8, decode_measurement, humidity_tenths_percent, temperature_tenths_c, EnvironmentReading,
    };

    #[test]
    fn crc_matches_sensirion_reference_vector() {
        assert_eq!(crc8(&[0xBE, 0xEF]), 0x92);
    }

    #[test]
    fn converts_raw_midpoints_with_sample_offset() {
        assert_eq!(temperature_tenths_c(32_768), 410);
        assert_eq!(humidity_tenths_percent(32_768), 500);
    }

    #[test]
    fn rejects_measurement_crc_mismatch() {
        assert!(decode_measurement([0, 0, 0, 0, 0, 0]).is_err());
    }
    #[test]
    fn renders_fahrenheit_for_us_product_presentation() {
        let reading = EnvironmentReading {
            temperature_tenths_c: 231,
            humidity_tenths_percent: 487,
        };
        assert_eq!(reading.temperature_tenths_f(), 736);
        assert_eq!(
            reading.temperature_label(TemperatureUnit::Fahrenheit),
            "73.6 F"
        );
        assert_eq!(
            reading.temperature_label(TemperatureUnit::Celsius),
            "23.1 C"
        );
    }
}
