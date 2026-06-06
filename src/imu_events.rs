//! Native QMI8658 event bridge with bounded threshold controls.
//!
//! Raw QMI8658 samples remain behind the Rust-owned I2C boundary. This module
//! converts fixed-point accelerometer and gyroscope snapshots into debounced
//! tilt, shake, rotate and level events. The first product slice activates the
//! bridge only while the sample Motion Events screen is visible; future games
//! can subscribe through the same native boundary without receiving raw I2C.

use crate::imu::{Axis3Tenths, ImuReading};

/// Poll cadence used only while the Motion Events sample screen is active.
pub const IMU_EVENT_SAMPLE_INTERVAL_MS: u64 = 80;
/// Slow e-paper diagnostics heartbeat; events redraw immediately.
pub const IMU_EVENT_SCREEN_REFRESH_SECONDS: u64 = 5;
/// Number of interactive diagnostics rows.
pub const IMU_EVENT_CONTROL_COUNT: usize = 7;
const TILT_RELEASE_MG: i32 = 320;
const STABLE_SAMPLES_REQUIRED: u8 = 3;
const SHAKE_COOLDOWN_MS: u64 = 900;
const ROTATE_COOLDOWN_MS: u64 = 650;
const ROTATE_RELEASE_DPS: i32 = 45;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MotionAxis {
    PositiveX,
    NegativeX,
    PositiveY,
    NegativeY,
    PositiveZ,
    NegativeZ,
}

impl MotionAxis {
    #[must_use]
    pub const fn marker(self) -> &'static str {
        match self {
            Self::PositiveX => "+x",
            Self::NegativeX => "-x",
            Self::PositiveY => "+y",
            Self::NegativeY => "-y",
            Self::PositiveZ => "+z",
            Self::NegativeZ => "-z",
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::PositiveX => "+X",
            Self::NegativeX => "-X",
            Self::PositiveY => "+Y",
            Self::NegativeY => "-Y",
            Self::PositiveZ => "+Z",
            Self::NegativeZ => "-Z",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImuDetectedEventKind {
    Tilt(MotionAxis),
    Shake,
    Rotate(MotionAxis),
    Level,
}

impl ImuDetectedEventKind {
    #[must_use]
    pub const fn marker(self) -> &'static str {
        match self {
            Self::Tilt(_) => "tilt",
            Self::Shake => "shake",
            Self::Rotate(_) => "rotate",
            Self::Level => "level",
        }
    }

    #[must_use]
    pub fn label(self) -> String {
        match self {
            Self::Tilt(axis) => format!("TILT {}", axis.label()),
            Self::Shake => "SHAKE".into(),
            Self::Rotate(axis) => format!("ROTATE {}", axis.label()),
            Self::Level => "LEVEL".into(),
        }
    }

    #[must_use]
    pub const fn detail_marker(self) -> &'static str {
        match self {
            Self::Tilt(axis) | Self::Rotate(axis) => axis.marker(),
            Self::Shake => "impulse",
            Self::Level => "flat",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ImuDetectedEvent {
    pub kind: ImuDetectedEventKind,
    pub at_ms: u64,
}

impl ImuDetectedEvent {
    #[must_use]
    pub fn label(self) -> String {
        self.kind.label()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ImuEventThresholds {
    pub tilt_enter_mg: i32,
    pub shake_delta_mg: i32,
    pub rotate_dps: i32,
    pub level_tolerance_mg: i32,
    pub debounce_ms: u64,
}

impl Default for ImuEventThresholds {
    fn default() -> Self {
        Self {
            tilt_enter_mg: 550,
            shake_delta_mg: 420,
            rotate_dps: 120,
            level_tolerance_mg: 150,
            debounce_ms: 350,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ImuEventCounters {
    pub tilt: u32,
    pub shake: u32,
    pub rotate: u32,
    pub level: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImuControlOutcome {
    Updated,
    CountersReset,
    OpenDetails,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImuEventBridge {
    pub thresholds: ImuEventThresholds,
    pub counters: ImuEventCounters,
    pub selected_control: usize,
    pub samples: u32,
    pub last_event: Option<ImuDetectedEvent>,
    active_tilt: Option<MotionAxis>,
    pending_tilt: Option<MotionAxis>,
    pending_tilt_samples: u8,
    level_active: bool,
    pending_level_samples: u8,
    last_event_ms: Option<u64>,
    last_shake_ms: Option<u64>,
    last_rotate_ms: Option<u64>,
    rotate_active: bool,
}

impl Default for ImuEventBridge {
    fn default() -> Self {
        Self {
            thresholds: ImuEventThresholds::default(),
            counters: ImuEventCounters::default(),
            selected_control: 0,
            samples: 0,
            last_event: None,
            active_tilt: None,
            pending_tilt: None,
            pending_tilt_samples: 0,
            level_active: false,
            pending_level_samples: 0,
            last_event_ms: None,
            last_shake_ms: None,
            last_rotate_ms: None,
            rotate_active: false,
        }
    }
}

impl ImuEventBridge {
    #[must_use]
    pub fn process(&mut self, reading: ImuReading, now_ms: u64) -> Option<ImuDetectedEvent> {
        self.samples = self.samples.saturating_add(1);

        let shake_delta = (reading.motion_magnitude_mg as i32 - 1_000).abs();
        if shake_delta >= self.thresholds.shake_delta_mg
            && elapsed_since(self.last_shake_ms, now_ms) >= SHAKE_COOLDOWN_MS
        {
            self.last_shake_ms = Some(now_ms);
            return Some(self.record(ImuDetectedEventKind::Shake, now_ms));
        }

        if let Some(event) = self.detect_rotate(reading.gyroscope_dps_tenths, now_ms) {
            return Some(event);
        }

        if let Some(event) = self.detect_tilt(reading.acceleration_mg_tenths, now_ms) {
            return Some(event);
        }

        self.detect_level(reading.acceleration_mg_tenths, now_ms)
    }

    fn detect_rotate(&mut self, gyroscope: Axis3Tenths, now_ms: u64) -> Option<ImuDetectedEvent> {
        let strongest = strongest_axis(gyroscope);
        let dps = strongest.map_or(0, |(_, tenths)| tenths / 10);
        if self.rotate_active {
            if dps <= ROTATE_RELEASE_DPS {
                self.rotate_active = false;
            }
            return None;
        }
        let (axis, _) = strongest?;
        if dps < self.thresholds.rotate_dps
            || elapsed_since(self.last_rotate_ms, now_ms) < ROTATE_COOLDOWN_MS
            || elapsed_since(self.last_event_ms, now_ms) < self.thresholds.debounce_ms
        {
            return None;
        }
        self.rotate_active = true;
        self.last_rotate_ms = Some(now_ms);
        Some(self.record(ImuDetectedEventKind::Rotate(axis), now_ms))
    }

    fn detect_tilt(&mut self, acceleration: Axis3Tenths, now_ms: u64) -> Option<ImuDetectedEvent> {
        if let Some(active) = self.active_tilt {
            if axis_component_mg(acceleration, active).abs() <= TILT_RELEASE_MG {
                self.active_tilt = None;
            }
        }
        if self.active_tilt.is_some() {
            return None;
        }

        let candidate = strongest_planar_axis(acceleration)
            .filter(|(_, value)| *value >= self.thresholds.tilt_enter_mg)
            .map(|(axis, _)| axis);
        if candidate.is_none() {
            self.pending_tilt = None;
            self.pending_tilt_samples = 0;
            return None;
        }
        if candidate == self.pending_tilt {
            self.pending_tilt_samples = self.pending_tilt_samples.saturating_add(1);
        } else {
            self.pending_tilt = candidate;
            self.pending_tilt_samples = 1;
        }
        if self.pending_tilt_samples < STABLE_SAMPLES_REQUIRED
            || elapsed_since(self.last_event_ms, now_ms) < self.thresholds.debounce_ms
        {
            return None;
        }
        let axis = candidate.expect("tilt candidate");
        self.active_tilt = Some(axis);
        self.pending_tilt = None;
        self.pending_tilt_samples = 0;
        Some(self.record(ImuDetectedEventKind::Tilt(axis), now_ms))
    }

    fn detect_level(&mut self, acceleration: Axis3Tenths, now_ms: u64) -> Option<ImuDetectedEvent> {
        let x = acceleration.x.abs() / 10;
        let y = acceleration.y.abs() / 10;
        let z = acceleration.z.abs() / 10;
        let level = x <= self.thresholds.level_tolerance_mg
            && y <= self.thresholds.level_tolerance_mg
            && (z - 1_000).abs() <= self.thresholds.level_tolerance_mg;
        if !level {
            self.level_active = false;
            self.pending_level_samples = 0;
            return None;
        }
        if self.level_active {
            return None;
        }
        self.pending_level_samples = self.pending_level_samples.saturating_add(1);
        if self.pending_level_samples < STABLE_SAMPLES_REQUIRED
            || elapsed_since(self.last_event_ms, now_ms) < self.thresholds.debounce_ms
        {
            return None;
        }
        self.level_active = true;
        self.pending_level_samples = 0;
        Some(self.record(ImuDetectedEventKind::Level, now_ms))
    }

    fn record(&mut self, kind: ImuDetectedEventKind, at_ms: u64) -> ImuDetectedEvent {
        match kind {
            ImuDetectedEventKind::Tilt(_) => {
                self.counters.tilt = self.counters.tilt.saturating_add(1)
            }
            ImuDetectedEventKind::Shake => {
                self.counters.shake = self.counters.shake.saturating_add(1)
            }
            ImuDetectedEventKind::Rotate(_) => {
                self.counters.rotate = self.counters.rotate.saturating_add(1)
            }
            ImuDetectedEventKind::Level => {
                self.counters.level = self.counters.level.saturating_add(1)
            }
        }
        self.last_event_ms = Some(at_ms);
        let event = ImuDetectedEvent { kind, at_ms };
        self.last_event = Some(event);
        event
    }

    pub fn select_previous_control(&mut self) {
        self.selected_control = self
            .selected_control
            .checked_sub(1)
            .unwrap_or(IMU_EVENT_CONTROL_COUNT - 1);
    }

    pub fn select_next_control(&mut self) {
        self.selected_control = (self.selected_control + 1) % IMU_EVENT_CONTROL_COUNT;
    }

    #[must_use]
    pub fn apply_selected_control(&mut self) -> ImuControlOutcome {
        match self.selected_control {
            0 => {
                self.thresholds.tilt_enter_mg =
                    cycle_i32(self.thresholds.tilt_enter_mg, &[450, 550, 650, 750])
            }
            1 => {
                self.thresholds.shake_delta_mg =
                    cycle_i32(self.thresholds.shake_delta_mg, &[300, 420, 550, 700])
            }
            2 => {
                self.thresholds.rotate_dps =
                    cycle_i32(self.thresholds.rotate_dps, &[80, 120, 180, 240])
            }
            3 => {
                self.thresholds.level_tolerance_mg =
                    cycle_i32(self.thresholds.level_tolerance_mg, &[100, 150, 200, 250])
            }
            4 => {
                self.thresholds.debounce_ms =
                    cycle_u64(self.thresholds.debounce_ms, &[200, 350, 500, 750])
            }
            5 => {
                self.counters = ImuEventCounters::default();
                self.last_event = None;
                return ImuControlOutcome::CountersReset;
            }
            6 => return ImuControlOutcome::OpenDetails,
            _ => unreachable!("bounded IMU control row"),
        }
        self.reset_detection_latches();
        ImuControlOutcome::Updated
    }

    fn reset_detection_latches(&mut self) {
        self.active_tilt = None;
        self.pending_tilt = None;
        self.pending_tilt_samples = 0;
        self.level_active = false;
        self.pending_level_samples = 0;
        self.rotate_active = false;
    }

    #[must_use]
    pub const fn control_label(index: usize) -> &'static str {
        match index {
            0 => "Tilt enter",
            1 => "Shake delta",
            2 => "Rotate",
            3 => "Level tolerance",
            4 => "Debounce",
            5 => "Reset counters",
            6 => "Sensor details",
            _ => "--",
        }
    }

    #[must_use]
    pub fn control_value(&self, index: usize) -> String {
        match index {
            0 => format!("{} mg", self.thresholds.tilt_enter_mg),
            1 => format!("{} mg", self.thresholds.shake_delta_mg),
            2 => format!("{} dps", self.thresholds.rotate_dps),
            3 => format!("{} mg", self.thresholds.level_tolerance_mg),
            4 => format!("{} ms", self.thresholds.debounce_ms),
            5 => "SELECT".into(),
            6 => "OPEN".into(),
            _ => "--".into(),
        }
    }

    #[must_use]
    pub fn latest_label(&self) -> String {
        self.last_event
            .map_or_else(|| "NONE".into(), ImuDetectedEvent::label)
    }
}

fn elapsed_since(previous: Option<u64>, now_ms: u64) -> u64 {
    previous.map_or(u64::MAX, |then| now_ms.saturating_sub(then))
}

fn cycle_i32(current: i32, values: &[i32]) -> i32 {
    let index = values
        .iter()
        .position(|value| *value == current)
        .unwrap_or(0);
    values[(index + 1) % values.len()]
}

fn cycle_u64(current: u64, values: &[u64]) -> u64 {
    let index = values
        .iter()
        .position(|value| *value == current)
        .unwrap_or(0);
    values[(index + 1) % values.len()]
}

fn strongest_planar_axis(acceleration: Axis3Tenths) -> Option<(MotionAxis, i32)> {
    strongest_axis(Axis3Tenths {
        x: acceleration.x,
        y: acceleration.y,
        z: 0,
    })
    .map(|(axis, tenths)| (axis, tenths / 10))
}

fn strongest_axis(values: Axis3Tenths) -> Option<(MotionAxis, i32)> {
    let candidates = [
        (MotionAxis::PositiveX, values.x),
        (MotionAxis::PositiveY, values.y),
        (MotionAxis::PositiveZ, values.z),
    ];
    let (positive_axis, signed_value) = candidates
        .into_iter()
        .max_by_key(|(_, value)| value.abs())?;
    if signed_value == 0 {
        return None;
    }
    let axis = match (positive_axis, signed_value.is_positive()) {
        (MotionAxis::PositiveX, true) => MotionAxis::PositiveX,
        (MotionAxis::PositiveX, false) => MotionAxis::NegativeX,
        (MotionAxis::PositiveY, true) => MotionAxis::PositiveY,
        (MotionAxis::PositiveY, false) => MotionAxis::NegativeY,
        (MotionAxis::PositiveZ, true) => MotionAxis::PositiveZ,
        (MotionAxis::PositiveZ, false) => MotionAxis::NegativeZ,
        _ => unreachable!("positive-axis candidates only"),
    };
    Some((axis, signed_value.abs()))
}

fn axis_component_mg(acceleration: Axis3Tenths, axis: MotionAxis) -> i32 {
    match axis {
        MotionAxis::PositiveX | MotionAxis::NegativeX => acceleration.x / 10,
        MotionAxis::PositiveY | MotionAxis::NegativeY => acceleration.y / 10,
        MotionAxis::PositiveZ | MotionAxis::NegativeZ => acceleration.z / 10,
    }
}

#[cfg(test)]
mod tests {
    use super::{ImuControlOutcome, ImuDetectedEventKind, ImuEventBridge, MotionAxis};
    use crate::imu::{Axis3Tenths, ImuReading};

    fn reading(acc_x_mg: i32, acc_y_mg: i32, acc_z_mg: i32, gyro_z_dps: i32) -> ImuReading {
        ImuReading {
            acceleration_mg_tenths: Axis3Tenths {
                x: acc_x_mg * 10,
                y: acc_y_mg * 10,
                z: acc_z_mg * 10,
            },
            gyroscope_dps_tenths: Axis3Tenths {
                x: 0,
                y: 0,
                z: gyro_z_dps * 10,
            },
            motion_magnitude_mg:
                (((acc_x_mg * acc_x_mg + acc_y_mg * acc_y_mg + acc_z_mg * acc_z_mg) as f64).sqrt())
                    as u32,
            ..ImuReading::default()
        }
    }

    #[test]
    fn emits_debounced_tilt_after_stable_samples() {
        let mut bridge = ImuEventBridge::default();
        assert_eq!(bridge.process(reading(700, 0, 700, 0), 1_000), None);
        assert_eq!(bridge.process(reading(700, 0, 700, 0), 1_080), None);
        assert_eq!(
            bridge.process(reading(700, 0, 700, 0), 1_160).unwrap().kind,
            ImuDetectedEventKind::Tilt(MotionAxis::PositiveX)
        );
        assert_eq!(bridge.counters.tilt, 1);
        assert_eq!(bridge.process(reading(700, 0, 700, 0), 1_240), None);
    }

    #[test]
    fn emits_shake_with_cooldown() {
        let mut bridge = ImuEventBridge::default();
        assert_eq!(
            bridge.process(reading(0, 0, 1_600, 0), 1_000).unwrap().kind,
            ImuDetectedEventKind::Shake
        );
        assert_eq!(bridge.process(reading(0, 0, 1_600, 0), 1_200), None);
        assert_eq!(
            bridge.process(reading(0, 0, 1_600, 0), 2_000).unwrap().kind,
            ImuDetectedEventKind::Shake
        );
    }

    #[test]
    fn emits_rotate_and_level() {
        let mut bridge = ImuEventBridge::default();
        assert_eq!(
            bridge
                .process(reading(0, 0, 1_000, 180), 1_000)
                .unwrap()
                .kind,
            ImuDetectedEventKind::Rotate(MotionAxis::PositiveZ)
        );
        assert_eq!(bridge.process(reading(0, 0, 1_000, 0), 1_500), None);
        assert_eq!(bridge.process(reading(0, 0, 1_000, 0), 1_580), None);
        assert_eq!(
            bridge.process(reading(0, 0, 1_000, 0), 1_660).unwrap().kind,
            ImuDetectedEventKind::Level
        );
    }

    #[test]
    fn rotate_requires_neutral_release_before_repeating() {
        let mut bridge = ImuEventBridge::default();
        assert_eq!(
            bridge
                .process(reading(0, 0, 1_000, 180), 1_000)
                .unwrap()
                .kind,
            ImuDetectedEventKind::Rotate(MotionAxis::PositiveZ)
        );
        assert_eq!(bridge.process(reading(0, 0, 1_000, 180), 2_000), None);
        assert_eq!(bridge.process(reading(0, 0, 1_000, 0), 2_080), None);
        assert_eq!(
            bridge
                .process(reading(0, 0, 1_000, 180), 2_800)
                .unwrap()
                .kind,
            ImuDetectedEventKind::Rotate(MotionAxis::PositiveZ)
        );
    }

    #[test]
    fn threshold_rows_cycle_and_reset() {
        let mut bridge = ImuEventBridge::default();
        assert_eq!(bridge.apply_selected_control(), ImuControlOutcome::Updated);
        assert_eq!(bridge.thresholds.tilt_enter_mg, 650);
        for _ in 0..5 {
            bridge.select_next_control();
        }
        bridge.counters.shake = 3;
        assert_eq!(
            bridge.apply_selected_control(),
            ImuControlOutcome::CountersReset
        );
        assert_eq!(bridge.counters.shake, 0);
        bridge.select_next_control();
        assert_eq!(
            bridge.apply_selected_control(),
            ImuControlOutcome::OpenDetails
        );
    }
}
