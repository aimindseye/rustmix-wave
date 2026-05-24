use serde::{Deserialize, Serialize};

use super::state::ReaderOrientationModel;

pub const X4_READER_PORTRAIT_WIDTH: u16 = 480;
pub const X4_READER_PORTRAIT_HEIGHT: u16 = 800;
pub const X4_READER_LANDSCAPE_WIDTH: u16 = 800;
pub const X4_READER_LANDSCAPE_HEIGHT: u16 = 480;
pub const X4_READER_HEADER_HEIGHT: u16 = 40;
pub const X4_READER_FOOTER_HEIGHT: u16 = 32;
pub const X4_READER_CONTENT_MARGIN: u16 = 16;
pub const X4_READER_CONTENT_TOP_GAP: u16 = 8;
pub const X4_READER_CONTENT_BOTTOM_GAP: u16 = 8;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReaderViewportPhysicalRotationModel {
    Deg0,
    Deg90,
    Deg180,
    #[default]
    Deg270,
}

impl ReaderViewportPhysicalRotationModel {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Deg0 => "Deg0",
            Self::Deg90 => "Deg90",
            Self::Deg180 => "Deg180",
            Self::Deg270 => "Deg270",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReaderViewportChromeBasisModel {
    #[default]
    PortraitTopBottom,
    LandscapeTopBottom,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReaderViewportRectModel {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl ReaderViewportRectModel {
    pub const fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub const fn right(self) -> u16 {
        self.x.saturating_add(self.width)
    }

    pub const fn bottom(self) -> u16 {
        self.y.saturating_add(self.height)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReaderViewportCacheKeyFieldsModel {
    pub orientation: ReaderOrientationModel,
    pub logical_width: u16,
    pub logical_height: u16,
    pub content_width: u16,
    pub content_height: u16,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReaderViewportModel {
    pub orientation: ReaderOrientationModel,
    pub logical_width: u16,
    pub logical_height: u16,
    pub physical_rotation: ReaderViewportPhysicalRotationModel,
    pub chrome_basis: ReaderViewportChromeBasisModel,
    pub header_region: ReaderViewportRectModel,
    pub content_region: ReaderViewportRectModel,
    pub footer_region: ReaderViewportRectModel,
}

impl ReaderViewportModel {
    pub const fn for_orientation(orientation: ReaderOrientationModel) -> Self {
        let (logical_width, logical_height, physical_rotation, chrome_basis) = match orientation {
            ReaderOrientationModel::Portrait => (
                X4_READER_PORTRAIT_WIDTH,
                X4_READER_PORTRAIT_HEIGHT,
                ReaderViewportPhysicalRotationModel::Deg270,
                ReaderViewportChromeBasisModel::PortraitTopBottom,
            ),
            ReaderOrientationModel::Inverted => (
                X4_READER_PORTRAIT_WIDTH,
                X4_READER_PORTRAIT_HEIGHT,
                ReaderViewportPhysicalRotationModel::Deg90,
                ReaderViewportChromeBasisModel::PortraitTopBottom,
            ),
            ReaderOrientationModel::LandscapeCw => (
                X4_READER_LANDSCAPE_WIDTH,
                X4_READER_LANDSCAPE_HEIGHT,
                ReaderViewportPhysicalRotationModel::Deg0,
                ReaderViewportChromeBasisModel::LandscapeTopBottom,
            ),
            ReaderOrientationModel::LandscapeCcw => (
                X4_READER_LANDSCAPE_WIDTH,
                X4_READER_LANDSCAPE_HEIGHT,
                ReaderViewportPhysicalRotationModel::Deg180,
                ReaderViewportChromeBasisModel::LandscapeTopBottom,
            ),
        };

        let header_region =
            ReaderViewportRectModel::new(0, 0, logical_width, X4_READER_HEADER_HEIGHT);
        let footer_region = ReaderViewportRectModel::new(
            0,
            logical_height.saturating_sub(X4_READER_FOOTER_HEIGHT),
            logical_width,
            X4_READER_FOOTER_HEIGHT,
        );
        let content_y = X4_READER_HEADER_HEIGHT.saturating_add(X4_READER_CONTENT_TOP_GAP);
        let content_bottom = footer_region.y.saturating_sub(X4_READER_CONTENT_BOTTOM_GAP);
        let content_region = ReaderViewportRectModel::new(
            X4_READER_CONTENT_MARGIN,
            content_y,
            logical_width.saturating_sub(X4_READER_CONTENT_MARGIN.saturating_mul(2)),
            content_bottom.saturating_sub(content_y),
        );

        Self {
            orientation,
            logical_width,
            logical_height,
            physical_rotation,
            chrome_basis,
            header_region,
            content_region,
            footer_region,
        }
    }

    pub const fn cache_key_fields(self) -> ReaderViewportCacheKeyFieldsModel {
        ReaderViewportCacheKeyFieldsModel {
            orientation: self.orientation,
            logical_width: self.logical_width,
            logical_height: self.logical_height,
            content_width: self.content_region.width,
            content_height: self.content_region.height,
        }
    }

    pub const fn is_landscape(self) -> bool {
        matches!(
            self.orientation,
            ReaderOrientationModel::LandscapeCw | ReaderOrientationModel::LandscapeCcw
        )
    }

    pub const fn is_enabled_for_reader_ui(self) -> bool {
        matches!(
            self.orientation,
            ReaderOrientationModel::Portrait
                | ReaderOrientationModel::Inverted
                | ReaderOrientationModel::LandscapeCw
                | ReaderOrientationModel::LandscapeCcw
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portrait_and_inverted_keep_current_reader_geometry() {
        let portrait = ReaderViewportModel::for_orientation(ReaderOrientationModel::Portrait);
        let inverted = ReaderViewportModel::for_orientation(ReaderOrientationModel::Inverted);

        assert_eq!(portrait.logical_width, 480);
        assert_eq!(portrait.logical_height, 800);
        assert_eq!(
            portrait.physical_rotation,
            ReaderViewportPhysicalRotationModel::Deg270
        );
        assert_eq!(inverted.logical_width, 480);
        assert_eq!(inverted.logical_height, 800);
        assert_eq!(
            inverted.physical_rotation,
            ReaderViewportPhysicalRotationModel::Deg90
        );
        assert!(portrait.is_enabled_for_reader_ui());
        assert!(inverted.is_enabled_for_reader_ui());
    }

    #[test]
    fn landscape_viewports_are_modeled_but_not_ui_enabled() {
        let cw = ReaderViewportModel::for_orientation(ReaderOrientationModel::LandscapeCw);
        let ccw = ReaderViewportModel::for_orientation(ReaderOrientationModel::LandscapeCcw);

        assert_eq!(cw.logical_width, 800);
        assert_eq!(cw.logical_height, 480);
        assert_eq!(
            cw.physical_rotation,
            ReaderViewportPhysicalRotationModel::Deg0
        );
        assert_eq!(
            ccw.physical_rotation,
            ReaderViewportPhysicalRotationModel::Deg180
        );
        assert!(cw.is_landscape());
        assert!(ccw.is_landscape());
        assert!(cw.is_enabled_for_reader_ui());
        assert!(ccw.is_enabled_for_reader_ui());
    }

    #[test]
    fn cache_key_fields_include_orientation_and_viewport_size() {
        let portrait = ReaderViewportModel::for_orientation(ReaderOrientationModel::Portrait)
            .cache_key_fields();
        let landscape = ReaderViewportModel::for_orientation(ReaderOrientationModel::LandscapeCw)
            .cache_key_fields();

        assert_ne!(portrait.orientation, landscape.orientation);
        assert_ne!(portrait.logical_width, landscape.logical_width);
        assert_ne!(portrait.logical_height, landscape.logical_height);
        assert_ne!(portrait.content_width, landscape.content_width);
    }
}
