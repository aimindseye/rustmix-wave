//! Category dashboard model for the Rustmix X4 launcher.
//!
//! This module is intentionally UI-framework independent. It only owns the
//! category/item model, selection state, and semantic routes. The existing
//! Home/File Browser/Reader/Bookmarks/Settings implementations
//! should continue to own their current behavior.

#![allow(dead_code)]

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DashboardCategory {
    Network,
    Productivity,
    Games,
    Reader,
    System,
    Tools,
}

impl DashboardCategory {
    pub const fn title(self) -> &'static str {
        match self {
            Self::Network => "Network",
            Self::Productivity => "Productivity",
            Self::Games => "Games",
            Self::Reader => "Reader",
            Self::System => "System",
            Self::Tools => "Tools",
        }
    }

    pub const fn items(self) -> &'static [DashboardItem] {
        match self {
            Self::Network => &NETWORK_ITEMS,
            Self::Productivity => &PRODUCTIVITY_ITEMS,
            Self::Games => &GAMES_ITEMS,
            Self::Reader => &READER_ITEMS,
            Self::System => &SYSTEM_ITEMS,
            Self::Tools => &TOOLS_ITEMS,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DashboardItemKind {
    ExistingFlow,
    Placeholder,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DashboardRoute {
    WifiConnectPlaceholder,
    WifiTransferPlaceholder,
    NetworkStatusPlaceholder,
    Calendar,
    Flashcards,
    GamesComingSoonPlaceholder,
    ContinueReading,
    Library,
    Bookmarks,
    Settings,
    SleepImagePlaceholder,
    DeviceInfoPlaceholder,
    FileBrowser,
    Dictionary,
    UnitConverter,
}

impl DashboardRoute {
    pub const fn placeholder_title(self) -> &'static str {
        match self {
            Self::WifiConnectPlaceholder => "Wi-Fi Connect",
            Self::WifiTransferPlaceholder => "Wi-Fi Transfer",
            Self::NetworkStatusPlaceholder => "Network Status",
            Self::GamesComingSoonPlaceholder => "Coming soon",
            Self::SleepImagePlaceholder => "Sleep Image",
            Self::DeviceInfoPlaceholder => "Device Info",
            Self::Calendar
            | Self::Flashcards
            | Self::ContinueReading
            | Self::Library
            | Self::Bookmarks
            | Self::Settings
            | Self::FileBrowser
            | Self::Dictionary
            | Self::UnitConverter => "",
        }
    }

    pub const fn is_placeholder(self) -> bool {
        matches!(
            self,
            Self::WifiConnectPlaceholder
                | Self::WifiTransferPlaceholder
                | Self::NetworkStatusPlaceholder
                | Self::GamesComingSoonPlaceholder
                | Self::SleepImagePlaceholder
                | Self::DeviceInfoPlaceholder

        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DashboardItem {
    pub title: &'static str,
    pub kind: DashboardItemKind,
    pub route: DashboardRoute,
}

impl DashboardItem {
    pub const fn existing(title: &'static str, route: DashboardRoute) -> Self {
        Self {
            title,
            kind: DashboardItemKind::ExistingFlow,
            route,
        }
    }

    pub const fn placeholder(title: &'static str, route: DashboardRoute) -> Self {
        Self {
            title,
            kind: DashboardItemKind::Placeholder,
            route,
        }
    }

    pub const fn status_label(self) -> &'static str {
        match self.kind {
            DashboardItemKind::ExistingFlow => "Ready",
            DashboardItemKind::Placeholder => "Placeholder",
        }
    }
}

pub const DASHBOARD_CATEGORIES: [DashboardCategory; 6] = [
    DashboardCategory::Network,
    DashboardCategory::Productivity,
    DashboardCategory::Games,
    DashboardCategory::Reader,
    DashboardCategory::System,
    DashboardCategory::Tools,
];

pub const NETWORK_ITEMS: [DashboardItem; 3] = [
    DashboardItem::placeholder("Wi-Fi Connect", DashboardRoute::WifiConnectPlaceholder),
    DashboardItem::placeholder("Wi-Fi Transfer", DashboardRoute::WifiTransferPlaceholder),
    DashboardItem::placeholder("Network Status", DashboardRoute::NetworkStatusPlaceholder),
];

pub const PRODUCTIVITY_ITEMS: [DashboardItem; 2] = [
    DashboardItem::existing("Calendar", DashboardRoute::Calendar),
    DashboardItem::existing("Flashcards", DashboardRoute::Flashcards),
];

pub const GAMES_ITEMS: [DashboardItem; 1] = [DashboardItem::placeholder(
    "Coming soon",
    DashboardRoute::GamesComingSoonPlaceholder,
)];

pub const READER_ITEMS: [DashboardItem; 3] = [
    DashboardItem::existing("Continue Reading", DashboardRoute::ContinueReading),
    DashboardItem::existing("Library", DashboardRoute::Library),
    DashboardItem::existing("Bookmarks", DashboardRoute::Bookmarks),
];

pub const SYSTEM_ITEMS: [DashboardItem; 3] = [
    DashboardItem::existing("Settings", DashboardRoute::Settings),
    DashboardItem::placeholder("Sleep Image", DashboardRoute::SleepImagePlaceholder),
    DashboardItem::placeholder("Device Info", DashboardRoute::DeviceInfoPlaceholder),
];

pub const TOOLS_ITEMS: [DashboardItem; 3] = [
    DashboardItem::existing("File Browser", DashboardRoute::FileBrowser),
    DashboardItem::existing("Dictionary", DashboardRoute::Dictionary),
    DashboardItem::existing("Unit Converter", DashboardRoute::UnitConverter),
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DashboardCommand {
    None,
    Open(DashboardRoute),
    Back,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DashboardNav {
    Up,
    Down,
    Left,
    Right,
    Select,
    Back,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AppCategoryDashboard {
    selected_category: usize,
    selected_item: usize,
}

impl Default for AppCategoryDashboard {
    fn default() -> Self {
        Self::new()
    }
}

impl AppCategoryDashboard {
    pub const fn new() -> Self {
        Self {
            selected_category: 0,
            selected_item: 0,
        }
    }

    pub const fn selected_category_index(&self) -> usize {
        self.selected_category
    }

    pub const fn selected_item_index(&self) -> usize {
        self.selected_item
    }

    pub fn selected_category(&self) -> DashboardCategory {
        DASHBOARD_CATEGORIES[self.selected_category]
    }

    pub fn selected_items(&self) -> &'static [DashboardItem] {
        self.selected_category().items()
    }

    pub fn selected_item(&self) -> DashboardItem {
        let items = self.selected_items();
        items[self.selected_item.min(items.len().saturating_sub(1))]
    }

    pub fn handle_nav(&mut self, nav: DashboardNav) -> DashboardCommand {
        match nav {
            DashboardNav::Up => {
                self.move_item_up();
                DashboardCommand::None
            }
            DashboardNav::Down => {
                self.move_item_down();
                DashboardCommand::None
            }
            DashboardNav::Left => {
                self.move_category_left();
                DashboardCommand::None
            }
            DashboardNav::Right => {
                self.move_category_right();
                DashboardCommand::None
            }
            DashboardNav::Select => DashboardCommand::Open(self.selected_item().route),
            DashboardNav::Back => DashboardCommand::Back,
        }
    }

    pub fn move_category_left(&mut self) {
        if self.selected_category == 0 {
            self.selected_category = DASHBOARD_CATEGORIES.len() - 1;
        } else {
            self.selected_category -= 1;
        }
        self.normalize_item_index();
    }

    pub fn move_category_right(&mut self) {
        self.selected_category = (self.selected_category + 1) % DASHBOARD_CATEGORIES.len();
        self.normalize_item_index();
    }

    pub fn move_item_up(&mut self) {
        if self.selected_item == 0 {
            self.selected_item = self.selected_items().len().saturating_sub(1);
        } else {
            self.selected_item -= 1;
        }
    }

    pub fn move_item_down(&mut self) {
        let item_count = self.selected_items().len();
        if item_count > 0 {
            self.selected_item = (self.selected_item + 1) % item_count;
        }
    }

    fn normalize_item_index(&mut self) {
        let max_item = self.selected_items().len().saturating_sub(1);
        if self.selected_item > max_item {
            self.selected_item = max_item;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_requested_category_order() {
        let titles = DASHBOARD_CATEGORIES.map(DashboardCategory::title);
        assert_eq!(
            titles,
            ["Network", "Productivity", "Games", "Reader", "System", "Tools"]
        );
    }

    #[test]
    fn exposes_requested_reader_entries() {
        let titles = READER_ITEMS.map(|item| item.title);
        assert_eq!(titles, ["Continue Reading", "Library", "Bookmarks"]);
    }

    #[test]
    fn select_returns_semantic_route() {
        let mut dashboard = AppCategoryDashboard::new();
        dashboard.move_category_right();
        assert_eq!(dashboard.selected_category(), DashboardCategory::Productivity);
        assert_eq!(
            dashboard.handle_nav(DashboardNav::Select),
            DashboardCommand::Open(DashboardRoute::Calendar)
        );
    }
}
