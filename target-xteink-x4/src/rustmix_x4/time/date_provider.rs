use super::weekday::Weekday;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DateTrust {
    Unknown,
    UserSet,
    SyncProvided,
    NetworkProvided,
    BuildFallback,
}

impl DateTrust {
    pub const fn is_trusted(self) -> bool {
        match self {
            Self::Unknown | Self::BuildFallback => false,
            Self::UserSet | Self::SyncProvided | Self::NetworkProvided => true,
        }
    }

    pub const fn source_name(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::UserSet => "user-set",
            Self::SyncProvided => "sync-provided",
            Self::NetworkProvided => "network-provided",
            Self::BuildFallback => "build-fallback",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CalendarDate {
    pub year: u16,
    pub month: u8,
    pub day: u8,
}

impl CalendarDate {
    pub const fn new(year: u16, month: u8, day: u8) -> Self {
        Self { year, month, day }
    }

    pub const fn is_valid(self) -> bool {
        if self.year < 1970 || self.month < 1 || self.month > 12 || self.day < 1 {
            return false;
        }

        self.day <= days_in_month(self.year, self.month)
    }

    pub const fn weekday(self) -> Option<Weekday> {
        if !self.is_valid() {
            return None;
        }

        Some(weekday_for_date(self.year, self.month, self.day))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResolvedDate {
    pub date: Option<CalendarDate>,
    pub trust: DateTrust,
}

impl ResolvedDate {
    pub const fn unknown() -> Self {
        Self {
            date: None,
            trust: DateTrust::Unknown,
        }
    }

    pub const fn new(date: CalendarDate, trust: DateTrust) -> Self {
        Self {
            date: Some(date),
            trust,
        }
    }

    pub const fn trusted_weekday(self) -> Option<Weekday> {
        if !self.trust.is_trusted() {
            return None;
        }

        match self.date {
            Some(date) => date.weekday(),
            None => None,
        }
    }
}

pub trait DateProvider {
    fn resolved_date(&self) -> ResolvedDate;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FixedDateProvider {
    resolved: ResolvedDate,
}

impl FixedDateProvider {
    pub const fn unknown() -> Self {
        Self {
            resolved: ResolvedDate::unknown(),
        }
    }

    pub const fn new(date: CalendarDate, trust: DateTrust) -> Self {
        Self {
            resolved: ResolvedDate::new(date, trust),
        }
    }
}

impl DateProvider for FixedDateProvider {
    fn resolved_date(&self) -> ResolvedDate {
        self.resolved
    }
}

pub const fn is_leap_year(year: u16) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

pub const fn days_in_month(year: u16, month: u8) -> u8 {
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
        _ => 0,
    }
}

pub const fn weekday_for_date(year: u16, month: u8, day: u8) -> Weekday {
    let mut y = year as i32;
    let mut m = month as i32;
    let d = day as i32;

    if m < 3 {
        m += 12;
        y -= 1;
    }

    let k = y % 100;
    let j = y / 100;
    let h = (d + ((13 * (m + 1)) / 5) + k + (k / 4) + (j / 4) + (5 * j)) % 7;

    match h {
        0 => Weekday::Saturday,
        1 => Weekday::Sunday,
        2 => Weekday::Monday,
        3 => Weekday::Tuesday,
        4 => Weekday::Wednesday,
        5 => Weekday::Thursday,
        _ => Weekday::Friday,
    }
}

#[cfg(test)]
mod tests {
    use super::{CalendarDate, DateProvider, DateTrust, FixedDateProvider, Weekday};

    #[test]
    fn date_validation_rejects_invalid_month_and_day() {
        assert!(!CalendarDate::new(2026, 0, 1).is_valid());
        assert!(!CalendarDate::new(2026, 13, 1).is_valid());
        assert!(!CalendarDate::new(2026, 2, 29).is_valid());
        assert!(CalendarDate::new(2024, 2, 29).is_valid());
    }

    #[test]
    fn weekday_calculation_matches_known_dates() {
        assert_eq!(
            CalendarDate::new(2026, 5, 5).weekday(),
            Some(Weekday::Tuesday)
        );
        assert_eq!(
            CalendarDate::new(2026, 5, 4).weekday(),
            Some(Weekday::Monday)
        );
        assert_eq!(
            CalendarDate::new(2026, 5, 10).weekday(),
            Some(Weekday::Sunday)
        );
    }

    #[test]
    fn trusted_weekday_requires_trusted_date_source() {
        let trusted =
            FixedDateProvider::new(CalendarDate::new(2026, 5, 5), DateTrust::SyncProvided);
        assert_eq!(
            trusted.resolved_date().trusted_weekday(),
            Some(Weekday::Tuesday)
        );

        let unknown = FixedDateProvider::new(CalendarDate::new(2026, 5, 5), DateTrust::Unknown);
        assert_eq!(unknown.resolved_date().trusted_weekday(), None);
    }
}
