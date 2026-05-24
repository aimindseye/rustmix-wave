use alloc::string::String;
use core::fmt::{self, Write};

pub const TIME_FILE: &str = "TIME.TXT";
pub const TIMEZONE_ID: &str = "America/New_York";
pub const PANCHANG_LOCATION_NAME: &str = "Jersey City, NJ";
const UNIX_NTP_DELTA: u64 = 2_208_988_800;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClockFreshness {
    Unsynced,
    Cached,
    Live,
}

impl ClockFreshness {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unsynced => "Unsynced",
            Self::Cached => "Cached",
            Self::Live => "Live",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CalendarDate {
    pub year: i32,
    pub month: u8,
    pub day: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PanchangLite {
    pub tithi: &'static str,
    pub paksha: &'static str,
    pub month: &'static str,
    pub weekday: &'static str,
    pub location: &'static str,
    pub timezone: &'static str,
    pub note: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimeCache {
    pub timezone: &'static str,
    pub last_sync_unix: Option<u64>,
    pub last_sync_uptime_secs: Option<u32>,
    pub last_sync_ok: bool,
    pub last_sync_source: &'static str,
    pub last_sync_error: String,
    pub last_sync_ip: String,
    pub display_offset_minutes: i16,
}

impl Default for TimeCache {
    fn default() -> Self {
        Self {
            timezone: TIMEZONE_ID,
            last_sync_unix: None,
            last_sync_uptime_secs: None,
            last_sync_ok: false,
            last_sync_source: "",
            last_sync_error: String::new(),
            last_sync_ip: String::new(),
            display_offset_minutes: -300,
        }
    }
}

impl TimeCache {
    pub fn synced(unix: u64, uptime_secs: u32, ip: Option<[u8; 4]>, source: &'static str) -> Self {
        let offset = eastern_offset_minutes(unix);
        let mut cache = Self {
            timezone: TIMEZONE_ID,
            last_sync_unix: Some(unix),
            last_sync_uptime_secs: Some(uptime_secs),
            last_sync_ok: true,
            last_sync_source: source,
            last_sync_error: String::new(),
            last_sync_ip: String::new(),
            display_offset_minutes: offset,
        };
        if let Some(ip) = ip {
            let _ = write!(
                cache.last_sync_ip,
                "{}.{}.{}.{}",
                ip[0], ip[1], ip[2], ip[3]
            );
        }
        cache
    }

    pub fn with_error(mut self, error: &str, uptime_secs: u32) -> Self {
        self.last_sync_ok = false;
        self.last_sync_uptime_secs = Some(uptime_secs);
        self.last_sync_error.clear();
        push_trimmed(&mut self.last_sync_error, error);
        self
    }

    pub fn has_cached_time(&self) -> bool {
        self.last_sync_unix.is_some()
    }

    pub fn is_synced(&self) -> bool {
        self.last_sync_unix.is_some() && self.last_sync_ok
    }

    pub fn is_live(&self, uptime_secs: u32) -> bool {
        self.live_unix(uptime_secs).is_some()
    }

    pub fn freshness(&self, uptime_secs: u32) -> ClockFreshness {
        if self.live_unix(uptime_secs).is_some() {
            ClockFreshness::Live
        } else if self.last_sync_unix.is_some() {
            ClockFreshness::Cached
        } else {
            ClockFreshness::Unsynced
        }
    }

    pub fn live_unix(&self, uptime_secs: u32) -> Option<u64> {
        if !self.last_sync_ok {
            return None;
        }
        let base = self.last_sync_unix?;
        let synced_at = self.last_sync_uptime_secs?;
        if uptime_secs < synced_at {
            return None;
        }
        Some(base.saturating_add(u64::from(uptime_secs - synced_at)))
    }

    pub fn display_unix(&self, uptime_secs: u32) -> Option<u64> {
        self.live_unix(uptime_secs).or(self.last_sync_unix)
    }

    pub fn display_date(&self, uptime_secs: u32) -> Option<CalendarDate> {
        let local = local_datetime(self.display_unix(uptime_secs)?);
        Some(CalendarDate {
            year: local.year,
            month: local.month,
            day: local.day,
        })
    }

    pub fn display_panchang_lite(&self, uptime_secs: u32) -> Option<PanchangLite> {
        Some(panchang_lite_for_date(self.display_date(uptime_secs)?))
    }

    pub fn display_weekday_index(&self, uptime_secs: u32) -> Option<u8> {
        self.display_unix(uptime_secs)
            .map(|unix| local_datetime(unix).weekday)
    }

    pub fn estimated_unix(&self, uptime_secs: u32) -> Option<u64> {
        self.live_unix(uptime_secs)
    }

    pub fn write_home_time<W: Write>(&self, uptime_secs: u32, out: &mut W) -> fmt::Result {
        match self.freshness(uptime_secs) {
            ClockFreshness::Live => {
                let Some(unix) = self.live_unix(uptime_secs) else {
                    return write!(out, "Time unsynced");
                };
                let local = local_datetime(unix);
                write!(
                    out,
                    "{} {} {}  {}:{:02} {}",
                    weekday_name(local.weekday),
                    month_name(local.month),
                    local.day,
                    hour12(local.hour).0,
                    local.minute,
                    hour12(local.hour).1
                )
            }
            ClockFreshness::Cached => {
                let Some(unix) = self.last_sync_unix else {
                    return write!(out, "Time unsynced");
                };
                let local = local_datetime(unix);
                write!(
                    out,
                    "Cached {} {} {}",
                    weekday_name(local.weekday),
                    month_name(local.month),
                    local.day
                )
            }
            ClockFreshness::Unsynced => write!(out, "Time unsynced"),
        }
    }

    pub fn write_time_value<W: Write>(&self, uptime_secs: u32, out: &mut W) -> fmt::Result {
        match self.freshness(uptime_secs) {
            ClockFreshness::Live => {
                let Some(unix) = self.live_unix(uptime_secs) else {
                    return write!(out, "unsynced");
                };
                let local = local_datetime(unix);
                let (hour, suffix) = hour12(local.hour);
                write!(out, "{}:{:02} {}", hour, local.minute, suffix)
            }
            ClockFreshness::Cached => {
                let Some(unix) = self.last_sync_unix else {
                    return write!(out, "unsynced");
                };
                let local = local_datetime(unix);
                let (hour, suffix) = hour12(local.hour);
                write!(out, "cached {}:{:02} {}", hour, local.minute, suffix)
            }
            ClockFreshness::Unsynced => write!(out, "unsynced"),
        }
    }

    pub fn write_date_value<W: Write>(&self, uptime_secs: u32, out: &mut W) -> fmt::Result {
        if let Some(unix) = self.display_unix(uptime_secs) {
            let local = local_datetime(unix);
            write!(
                out,
                "{}, {} {} {}",
                weekday_name(local.weekday),
                month_name(local.month),
                local.day,
                local.year
            )
        } else {
            write!(out, "--")
        }
    }

    pub fn write_sync_summary<W: Write>(&self, uptime_secs: u32, out: &mut W) -> fmt::Result {
        match self.freshness(uptime_secs) {
            ClockFreshness::Live => {
                write!(out, "Live ")?;
                self.write_time_value(uptime_secs, out)
            }
            ClockFreshness::Cached => write!(out, "Cached; resync"),
            ClockFreshness::Unsynced => {
                if self.last_sync_error.is_empty() {
                    write!(out, "Never synced")
                } else {
                    write!(out, "Failed: {}", self.last_sync_error.as_str())
                }
            }
        }
    }

    pub fn write_clock_detail<W: Write>(&self, uptime_secs: u32, out: &mut W) -> fmt::Result {
        match self.freshness(uptime_secs) {
            ClockFreshness::Live => write!(out, "Clock: live from current boot"),
            ClockFreshness::Cached => write!(out, "Clock: cached; Select resyncs"),
            ClockFreshness::Unsynced => write!(out, "Clock: unsynced; Select syncs"),
        }
    }

    pub fn write_last_sync<W: Write>(&self, out: &mut W) -> fmt::Result {
        if let Some(unix) = self.last_sync_unix {
            let local = local_datetime(unix);
            let (hour, suffix) = hour12(local.hour);
            write!(
                out,
                "{} {} {} {:02}:{:02} {}",
                month_name(local.month),
                local.day,
                local.year,
                hour,
                local.minute,
                suffix
            )
        } else {
            write!(out, "Never")
        }
    }
}

pub fn parse_time_txt(data: &[u8]) -> TimeCache {
    let mut cache = TimeCache::default();
    let Ok(text) = core::str::from_utf8(data) else {
        return cache;
    };
    for line in text.lines() {
        let Some((raw_key, raw_value)) = line.split_once('=') else {
            continue;
        };
        let key = raw_key.trim();
        let value = raw_value.trim();
        match key {
            "timezone" if value == TIMEZONE_ID => cache.timezone = TIMEZONE_ID,
            "last_sync_unix" => {
                cache.last_sync_unix = parse_u64(value);
            }
            "last_sync_monotonic_ms" => {
                cache.last_sync_uptime_secs =
                    parse_u64(value).map(|ms| (ms / 1000).min(u64::from(u32::MAX)) as u32);
            }
            "last_sync_ok" => cache.last_sync_ok = value == "1",
            "last_sync_source" if value == "ntp" => cache.last_sync_source = "ntp",
            "last_sync_error" => {
                cache.last_sync_error.clear();
                push_trimmed(&mut cache.last_sync_error, value);
            }
            "last_sync_ip" => {
                cache.last_sync_ip.clear();
                push_trimmed(&mut cache.last_sync_ip, value);
            }
            "display_offset_minutes" => {
                if let Some(offset) = parse_i16(value) {
                    cache.display_offset_minutes = offset;
                }
            }
            _ => {}
        }
    }
    if cache.last_sync_unix.is_none() {
        cache.last_sync_ok = false;
    }
    cache
}

pub fn write_time_txt(cache: &TimeCache, out: &mut [u8]) -> usize {
    let mut writer = SliceWriter { out, pos: 0 };
    let _ = writeln!(writer, "timezone={}", TIMEZONE_ID);
    if let Some(unix) = cache.last_sync_unix {
        let _ = writeln!(writer, "last_sync_unix={}", unix);
    } else {
        let _ = writeln!(writer, "last_sync_unix=");
    }
    if let Some(uptime) = cache.last_sync_uptime_secs {
        let _ = writeln!(
            writer,
            "last_sync_monotonic_ms={}",
            u64::from(uptime) * 1000
        );
    } else {
        let _ = writeln!(writer, "last_sync_monotonic_ms=");
    }
    let _ = writeln!(
        writer,
        "last_sync_ok={}",
        if cache.last_sync_ok { 1 } else { 0 }
    );
    let _ = writeln!(writer, "last_sync_source={}", cache.last_sync_source);
    let _ = writeln!(writer, "last_sync_error={}", cache.last_sync_error);
    let _ = writeln!(writer, "last_sync_ip={}", cache.last_sync_ip);
    let _ = writeln!(
        writer,
        "display_offset_minutes={}",
        cache.display_offset_minutes
    );
    writer.pos
}

pub fn ntp_seconds_to_unix(ntp_seconds: u64) -> Option<u64> {
    ntp_seconds.checked_sub(UNIX_NTP_DELTA)
}

pub fn parse_ntp_unix_seconds(packet: &[u8]) -> Option<u64> {
    if packet.len() < 48 {
        return None;
    }
    let seconds = u32::from_be_bytes([packet[40], packet[41], packet[42], packet[43]]) as u64;
    if seconds == 0 {
        return None;
    }
    ntp_seconds_to_unix(seconds)
}

pub fn calendar_weekday_for_date(year: i32, month: u8, day: u8) -> u8 {
    weekday_for_date(year, month, day)
}

pub const fn calendar_days_in_month(year: i32, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

pub const fn calendar_month_name(month: u8) -> &'static str {
    match month {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => "Month",
    }
}

const fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

pub fn panchang_lite_for_date(date: CalendarDate) -> PanchangLite {
    let tithi_num = approximate_tithi_number(date);
    PanchangLite {
        tithi: panchang_tithi_name(tithi_num),
        paksha: if tithi_num <= 15 { "Shukla" } else { "Krishna" },
        month: approximate_hindu_month(date.month, date.day),
        weekday: panchang_weekday_name(calendar_weekday_for_date(date.year, date.month, date.day)),
        location: PANCHANG_LOCATION_NAME,
        timezone: TIMEZONE_ID,
        note: "Offline lite estimate; festivals later",
    }
}

fn approximate_tithi_number(date: CalendarDate) -> u8 {
    const SCALE: i64 = 100_000;
    const SYNODIC_MONTH_X: i64 = 2_953_059;
    const REF_NEW_MOON_DAY_X: i64 = 1_096_276_000;

    let day_x = days_from_civil(date.year, date.month, date.day)
        .saturating_mul(SCALE)
        .saturating_add(SCALE / 2);
    let phase_x = (day_x - REF_NEW_MOON_DAY_X).rem_euclid(SYNODIC_MONTH_X);
    ((phase_x.saturating_mul(30) / SYNODIC_MONTH_X) as u8).saturating_add(1)
}

fn panchang_tithi_name(tithi_num: u8) -> &'static str {
    match tithi_num {
        1 | 16 => "Pratipada",
        2 | 17 => "Dwitiya",
        3 | 18 => "Tritiya",
        4 | 19 => "Chaturthi",
        5 | 20 => "Panchami",
        6 | 21 => "Shashthi",
        7 | 22 => "Saptami",
        8 | 23 => "Ashtami",
        9 | 24 => "Navami",
        10 | 25 => "Dashami",
        11 | 26 => "Ekadashi",
        12 | 27 => "Dwadashi",
        13 | 28 => "Trayodashi",
        14 | 29 => "Chaturdashi",
        15 => "Purnima",
        30 => "Amavasya",
        _ => "--",
    }
}

fn approximate_hindu_month(month: u8, day: u8) -> &'static str {
    match month {
        1 => {
            if day < 20 {
                "Pausha"
            } else {
                "Magha"
            }
        }
        2 => {
            if day < 19 {
                "Magha"
            } else {
                "Phalguna"
            }
        }
        3 => {
            if day < 21 {
                "Phalguna"
            } else {
                "Chaitra"
            }
        }
        4 => {
            if day < 20 {
                "Chaitra"
            } else {
                "Vaishakha"
            }
        }
        5 => {
            if day < 21 {
                "Vaishakha"
            } else {
                "Jyeshtha"
            }
        }
        6 => {
            if day < 22 {
                "Jyeshtha"
            } else {
                "Ashadha"
            }
        }
        7 => {
            if day < 23 {
                "Ashadha"
            } else {
                "Shravana"
            }
        }
        8 => {
            if day < 23 {
                "Shravana"
            } else {
                "Bhadrapada"
            }
        }
        9 => {
            if day < 23 {
                "Bhadrapada"
            } else {
                "Ashwin"
            }
        }
        10 => {
            if day < 23 {
                "Ashwin"
            } else {
                "Kartika"
            }
        }
        11 => {
            if day < 22 {
                "Kartika"
            } else {
                "Margashirsha"
            }
        }
        12 => {
            if day < 22 {
                "Margashirsha"
            } else {
                "Pausha"
            }
        }
        _ => "--",
    }
}

fn panchang_weekday_name(idx: u8) -> &'static str {
    match idx {
        0 => "Sunday / Ravivar",
        1 => "Monday / Somvar",
        2 => "Tuesday / Mangalvar",
        3 => "Wednesday / Budhvar",
        4 => "Thursday / Guruvar",
        5 => "Friday / Shukravar",
        6 => "Saturday / Shanivar",
        _ => "--",
    }
}

pub fn battery_label<W: Write>(mv: u16, out: &mut W) -> fmt::Result {
    if let Some(pct) = battery_percent_value(mv) {
        write!(out, "Batt {}%", pct)
    } else {
        write!(out, "Batt --")
    }
}

pub fn battery_percent_value(mv: u16) -> Option<u8> {
    if mv == 0 {
        None
    } else {
        let mv = mv.clamp(3300, 4200);
        Some((((u32::from(mv) - 3300) * 100) / 900) as u8)
    }
}

fn parse_u64(input: &str) -> Option<u64> {
    if input.is_empty() {
        return None;
    }
    let mut value = 0u64;
    for b in input.bytes() {
        if !b.is_ascii_digit() {
            return None;
        }
        value = value.checked_mul(10)?.checked_add(u64::from(b - b'0'))?;
    }
    Some(value)
}

fn parse_i16(input: &str) -> Option<i16> {
    input.parse().ok()
}

fn push_trimmed(out: &mut String, input: &str) {
    for ch in input.chars().take(47) {
        out.push(ch);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct LocalDateTime {
    year: i32,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    weekday: u8,
}

fn local_datetime(unix: u64) -> LocalDateTime {
    let offset = eastern_offset_minutes(unix);
    let local = if offset < 0 {
        unix.saturating_sub((-offset) as u64 * 60)
    } else {
        unix.saturating_add(offset as u64 * 60)
    };
    datetime_from_unix(local)
}

fn eastern_offset_minutes(unix: u64) -> i16 {
    let utc = datetime_from_unix(unix);
    let dst_start = unix_from_ymdhms(utc.year, 3, nth_weekday_day(utc.year, 3, 0, 2), 7, 0, 0);
    let dst_end = unix_from_ymdhms(utc.year, 11, nth_weekday_day(utc.year, 11, 0, 1), 6, 0, 0);
    if unix >= dst_start && unix < dst_end {
        -240
    } else {
        -300
    }
}

fn datetime_from_unix(unix: u64) -> LocalDateTime {
    let days = (unix / 86_400) as i64;
    let seconds = (unix % 86_400) as u32;
    let (year, month, day) = civil_from_days(days);
    LocalDateTime {
        year,
        month,
        day,
        hour: (seconds / 3600) as u8,
        minute: ((seconds % 3600) / 60) as u8,
        weekday: ((days + 4).rem_euclid(7)) as u8,
    }
}

fn unix_from_ymdhms(year: i32, month: u8, day: u8, hour: u8, minute: u8, second: u8) -> u64 {
    let days = days_from_civil(year, month, day);
    (days as u64) * 86_400 + u64::from(hour) * 3600 + u64::from(minute) * 60 + u64::from(second)
}

fn civil_from_days(days: i64) -> (i32, u8, u8) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    let year = y + if m <= 2 { 1 } else { 0 };
    (year as i32, m as u8, d as u8)
}

fn days_from_civil(year: i32, month: u8, day: u8) -> i64 {
    let mut y = i64::from(year);
    let m = i64::from(month);
    let d = i64::from(day);
    y -= if m <= 2 { 1 } else { 0 };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = m + if m > 2 { -3 } else { 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn nth_weekday_day(year: i32, month: u8, weekday: u8, nth: u8) -> u8 {
    let first_weekday = weekday_for_date(year, month, 1);
    let delta = (7 + i16::from(weekday) - i16::from(first_weekday)).rem_euclid(7) as u8;
    let first = 1 + delta;
    first + (nth - 1) * 7
}

fn weekday_for_date(year: i32, month: u8, day: u8) -> u8 {
    let days = days_from_civil(year, month, day);
    ((days + 4).rem_euclid(7)) as u8
}

fn weekday_name(idx: u8) -> &'static str {
    ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"][idx as usize]
}

fn month_name(month: u8) -> &'static str {
    [
        "", "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ][month as usize]
}

fn hour12(hour: u8) -> (u8, &'static str) {
    let suffix = if hour < 12 { "AM" } else { "PM" };
    let hour = match hour % 12 {
        0 => 12,
        h => h,
    };
    (hour, suffix)
}

struct SliceWriter<'a> {
    out: &'a mut [u8],
    pos: usize,
}

impl Write for SliceWriter<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let bytes = s.as_bytes();
        let remaining = self.out.len().saturating_sub(self.pos);
        let n = bytes.len().min(remaining);
        self.out[self.pos..self.pos + n].copy_from_slice(&bytes[..n]);
        self.pos += n;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_successful_time_cache() {
        let cache = parse_time_txt(
            b"timezone=America/New_York\nlast_sync_unix=1714963320\nlast_sync_monotonic_ms=5000\nlast_sync_ok=1\nlast_sync_source=ntp\n",
        );
        assert_eq!(cache.last_sync_unix, Some(1714963320));
        assert_eq!(cache.last_sync_uptime_secs, Some(5));
        assert!(cache.last_sync_ok);
    }

    #[test]
    fn invalid_epoch_is_unsynced() {
        let cache = parse_time_txt(b"last_sync_unix=bad\nlast_sync_ok=1\n");
        assert_eq!(cache.last_sync_unix, None);
        assert!(!cache.is_synced());
    }

    #[test]
    fn formats_eastern_date() {
        let cache = TimeCache::synced(1714963320, 0, None, "ntp");
        let mut out = String::new();
        cache.write_date_value(0, &mut out).unwrap();
        assert_eq!(out.as_str(), "Sun, May 5 2024");
    }

    #[test]
    fn synced_time_is_live_during_same_boot() {
        let cache = TimeCache::synced(1_714_963_320, 10, None, "ntp");
        assert_eq!(cache.freshness(70), ClockFreshness::Live);
        assert_eq!(cache.estimated_unix(70), Some(1_714_963_380));
    }

    #[test]
    fn synced_time_is_cached_after_reboot() {
        let cache = TimeCache::synced(1_714_963_320, 70, None, "ntp");
        assert_eq!(cache.freshness(3), ClockFreshness::Cached);
        assert_eq!(cache.estimated_unix(3), None);
        assert_eq!(cache.display_unix(3), Some(1_714_963_320));
    }

    #[test]
    fn converts_ntp_seconds() {
        assert_eq!(ntp_seconds_to_unix(2_208_988_801), Some(1));
    }

    #[test]
    fn rejects_short_ntp_response() {
        assert_eq!(parse_ntp_unix_seconds(&[0; 16]), None);
    }
}
