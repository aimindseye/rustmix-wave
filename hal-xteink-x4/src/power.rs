use rustmix_core::hal::{BatteryReading, ChargeState, PowerHal, PowerPolicy, PowerStatus};

pub const DIVIDER_MULT: u32 = 2;
pub const X4_DEFAULT_ADC_MV: u16 = 2_050;
pub const X4_IDLE_TIMEOUT_MS: u32 = 300_000;
pub const X4_LOW_BATTERY_MV: u16 = 3_600;
pub const X4_CRITICAL_BATTERY_MV: u16 = 3_400;

pub const DISCHARGE_CURVE: &[(u32, u8)] = &[
    (4200, 100),
    (4060, 90),
    (3980, 80),
    (3920, 70),
    (3870, 60),
    (3830, 50),
    (3790, 40),
    (3750, 30),
    (3700, 20),
    (3600, 10),
    (3400, 5),
    (3000, 0),
];

pub struct X4Power {
    adc_mv: u16,
    charge_state: ChargeState,
    last_light_sleep_ms: Option<u32>,
}

impl X4Power {
    pub const fn new() -> Self {
        Self {
            adc_mv: X4_DEFAULT_ADC_MV,
            charge_state: ChargeState::Unknown,
            last_light_sleep_ms: None,
        }
    }

    pub fn set_adc_mv(&mut self, adc_mv: u16) {
        self.adc_mv = adc_mv;
    }

    pub fn set_charge_state(&mut self, charge_state: ChargeState) {
        self.charge_state = charge_state;
    }

    pub fn set_charging_detected(&mut self, charging: bool) {
        self.charge_state = if charging {
            ChargeState::Charging
        } else {
            ChargeState::NotCharging
        };
    }

    pub fn reading_snapshot(&mut self) -> BatteryReading {
        self.reading()
    }

    pub fn status_snapshot(&mut self) -> PowerStatus {
        self.status()
    }

    pub fn last_light_sleep_ms(&self) -> Option<u32> {
        self.last_light_sleep_ms
    }
}

impl Default for X4Power {
    fn default() -> Self {
        Self::new()
    }
}

impl PowerHal for X4Power {
    fn adc_mv(&mut self) -> u16 {
        self.adc_mv
    }

    fn battery_mv(&mut self) -> u16 {
        adc_to_battery_mv(self.adc_mv)
    }

    fn battery_pct(&mut self) -> u8 {
        battery_percentage(self.battery_mv())
    }

    fn policy(&self) -> PowerPolicy {
        PowerPolicy::new(
            X4_IDLE_TIMEOUT_MS,
            X4_LOW_BATTERY_MV,
            X4_CRITICAL_BATTERY_MV,
        )
    }

    fn charge_state(&self) -> ChargeState {
        self.charge_state
    }

    fn deep_sleep(&mut self, _wake_after_ms: u32) -> ! {
        loop {
            core::hint::spin_loop();
        }
    }

    fn light_sleep(&mut self, ms: u32) {
        self.last_light_sleep_ms = Some(ms);
    }
}

pub fn adc_to_battery_mv(adc_mv: u16) -> u16 {
    (adc_mv as u32 * DIVIDER_MULT) as u16
}

pub fn battery_percentage(battery_mv: u16) -> u8 {
    let mv = battery_mv as u32;
    if mv >= DISCHARGE_CURVE[0].0 {
        return DISCHARGE_CURVE[0].1;
    }
    let last = DISCHARGE_CURVE.len() - 1;
    if mv <= DISCHARGE_CURVE[last].0 {
        return DISCHARGE_CURVE[last].1;
    }
    let mut i = 0;
    while i + 1 < DISCHARGE_CURVE.len() {
        let (mv_hi, pct_hi) = DISCHARGE_CURVE[i];
        let (mv_lo, pct_lo) = DISCHARGE_CURVE[i + 1];
        if mv >= mv_lo {
            let span_mv = mv_hi - mv_lo;
            if span_mv == 0 {
                return pct_hi;
            }
            let span_pct = (pct_hi - pct_lo) as u32;
            let frac = mv - mv_lo;
            return (pct_lo as u32 + frac * span_pct / span_mv) as u8;
        }
        i += 1;
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustmix_core::hal::PowerHal;

    #[test]
    fn adc_reading_uses_x4_divider() {
        let mut power = X4Power::default();
        power.set_adc_mv(2_050);
        assert_eq!(power.adc_mv(), 2_050);
        assert_eq!(power.battery_mv(), 4_100);
    }

    #[test]
    fn default_power_status_matches_typical_boot_voltage() {
        let mut power = X4Power::default();
        let status = power.status_snapshot();
        assert_eq!(status.reading.battery_mv, 4_100);
        assert!(!status.low_battery);
        assert!(!status.critical_battery);
    }

    #[test]
    fn battery_percentage_interpolates_curve() {
        let mut power = X4Power::default();
        power.set_adc_mv(2_100);
        assert_eq!(power.battery_pct(), 100);

        power.set_adc_mv(1_800);
        assert_eq!(power.battery_pct(), 10);
    }

    #[test]
    fn charge_detect_state_is_reported() {
        let mut power = X4Power::default();
        power.set_charging_detected(true);
        assert_eq!(power.charge_state(), ChargeState::Charging);
        assert!(power.is_charging());

        power.set_charging_detected(false);
        assert_eq!(power.charge_state(), ChargeState::NotCharging);
        assert!(!power.is_charging());
    }

    #[test]
    fn light_sleep_request_is_recorded_for_target_adapter() {
        let mut power = X4Power::default();
        power.light_sleep(30_000);
        assert_eq!(power.last_light_sleep_ms(), Some(30_000));
    }
}
