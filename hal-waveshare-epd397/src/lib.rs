//! Waveshare ESP32-S3 e-Paper 3.97 HAL skeleton.
//!
//! Repository Bootstrap v0 only establishes the target boundary.
//! Display, input, audio, storage, and power implementations are intentionally
//! added in later slices.
//!
//! Accepted future display source:
//! - Focus Hub free display backend.
//! - DisplayBackendAdapter.
//! - ShellDisplayBridge portrait mapping.
//!
//! Important pin ownership note:
//! - GPIO3 is EPD_BUSY and must not be reused for rotary/input.

pub mod board {
    pub const TARGET_NAME: &str = "waveshare-esp32-s3-epaper-3.97";
    pub const DISPLAY_WIDTH_NATIVE: usize = 800;
    pub const DISPLAY_HEIGHT_NATIVE: usize = 480;
    pub const DISPLAY_WIDTH_PORTRAIT: usize = 480;
    pub const DISPLAY_HEIGHT_PORTRAIT: usize = 800;
}

pub mod display {
    //! Placeholder for the accepted Focus Hub display backend import.
}

pub mod input {
    //! Placeholder for safe rotary input mapping.
    //! GPIO3 is reserved for EPD_BUSY and must not be used as input.
}

pub mod audio {
    //! Placeholder for future voice/audio codec bring-up.
}

pub mod storage {
    //! Placeholder for TF/SD storage adaptation.
}

pub mod power {
    //! Placeholder for PMU/battery services.
}

pub mod rtc {
    //! Placeholder for RTC/time services.
}

pub mod sensors {
    //! Placeholder for environment and IMU sensors.
}

pub mod wifi {
    //! Placeholder for Wi-Fi transfer and assistant connectivity.
}
