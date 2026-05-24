use serde::{Deserialize, Serialize};

pub const X4_SCREEN_WIDTH: u16 = 800;
pub const X4_SCREEN_HEIGHT: u16 = 480;
pub const X4_HEADER_HEIGHT: u16 = 32;
pub const X4_FOOTER_HEIGHT: u16 = 28;
pub const X4_BODY_Y: i16 = X4_HEADER_HEIGHT as i16;
pub const X4_BODY_HEIGHT: u16 = X4_SCREEN_HEIGHT - X4_HEADER_HEIGHT - X4_FOOTER_HEIGHT;
pub const X4_FOOTER_Y: i16 = (X4_SCREEN_HEIGHT - X4_FOOTER_HEIGHT) as i16;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisplaySizeModel {
    pub width: u16,
    pub height: u16,
}

impl DisplaySizeModel {
    pub const fn new(width: u16, height: u16) -> Self {
        Self { width, height }
    }

    pub const fn x4_screen() -> Self {
        Self::new(X4_SCREEN_WIDTH, X4_SCREEN_HEIGHT)
    }

    pub fn contains_region(self, region: DisplayRegionModel) -> bool {
        region.x >= 0
            && region.y >= 0
            && region.right() <= self.width as i16
            && region.bottom() <= self.height as i16
    }
}

impl Default for DisplaySizeModel {
    fn default() -> Self {
        Self::x4_screen()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisplayPointModel {
    pub x: i16,
    pub y: i16,
}

impl DisplayPointModel {
    pub const fn new(x: i16, y: i16) -> Self {
        Self { x, y }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisplayRegionModel {
    pub x: i16,
    pub y: i16,
    pub width: u16,
    pub height: u16,
}

impl DisplayRegionModel {
    pub const fn new(x: i16, y: i16, width: u16, height: u16) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub const fn empty() -> Self {
        Self::new(0, 0, 0, 0)
    }

    pub fn right(self) -> i16 {
        self.x.saturating_add(self.width as i16)
    }

    pub fn bottom(self) -> i16 {
        self.y.saturating_add(self.height as i16)
    }

    pub const fn is_empty(self) -> bool {
        self.width == 0 || self.height == 0
    }

    pub fn contains_point(self, point: DisplayPointModel) -> bool {
        point.x >= self.x && point.y >= self.y && point.x < self.right() && point.y < self.bottom()
    }

    pub fn intersects(self, other: Self) -> bool {
        !self.is_empty()
            && !other.is_empty()
            && self.x < other.right()
            && self.right() > other.x
            && self.y < other.bottom()
            && self.bottom() > other.y
    }

    pub fn intersection(self, other: Self) -> Option<Self> {
        if !self.intersects(other) {
            return None;
        }

        let left = self.x.max(other.x);
        let top = self.y.max(other.y);
        let right = self.right().min(other.right());
        let bottom = self.bottom().min(other.bottom());
        Some(Self::new(
            left,
            top,
            (right - left) as u16,
            (bottom - top) as u16,
        ))
    }

    pub fn clipped_to_screen(self, screen: DisplaySizeModel) -> Option<Self> {
        self.intersection(DisplayRegionModel::new(0, 0, screen.width, screen.height))
    }

    pub fn is_inside(self, screen: DisplaySizeModel) -> bool {
        screen.contains_region(self)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DisplayOrientationModel {
    Landscape0,
    Landscape180,
    Portrait90,
    #[default]
    X4Portrait270,
}

impl DisplayOrientationModel {
    pub const fn is_x4_default(self) -> bool {
        matches!(self, Self::X4Portrait270)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DisplayAppContextLayoutModel {
    #[default]
    HomeDashboard,
    FilesLibrary,
    Reader,
    Settings,
    DateTime,
    WifiTransfer,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DisplayChromeRoleModel {
    #[default]
    Body,
    Header,
    FooterStatus,
    BatteryStatus,
    ReaderProgress,
    PopupMessage,
    CacheDiagnostic,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DisplayDiagnosticPlacementModel {
    Header,
    #[default]
    BodyNotice,
    PopupMessage,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisplayChromeLayoutModel {
    pub context: DisplayAppContextLayoutModel,
    pub screen: DisplaySizeModel,
    pub orientation: DisplayOrientationModel,
    pub header: DisplayRegionModel,
    pub body: DisplayRegionModel,
    pub footer_status: DisplayRegionModel,
    pub battery_status: DisplayRegionModel,
    pub reader_progress: DisplayRegionModel,
    pub popup_message: DisplayRegionModel,
    pub cache_diagnostic: DisplayRegionModel,
}

impl DisplayChromeLayoutModel {
    pub const fn x4_base(context: DisplayAppContextLayoutModel) -> Self {
        Self {
            context,
            screen: DisplaySizeModel::new(X4_SCREEN_WIDTH, X4_SCREEN_HEIGHT),
            orientation: DisplayOrientationModel::X4Portrait270,
            header: DisplayRegionModel::new(0, 0, X4_SCREEN_WIDTH, X4_HEADER_HEIGHT),
            body: DisplayRegionModel::new(0, X4_BODY_Y, X4_SCREEN_WIDTH, X4_BODY_HEIGHT),
            footer_status: DisplayRegionModel::new(
                0,
                X4_FOOTER_Y,
                X4_SCREEN_WIDTH,
                X4_FOOTER_HEIGHT,
            ),
            battery_status: DisplayRegionModel::new(734, 4, 58, 22),
            reader_progress: DisplayRegionModel::new(8, X4_FOOTER_Y + 4, 300, 20),
            popup_message: DisplayRegionModel::new(64, 96, 672, 160),
            cache_diagnostic: DisplayRegionModel::new(8, X4_BODY_Y + 4, 784, 24),
        }
    }

    pub const fn reader() -> Self {
        Self::x4_base(DisplayAppContextLayoutModel::Reader)
    }

    pub const fn home_dashboard() -> Self {
        Self::x4_base(DisplayAppContextLayoutModel::HomeDashboard)
    }

    pub const fn files_library() -> Self {
        Self::x4_base(DisplayAppContextLayoutModel::FilesLibrary)
    }

    pub const fn settings() -> Self {
        Self::x4_base(DisplayAppContextLayoutModel::Settings)
    }

    pub const fn date_time() -> Self {
        Self::x4_base(DisplayAppContextLayoutModel::DateTime)
    }

    pub const fn wifi_transfer() -> Self {
        Self::x4_base(DisplayAppContextLayoutModel::WifiTransfer)
    }

    pub fn region_for_role(self, role: DisplayChromeRoleModel) -> DisplayRegionModel {
        match role {
            DisplayChromeRoleModel::Header => self.header,
            DisplayChromeRoleModel::Body => self.body,
            DisplayChromeRoleModel::FooterStatus => self.footer_status,
            DisplayChromeRoleModel::BatteryStatus => self.battery_status,
            DisplayChromeRoleModel::ReaderProgress => self.reader_progress,
            DisplayChromeRoleModel::PopupMessage => self.popup_message,
            DisplayChromeRoleModel::CacheDiagnostic => self.cache_diagnostic,
        }
    }

    pub fn all_regions_inside_screen(self) -> bool {
        self.header.is_inside(self.screen)
            && self.body.is_inside(self.screen)
            && self.footer_status.is_inside(self.screen)
            && self.battery_status.is_inside(self.screen)
            && self.reader_progress.is_inside(self.screen)
            && self.popup_message.is_inside(self.screen)
            && self.cache_diagnostic.is_inside(self.screen)
    }

    pub fn cache_diagnostic_uses_header(self) -> bool {
        self.cache_diagnostic.intersects(self.header)
    }

    pub fn popup_overlaps_critical_chrome(self) -> bool {
        self.popup_message.intersects(self.header)
            || self.popup_message.intersects(self.footer_status)
    }
}

impl Default for DisplayChromeLayoutModel {
    fn default() -> Self {
        Self::reader()
    }
}

pub const fn x4_screen_size() -> DisplaySizeModel {
    DisplaySizeModel::new(X4_SCREEN_WIDTH, X4_SCREEN_HEIGHT)
}

pub const fn layout_for_context(context: DisplayAppContextLayoutModel) -> DisplayChromeLayoutModel {
    match context {
        DisplayAppContextLayoutModel::HomeDashboard => DisplayChromeLayoutModel::home_dashboard(),
        DisplayAppContextLayoutModel::FilesLibrary => DisplayChromeLayoutModel::files_library(),
        DisplayAppContextLayoutModel::Reader => DisplayChromeLayoutModel::reader(),
        DisplayAppContextLayoutModel::Settings => DisplayChromeLayoutModel::settings(),
        DisplayAppContextLayoutModel::DateTime => DisplayChromeLayoutModel::date_time(),
        DisplayAppContextLayoutModel::WifiTransfer => DisplayChromeLayoutModel::wifi_transfer(),
    }
}

pub fn safe_clip_region(
    region: DisplayRegionModel,
    screen: DisplaySizeModel,
) -> Option<DisplayRegionModel> {
    region.clipped_to_screen(screen)
}

pub const fn cache_diagnostic_placement() -> DisplayDiagnosticPlacementModel {
    DisplayDiagnosticPlacementModel::BodyNotice
}

pub fn reader_cache_diagnostic_region() -> DisplayRegionModel {
    DisplayChromeLayoutModel::reader().cache_diagnostic
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn x4_screen_is_800_by_480_with_portrait_metadata() {
        let layout = DisplayChromeLayoutModel::reader();
        assert_eq!(layout.screen, DisplaySizeModel::new(800, 480));
        assert_eq!(layout.orientation, DisplayOrientationModel::X4Portrait270);
        assert!(layout.orientation.is_x4_default());
    }

    #[test]
    fn reader_header_body_and_footer_do_not_overlap() {
        let layout = DisplayChromeLayoutModel::reader();
        assert!(!layout.header.intersects(layout.body));
        assert!(!layout.body.intersects(layout.footer_status));
        assert!(!layout.header.intersects(layout.footer_status));
        assert_eq!(layout.header.bottom(), layout.body.y);
        assert_eq!(layout.body.bottom(), layout.footer_status.y);
    }

    #[test]
    fn cache_diagnostics_are_not_placed_in_header() {
        let layout = DisplayChromeLayoutModel::reader();
        assert_eq!(
            cache_diagnostic_placement(),
            DisplayDiagnosticPlacementModel::BodyNotice
        );
        assert!(!layout.cache_diagnostic_uses_header());
        assert!(layout.cache_diagnostic.intersects(layout.body));
    }

    #[test]
    fn battery_and_reader_progress_are_consistent_with_chrome() {
        let layout = DisplayChromeLayoutModel::reader();
        assert!(layout.battery_status.intersects(layout.header));
        assert!(layout.reader_progress.intersects(layout.footer_status));
        assert!(!layout.battery_status.intersects(layout.body));
    }

    #[test]
    fn popup_message_region_does_not_overlap_critical_chrome() {
        let layout = DisplayChromeLayoutModel::settings();
        assert!(!layout.popup_overlaps_critical_chrome());
        assert!(layout.popup_message.intersects(layout.body));
    }

    #[test]
    fn all_context_layouts_are_inside_800_by_480_bounds() {
        for context in [
            DisplayAppContextLayoutModel::HomeDashboard,
            DisplayAppContextLayoutModel::FilesLibrary,
            DisplayAppContextLayoutModel::Reader,
            DisplayAppContextLayoutModel::Settings,
            DisplayAppContextLayoutModel::DateTime,
            DisplayAppContextLayoutModel::WifiTransfer,
        ] {
            let layout = layout_for_context(context);
            assert_eq!(layout.screen, x4_screen_size());
            assert!(layout.all_regions_inside_screen());
        }
    }

    #[test]
    fn safe_clipping_intersects_screen_bounds() {
        let clipped = safe_clip_region(
            DisplayRegionModel::new(790, 470, 40, 40),
            DisplaySizeModel::x4_screen(),
        )
        .unwrap();
        assert_eq!(clipped, DisplayRegionModel::new(790, 470, 10, 10));
    }

    #[test]
    fn role_lookup_returns_expected_regions() {
        let layout = DisplayChromeLayoutModel::reader();
        assert_eq!(
            layout.region_for_role(DisplayChromeRoleModel::Header),
            layout.header
        );
        assert_eq!(
            layout.region_for_role(DisplayChromeRoleModel::Body),
            layout.body
        );
        assert_eq!(
            layout.region_for_role(DisplayChromeRoleModel::CacheDiagnostic),
            layout.cache_diagnostic
        );
    }
}
