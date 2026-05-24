#![no_std]

use rustmix_core::hal::Hal;

pub mod display;
pub mod display_smoke;
pub mod input;
pub mod power;
pub mod storage;

pub use display::X4Display;
pub use display_smoke::{
    LibraryFileKind, LibraryListItem, ReaderPage, X4_LIBRARY_MAX_ITEMS, X4_READER_TEXT_BYTES,
    X4Ssd1677Smoke,
};
pub use input::{ROW1_THRESHOLDS, ROW2_THRESHOLDS, X4_INPUT_TIMING, X4Input};
pub use power::{
    DISCHARGE_CURVE, DIVIDER_MULT, X4_CRITICAL_BATTERY_MV, X4_DEFAULT_ADC_MV, X4_IDLE_TIMEOUT_MS,
    X4_LOW_BATTERY_MV, X4Power, adc_to_battery_mv, battery_percentage,
};
pub use storage::{X4Dir, X4File, X4Storage};

pub struct X4Hal {
    display: X4Display,
    input: X4Input,
    power: X4Power,
    storage: X4Storage,
}

impl X4Hal {
    pub fn new_placeholder() -> Self {
        Self {
            display: X4Display::default(),
            input: X4Input::default(),
            power: X4Power::default(),
            storage: X4Storage::default(),
        }
    }
}

impl Default for X4Hal {
    fn default() -> Self {
        Self::new_placeholder()
    }
}

impl Hal for X4Hal {
    type Display = X4Display;
    type Input = X4Input;
    type Power = X4Power;
    type Storage = X4Storage;

    fn display(&mut self) -> &mut Self::Display {
        &mut self.display
    }

    fn input(&mut self) -> &mut Self::Input {
        &mut self.input
    }

    fn power(&mut self) -> &mut Self::Power {
        &mut self.power
    }

    fn storage(&mut self) -> &mut Self::Storage {
        &mut self.storage
    }

    const CAP_PSRAM: bool = false;
    const CAP_4GRAY: bool = false;
    const CAP_RTC: bool = false;
    const CAP_IMU: bool = false;
    const CAP_ENV_SENSOR: bool = false;
    const CAP_TOUCH: bool = false;
    const CAP_XTC_FORMAT: bool = true;
    const CAP_SUNLIGHT_FIX: bool = true;
    const CAP_SHARED_SPI_SD_EPD: bool = true;
}
