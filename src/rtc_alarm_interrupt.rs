//! PCF85063 active-low alarm interrupt readiness boundary.
//!
//! The uploaded Waveshare reference routes the PCF85063 interrupt output to
//! GPIO45.  This milestone deliberately validates the physical line while the
//! FreeRTOS firmware loop is still running.  MCU deep-sleep entry is deferred
//! until the board-level active-low route has passed a physical smoke test.

/// Board-specific PCF85063 alarm interrupt pin from the uploaded Waveshare BSP.
pub const RTC_ALARM_INTERRUPT_GPIO: u8 = 45;

/// Product-facing interpretation of the active-low PCF85063 interrupt line.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RtcAlarmInterruptLevel {
    Released,
    Asserted,
}

impl RtcAlarmInterruptLevel {
    /// Convert a raw digital input level into the PCF85063 active-low meaning.
    #[must_use]
    pub const fn from_gpio_high(high: bool) -> Self {
        if high {
            Self::Released
        } else {
            Self::Asserted
        }
    }

    #[must_use]
    pub const fn asserted(self) -> bool {
        matches!(self, Self::Asserted)
    }

    #[must_use]
    pub const fn marker(self) -> &'static str {
        match self {
            Self::Released => "released",
            Self::Asserted => "asserted",
        }
    }

    #[must_use]
    pub const fn raw_level_marker(self) -> &'static str {
        match self {
            Self::Released => "high",
            Self::Asserted => "low",
        }
    }
}

/// One GPIO45 sample with edge information for concise monitor logging.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RtcAlarmInterruptSample {
    pub level: RtcAlarmInterruptLevel,
    pub changed: bool,
}

impl RtcAlarmInterruptSample {
    #[must_use]
    pub const fn asserted(self) -> bool {
        self.level.asserted()
    }
}

#[cfg(target_os = "espidf")]
pub mod espidf {
    use esp_idf_svc::hal::gpio::{Input, PinDriver};

    use super::{RtcAlarmInterruptLevel, RtcAlarmInterruptSample};

    /// Own the configured GPIO45 input and expose a small polling boundary.
    ///
    /// The PCF85063 interrupt output is active-low.  Keeping the hardware
    /// wrapper tiny makes it straightforward to replace active-loop polling
    /// with an RTC-capable wake source in the following deep-sleep milestone.
    pub struct RtcAlarmInterruptMonitor<'d> {
        pin: PinDriver<'d, Input>,
        last_level: RtcAlarmInterruptLevel,
    }

    impl<'d> RtcAlarmInterruptMonitor<'d> {
        #[must_use]
        pub fn new(pin: PinDriver<'d, Input>) -> Self {
            let last_level = RtcAlarmInterruptLevel::from_gpio_high(pin.is_high());
            Self { pin, last_level }
        }

        #[must_use]
        pub fn sample(&mut self) -> RtcAlarmInterruptSample {
            let level = RtcAlarmInterruptLevel::from_gpio_high(self.pin.is_high());
            let changed = level != self.last_level;
            self.last_level = level;
            RtcAlarmInterruptSample { level, changed }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{RtcAlarmInterruptLevel, RtcAlarmInterruptSample, RTC_ALARM_INTERRUPT_GPIO};

    #[test]
    fn uploaded_bsp_gpio45_contract_is_explicit() {
        assert_eq!(RTC_ALARM_INTERRUPT_GPIO, 45);
    }

    #[test]
    fn active_low_mapping_is_explicit() {
        assert_eq!(
            RtcAlarmInterruptLevel::from_gpio_high(true),
            RtcAlarmInterruptLevel::Released
        );
        assert_eq!(
            RtcAlarmInterruptLevel::from_gpio_high(false),
            RtcAlarmInterruptLevel::Asserted
        );
        assert!(!RtcAlarmInterruptLevel::Released.asserted());
        assert!(RtcAlarmInterruptLevel::Asserted.asserted());
    }

    #[test]
    fn sample_exposes_asserted_state_without_gpio_handle() {
        let sample = RtcAlarmInterruptSample {
            level: RtcAlarmInterruptLevel::Asserted,
            changed: true,
        };
        assert!(sample.asserted());
        assert_eq!(sample.level.marker(), "asserted");
        assert_eq!(sample.level.raw_level_marker(), "low");
    }
}
