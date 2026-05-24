#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChargeState {
    Unknown,
    NotCharging,
    Charging,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BatteryReading {
    pub adc_mv: u16,
    pub battery_mv: u16,
    pub percentage: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PowerPolicy {
    pub idle_timeout_ms: u32,
    pub low_battery_mv: u16,
    pub critical_battery_mv: u16,
}

impl PowerPolicy {
    pub const fn new(idle_timeout_ms: u32, low_battery_mv: u16, critical_battery_mv: u16) -> Self {
        Self {
            idle_timeout_ms,
            low_battery_mv,
            critical_battery_mv,
        }
    }
}

impl Default for PowerPolicy {
    fn default() -> Self {
        // Conservative defaults for the X4/e-ink reader profile. These are
        // policy markers only; real sleep entry remains target-owned.
        Self::new(300_000, 3_600, 3_400)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PowerStatus {
    pub reading: BatteryReading,
    pub charge_state: ChargeState,
    pub low_battery: bool,
    pub critical_battery: bool,
}

pub trait PowerHal {
    /// Raw ADC-domain battery measurement in millivolts.
    fn adc_mv(&mut self) -> u16;

    /// Actual estimated battery voltage in millivolts.
    fn battery_mv(&mut self) -> u16;

    /// Normalized battery percentage for UI/status use.
    fn battery_pct(&mut self) -> u8;

    fn policy(&self) -> PowerPolicy {
        PowerPolicy::default()
    }

    fn reading(&mut self) -> BatteryReading {
        BatteryReading {
            adc_mv: self.adc_mv(),
            battery_mv: self.battery_mv(),
            percentage: self.battery_pct(),
        }
    }

    fn charge_state(&self) -> ChargeState {
        ChargeState::Unknown
    }

    fn is_charging(&self) -> bool {
        matches!(self.charge_state(), ChargeState::Charging)
    }

    fn status(&mut self) -> PowerStatus {
        let reading = self.reading();
        let policy = self.policy();
        PowerStatus {
            reading,
            charge_state: self.charge_state(),
            low_battery: reading.battery_mv <= policy.low_battery_mv,
            critical_battery: reading.battery_mv <= policy.critical_battery_mv,
        }
    }

    fn deep_sleep(&mut self, wake_after_ms: u32) -> !;
    fn light_sleep(&mut self, ms: u32);
}
