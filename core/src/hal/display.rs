#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefreshMode {
    Full,
    Partial,
    Fast,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisplayDepth {
    Mono1Bit,
    Gray2Bit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rotation {
    Deg0,
    Deg90,
    Deg180,
    Deg270,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DisplayGeometry {
    pub native_width: u16,
    pub native_height: u16,
    pub logical_width: u16,
    pub logical_height: u16,
    pub rotation: Rotation,
    pub strip_rows: u16,
    pub depth: DisplayDepth,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DisplayBusConfig {
    pub probe_khz: u32,
    pub runtime_mhz: u32,
    pub shared_sd_epd_bus: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisplayError {
    InitFailed,
    DrawFailed,
    RefreshFailed,
    SleepFailed,
    Unsupported,
}

pub trait DisplayHal {
    /// Geometry exposed to the UI/runtime layer.
    ///
    /// For the X4 this is logical portrait 480x800, even though the native
    /// panel is 800x480 and rotated at runtime.
    fn geometry(&self) -> DisplayGeometry;

    /// Bus topology relevant to board bootstrap.
    ///
    /// The current X4 implementation probes SD at 400 kHz and only later moves
    /// the shared SPI bus to 20 MHz for runtime display/file operations.
    fn bus_config(&self) -> DisplayBusConfig;

    /// Hardware init / wake path.
    fn init(&mut self) -> Result<(), DisplayError>;

    /// Called before a strip-rendered frame begins.
    fn begin_frame(&mut self) -> Result<(), DisplayError> {
        Ok(())
    }

    /// Push one logical strip into the panel pipeline.
    fn draw_strip(&mut self, y_start: u16, buf: &[u8]) -> Result<(), DisplayError>;

    /// Complete the frame with the requested refresh mode.
    fn end_frame(&mut self, mode: RefreshMode) -> Result<(), DisplayError>;

    /// Put the panel into sleep / low-power state.
    fn sleep(&mut self) -> Result<(), DisplayError>;

    /// X4-only optional behavior to mitigate display fading / sunlight quirks.
    fn set_sunlight_fix(&mut self, _enabled: bool) -> Result<(), DisplayError> {
        Ok(())
    }
}
