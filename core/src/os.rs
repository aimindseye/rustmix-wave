use crate::hal::{
    DisplayBusConfig, DisplayGeometry, DisplayHal, Hal, PowerHal, StorageHal, StorageProbe,
};
use crate::services::{PowerManager, StorageService};
use crate::ui::activity::ActivityManager;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BootstrapReport {
    pub storage: StorageProbe,
    pub display: DisplayGeometry,
    pub display_bus: DisplayBusConfig,
    pub battery_mv: u16,
    pub battery_pct: u8,
}

/// High-level OS shell placeholder.
/// Real runtime orchestration will be introduced after HAL seams are refined.
pub struct RustmixOs<H: Hal> {
    pub hal: H,
    pub activity_manager: ActivityManager,
    pub storage_service: StorageService,
    pub power_manager: PowerManager,
}

impl<H: Hal> RustmixOs<H> {
    pub fn new(hal: H) -> Self {
        Self {
            hal,
            activity_manager: ActivityManager::new(),
            storage_service: StorageService::new(),
            power_manager: PowerManager::new(),
        }
    }

    /// First real extracted X4 slice.
    ///
    /// Mirrors the proven bootstrap order from `x4-reader-os-rs` at a seam level:
    /// 1. probe/mount SD while the shared bus is still in low-speed mode
    /// 2. init the display
    /// 3. sample battery state for status/boot reporting
    pub fn boot_storage_display_power(&mut self) -> Result<BootstrapReport, &'static str> {
        let storage = self
            .hal
            .storage()
            .init_card()
            .map_err(|_| "storage init failed")?;

        self.hal
            .storage()
            .mount()
            .map_err(|_| "storage mount failed")?;

        self.hal
            .display()
            .init()
            .map_err(|_| "display init failed")?;

        let display = self.hal.display().geometry();
        let display_bus = self.hal.display().bus_config();
        let battery_mv = self.hal.power().battery_mv();
        let battery_pct = self.hal.power().battery_pct();

        Ok(BootstrapReport {
            storage,
            display,
            display_bus,
            battery_mv,
            battery_pct,
        })
    }
}
