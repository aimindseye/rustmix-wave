#![allow(dead_code)]

use super::input_semantics_runtime::RustmixPhysicalButton;

pub struct RustmixInputAdcRuntimeBridge;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RustmixAdcLadderRow {
    Row1Gpio1,
    Row2Gpio2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RustmixAdcButtonBand {
    pub center_mv: u16,
    pub tolerance_mv: u16,
    pub button: RustmixPhysicalButton,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RustmixInputTimingPolicy {
    pub oversample_count: u32,
    pub debounce_window_ms: u64,
    pub long_press_window_ms: u64,
    pub repeat_interval_ms: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RustmixInputAdcRuntimeReport {
    pub row1_classification_ok: bool,
    pub row2_classification_ok: bool,
    pub boundary_rejection_ok: bool,
    pub timing_policy_ok: bool,
    pub physical_adc_sampling_owned: bool,
    pub debounce_loop_owned: bool,
}

impl RustmixInputAdcRuntimeReport {
    pub const fn preflight_ok(self) -> bool {
        self.row1_classification_ok
            && self.row2_classification_ok
            && self.boundary_rejection_ok
            && self.timing_policy_ok
            && !self.physical_adc_sampling_owned
            && !self.debounce_loop_owned
    }
}

impl RustmixInputAdcRuntimeBridge {
    pub const IMPLEMENTATION_OWNER: &'static str = "Rustmix-owned input ADC classification facade";
    pub const PHYSICAL_ADC_SAMPLING_OWNER: &'static str = "Rustmix-owned X4 runtime";
    pub const DEBOUNCE_LOOP_OWNER: &'static str = "Rustmix-owned X4 runtime";
    pub const PHYSICAL_ADC_SAMPLING_OWNED_BY_BRIDGE: bool = false;
    pub const DEBOUNCE_LOOP_OWNED_BY_BRIDGE: bool = false;

    pub const ROW1_GPIO: u8 = 1;
    pub const ROW2_GPIO: u8 = 2;
    pub const POWER_GPIO: u8 = 3;
    pub const DEFAULT_TOLERANCE_MV: u16 = 150;
    pub const LOW_RAIL_TOLERANCE_MV: u16 = 50;

    pub const ROW1_BANDS: [RustmixAdcButtonBand; 4] = [
        RustmixAdcButtonBand {
            center_mv: 3,
            tolerance_mv: Self::LOW_RAIL_TOLERANCE_MV,
            button: RustmixPhysicalButton::Right,
        },
        RustmixAdcButtonBand {
            center_mv: 1113,
            tolerance_mv: Self::DEFAULT_TOLERANCE_MV,
            button: RustmixPhysicalButton::Left,
        },
        RustmixAdcButtonBand {
            center_mv: 1984,
            tolerance_mv: Self::DEFAULT_TOLERANCE_MV,
            button: RustmixPhysicalButton::Confirm,
        },
        RustmixAdcButtonBand {
            center_mv: 2556,
            tolerance_mv: Self::DEFAULT_TOLERANCE_MV,
            button: RustmixPhysicalButton::Back,
        },
    ];

    pub const ROW2_BANDS: [RustmixAdcButtonBand; 2] = [
        RustmixAdcButtonBand {
            center_mv: 3,
            tolerance_mv: Self::LOW_RAIL_TOLERANCE_MV,
            button: RustmixPhysicalButton::VolDown,
        },
        RustmixAdcButtonBand {
            center_mv: 1659,
            tolerance_mv: Self::DEFAULT_TOLERANCE_MV,
            button: RustmixPhysicalButton::VolUp,
        },
    ];

    pub const TIMING_POLICY: RustmixInputTimingPolicy = RustmixInputTimingPolicy {
        oversample_count: 4,
        debounce_window_ms: 15,
        long_press_window_ms: 1000,
        repeat_interval_ms: 150,
    };

    pub fn active_runtime_preflight() -> bool {
        Self::preflight_report().preflight_ok()
    }

    pub fn preflight_report() -> RustmixInputAdcRuntimeReport {
        RustmixInputAdcRuntimeReport {
            row1_classification_ok: Self::row1_classification_ok(),
            row2_classification_ok: Self::row2_classification_ok(),
            boundary_rejection_ok: Self::boundary_rejection_ok(),
            timing_policy_ok: Self::timing_policy_ok(),
            physical_adc_sampling_owned: Self::PHYSICAL_ADC_SAMPLING_OWNED_BY_BRIDGE,
            debounce_loop_owned: Self::DEBOUNCE_LOOP_OWNED_BY_BRIDGE,
        }
    }

    pub const fn classify_mv(
        row: RustmixAdcLadderRow,
        millivolts: u16,
    ) -> Option<RustmixPhysicalButton> {
        let bands: &[RustmixAdcButtonBand] = match row {
            RustmixAdcLadderRow::Row1Gpio1 => &Self::ROW1_BANDS,
            RustmixAdcLadderRow::Row2Gpio2 => &Self::ROW2_BANDS,
        };

        let mut idx = 0;
        while idx < bands.len() {
            let band = bands[idx];
            let low = band.center_mv.saturating_sub(band.tolerance_mv);
            let high = band.center_mv.saturating_add(band.tolerance_mv);
            if millivolts >= low && millivolts <= high {
                return Some(band.button);
            }
            idx += 1;
        }

        None
    }

    fn row1_classification_ok() -> bool {
        Self::classify_mv(RustmixAdcLadderRow::Row1Gpio1, 3) == Some(RustmixPhysicalButton::Right)
            && Self::classify_mv(RustmixAdcLadderRow::Row1Gpio1, 1113)
                == Some(RustmixPhysicalButton::Left)
            && Self::classify_mv(RustmixAdcLadderRow::Row1Gpio1, 1984)
                == Some(RustmixPhysicalButton::Confirm)
            && Self::classify_mv(RustmixAdcLadderRow::Row1Gpio1, 2556)
                == Some(RustmixPhysicalButton::Back)
    }

    fn row2_classification_ok() -> bool {
        Self::classify_mv(RustmixAdcLadderRow::Row2Gpio2, 3) == Some(RustmixPhysicalButton::VolDown)
            && Self::classify_mv(RustmixAdcLadderRow::Row2Gpio2, 1659)
                == Some(RustmixPhysicalButton::VolUp)
    }

    fn boundary_rejection_ok() -> bool {
        Self::classify_mv(RustmixAdcLadderRow::Row1Gpio1, 54).is_none()
            && Self::classify_mv(RustmixAdcLadderRow::Row1Gpio1, 960).is_none()
            && Self::classify_mv(RustmixAdcLadderRow::Row2Gpio2, 1540).is_none()
            && Self::classify_mv(RustmixAdcLadderRow::Row2Gpio2, 1900).is_none()
    }

    const fn timing_policy_ok() -> bool {
        Self::TIMING_POLICY.oversample_count == 4
            && Self::TIMING_POLICY.debounce_window_ms == 15
            && Self::TIMING_POLICY.long_press_window_ms == 1000
            && Self::TIMING_POLICY.repeat_interval_ms == 150
    }
}

#[cfg(test)]
mod tests {
    use super::{RustmixAdcLadderRow, RustmixInputAdcRuntimeBridge};
    use crate::rustmix_x4::input::input_semantics_runtime::RustmixPhysicalButton;

    #[test]
    fn adc_classification_probe_is_pure_and_valid() {
        assert!(RustmixInputAdcRuntimeBridge::active_runtime_preflight());
    }

    #[test]
    fn classifies_known_ladder_centers() {
        assert_eq!(
            RustmixInputAdcRuntimeBridge::classify_mv(RustmixAdcLadderRow::Row1Gpio1, 1984),
            Some(RustmixPhysicalButton::Confirm)
        );
        assert_eq!(
            RustmixInputAdcRuntimeBridge::classify_mv(RustmixAdcLadderRow::Row2Gpio2, 1659),
            Some(RustmixPhysicalButton::VolUp)
        );
    }
}
