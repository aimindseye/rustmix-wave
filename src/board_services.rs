//! Sample-app board-service facade for RTC, environment, battery and motion status.

use core::fmt::Debug;

use embedded_hal::{delay::DelayNs, i2c::I2c};
use log::warn;

use crate::{
    environment::{EnvironmentReading, Shtc3},
    imu::{ImuReading, Qmi8658},
    ntp::rtc_storage_wall_clock_from_utc,
    power::{Axp2101, PowerSnapshot},
    power_key::PowerKeyEvent,
    regional::{RegionalPreferences, TemperatureUnit},
    rtc::{Pcf85063, RtcDateTime},
    shared_i2c::SharedI2cBus,
};

/// Hardware-independent snapshot consumed by product screens.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BoardSnapshot {
    pub rtc: Option<RtcDateTime>,
    pub environment: Option<EnvironmentReading>,
    pub power: Option<PowerSnapshot>,
    pub imu: Option<ImuReading>,
    pub rtc_clock_integrity_was_lost: bool,
    pub environment_sensor_id: Option<u16>,
    pub imu_address: Option<u8>,
    pub imu_revision: Option<u8>,
}

impl BoardSnapshot {
    #[must_use]
    pub fn time_label(self, regional: RegionalPreferences) -> String {
        self.rtc.map_or_else(
            || "--:--".into(),
            |rtc| regional.localize_rtc(rtc).time_hm(),
        )
    }

    #[must_use]
    pub fn date_time_label(self, regional: RegionalPreferences) -> String {
        self.rtc.map_or_else(
            || "RTC unavailable".into(),
            |rtc| regional.localize_rtc(rtc).date_time(),
        )
    }

    #[must_use]
    pub fn battery_label(self) -> String {
        self.power
            .and_then(|snapshot| snapshot.battery_percent)
            .map_or_else(|| "BAT --".into(), |percent| format!("BAT {percent}%"))
    }

    #[must_use]
    pub fn temperature_label(self, unit: TemperatureUnit) -> String {
        self.environment.map_or_else(
            || format!("--.-{}", unit.suffix()),
            |reading| reading.temperature_label(unit),
        )
    }

    #[must_use]
    pub fn humidity_label(self) -> String {
        self.environment
            .map_or_else(|| "--.-%".into(), EnvironmentReading::humidity_label)
    }

    #[must_use]
    pub fn motion_label(self) -> String {
        self.imu.map_or_else(
            || "IMU --".into(),
            |reading| format!("IMU {} mg", reading.motion_magnitude_mg),
        )
    }
}

/// Startup availability report. Missing optional services do not block display.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BoardInitReport {
    pub rtc_available: bool,
    pub environment_available: bool,
    pub power_monitoring_available: bool,
    pub imu_available: bool,
    pub rtc_clock_integrity_was_lost: bool,
    pub environment_sensor_id: Option<u16>,
    pub imu_address: Option<u8>,
    pub imu_revision: Option<u8>,
}

/// Owns independent protocol drivers backed by one cloneable I2C adapter.
pub struct BoardServices<I2C> {
    rtc: Pcf85063<SharedI2cBus<I2C>>,
    environment: Shtc3<SharedI2cBus<I2C>>,
    power: Axp2101<SharedI2cBus<I2C>>,
    imu: Qmi8658<SharedI2cBus<I2C>>,
    init_report: BoardInitReport,
}

impl<I2C> BoardServices<I2C>
where
    I2C: I2c,
    I2C::Error: Debug,
{
    #[must_use]
    pub fn new(bus: SharedI2cBus<I2C>) -> Self {
        Self {
            rtc: Pcf85063::new(bus.clone()),
            environment: Shtc3::new(bus.clone()),
            power: Axp2101::new(bus.clone()),
            imu: Qmi8658::new(bus),
            init_report: BoardInitReport::default(),
        }
    }

    /// Initialize optional sample-app services independently so a missing
    /// sensor never prevents the verified e-paper shell from booting.
    pub fn initialize<D: DelayNs>(&mut self, delay: &mut D) -> BoardInitReport {
        match self.rtc.initialize() {
            Ok(report) => {
                self.init_report.rtc_available = true;
                self.init_report.rtc_clock_integrity_was_lost = report.clock_integrity_was_lost;
            }
            Err(error) => warn!("sample-services: RTC init unavailable: {error:#}"),
        }

        match self.environment.initialize(delay) {
            Ok(id) => {
                self.init_report.environment_available = true;
                self.init_report.environment_sensor_id = Some(id);
            }
            Err(error) => warn!("sample-services: SHTC3 init unavailable: {error:#}"),
        }

        match self.power.initialize_sample_monitoring() {
            Ok(()) => self.init_report.power_monitoring_available = true,
            Err(error) => warn!("sample-services: AXP2101 monitoring unavailable: {error:#}"),
        }

        match self.imu.initialize() {
            Ok(report) => {
                self.init_report.imu_available = true;
                self.init_report.imu_address = Some(report.address);
                self.init_report.imu_revision = Some(report.revision);
            }
            Err(error) => warn!("sample-services: QMI8658 init unavailable: {error:#}"),
        }

        self.init_report
    }

    /// Persist one validated UTC SNTP result into the PCF85063 wall-clock basis
    /// retained from the uploaded sample app.
    pub fn sync_rtc_from_utc(&mut self, utc: RtcDateTime) -> anyhow::Result<RtcDateTime> {
        let stored = rtc_storage_wall_clock_from_utc(utc);
        self.rtc.write_datetime(stored)?;
        self.init_report.rtc_available = true;
        self.init_report.rtc_clock_integrity_was_lost = false;
        Ok(stored)
    }

    /// Read only the RTC wall clock for alarm polling without waking other
    /// optional sensors or generating a full dashboard snapshot.
    pub fn read_rtc(&mut self) -> anyhow::Result<RtcDateTime> {
        self.rtc.read_datetime()
    }

    /// Program the PCF85063 single hardware alarm slot using retained RTC
    /// storage-basis fields selected by the alarm domain scheduler.
    pub fn program_rtc_alarm(&mut self, stored: RtcDateTime) -> anyhow::Result<()> {
        self.rtc.program_alarm(stored)
    }

    /// Disable the PCF85063 hardware alarm slot when no schedule is armed.
    pub fn disable_rtc_alarm(&mut self) -> anyhow::Result<()> {
        self.rtc.disable_alarm()
    }

    /// Read and clear the sticky PCF85063 alarm flag. A missing RTC remains a
    /// non-fatal service error handled by the caller.
    pub fn take_rtc_alarm_flag(&mut self) -> anyhow::Result<bool> {
        let asserted = self.rtc.alarm_flag()?;
        if asserted {
            self.rtc.clear_alarm_flag()?;
        }
        Ok(asserted)
    }

    /// Enable PMIC short-menu and long-sleep Power-key event polling.
    pub fn initialize_power_key_events(&mut self) -> anyhow::Result<()> {
        self.power.initialize_power_key_events()
    }

    /// Return and clear one PMIC short power-key event when present.
    pub fn take_power_key_event(&mut self) -> anyhow::Result<Option<PowerKeyEvent>> {
        self.power.take_power_key_event()
    }

    /// Read one bounded QMI8658 sample without waking unrelated services.
    /// The Motion Events screen uses this for its native 80 ms sampler.
    pub fn read_imu_motion(&mut self) -> anyhow::Result<ImuReading> {
        if !self.init_report.imu_available {
            anyhow::bail!("QMI8658 service unavailable");
        }
        self.imu.read_motion()
    }

    /// Capture a best-effort status snapshot. Each optional field remains
    /// independent so one absent sensor cannot blank unrelated status values.
    pub fn read_snapshot<D: DelayNs>(&mut self, delay: &mut D) -> BoardSnapshot {
        let rtc = match self.rtc.read_datetime() {
            Ok(value) => Some(value),
            Err(error) => {
                warn!("sample-services: RTC read unavailable: {error:#}");
                None
            }
        };
        let environment = match self.environment.read_environment(delay) {
            Ok(value) => Some(value),
            Err(error) => {
                warn!("sample-services: SHTC3 read unavailable: {error:#}");
                None
            }
        };
        let power = match self.power.read_power_snapshot() {
            Ok(value) => Some(value),
            Err(error) => {
                warn!("sample-services: AXP2101 status unavailable: {error:#}");
                None
            }
        };
        let imu = if self.init_report.imu_available {
            match self.imu.read_motion() {
                Ok(value) => Some(value),
                Err(error) => {
                    warn!("sample-services: QMI8658 read unavailable: {error:#}");
                    None
                }
            }
        } else {
            None
        };

        BoardSnapshot {
            rtc,
            environment,
            power,
            imu,
            rtc_clock_integrity_was_lost: self.init_report.rtc_clock_integrity_was_lost,
            environment_sensor_id: self.init_report.environment_sensor_id,
            imu_address: self.init_report.imu_address,
            imu_revision: self.init_report.imu_revision,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::BoardSnapshot;
    use crate::{
        environment::EnvironmentReading,
        imu::{Axis3Tenths, DominantAxis, ImuReading},
        power::PowerSnapshot,
        regional::{RegionalPreferences, TemperatureUnit},
        rtc::RtcDateTime,
    };

    #[test]
    fn unavailable_snapshot_renders_placeholders() {
        let snapshot = BoardSnapshot::default();
        assert_eq!(snapshot.time_label(RegionalPreferences::default()), "--:--");
        assert_eq!(snapshot.battery_label(), "BAT --");
        assert_eq!(
            snapshot.temperature_label(TemperatureUnit::Fahrenheit),
            "--.- F"
        );
        assert_eq!(snapshot.humidity_label(), "--.-%");
        assert_eq!(snapshot.motion_label(), "IMU --");
    }

    #[test]
    fn available_snapshot_renders_sample_status() {
        let snapshot = BoardSnapshot {
            rtc: Some(RtcDateTime {
                year: 2026,
                month: 6,
                day: 3,
                weekday: 3,
                hour: 14,
                minute: 5,
                second: 8,
            }),
            environment: Some(EnvironmentReading {
                temperature_tenths_c: 231,
                humidity_tenths_percent: 487,
            }),
            power: Some(PowerSnapshot {
                battery_percent: Some(82),
                battery_voltage_mv: Some(3980),
                vbus_present: true,
                charging: true,
            }),
            imu: Some(ImuReading {
                acceleration_mg_tenths: Axis3Tenths {
                    x: 0,
                    y: 0,
                    z: 10_000,
                },
                gyroscope_dps_tenths: Axis3Tenths::default(),
                temperature_tenths_c: 250,
                motion_magnitude_mg: 1_000,
                dominant_axis: DominantAxis::PositiveZ,
                status0: 0x03,
            }),
            ..BoardSnapshot::default()
        };
        assert_eq!(snapshot.time_label(RegionalPreferences::default()), "02:05");
        assert_eq!(snapshot.battery_label(), "BAT 82%");
        assert_eq!(
            snapshot.temperature_label(TemperatureUnit::Fahrenheit),
            "73.6 F"
        );
        assert_eq!(snapshot.humidity_label(), "48.7%");
        assert_eq!(snapshot.motion_label(), "IMU 1000 mg");
    }
}
