#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonId {
    Up,
    Down,
    Left,
    Right,
    Select,
    Back,
    Power,
}

impl ButtonId {
    pub const fn name(self) -> &'static str {
        match self {
            ButtonId::Up => "Up",
            ButtonId::Down => "Down",
            ButtonId::Left => "Left",
            ButtonId::Right => "Right",
            ButtonId::Select => "Select",
            ButtonId::Back => "Back",
            ButtonId::Power => "Power",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonEventKind {
    Press,
    Release,
    LongPress,
    Repeat,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputEvent {
    pub button: ButtonId,
    pub kind: ButtonEventKind,
}

impl InputEvent {
    pub const fn new(button: ButtonId, kind: ButtonEventKind) -> Self {
        Self { button, kind }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ButtonThreshold {
    pub center_mv: u16,
    pub tolerance_mv: u16,
    pub logical: ButtonId,
}

impl ButtonThreshold {
    pub const fn new(center_mv: u16, tolerance_mv: u16, logical: ButtonId) -> Self {
        Self {
            center_mv,
            tolerance_mv,
            logical,
        }
    }

    pub fn low(self) -> u16 {
        self.center_mv.saturating_sub(self.tolerance_mv)
    }

    pub fn high(self) -> u16 {
        self.center_mv.saturating_add(self.tolerance_mv)
    }

    pub fn contains(self, mv: u16) -> bool {
        mv >= self.low() && mv <= self.high()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputTiming {
    pub debounce_ms: u32,
    pub long_press_ms: u32,
    pub repeat_ms: u32,
    pub adc_oversample: u8,
}

impl InputTiming {
    pub const fn new(
        debounce_ms: u32,
        long_press_ms: u32,
        repeat_ms: u32,
        adc_oversample: u8,
    ) -> Self {
        Self {
            debounce_ms,
            long_press_ms,
            repeat_ms,
            adc_oversample,
        }
    }
}

impl Default for InputTiming {
    fn default() -> Self {
        // Mirrors the proven x4-reader-os-rs timing constants.
        Self::new(15, 1_000, 150, 4)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputSample {
    pub row1_mv: u16,
    pub row2_mv: u16,
    pub power_low: bool,
    pub at_ms: u32,
}

impl InputSample {
    pub const fn new(row1_mv: u16, row2_mv: u16, power_low: bool, at_ms: u32) -> Self {
        Self {
            row1_mv,
            row2_mv,
            power_low,
            at_ms,
        }
    }
}

pub trait InputHal {
    /// Non-blocking poll of already-decoded input events.
    fn poll(&mut self) -> Option<InputEvent>;

    /// Mirrors the current X4 `InputDriver::reset_hold_state()` seam so the
    /// runtime can consume a navigation event without immediately triggering a
    /// second hold/repeat action.
    fn reset_hold_state(&mut self) {}

    /// Device input timing policy, kept in the HAL so core code can reason
    /// about debouncing/repeat behavior without knowing board-specific ADCs.
    fn timing(&self) -> InputTiming {
        InputTiming::default()
    }

    /// X4-specific ladder thresholds exported as data so the future HAL can
    /// keep the proven decode values close to the hardware implementation.
    fn row1_thresholds(&self) -> &[ButtonThreshold] {
        &[]
    }

    fn row2_thresholds(&self) -> &[ButtonThreshold] {
        &[]
    }
}
