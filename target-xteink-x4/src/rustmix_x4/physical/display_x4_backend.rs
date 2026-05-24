#![allow(dead_code)]

/// X4 compatibility backend for the Rustmix display runtime owner.
///
/// This backend is deliberately descriptive. It records that the active
/// SSD1677/e-paper executor remains the existing imported X4 runtime. It must
/// not initialize the display, send SSD1677 commands, draw pixels, perform full
/// or partial refreshes, wait on BUSY, or touch shared SPI chip-select lines.
pub struct RustmixDisplayX4Backend;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RustmixDisplayX4BackendReport {
    pub active_hardware_executor: bool,
    pub active_ssd1677_executor_owner: &'static str,
    pub active_draw_executor_owner: &'static str,
    pub active_full_refresh_executor_owner: &'static str,
    pub active_partial_refresh_executor_owner: &'static str,
    pub active_busy_wait_executor_owner: &'static str,
    pub active_rotation_executor_owner: &'static str,
    pub active_strip_render_executor_owner: &'static str,
    pub ssd1677_executor_moved_to_rustmix: bool,
    pub draw_executor_moved_to_rustmix: bool,
    pub refresh_executor_moved_to_rustmix: bool,
    pub partial_refresh_executor_moved_to_rustmix: bool,
    pub spi_transaction_executor_moved_to_rustmix: bool,
    pub storage_behavior_changed: bool,
    pub reader_file_browser_behavior_changed: bool,
}

impl RustmixDisplayX4BackendReport {
    pub const fn bridge_ok(self) -> bool {
        self.active_hardware_executor
            && self.active_ssd1677_executor_owner.len()
                == RustmixDisplayX4Backend::ACTIVE_SSD1677_EXECUTOR_OWNER.len()
            && self.active_draw_executor_owner.len()
                == RustmixDisplayX4Backend::ACTIVE_DRAW_EXECUTOR_OWNER.len()
            && self.active_full_refresh_executor_owner.len()
                == RustmixDisplayX4Backend::ACTIVE_FULL_REFRESH_EXECUTOR_OWNER.len()
            && self.active_partial_refresh_executor_owner.len()
                == RustmixDisplayX4Backend::ACTIVE_PARTIAL_REFRESH_EXECUTOR_OWNER.len()
            && self.active_busy_wait_executor_owner.len()
                == RustmixDisplayX4Backend::ACTIVE_BUSY_WAIT_EXECUTOR_OWNER.len()
            && self.active_rotation_executor_owner.len()
                == RustmixDisplayX4Backend::ACTIVE_ROTATION_EXECUTOR_OWNER.len()
            && self.active_strip_render_executor_owner.len()
                == RustmixDisplayX4Backend::ACTIVE_STRIP_RENDER_EXECUTOR_OWNER.len()
            && !self.ssd1677_executor_moved_to_rustmix
            && !self.draw_executor_moved_to_rustmix
            && !self.refresh_executor_moved_to_rustmix
            && !self.partial_refresh_executor_moved_to_rustmix
            && !self.spi_transaction_executor_moved_to_rustmix
            && !self.storage_behavior_changed
            && !self.reader_file_browser_behavior_changed
    }
}

impl RustmixDisplayX4Backend {
    pub const BACKEND_NAME: &'static str = "X4Compatibility";
    pub const ACTIVE_HARDWARE_EXECUTOR: bool = true;

    pub const ACTIVE_SSD1677_EXECUTOR_OWNER: &'static str = "Rustmix-owned X4 runtime";
    pub const ACTIVE_DRAW_EXECUTOR_OWNER: &'static str = "Rustmix-owned X4 runtime";
    pub const ACTIVE_FULL_REFRESH_EXECUTOR_OWNER: &'static str = "Rustmix-owned X4 runtime";
    pub const ACTIVE_PARTIAL_REFRESH_EXECUTOR_OWNER: &'static str =
        "Rustmix-owned X4 runtime";
    pub const ACTIVE_BUSY_WAIT_EXECUTOR_OWNER: &'static str = "Rustmix-owned X4 runtime";
    pub const ACTIVE_ROTATION_EXECUTOR_OWNER: &'static str = "Rustmix-owned X4 runtime";
    pub const ACTIVE_STRIP_RENDER_EXECUTOR_OWNER: &'static str = "Rustmix-owned X4 runtime";

    pub const SSD1677_EXECUTOR_MOVED_TO_RUSTMIX: bool = false;
    pub const DRAW_EXECUTOR_MOVED_TO_RUSTMIX: bool = false;
    pub const REFRESH_EXECUTOR_MOVED_TO_RUSTMIX: bool = false;
    pub const PARTIAL_REFRESH_EXECUTOR_MOVED_TO_RUSTMIX: bool = false;
    pub const SPI_TRANSACTION_EXECUTOR_MOVED_TO_RUSTMIX: bool = false;
    pub const STORAGE_BEHAVIOR_CHANGED: bool = false;
    pub const READER_FILE_BROWSER_BEHAVIOR_CHANGED: bool = false;

    pub const fn report() -> RustmixDisplayX4BackendReport {
        RustmixDisplayX4BackendReport {
            active_hardware_executor: Self::ACTIVE_HARDWARE_EXECUTOR,
            active_ssd1677_executor_owner: Self::ACTIVE_SSD1677_EXECUTOR_OWNER,
            active_draw_executor_owner: Self::ACTIVE_DRAW_EXECUTOR_OWNER,
            active_full_refresh_executor_owner: Self::ACTIVE_FULL_REFRESH_EXECUTOR_OWNER,
            active_partial_refresh_executor_owner: Self::ACTIVE_PARTIAL_REFRESH_EXECUTOR_OWNER,
            active_busy_wait_executor_owner: Self::ACTIVE_BUSY_WAIT_EXECUTOR_OWNER,
            active_rotation_executor_owner: Self::ACTIVE_ROTATION_EXECUTOR_OWNER,
            active_strip_render_executor_owner: Self::ACTIVE_STRIP_RENDER_EXECUTOR_OWNER,
            ssd1677_executor_moved_to_rustmix: Self::SSD1677_EXECUTOR_MOVED_TO_RUSTMIX,
            draw_executor_moved_to_rustmix: Self::DRAW_EXECUTOR_MOVED_TO_RUSTMIX,
            refresh_executor_moved_to_rustmix: Self::REFRESH_EXECUTOR_MOVED_TO_RUSTMIX,
            partial_refresh_executor_moved_to_rustmix:
                Self::PARTIAL_REFRESH_EXECUTOR_MOVED_TO_RUSTMIX,
            spi_transaction_executor_moved_to_rustmix:
                Self::SPI_TRANSACTION_EXECUTOR_MOVED_TO_RUSTMIX,
            storage_behavior_changed: Self::STORAGE_BEHAVIOR_CHANGED,
            reader_file_browser_behavior_changed: Self::READER_FILE_BROWSER_BEHAVIOR_CHANGED,
        }
    }

    pub const fn bridge_ok() -> bool {
        Self::report().bridge_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::RustmixDisplayX4Backend;

    #[test]
    fn x4_backend_keeps_active_display_executor() {
        assert!(RustmixDisplayX4Backend::bridge_ok());
        assert_eq!(RustmixDisplayX4Backend::BACKEND_NAME, "X4Compatibility");
        assert!(RustmixDisplayX4Backend::ACTIVE_HARDWARE_EXECUTOR);
    }
}
