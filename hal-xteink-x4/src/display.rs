use rustmix_core::hal::{
    DisplayBusConfig, DisplayDepth, DisplayError, DisplayGeometry, DisplayHal, RefreshMode,
    Rotation,
};

pub const X4_NATIVE_WIDTH: u16 = 800;
pub const X4_NATIVE_HEIGHT: u16 = 480;
pub const X4_LOGICAL_WIDTH: u16 = 480;
pub const X4_LOGICAL_HEIGHT: u16 = 800;
pub const X4_STRIP_ROWS: u16 = 40;
pub const X4_SD_PROBE_KHZ: u32 = 400;
pub const X4_RUNTIME_SPI_MHZ: u32 = 20;

#[derive(Default)]
pub struct X4Display {
    initialised: bool,
    sunlight_fix: bool,
    last_refresh: Option<RefreshMode>,
}

impl X4Display {
    pub fn is_initialised(&self) -> bool {
        self.initialised
    }

    pub fn last_refresh(&self) -> Option<RefreshMode> {
        self.last_refresh
    }

    pub fn sunlight_fix_enabled(&self) -> bool {
        self.sunlight_fix
    }
}

impl DisplayHal for X4Display {
    fn geometry(&self) -> DisplayGeometry {
        DisplayGeometry {
            native_width: X4_NATIVE_WIDTH,
            native_height: X4_NATIVE_HEIGHT,
            logical_width: X4_LOGICAL_WIDTH,
            logical_height: X4_LOGICAL_HEIGHT,
            rotation: Rotation::Deg270,
            strip_rows: X4_STRIP_ROWS,
            depth: DisplayDepth::Mono1Bit,
        }
    }

    fn bus_config(&self) -> DisplayBusConfig {
        DisplayBusConfig {
            probe_khz: X4_SD_PROBE_KHZ,
            runtime_mhz: X4_RUNTIME_SPI_MHZ,
            shared_sd_epd_bus: true,
        }
    }

    fn init(&mut self) -> Result<(), DisplayError> {
        self.initialised = true;
        Ok(())
    }

    fn begin_frame(&mut self) -> Result<(), DisplayError> {
        if !self.initialised {
            return Err(DisplayError::InitFailed);
        }
        Ok(())
    }

    fn draw_strip(&mut self, _y_start: u16, _buf: &[u8]) -> Result<(), DisplayError> {
        if !self.initialised {
            return Err(DisplayError::InitFailed);
        }
        Ok(())
    }

    fn end_frame(&mut self, mode: RefreshMode) -> Result<(), DisplayError> {
        if !self.initialised {
            return Err(DisplayError::InitFailed);
        }
        self.last_refresh = Some(mode);
        Ok(())
    }

    fn sleep(&mut self) -> Result<(), DisplayError> {
        if !self.initialised {
            return Err(DisplayError::InitFailed);
        }
        self.initialised = false;
        Ok(())
    }

    fn set_sunlight_fix(&mut self, enabled: bool) -> Result<(), DisplayError> {
        self.sunlight_fix = enabled;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustmix_core::hal::DisplayHal;

    #[test]
    fn x4_geometry_matches_proven_portrait_runtime() {
        let display = X4Display::default();
        let geometry = display.geometry();
        assert_eq!(geometry.native_width, 800);
        assert_eq!(geometry.native_height, 480);
        assert_eq!(geometry.logical_width, 480);
        assert_eq!(geometry.logical_height, 800);
        assert_eq!(geometry.rotation, Rotation::Deg270);
        assert_eq!(geometry.strip_rows, 40);
        assert_eq!(geometry.depth, DisplayDepth::Mono1Bit);
    }

    #[test]
    fn x4_bus_config_preserves_sd_probe_then_runtime_speed() {
        let display = X4Display::default();
        let bus = display.bus_config();
        assert!(bus.shared_sd_epd_bus);
        assert_eq!(bus.probe_khz, 400);
        assert_eq!(bus.runtime_mhz, 20);
    }
}
