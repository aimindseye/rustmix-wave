//! PCF85063 real-time clock support ported from the Waveshare sample.

use core::fmt::Debug;

use anyhow::{anyhow, bail, Result};
use embedded_hal::i2c::I2c;

/// PCF85063 7-bit I2C address used by the sample firmware.
pub const PCF85063_ADDRESS: u8 = 0x51;
const CONTROL_1_REG: u8 = 0x00;
const CONTROL_2_REG: u8 = 0x01;
const SECONDS_REG: u8 = 0x04;
/// PCF85063 alarm registers retained from the uploaded Waveshare reference.
pub const SECOND_ALARM_REG: u8 = 0x0B;
pub const MINUTES_ALARM_REG: u8 = 0x0C;
pub const HOUR_ALARM_REG: u8 = 0x0D;
pub const DAY_ALARM_REG: u8 = 0x0E;
pub const WEEKDAY_ALARM_REG: u8 = 0x0F;
const CLOCK_INTEGRITY_LOST: u8 = 1 << 7;
const ALARM_FIELD_DISABLED: u8 = 1 << 7;
const ALARM_INTERRUPT_ENABLE: u8 = 1 << 7;
const ALARM_FLAG: u8 = 1 << 6;

/// Date and time snapshot read from the RTC.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RtcDateTime {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub weekday: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

impl RtcDateTime {
    /// Shift wall-clock fields by a signed minute delta while preserving a
    /// valid Gregorian date and weekday.
    #[must_use]
    pub fn shift_minutes(self, delta_minutes: i32) -> Self {
        let total_minutes = i32::from(self.hour) * 60 + i32::from(self.minute) + delta_minutes;
        let day_delta = total_minutes.div_euclid(24 * 60);
        let minute_of_day = total_minutes.rem_euclid(24 * 60);

        let (year, month, day) = shift_date(self.year, self.month, self.day, day_delta);
        Self {
            year,
            month,
            day,
            weekday: (i32::from(self.weekday) + day_delta).rem_euclid(7) as u8,
            hour: (minute_of_day / 60) as u8,
            minute: (minute_of_day % 60) as u8,
            second: self.second,
        }
    }

    /// Render a compact header-friendly `HH:MM` value.
    #[must_use]
    pub fn time_hm(self) -> String {
        format!("{:02}:{:02}", self.hour, self.minute)
    }

    /// Render a date and time value for the Clock screen.
    #[must_use]
    pub fn date_time(self) -> String {
        format!(
            "{:04}-{:02}-{:02}  {:02}:{:02}:{:02}",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )
    }
}

/// Result of RTC startup normalization.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RtcInitReport {
    /// The oscillator-stop / clock-integrity bit was set at boot and cleared.
    pub clock_integrity_was_lost: bool,
}

/// Narrow register-level PCF85063 driver.
pub struct Pcf85063<I2C> {
    i2c: I2C,
}

impl<I2C> Pcf85063<I2C> {
    #[must_use]
    pub fn new(i2c: I2C) -> Self {
        Self { i2c }
    }
}

impl<I2C> Pcf85063<I2C>
where
    I2C: I2c,
    I2C::Error: Debug,
{
    /// Normalize the control register and clear the clock-integrity bit when
    /// the backup domain reports an oscillator stop.
    pub fn initialize(&mut self) -> Result<RtcInitReport> {
        let control = self.read_register(CONTROL_1_REG)?;
        self.write_register(CONTROL_1_REG, control & !(1 << 7))?;

        let seconds = self.read_register(SECONDS_REG)?;
        let clock_integrity_was_lost = seconds & CLOCK_INTEGRITY_LOST != 0;
        if clock_integrity_was_lost {
            self.write_register(SECONDS_REG, seconds & !CLOCK_INTEGRITY_LOST)?;
        }

        Ok(RtcInitReport {
            clock_integrity_was_lost,
        })
    }

    /// Read the seven contiguous time registers in one transaction.
    pub fn read_datetime(&mut self) -> Result<RtcDateTime> {
        let mut registers = [0_u8; 7];
        self.i2c
            .write_read(PCF85063_ADDRESS, &[SECONDS_REG], &mut registers)
            .map_err(|error| anyhow!("PCF85063 time read failed: {error:?}"))?;
        decode_datetime_registers(registers)
    }

    /// Program the single PCF85063 hardware alarm slot using stored RTC
    /// wall-clock fields. The domain scheduler selects the earliest local
    /// occurrence and converts it to this retained storage basis first.
    pub fn program_alarm(&mut self, value: RtcDateTime) -> Result<()> {
        validate_datetime(value)?;
        let registers = [
            SECOND_ALARM_REG,
            encode_bcd(value.second)? & !ALARM_FIELD_DISABLED,
            encode_bcd(value.minute)? & !ALARM_FIELD_DISABLED,
            encode_bcd(value.hour)? & !ALARM_FIELD_DISABLED,
            encode_bcd(value.day)? & !ALARM_FIELD_DISABLED,
            ALARM_FIELD_DISABLED, // weekday comparison disabled; absolute day is enough.
        ];
        self.i2c
            .write(PCF85063_ADDRESS, &registers)
            .map_err(|error| anyhow!("PCF85063 alarm write failed: {error:?}"))?;
        let control = self.read_register(CONTROL_2_REG)?;
        self.write_register(
            CONTROL_2_REG,
            (control & !ALARM_FLAG) | ALARM_INTERRUPT_ENABLE,
        )
    }

    /// Disable all alarm compare fields and the alarm interrupt-enable bit.
    pub fn disable_alarm(&mut self) -> Result<()> {
        for register in [
            SECOND_ALARM_REG,
            MINUTES_ALARM_REG,
            HOUR_ALARM_REG,
            DAY_ALARM_REG,
            WEEKDAY_ALARM_REG,
        ] {
            let value = self.read_register(register)?;
            self.write_register(register, value | ALARM_FIELD_DISABLED)?;
        }
        let control = self.read_register(CONTROL_2_REG)?;
        self.write_register(
            CONTROL_2_REG,
            control & !(ALARM_INTERRUPT_ENABLE | ALARM_FLAG),
        )
    }

    /// Return whether the PCF85063 alarm flag is currently asserted.
    pub fn alarm_flag(&mut self) -> Result<bool> {
        Ok(self.read_register(CONTROL_2_REG)? & ALARM_FLAG != 0)
    }

    /// Clear the sticky PCF85063 alarm flag while preserving alarm enablement.
    pub fn clear_alarm_flag(&mut self) -> Result<()> {
        let control = self.read_register(CONTROL_2_REG)?;
        self.write_register(CONTROL_2_REG, control & !ALARM_FLAG)
    }

    /// Write RTC wall-clock fields after a validated SNTP synchronization.
    pub fn write_datetime(&mut self, value: RtcDateTime) -> Result<()> {
        validate_datetime(value)?;
        let year = value
            .year
            .checked_sub(2000)
            .ok_or_else(|| anyhow!("PCF85063 year must be in the 2000..=2099 range"))?;
        if year > 99 {
            bail!("PCF85063 year must be in the 2000..=2099 range");
        }
        let registers = [
            SECONDS_REG,
            encode_bcd(value.second)?,
            encode_bcd(value.minute)?,
            encode_bcd(value.hour)?,
            encode_bcd(value.day)?,
            value.weekday & 0x07,
            encode_bcd(value.month)?,
            encode_bcd(year as u8)?,
        ];
        self.i2c
            .write(PCF85063_ADDRESS, &registers)
            .map_err(|error| anyhow!("PCF85063 time write failed: {error:?}"))
    }

    fn read_register(&mut self, register: u8) -> Result<u8> {
        let mut value = [0_u8; 1];
        self.i2c
            .write_read(PCF85063_ADDRESS, &[register], &mut value)
            .map_err(|error| anyhow!("PCF85063 read 0x{register:02X} failed: {error:?}"))?;
        Ok(value[0])
    }

    fn write_register(&mut self, register: u8, value: u8) -> Result<()> {
        self.i2c
            .write(PCF85063_ADDRESS, &[register, value])
            .map_err(|error| anyhow!("PCF85063 write 0x{register:02X} failed: {error:?}"))
    }
}

fn validate_datetime(value: RtcDateTime) -> Result<()> {
    if !(2000..=2099).contains(&value.year)
        || !(1..=12).contains(&value.month)
        || value.day == 0
        || value.day > days_in_month(value.year, value.month)
        || value.weekday > 6
        || value.hour > 23
        || value.minute > 59
        || value.second > 59
    {
        bail!("PCF85063 write received an invalid calendar value");
    }
    Ok(())
}

fn encode_bcd(value: u8) -> Result<u8> {
    if value > 99 {
        bail!("cannot encode BCD value {value}");
    }
    Ok((value / 10) << 4 | (value % 10))
}

fn decode_datetime_registers(registers: [u8; 7]) -> Result<RtcDateTime> {
    let second = decode_bcd(registers[0] & 0x7F)?;
    let minute = decode_bcd(registers[1] & 0x7F)?;
    let hour = decode_bcd(registers[2] & 0x3F)?;
    let day = decode_bcd(registers[3] & 0x3F)?;
    let weekday = registers[4] & 0x07;
    let month = decode_bcd(registers[5] & 0x1F)?;
    let year = 2000 + u16::from(decode_bcd(registers[6])?);

    if second > 59 || minute > 59 || hour > 23 || !(1..=31).contains(&day) {
        bail!("PCF85063 returned an invalid time register set");
    }
    if !(1..=12).contains(&month) || weekday > 6 {
        bail!("PCF85063 returned an invalid calendar register set");
    }

    Ok(RtcDateTime {
        year,
        month,
        day,
        weekday,
        hour,
        minute,
        second,
    })
}

fn shift_date(mut year: u16, mut month: u8, mut day: u8, mut day_delta: i32) -> (u16, u8, u8) {
    while day_delta > 0 {
        let days_this_month = days_in_month(year, month);
        if day < days_this_month {
            day += 1;
        } else {
            day = 1;
            if month == 12 {
                month = 1;
                year += 1;
            } else {
                month += 1;
            }
        }
        day_delta -= 1;
    }

    while day_delta < 0 {
        if day > 1 {
            day -= 1;
        } else {
            if month == 1 {
                month = 12;
                year -= 1;
            } else {
                month -= 1;
            }
            day = days_in_month(year, month);
        }
        day_delta += 1;
    }

    (year, month, day)
}

fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn is_leap_year(year: u16) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

fn decode_bcd(value: u8) -> Result<u8> {
    let high = value >> 4;
    let low = value & 0x0F;
    if high > 9 || low > 9 {
        bail!("invalid BCD value 0x{value:02X}");
    }
    Ok(high * 10 + low)
}

#[cfg(test)]
mod tests {
    use super::{
        decode_bcd, decode_datetime_registers, encode_bcd, validate_datetime, RtcDateTime,
        ALARM_FIELD_DISABLED, ALARM_FLAG, ALARM_INTERRUPT_ENABLE, DAY_ALARM_REG, HOUR_ALARM_REG,
        MINUTES_ALARM_REG, SECOND_ALARM_REG, WEEKDAY_ALARM_REG,
    };

    #[test]
    fn exposes_reference_alarm_register_contract() {
        assert_eq!(SECOND_ALARM_REG, 0x0B);
        assert_eq!(MINUTES_ALARM_REG, 0x0C);
        assert_eq!(HOUR_ALARM_REG, 0x0D);
        assert_eq!(DAY_ALARM_REG, 0x0E);
        assert_eq!(WEEKDAY_ALARM_REG, 0x0F);
        assert_eq!(ALARM_FIELD_DISABLED, 0x80);
        assert_eq!(ALARM_INTERRUPT_ENABLE, 0x80);
        assert_eq!(ALARM_FLAG, 0x40);
    }

    #[test]
    fn decodes_reference_style_rtc_registers() {
        assert_eq!(
            decode_datetime_registers([0x45, 0x59, 0x23, 0x31, 0x02, 0x12, 0x26]).unwrap(),
            RtcDateTime {
                year: 2026,
                month: 12,
                day: 31,
                weekday: 2,
                hour: 23,
                minute: 59,
                second: 45,
            }
        );
    }

    #[test]
    fn rejects_invalid_bcd() {
        assert!(decode_bcd(0xFA).is_err());
    }

    #[test]
    fn encodes_valid_bcd_and_rejects_invalid_write_calendar() {
        assert_eq!(encode_bcd(59).unwrap(), 0x59);
        assert!(encode_bcd(100).is_err());
        assert!(validate_datetime(RtcDateTime {
            year: 2026,
            month: 2,
            day: 30,
            weekday: 1,
            hour: 12,
            minute: 0,
            second: 0,
        })
        .is_err());
    }

    #[test]
    fn renders_compact_and_full_time_labels() {
        let value = RtcDateTime {
            year: 2026,
            month: 6,
            day: 3,
            weekday: 3,
            hour: 13,
            minute: 7,
            second: 9,
        };
        assert_eq!(value.time_hm(), "13:07");
        assert_eq!(value.date_time(), "2026-06-03  13:07:09");
    }
    #[test]
    fn shifts_across_previous_day_and_month_boundary() {
        let value = RtcDateTime {
            year: 2026,
            month: 3,
            day: 1,
            weekday: 0,
            hour: 1,
            minute: 15,
            second: 9,
        };
        assert_eq!(
            value.shift_minutes(-180),
            RtcDateTime {
                year: 2026,
                month: 2,
                day: 28,
                weekday: 6,
                hour: 22,
                minute: 15,
                second: 9,
            }
        );
    }

    #[test]
    fn shifts_across_leap_day_and_year_boundary() {
        let value = RtcDateTime {
            year: 2024,
            month: 2,
            day: 28,
            weekday: 3,
            hour: 23,
            minute: 45,
            second: 1,
        };
        assert_eq!(value.shift_minutes(30).date_time(), "2024-02-29  00:15:01");

        let year_end = RtcDateTime {
            year: 2026,
            month: 12,
            day: 31,
            weekday: 4,
            hour: 23,
            minute: 55,
            second: 2,
        };
        assert_eq!(
            year_end.shift_minutes(10).date_time(),
            "2027-01-01  00:05:02"
        );
    }
}
