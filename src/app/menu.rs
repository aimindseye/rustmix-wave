//! Data-driven category menu definitions for the main product shell.
//!
//! Category rows contain applications only. Hierarchical navigation uses the
//! dedicated GPIO0 Boot-button long press instead of synthetic Back rows.

use super::router::ScreenRoute;

pub const MAIN_CATEGORY_COUNT: usize = 5;
pub const CATEGORY_COUNT: usize = 5;
pub const CATEGORY_PAGE_SIZE: usize = 6;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MenuEntry {
    pub label: &'static str,
    pub subtitle: &'static str,
    pub badge: &'static str,
    pub route: ScreenRoute,
}

const HOME_ENTRIES: [MenuEntry; MAIN_CATEGORY_COUNT] = [
    MenuEntry {
        label: "Reader",
        subtitle: "Books, progress and bookmarks",
        badge: "3",
        route: ScreenRoute::Reader,
    },
    MenuEntry {
        label: "Productivity",
        subtitle: "Calendar and voice notes",
        badge: "2",
        route: ScreenRoute::Productivity,
    },
    MenuEntry {
        label: "Games",
        subtitle: "E-paper friendly games",
        badge: "SD",
        route: ScreenRoute::Games,
    },
    MenuEntry {
        label: "Tools",
        subtitle: "Files, dictionary and conversion",
        badge: "3",
        route: ScreenRoute::Tools,
    },
    MenuEntry {
        label: "Settings",
        subtitle: "Device services and display",
        badge: "9",
        route: ScreenRoute::Settings,
    },
];

const READER_ENTRIES: [MenuEntry; 3] = [
    MenuEntry {
        label: "Continue Reading",
        subtitle: "Resume the last saved book",
        badge: "READY",
        route: ScreenRoute::ContinueReading,
    },
    MenuEntry {
        label: "Library",
        subtitle: "TXT and EPUB book library",
        badge: "READY",
        route: ScreenRoute::Library,
    },
    MenuEntry {
        label: "Bookmarks",
        subtitle: "Saved reading positions",
        badge: "READY",
        route: ScreenRoute::Bookmarks,
    },
];

const PRODUCTIVITY_ENTRIES: [MenuEntry; 2] = [
    MenuEntry {
        label: "Calendar",
        subtitle: "US agenda and personal editor",
        badge: "READY",
        route: ScreenRoute::Calendar,
    },
    MenuEntry {
        label: "Voice Notes",
        subtitle: "Record PCM WAV notes to SD",
        badge: "READY",
        route: ScreenRoute::VoiceNotes,
    },
];

const GAMES_ENTRIES: [MenuEntry; 1] = [MenuEntry {
    label: "SD Lua Apps",
    subtitle: "SD-loaded apps with native canvas",
    badge: "READY",
    route: ScreenRoute::LuaApps,
}];

const TOOLS_ENTRIES: [MenuEntry; 3] = [
    MenuEntry {
        label: "File Browser",
        subtitle: "Read-only SDMMC browser",
        badge: "READY",
        route: ScreenRoute::Files,
    },
    MenuEntry {
        label: "Dictionary",
        subtitle: "Offline prefix lookup",
        badge: "READY",
        route: ScreenRoute::Dictionary,
    },
    MenuEntry {
        label: "Unit Converter",
        subtitle: "Offline fixed-point conversions",
        badge: "READY",
        route: ScreenRoute::UnitConverter,
    },
];

const SETTINGS_ENTRIES: [MenuEntry; 9] = [
    MenuEntry {
        label: "Alarms",
        subtitle: "RTC schedules, snooze and dismiss",
        badge: "READY",
        route: ScreenRoute::Alarms,
    },
    MenuEntry {
        label: "Audio",
        subtitle: "ES8311 playback and alarm chime",
        badge: "READY",
        route: ScreenRoute::Audio,
    },
    MenuEntry {
        label: "Clock",
        subtitle: "RTC, power and localized time",
        badge: "READY",
        route: ScreenRoute::Clock,
    },
    MenuEntry {
        label: "Display",
        subtitle: "Global UI font and size",
        badge: "NEW",
        route: ScreenRoute::Display,
    },
    MenuEntry {
        label: "Device Info",
        subtitle: "Firmware and board ownership",
        badge: "READY",
        route: ScreenRoute::DeviceInfo,
    },
    MenuEntry {
        label: "Environment",
        subtitle: "SHTC3 temperature and humidity",
        badge: "READY",
        route: ScreenRoute::Environment,
    },
    MenuEntry {
        label: "Motion",
        subtitle: "QMI8658 accelerometer and gyroscope",
        badge: "READY",
        route: ScreenRoute::Motion,
    },
    MenuEntry {
        label: "Network",
        subtitle: "SD config, Wi-Fi and SNTP status",
        badge: "READY",
        route: ScreenRoute::Network,
    },
    MenuEntry {
        label: "Weather",
        subtitle: "Open-Meteo conditions and forecast",
        badge: "READY",
        route: ScreenRoute::Weather,
    },
];

#[must_use]
pub const fn home_entries() -> &'static [MenuEntry] {
    &HOME_ENTRIES
}

#[must_use]
pub const fn category_entries(route: ScreenRoute) -> &'static [MenuEntry] {
    match route {
        ScreenRoute::Reader => &READER_ENTRIES,
        ScreenRoute::Productivity => &PRODUCTIVITY_ENTRIES,
        ScreenRoute::Games => &GAMES_ENTRIES,
        ScreenRoute::Tools => &TOOLS_ENTRIES,
        ScreenRoute::Settings => &SETTINGS_ENTRIES,
        _ => &[],
    }
}

#[must_use]
pub const fn category_index(route: ScreenRoute) -> Option<usize> {
    match route {
        ScreenRoute::Reader => Some(0),
        ScreenRoute::Productivity => Some(1),
        ScreenRoute::Games => Some(2),
        ScreenRoute::Tools => Some(3),
        ScreenRoute::Settings => Some(4),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{category_entries, home_entries, MAIN_CATEGORY_COUNT};
    use crate::app::router::ScreenRoute;

    #[test]
    fn exposes_requested_main_category_counts_without_synthetic_back_rows() {
        assert_eq!(home_entries().len(), MAIN_CATEGORY_COUNT);
        assert_eq!(category_entries(ScreenRoute::Reader).len(), 3);
        assert_eq!(category_entries(ScreenRoute::Productivity).len(), 2);
        assert_eq!(category_entries(ScreenRoute::Games).len(), 1);
        assert_eq!(category_entries(ScreenRoute::Tools).len(), 3);
        assert_eq!(category_entries(ScreenRoute::Settings).len(), 9);
        for route in [
            ScreenRoute::Reader,
            ScreenRoute::Productivity,
            ScreenRoute::Games,
            ScreenRoute::Tools,
            ScreenRoute::Settings,
        ] {
            assert!(category_entries(route)
                .iter()
                .all(|entry| entry.route != ScreenRoute::Home));
        }
    }

    #[test]
    fn reader_contains_ready_txt_library() {
        let reader = category_entries(ScreenRoute::Reader);
        for route in [
            ScreenRoute::ContinueReading,
            ScreenRoute::Library,
            ScreenRoute::Bookmarks,
        ] {
            let entry = reader
                .iter()
                .find(|entry| entry.route == route)
                .expect("Reader entry");
            assert_eq!(entry.badge, "READY");
        }
    }

    #[test]
    fn tools_contains_ready_dictionary() {
        let tools = category_entries(ScreenRoute::Tools);
        let dictionary = tools
            .iter()
            .find(|entry| entry.route == ScreenRoute::Dictionary)
            .expect("Dictionary entry");
        assert_eq!(dictionary.badge, "READY");
    }

    #[test]
    fn tools_contains_ready_unit_converter() {
        let tools = category_entries(ScreenRoute::Tools);
        let converter = tools
            .iter()
            .find(|entry| entry.route == ScreenRoute::UnitConverter)
            .expect("Unit Converter entry");
        assert_eq!(converter.badge, "READY");
    }

    #[test]
    fn games_contains_ready_sd_lua_catalog() {
        let games = category_entries(ScreenRoute::Games);
        let catalog = games
            .iter()
            .find(|entry| entry.route == ScreenRoute::LuaApps)
            .expect("SD Lua Apps entry");
        assert_eq!(catalog.badge, "READY");
    }

    #[test]
    fn settings_contains_display_before_existing_diagnostics() {
        let settings = category_entries(ScreenRoute::Settings);
        assert!(settings
            .iter()
            .any(|entry| entry.route == ScreenRoute::Display));
        assert!(settings
            .iter()
            .any(|entry| entry.route == ScreenRoute::Weather));
    }
}
