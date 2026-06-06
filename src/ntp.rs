//! Host-testable UTC and RTC storage-basis conversion helpers for SNTP.

use crate::{regional::SAMPLE_RTC_STORAGE_UTC_OFFSET_MINUTES, rtc::RtcDateTime};

/// Treat system time values after 2020-01-01 as synchronized for this product.
pub const MIN_VALID_SNTP_UNIX_SECONDS: u64 = 1_577_836_800;

/// Convert Unix seconds into a UTC calendar snapshot.
#[must_use]
pub fn utc_from_unix_seconds(seconds: u64) -> RtcDateTime {
    let days = (seconds / 86_400) as i64;
    let seconds_of_day = seconds % 86_400;
    let (year, month, day) = civil_from_days(days);
    RtcDateTime {
        year,
        month,
        day,
        weekday: ((days + 4).rem_euclid(7)) as u8,
        hour: (seconds_of_day / 3_600) as u8,
        minute: ((seconds_of_day % 3_600) / 60) as u8,
        second: (seconds_of_day % 60) as u8,
    }
}

/// Convert UTC received from SNTP into the RTC wall-clock basis retained from
/// the uploaded sample application.
#[must_use]
pub fn rtc_storage_wall_clock_from_utc(utc: RtcDateTime) -> RtcDateTime {
    utc.shift_minutes(i32::from(SAMPLE_RTC_STORAGE_UTC_OFFSET_MINUTES))
}

/// Convert days since 1970-01-01 into Gregorian date fields.
fn civil_from_days(days_since_epoch: i64) -> (u16, u8, u8) {
    let z = days_since_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    (year as u16, month as u8, day as u8)
}

#[cfg(test)]
mod tests {
    use super::{rtc_storage_wall_clock_from_utc, utc_from_unix_seconds};

    #[test]
    fn converts_unix_epoch_and_reference_2026_instant() {
        assert_eq!(utc_from_unix_seconds(0).date_time(), "1970-01-01  00:00:00");
        assert_eq!(
            utc_from_unix_seconds(1_780_488_000).date_time(),
            "2026-06-03  12:00:00"
        );
    }

    #[test]
    fn converts_utc_into_sample_rtc_storage_basis() {
        let stored = rtc_storage_wall_clock_from_utc(utc_from_unix_seconds(1_780_488_000));
        assert_eq!(stored.date_time(), "2026-06-03  20:00:00");
    }
}
