#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Weekday {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

impl Weekday {
    pub const fn short_name(self) -> &'static str {
        match self {
            Self::Monday => "mon",
            Self::Tuesday => "tue",
            Self::Wednesday => "wed",
            Self::Thursday => "thu",
            Self::Friday => "fri",
            Self::Saturday => "sat",
            Self::Sunday => "sun",
        }
    }

    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Monday => "Monday",
            Self::Tuesday => "Tuesday",
            Self::Wednesday => "Wednesday",
            Self::Thursday => "Thursday",
            Self::Friday => "Friday",
            Self::Saturday => "Saturday",
            Self::Sunday => "Sunday",
        }
    }

    pub const fn index_from_monday(self) -> u8 {
        match self {
            Self::Monday => 0,
            Self::Tuesday => 1,
            Self::Wednesday => 2,
            Self::Thursday => 3,
            Self::Friday => 4,
            Self::Saturday => 5,
            Self::Sunday => 6,
        }
    }

    pub const fn from_monday_index(index: u8) -> Option<Self> {
        match index {
            0 => Some(Self::Monday),
            1 => Some(Self::Tuesday),
            2 => Some(Self::Wednesday),
            3 => Some(Self::Thursday),
            4 => Some(Self::Friday),
            5 => Some(Self::Saturday),
            6 => Some(Self::Sunday),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Weekday;

    #[test]
    fn weekday_names_are_stable_for_sleep_image_paths() {
        assert_eq!(Weekday::Monday.short_name(), "mon");
        assert_eq!(Weekday::Sunday.short_name(), "sun");
        assert_eq!(Weekday::Wednesday.display_name(), "Wednesday");
    }

    #[test]
    fn weekday_round_trip_preserves_monday_based_index() {
        for index in 0..=6 {
            let weekday = Weekday::from_monday_index(index).expect("weekday index");
            assert_eq!(weekday.index_from_monday(), index);
        }
        assert_eq!(Weekday::from_monday_index(7), None);
    }
}
