pub mod display;
pub mod input;
pub mod power;
pub mod storage;

pub use display::{
    DisplayBusConfig, DisplayDepth, DisplayError, DisplayGeometry, DisplayHal, RefreshMode,
    Rotation,
};
pub use input::{
    ButtonEventKind, ButtonId, ButtonThreshold, InputEvent, InputHal, InputSample, InputTiming,
};
pub use power::{BatteryReading, ChargeState, PowerHal, PowerPolicy, PowerStatus};
pub use storage::{DirEntry, OpenMode, StorageError, StorageHal, StorageProbe, StorageState};

pub trait Hal: Sized + 'static {
    type Display: DisplayHal;
    type Input: InputHal;
    type Power: PowerHal;
    type Storage: StorageHal;

    fn display(&mut self) -> &mut Self::Display;
    fn input(&mut self) -> &mut Self::Input;
    fn power(&mut self) -> &mut Self::Power;
    fn storage(&mut self) -> &mut Self::Storage;

    const CAP_PSRAM: bool = false;
    const CAP_4GRAY: bool = false;
    const CAP_RTC: bool = false;
    const CAP_IMU: bool = false;
    const CAP_ENV_SENSOR: bool = false;
    const CAP_TOUCH: bool = false;
    const CAP_XTC_FORMAT: bool = false;
    const CAP_SUNLIGHT_FIX: bool = false;

    /// Shared-SPI topology matters on the X4 because SD probing happens before
    /// the display switches to runtime bus speed.
    const CAP_SHARED_SPI_SD_EPD: bool = false;
}
