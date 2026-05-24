#![allow(dead_code)]

/// Rustmix-owned display boundary metadata for the Adafruit Xteink X4.
///
/// The current implementation intentionally does not move physical SSD1677/SPI/display refresh
/// behavior out of the imported X4/X4 runtime. This module makes the display
/// contract explicit so a later implementation can extract one behavior at a time with
/// checks around geometry, pins, rotation, RAM commands, and shared-bus rules.
#[cfg(target_arch = "riscv32")]
pub struct RustmixDisplayBoundary;

#[cfg(target_arch = "riscv32")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DisplayRuntimeOwner {
    /// The current working implementation remains the imported X4/X4 runtime.
    ImportedX4Runtime,
}

#[cfg(target_arch = "riscv32")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X4DisplayPins {
    pub epd_cs_gpio: u8,
    pub epd_dc_gpio: u8,
    pub epd_rst_gpio: u8,
    pub epd_busy_gpio: u8,
    pub spi_sclk_gpio: u8,
    pub spi_mosi_gpio: u8,
    pub spi_miso_gpio: u8,
}

#[cfg(target_arch = "riscv32")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X4DisplayGeometry {
    pub native_width: u16,
    pub native_height: u16,
    pub logical_width: u16,
    pub logical_height: u16,
    pub rotation_degrees: u16,
    pub strip_rows: u16,
}

#[cfg(target_arch = "riscv32")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ssd1677RefreshContract {
    pub current_ram_command: u8,
    pub previous_ram_command: u8,
    pub display_update_control_2_command: u8,
    pub master_activation_command: u8,
    pub physical_refresh_moved_to_boundary: bool,
}

#[cfg(target_arch = "riscv32")]
impl RustmixDisplayBoundary {
    pub const DISPLAY_BOUNDARY_MARKER: &'static str = "x4-display-boundary-ok";
    pub const BOUNDARY_SCAFFOLD_MARKER: &'static str = "x4-boundary-scaffold-ok";

    /// Current source of truth for display behavior.
    pub const IMPLEMENTATION_OWNER: &'static str = "Rustmix-owned X4 runtime";
    pub const RUNTIME_OWNER: DisplayRuntimeOwner = DisplayRuntimeOwner::ImportedX4Runtime;

    /// The current implementation records the contract only. It must not move physical display IO.
    pub const PHYSICAL_DISPLAY_INIT_MOVED_TO_BOUNDARY: bool = false;
    pub const PHYSICAL_DISPLAY_REFRESH_MOVED_TO_BOUNDARY: bool = false;
    pub const SSD1677_SPI_TRANSACTIONS_MOVED_TO_BOUNDARY: bool = false;
    pub const FRAMEBUFFER_OR_STRIP_RENDER_MOVED_TO_BOUNDARY: bool = false;

    /// Xteink X4 e-paper pins.
    pub const EPD_CS_GPIO: u8 = 21;
    pub const EPD_DC_GPIO: u8 = 4;
    pub const EPD_RST_GPIO: u8 = 5;
    pub const EPD_BUSY_GPIO: u8 = 6;

    /// Shared SPI bus pins used by both SSD1677 and SD.
    pub const SPI_SCLK_GPIO: u8 = 8;
    pub const SPI_MOSI_GPIO: u8 = 10;
    pub const SPI_MISO_GPIO: u8 = 7;
    pub const SHARES_SPI_WITH_STORAGE: bool = true;
    pub const STORAGE_SD_CS_GPIO: u8 = 12;

    /// X4 SSD1677 panel geometry as used by the working imported runtime.
    pub const NATIVE_WIDTH: u16 = 800;
    pub const NATIVE_HEIGHT: u16 = 480;
    pub const LOGICAL_WIDTH: u16 = 480;
    pub const LOGICAL_HEIGHT: u16 = 800;
    pub const ROTATION_DEGREES: u16 = 270;
    pub const STRIP_ROWS: u16 = 40;

    /// SSD1677 command ownership notes. The imported runtime remains authoritative.
    pub const SSD1677_WRITE_RAM_CMD: u8 = 0x24;
    pub const SSD1677_WRITE_PREVIOUS_RAM_CMD: u8 = 0x26;
    pub const SSD1677_DISPLAY_UPDATE_CONTROL_2_CMD: u8 = 0x22;
    pub const SSD1677_MASTER_ACTIVATION_CMD: u8 = 0x20;

    pub const REFRESH_OWNER: &'static str = "target-xteink-x4/src/rustmix_x4 SSD1677 path";
    pub const ROTATION_OWNER: &'static str = "target-xteink-x4/src/rustmix_x4 display transform";
    pub const STRIP_RENDER_OWNER: &'static str = "target-xteink-x4/src/rustmix_x4 strip renderer";

    pub fn emit_display_boundary_marker() {
        esp_println::println!("{}", Self::DISPLAY_BOUNDARY_MARKER);
    }

    pub const fn owns_physical_display_runtime() -> bool {
        Self::PHYSICAL_DISPLAY_INIT_MOVED_TO_BOUNDARY
            || Self::PHYSICAL_DISPLAY_REFRESH_MOVED_TO_BOUNDARY
            || Self::SSD1677_SPI_TRANSACTIONS_MOVED_TO_BOUNDARY
            || Self::FRAMEBUFFER_OR_STRIP_RENDER_MOVED_TO_BOUNDARY
    }

    pub const fn uses_shared_spi_with_storage() -> bool {
        Self::SHARES_SPI_WITH_STORAGE
    }

    pub const fn pins() -> X4DisplayPins {
        X4DisplayPins {
            epd_cs_gpio: Self::EPD_CS_GPIO,
            epd_dc_gpio: Self::EPD_DC_GPIO,
            epd_rst_gpio: Self::EPD_RST_GPIO,
            epd_busy_gpio: Self::EPD_BUSY_GPIO,
            spi_sclk_gpio: Self::SPI_SCLK_GPIO,
            spi_mosi_gpio: Self::SPI_MOSI_GPIO,
            spi_miso_gpio: Self::SPI_MISO_GPIO,
        }
    }

    pub const fn geometry() -> X4DisplayGeometry {
        X4DisplayGeometry {
            native_width: Self::NATIVE_WIDTH,
            native_height: Self::NATIVE_HEIGHT,
            logical_width: Self::LOGICAL_WIDTH,
            logical_height: Self::LOGICAL_HEIGHT,
            rotation_degrees: Self::ROTATION_DEGREES,
            strip_rows: Self::STRIP_ROWS,
        }
    }

    pub const fn refresh_contract() -> Ssd1677RefreshContract {
        Ssd1677RefreshContract {
            current_ram_command: Self::SSD1677_WRITE_RAM_CMD,
            previous_ram_command: Self::SSD1677_WRITE_PREVIOUS_RAM_CMD,
            display_update_control_2_command: Self::SSD1677_DISPLAY_UPDATE_CONTROL_2_CMD,
            master_activation_command: Self::SSD1677_MASTER_ACTIVATION_CMD,
            physical_refresh_moved_to_boundary: Self::PHYSICAL_DISPLAY_REFRESH_MOVED_TO_BOUNDARY,
        }
    }

    pub const fn native_area_pixels() -> u32 {
        Self::NATIVE_WIDTH as u32 * Self::NATIVE_HEIGHT as u32
    }

    pub const fn logical_area_pixels() -> u32 {
        Self::LOGICAL_WIDTH as u32 * Self::LOGICAL_HEIGHT as u32
    }

    pub const fn is_native_landscape() -> bool {
        Self::NATIVE_WIDTH > Self::NATIVE_HEIGHT
    }

    pub const fn is_logical_portrait() -> bool {
        Self::LOGICAL_HEIGHT > Self::LOGICAL_WIDTH
    }

    pub const fn is_known_ssd1677_command(command: u8) -> bool {
        matches!(
            command,
            Self::SSD1677_WRITE_RAM_CMD
                | Self::SSD1677_WRITE_PREVIOUS_RAM_CMD
                | Self::SSD1677_DISPLAY_UPDATE_CONTROL_2_CMD
                | Self::SSD1677_MASTER_ACTIVATION_CMD
        )
    }

    pub const fn strip_row_count_is_aligned() -> bool {
        Self::NATIVE_HEIGHT % Self::STRIP_ROWS == 0 || Self::LOGICAL_HEIGHT % Self::STRIP_ROWS == 0
    }
}

#[cfg(target_arch = "riscv32")]
impl RustmixDisplayBoundary {
    /// Backward-compatible scaffold marker.
    ///
    /// The display boundary expands, but the existing acceptance contract still
    /// expects this scaffold marker to remain available through the display
    /// boundary.
    pub fn emit_scaffold_marker() {
        esp_println::println!("{}", Self::BOUNDARY_SCAFFOLD_MARKER);
    }
}
