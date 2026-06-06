//! Regional presentation policy for RTC and sensor values.
//!
//! The PCF85063 stores wall-clock fields without a timezone identifier. The
//! uploaded Waveshare sample uses UTC+08:00 as its RTC basis. Keep that basis
//! explicit and apply a timezone profile only at the presentation boundary.

use anyhow::{bail, Result};

use crate::rtc::RtcDateTime;

/// RTC wall-clock basis inherited from the uploaded sample application.
pub const SAMPLE_RTC_STORAGE_UTC_OFFSET_MINUTES: i16 = 8 * 60;
/// Daylight offset retained as the fallback when a date is unavailable.
pub const DEFAULT_DISPLAY_UTC_OFFSET_MINUTES: i16 = -4 * 60;
/// Product-facing default timezone profile.
pub const DEFAULT_TIMEZONE_NAME: &str = "America/New_York";
/// Daylight abbreviation retained as the fallback when a date is unavailable.
pub const DEFAULT_TIMEZONE_ABBREVIATION: &str = "EDT";

/// Temperature unit used by product-facing screens.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TemperatureUnit {
    Celsius,
    #[default]
    Fahrenheit,
}

impl TemperatureUnit {
    #[must_use]
    pub const fn suffix(self) -> &'static str {
        match self {
            Self::Celsius => " C",
            Self::Fahrenheit => " F",
        }
    }

    #[must_use]
    pub const fn marker(self) -> &'static str {
        match self {
            Self::Celsius => "celsius",
            Self::Fahrenheit => "fahrenheit",
        }
    }
}

/// Supported timezone profiles for the first Wi-Fi/NTP milestone.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TimeZoneProfile {
    #[default]
    AmericaNewYork,
    Utc,
}

impl TimeZoneProfile {
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "America/New_York" => Ok(Self::AmericaNewYork),
            "UTC" => Ok(Self::Utc),
            _ => bail!("unsupported timezone profile {value:?}"),
        }
    }

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::AmericaNewYork => "America/New_York",
            Self::Utc => "UTC",
        }
    }

    #[must_use]
    pub fn offset_minutes_for_utc(self, utc: RtcDateTime) -> i16 {
        match self {
            Self::AmericaNewYork if is_new_york_dst(utc) => -4 * 60,
            Self::AmericaNewYork => -5 * 60,
            Self::Utc => 0,
        }
    }

    #[must_use]
    pub fn abbreviation_for_utc(self, utc: RtcDateTime) -> &'static str {
        match self {
            Self::AmericaNewYork if is_new_york_dst(utc) => "EDT",
            Self::AmericaNewYork => "EST",
            Self::Utc => "UTC",
        }
    }
}

/// Regional presentation settings owned by UI state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RegionalPreferences {
    pub rtc_storage_utc_offset_minutes: i16,
    pub timezone: TimeZoneProfile,
    pub temperature_unit: TemperatureUnit,
}

impl Default for RegionalPreferences {
    fn default() -> Self {
        Self {
            rtc_storage_utc_offset_minutes: SAMPLE_RTC_STORAGE_UTC_OFFSET_MINUTES,
            timezone: TimeZoneProfile::default(),
            temperature_unit: TemperatureUnit::default(),
        }
    }
}

impl RegionalPreferences {
    /// Apply a validated boot-time timezone profile from removable storage.
    pub fn with_timezone_name(mut self, timezone: &str) -> Result<Self> {
        self.timezone = TimeZoneProfile::parse(timezone)?;
        Ok(self)
    }

    #[must_use]
    pub const fn timezone_name(self) -> &'static str {
        self.timezone.name()
    }

    /// Convert the stored RTC wall clock into UTC.
    #[must_use]
    pub fn rtc_to_utc(self, rtc: RtcDateTime) -> RtcDateTime {
        rtc.shift_minutes(-i32::from(self.rtc_storage_utc_offset_minutes))
    }

    /// Convert stored RTC fields into the selected timezone with automatic DST.
    #[must_use]
    pub fn localize_rtc(self, rtc: RtcDateTime) -> RtcDateTime {
        let utc = self.rtc_to_utc(rtc);
        utc.shift_minutes(i32::from(self.timezone.offset_minutes_for_utc(utc)))
    }

    /// Convert a local schedule value into the retained RTC storage basis.
    /// For New York's repeated fall-back hour, prefer the earlier valid UTC
    /// candidate. Alarm definitions remain deterministic without storing a
    /// separate DST flag in removable storage.
    #[must_use]
    pub fn local_to_rtc(self, local: RtcDateTime) -> RtcDateTime {
        let utc = match self.timezone {
            TimeZoneProfile::Utc => local,
            TimeZoneProfile::AmericaNewYork => {
                let daylight = local.shift_minutes(4 * 60);
                let standard = local.shift_minutes(5 * 60);
                let daylight_valid = self.timezone.offset_minutes_for_utc(daylight) == -4 * 60;
                let standard_valid = self.timezone.offset_minutes_for_utc(standard) == -5 * 60;
                match (daylight_valid, standard_valid) {
                    (true, _) => daylight,
                    (false, true) => standard,
                    (false, false) => standard,
                }
            }
        };
        utc.shift_minutes(i32::from(self.rtc_storage_utc_offset_minutes))
    }

    /// Render the selected timezone using the RTC date when available.
    #[must_use]
    pub fn timezone_label_for_rtc(self, rtc: Option<RtcDateTime>) -> String {
        if let Some(rtc) = rtc {
            let utc = self.rtc_to_utc(rtc);
            return format!(
                "{} {}",
                self.timezone.abbreviation_for_utc(utc),
                format_utc_offset(self.timezone.offset_minutes_for_utc(utc))
            );
        }
        match self.timezone {
            TimeZoneProfile::AmericaNewYork => format!(
                "{} {}",
                DEFAULT_TIMEZONE_ABBREVIATION,
                format_utc_offset(DEFAULT_DISPLAY_UTC_OFFSET_MINUTES)
            ),
            TimeZoneProfile::Utc => "UTC UTC+00:00".into(),
        }
    }

    /// Compatibility label for startup logs before an RTC snapshot exists.
    #[must_use]
    pub fn timezone_label(self) -> String {
        self.timezone_label_for_rtc(None)
    }

    #[must_use]
    pub fn rtc_storage_label(self) -> String {
        format_utc_offset(self.rtc_storage_utc_offset_minutes)
    }
}

/// Render an offset using the user-facing `UTC+HH:MM` form.
#[must_use]
pub fn format_utc_offset(minutes: i16) -> String {
    let sign = if minutes < 0 { '-' } else { '+' };
    let magnitude = i32::from(minutes).abs();
    format!("UTC{sign}{:02}:{:02}", magnitude / 60, magnitude % 60)
}

fn is_new_york_dst(utc: RtcDateTime) -> bool {
    let start_day = nth_sunday_of_month(utc.year, 3, 2);
    let end_day = nth_sunday_of_month(utc.year, 11, 1);
    let start = date_time_key(utc.year, 3, start_day, 7, 0, 0);
    let end = date_time_key(utc.year, 11, end_day, 6, 0, 0);
    let current = date_time_key(
        utc.year, utc.month, utc.day, utc.hour, utc.minute, utc.second,
    );
    current >= start && current < end
}

fn nth_sunday_of_month(year: u16, month: u8, nth: u8) -> u8 {
    let first_weekday = weekday_for_date(year, month, 1);
    let first_sunday = if first_weekday == 0 {
        1
    } else {
        8 - first_weekday
    };
    first_sunday + 7 * (nth - 1)
}

/// Sunday is zero, matching the RTC weekday convention used by this firmware.
fn weekday_for_date(year: u16, month: u8, day: u8) -> u8 {
    let table = [0_i32, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let mut year = i32::from(year);
    if month < 3 {
        year -= 1;
    }
    (year + year / 4 - year / 100 + year / 400 + table[usize::from(month - 1)] + i32::from(day))
        .rem_euclid(7) as u8
}

fn date_time_key(year: u16, month: u8, day: u8, hour: u8, minute: u8, second: u8) -> u64 {
    u64::from(year) * 10_000_000_000
        + u64::from(month) * 100_000_000
        + u64::from(day) * 1_000_000
        + u64::from(hour) * 10_000
        + u64::from(minute) * 100
        + u64::from(second)
}

#[cfg(test)]
mod tests {
    use super::{
        format_utc_offset, RegionalPreferences, TemperatureUnit, TimeZoneProfile,
        DEFAULT_DISPLAY_UTC_OFFSET_MINUTES, SAMPLE_RTC_STORAGE_UTC_OFFSET_MINUTES,
    };
    use crate::rtc::RtcDateTime;

    fn utc(month: u8, day: u8, hour: u8) -> RtcDateTime {
        RtcDateTime {
            year: 2026,
            month,
            day,
            weekday: 0,
            hour,
            minute: 0,
            second: 0,
        }
    }

    #[test]
    fn defaults_to_new_york_and_fahrenheit() {
        let preferences = RegionalPreferences::default();
        assert_eq!(preferences.timezone_name(), "America/New_York");
        assert_eq!(preferences.temperature_unit, TemperatureUnit::Fahrenheit);
        assert_eq!(preferences.timezone_label(), "EDT UTC-04:00");
    }

    #[test]
    fn converts_local_alarm_schedule_back_into_rtc_storage_basis() {
        let preferences = RegionalPreferences::default();
        let local = RtcDateTime {
            year: 2026,
            month: 6,
            day: 3,
            weekday: 3,
            hour: 7,
            minute: 30,
            second: 0,
        };
        let stored = preferences.local_to_rtc(local);
        assert_eq!(stored.date_time(), "2026-06-03  19:30:00");
        assert_eq!(preferences.localize_rtc(stored), local);
    }

    #[test]
    fn records_uploaded_sample_rtc_storage_basis_explicitly() {
        assert_eq!(SAMPLE_RTC_STORAGE_UTC_OFFSET_MINUTES, 480);
        assert_eq!(DEFAULT_DISPLAY_UTC_OFFSET_MINUTES, -240);
    }

    #[test]
    fn new_york_profile_applies_automatic_dst_transitions() {
        assert_eq!(
            TimeZoneProfile::AmericaNewYork.offset_minutes_for_utc(utc(1, 4, 12)),
            -300
        );
        assert_eq!(
            TimeZoneProfile::AmericaNewYork.offset_minutes_for_utc(utc(6, 4, 12)),
            -240
        );
        assert_eq!(
            TimeZoneProfile::AmericaNewYork.offset_minutes_for_utc(utc(3, 8, 6)),
            -300
        );
        assert_eq!(
            TimeZoneProfile::AmericaNewYork.offset_minutes_for_utc(utc(3, 8, 7)),
            -240
        );
        assert_eq!(
            TimeZoneProfile::AmericaNewYork.offset_minutes_for_utc(utc(11, 1, 5)),
            -240
        );
        assert_eq!(
            TimeZoneProfile::AmericaNewYork.offset_minutes_for_utc(utc(11, 1, 6)),
            -300
        );
    }

    #[test]
    fn localizes_sample_storage_clock_into_new_york_daylight_time() {
        let sample_wall_clock = RtcDateTime {
            year: 2026,
            month: 6,
            day: 4,
            weekday: 4,
            hour: 1,
            minute: 25,
            second: 30,
        };
        assert_eq!(
            RegionalPreferences::default()
                .localize_rtc(sample_wall_clock)
                .date_time(),
            "2026-06-03  13:25:30"
        );
    }

    #[test]
    fn accepts_utc_profile_and_formats_offsets() {
        let preferences = RegionalPreferences::default()
            .with_timezone_name("UTC")
            .unwrap();
        assert_eq!(preferences.timezone_name(), "UTC");
        assert_eq!(format_utc_offset(480), "UTC+08:00");
        assert_eq!(format_utc_offset(-240), "UTC-04:00");
        assert_eq!(format_utc_offset(330), "UTC+05:30");
    }
}
