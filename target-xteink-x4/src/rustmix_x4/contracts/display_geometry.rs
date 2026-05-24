#![allow(dead_code)]

pub struct RustmixDisplayGeometry;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RustmixDisplayOrientation {
    NativeLandscape,
    LogicalPortrait,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RustmixDisplaySize {
    pub width: u16,
    pub height: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RustmixDisplayPinContract {
    pub epd_cs_gpio: u8,
    pub epd_dc_gpio: u8,
    pub epd_rst_gpio: u8,
    pub epd_busy_gpio: u8,
    pub spi_sclk_gpio: u8,
    pub spi_mosi_gpio: u8,
    pub spi_miso_gpio: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RustmixSsd1677CommandContract {
    pub current_ram: u8,
    pub previous_ram: u8,
    pub update_control_2: u8,
    pub master_activate: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RustmixDisplayGeometryAdoptionReport {
    pub native_geometry_ok: bool,
    pub logical_geometry_ok: bool,
    pub rotation_ok: bool,
    pub strip_rows_ok: bool,
    pub commands_ok: bool,
    pub pins_ok: bool,
    pub physical_display_moved: bool,
}

impl RustmixDisplayGeometryAdoptionReport {
    pub const fn adoption_ok(self) -> bool {
        self.native_geometry_ok
            && self.logical_geometry_ok
            && self.rotation_ok
            && self.strip_rows_ok
            && self.commands_ok
            && self.pins_ok
            && !self.physical_display_moved
    }
}

impl RustmixDisplayGeometry {
    pub const IMPLEMENTATION_OWNER: &'static str = "Rustmix-owned pure display geometry helpers";
    pub const PHYSICAL_DISPLAY_OWNER: &'static str = "Rustmix-owned X4 runtime";
    pub const PHYSICAL_DISPLAY_MOVED_TO_BOUNDARY: bool = false;

    pub const NATIVE_WIDTH: u16 = 800;
    pub const NATIVE_HEIGHT: u16 = 480;
    pub const LOGICAL_WIDTH: u16 = 480;
    pub const LOGICAL_HEIGHT: u16 = 800;
    pub const ROTATION_DEGREES: u16 = 270;
    pub const STRIP_ROWS: u16 = 40;

    pub const EPD_CS_GPIO: u8 = 21;
    pub const EPD_DC_GPIO: u8 = 4;
    pub const EPD_RST_GPIO: u8 = 5;
    pub const EPD_BUSY_GPIO: u8 = 6;
    pub const SPI_SCLK_GPIO: u8 = 8;
    pub const SPI_MOSI_GPIO: u8 = 10;
    pub const SPI_MISO_GPIO: u8 = 7;

    pub const SSD1677_CURRENT_RAM_CMD: u8 = 0x24;
    pub const SSD1677_PREVIOUS_RAM_CMD: u8 = 0x26;
    pub const SSD1677_DISPLAY_UPDATE_CONTROL_2_CMD: u8 = 0x22;
    pub const SSD1677_MASTER_ACTIVATE_CMD: u8 = 0x20;

    pub const fn native_size() -> RustmixDisplaySize {
        RustmixDisplaySize {
            width: Self::NATIVE_WIDTH,
            height: Self::NATIVE_HEIGHT,
        }
    }

    pub const fn logical_size() -> RustmixDisplaySize {
        RustmixDisplaySize {
            width: Self::LOGICAL_WIDTH,
            height: Self::LOGICAL_HEIGHT,
        }
    }

    pub const fn orientation_for_size(size: RustmixDisplaySize) -> RustmixDisplayOrientation {
        if size.width > size.height {
            RustmixDisplayOrientation::NativeLandscape
        } else {
            RustmixDisplayOrientation::LogicalPortrait
        }
    }

    pub const fn pins() -> RustmixDisplayPinContract {
        RustmixDisplayPinContract {
            epd_cs_gpio: Self::EPD_CS_GPIO,
            epd_dc_gpio: Self::EPD_DC_GPIO,
            epd_rst_gpio: Self::EPD_RST_GPIO,
            epd_busy_gpio: Self::EPD_BUSY_GPIO,
            spi_sclk_gpio: Self::SPI_SCLK_GPIO,
            spi_mosi_gpio: Self::SPI_MOSI_GPIO,
            spi_miso_gpio: Self::SPI_MISO_GPIO,
        }
    }

    pub const fn ssd1677_commands() -> RustmixSsd1677CommandContract {
        RustmixSsd1677CommandContract {
            current_ram: Self::SSD1677_CURRENT_RAM_CMD,
            previous_ram: Self::SSD1677_PREVIOUS_RAM_CMD,
            update_control_2: Self::SSD1677_DISPLAY_UPDATE_CONTROL_2_CMD,
            master_activate: Self::SSD1677_MASTER_ACTIVATE_CMD,
        }
    }

    pub const fn is_known_ssd1677_command(command: u8) -> bool {
        matches!(
            command,
            Self::SSD1677_CURRENT_RAM_CMD
                | Self::SSD1677_PREVIOUS_RAM_CMD
                | Self::SSD1677_DISPLAY_UPDATE_CONTROL_2_CMD
                | Self::SSD1677_MASTER_ACTIVATE_CMD
        )
    }

    pub const fn strip_rows_align_with_native_height() -> bool {
        Self::NATIVE_HEIGHT % Self::STRIP_ROWS == 0
    }

    pub fn display_geometry_adoption_report() -> RustmixDisplayGeometryAdoptionReport {
        let native = Self::native_size();
        let logical = Self::logical_size();
        let pins = Self::pins();
        let commands = Self::ssd1677_commands();

        RustmixDisplayGeometryAdoptionReport {
            native_geometry_ok: native.width == 800
                && native.height == 480
                && Self::orientation_for_size(native) == RustmixDisplayOrientation::NativeLandscape,
            logical_geometry_ok: logical.width == 480
                && logical.height == 800
                && Self::orientation_for_size(logical)
                    == RustmixDisplayOrientation::LogicalPortrait,
            rotation_ok: Self::ROTATION_DEGREES == 270,
            strip_rows_ok: Self::STRIP_ROWS == 40 && Self::strip_rows_align_with_native_height(),
            commands_ok: commands.current_ram == 0x24
                && commands.previous_ram == 0x26
                && commands.update_control_2 == 0x22
                && commands.master_activate == 0x20
                && Self::is_known_ssd1677_command(0x24)
                && Self::is_known_ssd1677_command(0x26)
                && Self::is_known_ssd1677_command(0x22)
                && Self::is_known_ssd1677_command(0x20),
            pins_ok: pins.epd_cs_gpio == 21
                && pins.epd_dc_gpio == 4
                && pins.epd_rst_gpio == 5
                && pins.epd_busy_gpio == 6
                && pins.spi_sclk_gpio == 8
                && pins.spi_mosi_gpio == 10
                && pins.spi_miso_gpio == 7,
            physical_display_moved: Self::PHYSICAL_DISPLAY_MOVED_TO_BOUNDARY,
        }
    }

    pub fn active_runtime_adoption_probe() -> bool {
        Self::display_geometry_adoption_report().adoption_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::{RustmixDisplayGeometry, RustmixDisplayOrientation};

    #[test]
    fn display_geometry_adoption_probe_is_pure_and_valid() {
        assert!(RustmixDisplayGeometry::active_runtime_adoption_probe());
    }

    #[test]
    fn reports_expected_geometry_orientation() {
        assert_eq!(
            RustmixDisplayGeometry::orientation_for_size(RustmixDisplayGeometry::native_size()),
            RustmixDisplayOrientation::NativeLandscape
        );
        assert_eq!(
            RustmixDisplayGeometry::orientation_for_size(RustmixDisplayGeometry::logical_size()),
            RustmixDisplayOrientation::LogicalPortrait
        );
    }
}
