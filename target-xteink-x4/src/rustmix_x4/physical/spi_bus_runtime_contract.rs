#![allow(dead_code)]

use crate::rustmix_x4::physical::spi_bus_runtime::RustmixSpiBusRuntimeBridge;

/// Canonical Rustmix-owned SPI bus runtime contract metadata for the Xteink X4 target.
///
/// This module consolidates SPI ownership facts only. It intentionally does not
/// create a bus, configure a peripheral, select chip-select lines, probe SD,
/// initialize FAT, initialize SSD1677, refresh the display, or arbitrate access
/// at runtime. The active runtime remains in the imported X4/X4 firmware path.
pub struct RustmixSpiBusRuntimeContract;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RustmixSpiSharedDevice {
    Display,
    Storage,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RustmixSpiBehaviorOwner {
    VendorX4Runtime,
    RustmixContractMetadata,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RustmixSpiSharedUser {
    pub device: RustmixSpiSharedDevice,
    pub role: &'static str,
    pub chip_select_gpio: u8,
    pub shares_bus_with: RustmixSpiSharedDevice,
    pub physical_behavior_owner: RustmixSpiBehaviorOwner,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RustmixSpiPinOwnership {
    pub sclk_gpio: u8,
    pub mosi_gpio: u8,
    pub miso_gpio: u8,
    pub display_cs_gpio: u8,
    pub storage_cs_gpio: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RustmixSpiTimingOwnership {
    pub sd_probe_khz: u32,
    pub operational_mhz: u32,
    pub dma_channel: u8,
    pub dma_tx_bytes: usize,
    pub dma_rx_bytes: usize,
    pub sd_init_before_display_traffic: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RustmixSpiRuntimeContractReport {
    pub pins_ok: bool,
    pub timing_ok: bool,
    pub shared_users_documented: bool,
    pub bridge_alignment_ok: bool,
    pub arbitration_still_imported: bool,
    pub sd_runtime_still_imported: bool,
    pub display_runtime_still_imported: bool,
    pub no_runtime_behavior_moved: bool,
}

impl RustmixSpiRuntimeContractReport {
    pub const fn contract_ok(self) -> bool {
        self.pins_ok
            && self.timing_ok
            && self.shared_users_documented
            && self.bridge_alignment_ok
            && self.arbitration_still_imported
            && self.sd_runtime_still_imported
            && self.display_runtime_still_imported
            && self.no_runtime_behavior_moved
    }
}

impl RustmixSpiBusRuntimeContract {
    pub const SPI_BUS_RUNTIME_CONTRACT_MARKER: &'static str = "x4-spi-bus-runtime-contract-ok";

    pub const CONTRACT_OWNER: &'static str = "Rustmix physical SPI bus contract metadata";
    pub const ACTIVE_ARBITRATION_OWNER: &'static str = "Rustmix-owned X4 runtime";
    pub const ACTIVE_SD_RUNTIME_OWNER: &'static str = "Rustmix-owned X4 runtime";
    pub const ACTIVE_DISPLAY_RUNTIME_OWNER: &'static str = "Rustmix-owned X4 runtime";

    /// This consolidation layer is metadata only. Runtime hardware behavior is not moved.
    pub const SPI_ARBITRATION_MOVED_TO_RUSTMIX: bool = false;
    pub const SD_DRIVER_MOVED_TO_RUSTMIX: bool = false;
    pub const SD_MOUNT_OR_PROBE_MOVED_TO_RUSTMIX: bool = false;
    pub const FAT_BEHAVIOR_MOVED_TO_RUSTMIX: bool = false;
    pub const DISPLAY_DRIVER_MOVED_TO_RUSTMIX: bool = false;
    pub const DISPLAY_REFRESH_MOVED_TO_RUSTMIX: bool = false;

    pub const SPI_SCLK_GPIO: u8 = 8;
    pub const SPI_MOSI_GPIO: u8 = 10;
    pub const SPI_MISO_GPIO: u8 = 7;
    pub const DISPLAY_CS_GPIO: u8 = 21;
    pub const STORAGE_SD_CS_GPIO: u8 = 12;

    pub const SD_PROBE_KHZ: u32 = 400;
    pub const OPERATIONAL_MHZ: u32 = 20;
    pub const DMA_CHANNEL: u8 = 0;
    pub const DMA_TX_BYTES: usize = 4096;
    pub const DMA_RX_BYTES: usize = 4096;
    pub const SD_INIT_BEFORE_DISPLAY_TRAFFIC: bool = true;

    pub const SHARED_USERS: [RustmixSpiSharedUser; 2] = [
        RustmixSpiSharedUser {
            device: RustmixSpiSharedDevice::Display,
            role: "SSD1677 e-paper display over shared SPI",
            chip_select_gpio: Self::DISPLAY_CS_GPIO,
            shares_bus_with: RustmixSpiSharedDevice::Storage,
            physical_behavior_owner: RustmixSpiBehaviorOwner::VendorX4Runtime,
        },
        RustmixSpiSharedUser {
            device: RustmixSpiSharedDevice::Storage,
            role: "microSD storage over shared SPI",
            chip_select_gpio: Self::STORAGE_SD_CS_GPIO,
            shares_bus_with: RustmixSpiSharedDevice::Display,
            physical_behavior_owner: RustmixSpiBehaviorOwner::VendorX4Runtime,
        },
    ];

    pub const fn pins() -> RustmixSpiPinOwnership {
        RustmixSpiPinOwnership {
            sclk_gpio: Self::SPI_SCLK_GPIO,
            mosi_gpio: Self::SPI_MOSI_GPIO,
            miso_gpio: Self::SPI_MISO_GPIO,
            display_cs_gpio: Self::DISPLAY_CS_GPIO,
            storage_cs_gpio: Self::STORAGE_SD_CS_GPIO,
        }
    }

    pub const fn timing() -> RustmixSpiTimingOwnership {
        RustmixSpiTimingOwnership {
            sd_probe_khz: Self::SD_PROBE_KHZ,
            operational_mhz: Self::OPERATIONAL_MHZ,
            dma_channel: Self::DMA_CHANNEL,
            dma_tx_bytes: Self::DMA_TX_BYTES,
            dma_rx_bytes: Self::DMA_RX_BYTES,
            sd_init_before_display_traffic: Self::SD_INIT_BEFORE_DISPLAY_TRAFFIC,
        }
    }

    pub const fn owner_for_device(device: RustmixSpiSharedDevice) -> RustmixSpiBehaviorOwner {
        match device {
            RustmixSpiSharedDevice::Display => RustmixSpiBehaviorOwner::VendorX4Runtime,
            RustmixSpiSharedDevice::Storage => RustmixSpiBehaviorOwner::VendorX4Runtime,
        }
    }

    pub const fn chip_select_for_device(device: RustmixSpiSharedDevice) -> u8 {
        match device {
            RustmixSpiSharedDevice::Display => Self::DISPLAY_CS_GPIO,
            RustmixSpiSharedDevice::Storage => Self::STORAGE_SD_CS_GPIO,
        }
    }

    pub const fn selection_rule_allows(display_selected: bool, storage_selected: bool) -> bool {
        !(display_selected && storage_selected)
    }

    pub const fn arbitration_is_still_imported() -> bool {
        !Self::SPI_ARBITRATION_MOVED_TO_RUSTMIX
    }

    pub const fn sd_runtime_is_still_imported() -> bool {
        !Self::SD_DRIVER_MOVED_TO_RUSTMIX
            && !Self::SD_MOUNT_OR_PROBE_MOVED_TO_RUSTMIX
            && !Self::FAT_BEHAVIOR_MOVED_TO_RUSTMIX
    }

    pub const fn display_runtime_is_still_imported() -> bool {
        !Self::DISPLAY_DRIVER_MOVED_TO_RUSTMIX && !Self::DISPLAY_REFRESH_MOVED_TO_RUSTMIX
    }

    pub const fn no_runtime_behavior_moved() -> bool {
        Self::arbitration_is_still_imported()
            && Self::sd_runtime_is_still_imported()
            && Self::display_runtime_is_still_imported()
    }

    pub const fn pins_are_documented() -> bool {
        Self::SPI_SCLK_GPIO == 8
            && Self::SPI_MOSI_GPIO == 10
            && Self::SPI_MISO_GPIO == 7
            && Self::DISPLAY_CS_GPIO == 21
            && Self::STORAGE_SD_CS_GPIO == 12
    }

    pub const fn timing_is_documented() -> bool {
        Self::SD_PROBE_KHZ == 400
            && Self::OPERATIONAL_MHZ == 20
            && Self::DMA_CHANNEL == 0
            && Self::DMA_TX_BYTES == 4096
            && Self::DMA_RX_BYTES == 4096
            && Self::SD_INIT_BEFORE_DISPLAY_TRAFFIC
    }

    pub const fn shared_users_are_documented() -> bool {
        Self::SHARED_USERS.len() == 2
            && Self::SHARED_USERS[0].device as u8 != Self::SHARED_USERS[1].device as u8
            && Self::SHARED_USERS[0].chip_select_gpio == Self::DISPLAY_CS_GPIO
            && Self::SHARED_USERS[1].chip_select_gpio == Self::STORAGE_SD_CS_GPIO
    }

    pub fn matches_existing_runtime_facade() -> bool {
        let pins = RustmixSpiBusRuntimeBridge::PINS;
        let timing = RustmixSpiBusRuntimeBridge::TIMING;

        pins.sclk_gpio == Self::SPI_SCLK_GPIO
            && pins.mosi_gpio == Self::SPI_MOSI_GPIO
            && pins.miso_gpio == Self::SPI_MISO_GPIO
            && pins.epd_cs_gpio == Self::DISPLAY_CS_GPIO
            && pins.sd_cs_gpio == Self::STORAGE_SD_CS_GPIO
            && timing.sd_probe_khz == Self::SD_PROBE_KHZ
            && timing.operational_mhz == Self::OPERATIONAL_MHZ
            && timing.dma_channel == Self::DMA_CHANNEL
            && timing.dma_tx_bytes == Self::DMA_TX_BYTES
            && timing.dma_rx_bytes == Self::DMA_RX_BYTES
            && timing.sd_init_before_epd_traffic == Self::SD_INIT_BEFORE_DISPLAY_TRAFFIC
            && !RustmixSpiBusRuntimeBridge::PHYSICAL_SPI_OWNED_BY_BRIDGE
            && !RustmixSpiBusRuntimeBridge::PHYSICAL_SD_OWNED_BY_BRIDGE
            && !RustmixSpiBusRuntimeBridge::PHYSICAL_DISPLAY_OWNED_BY_BRIDGE
    }

    pub fn report() -> RustmixSpiRuntimeContractReport {
        RustmixSpiRuntimeContractReport {
            pins_ok: Self::pins_are_documented(),
            timing_ok: Self::timing_is_documented(),
            shared_users_documented: Self::shared_users_are_documented(),
            bridge_alignment_ok: Self::matches_existing_runtime_facade(),
            arbitration_still_imported: Self::arbitration_is_still_imported(),
            sd_runtime_still_imported: Self::sd_runtime_is_still_imported(),
            display_runtime_still_imported: Self::display_runtime_is_still_imported(),
            no_runtime_behavior_moved: Self::no_runtime_behavior_moved(),
        }
    }

    pub fn contract_ok() -> bool {
        Self::report().contract_ok()
    }

    #[cfg(target_arch = "riscv32")]
    pub fn emit_boot_marker() {
        if Self::contract_ok() {
            esp_println::println!("{}", Self::SPI_BUS_RUNTIME_CONTRACT_MARKER);
        } else {
            esp_println::println!("spi-bus-runtime-contract-failed");
        }
    }

    #[cfg(not(target_arch = "riscv32"))]
    pub fn emit_boot_marker() {
        if Self::contract_ok() {
            println!("{}", Self::SPI_BUS_RUNTIME_CONTRACT_MARKER);
        } else {
            println!("spi-bus-runtime-contract-failed");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{RustmixSpiBusRuntimeContract, RustmixSpiSharedDevice};

    #[test]
    fn spi_contract_is_metadata_only_and_ok() {
        assert!(RustmixSpiBusRuntimeContract::contract_ok());
        assert!(RustmixSpiBusRuntimeContract::no_runtime_behavior_moved());
    }

    #[test]
    fn display_and_storage_have_separate_chip_selects() {
        assert_eq!(
            RustmixSpiBusRuntimeContract::chip_select_for_device(RustmixSpiSharedDevice::Display),
            21
        );
        assert_eq!(
            RustmixSpiBusRuntimeContract::chip_select_for_device(RustmixSpiSharedDevice::Storage),
            12
        );
        assert!(RustmixSpiBusRuntimeContract::selection_rule_allows(
            true, false
        ));
        assert!(RustmixSpiBusRuntimeContract::selection_rule_allows(
            false, true
        ));
        assert!(!RustmixSpiBusRuntimeContract::selection_rule_allows(
            true, true
        ));
    }
}
