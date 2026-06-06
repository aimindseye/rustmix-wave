//! QMI8658 six-axis motion diagnostics ported from the uploaded sample app.
//!
//! The first IMU milestone intentionally keeps the protocol narrow: probe the
//! two documented SA0 addresses, verify `WHO_AM_I`, apply the sample app's
//! accelerometer and gyroscope profile, and read one contiguous diagnostic
//! frame. Wake-on-motion interrupts remain a later power-management slice.

use core::fmt::Debug;

use anyhow::{anyhow, bail, Result};
use embedded_hal::i2c::I2c;

/// QMI8658 SA0-low address used by the uploaded Waveshare sample.
pub const QMI8658_ADDRESS_LOW: u8 = 0x6A;
/// QMI8658 SA0-high fallback address supported by the uploaded sample.
pub const QMI8658_ADDRESS_HIGH: u8 = 0x6B;
/// Device identifier required by the sample driver's probe loop.
pub const QMI8658_WHO_AM_I_VALUE: u8 = 0x05;

const WHO_AM_I: u8 = 0x00;
const REVISION: u8 = 0x01;
const CTRL1: u8 = 0x02;
const CTRL2: u8 = 0x03;
const CTRL3: u8 = 0x04;
const CTRL5: u8 = 0x06;
const CTRL7: u8 = 0x08;
const STATUS0: u8 = 0x2E;
const TEMPERATURE_L: u8 = 0x33;

// Uploaded sample profile: accelerometer +/-8 g at 1000 Hz, gyroscope
// +/-512 dps at 1000 Hz, ACC + GYR enabled. The sample clears CTRL5 after
// configuration, so this focused port preserves that reviewed behavior.
const SAMPLE_CTRL1: u8 = 0x60;
const SAMPLE_CTRL2_ACC_8G_1000HZ: u8 = 0x23;
const SAMPLE_CTRL3_GYR_512DPS_1000HZ: u8 = 0x43;
const SAMPLE_CTRL5: u8 = 0x00;
const SAMPLE_CTRL7_ACC_GYR_ENABLE: u8 = 0x03;
const ACCEL_LSB_PER_G: i32 = 1 << 12;
const GYRO_LSB_PER_DPS: i32 = 64;

/// Signed fixed-point axis values in tenths of the displayed unit.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Axis3Tenths {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl Axis3Tenths {
    /// Render compact signed X / Y / Z values for logs.
    #[must_use]
    pub fn compact_label(self) -> String {
        format!(
            "x={} y={} z={}",
            format_tenths(self.x),
            format_tenths(self.y),
            format_tenths(self.z)
        )
    }
}

/// Dominant board-axis hint derived from the accelerometer vector.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum DominantAxis {
    PositiveX,
    NegativeX,
    PositiveY,
    NegativeY,
    PositiveZ,
    NegativeZ,
    #[default]
    Unknown,
}

impl DominantAxis {
    /// Stable product-facing label that avoids assuming enclosure orientation.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::PositiveX => "+X dominant",
            Self::NegativeX => "-X dominant",
            Self::PositiveY => "+Y dominant",
            Self::NegativeY => "-Y dominant",
            Self::PositiveZ => "+Z dominant",
            Self::NegativeZ => "-Z dominant",
            Self::Unknown => "unknown",
        }
    }
}

/// One hardware-independent QMI8658 diagnostic snapshot.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ImuReading {
    /// Acceleration in tenths of a milligravity unit.
    pub acceleration_mg_tenths: Axis3Tenths,
    /// Angular velocity in tenths of a degree per second.
    pub gyroscope_dps_tenths: Axis3Tenths,
    /// QMI8658 die temperature in tenths of a degree Celsius.
    pub temperature_tenths_c: i16,
    /// Magnitude of the acceleration vector in whole milligravity units.
    pub motion_magnitude_mg: u32,
    /// Dominant raw board axis derived from acceleration.
    pub dominant_axis: DominantAxis,
    /// QMI8658 STATUS0 value captured beside the sample.
    pub status0: u8,
}

impl ImuReading {
    /// Header-friendly motion magnitude label.
    #[must_use]
    pub fn magnitude_label(self) -> String {
        format!("{} mg", self.motion_magnitude_mg)
    }

    /// QMI8658 die-temperature label for diagnostics.
    #[must_use]
    pub fn temperature_label(self) -> String {
        format!("{} C", format_tenths(i32::from(self.temperature_tenths_c)))
    }
}

/// Successful QMI8658 startup report.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ImuInitReport {
    pub address: u8,
    pub who_am_i: u8,
    pub revision: u8,
}

/// Narrow register-level QMI8658 driver.
pub struct Qmi8658<I2C> {
    i2c: I2C,
    address: Option<u8>,
}

impl<I2C> Qmi8658<I2C> {
    #[must_use]
    pub fn new(i2c: I2C) -> Self {
        Self { i2c, address: None }
    }
}

impl<I2C> Qmi8658<I2C>
where
    I2C: I2c,
    I2C::Error: Debug,
{
    /// Probe both SA0 addresses, require the reference chip ID, and apply the
    /// uploaded sample app's accelerometer and gyroscope profile.
    pub fn initialize(&mut self) -> Result<ImuInitReport> {
        let address = self.probe_address()?;
        self.address = Some(address);
        let revision = self.read_register(address, REVISION)?;

        self.write_register(address, CTRL1, SAMPLE_CTRL1)?;
        self.write_register(address, CTRL2, SAMPLE_CTRL2_ACC_8G_1000HZ)?;
        self.write_register(address, CTRL3, SAMPLE_CTRL3_GYR_512DPS_1000HZ)?;
        self.write_register(address, CTRL5, SAMPLE_CTRL5)?;
        self.write_register(address, CTRL7, SAMPLE_CTRL7_ACC_GYR_ENABLE)?;

        let enabled = self.read_register(address, CTRL7)?;
        if enabled & SAMPLE_CTRL7_ACC_GYR_ENABLE != SAMPLE_CTRL7_ACC_GYR_ENABLE {
            bail!("QMI8658 CTRL7 verification failed: read 0x{enabled:02X}");
        }

        Ok(ImuInitReport {
            address,
            who_am_i: QMI8658_WHO_AM_I_VALUE,
            revision,
        })
    }

    /// Read STATUS0 and one contiguous temperature + accelerometer +
    /// gyroscope frame. The contiguous read keeps shared-I2C ownership small.
    pub fn read_motion(&mut self) -> Result<ImuReading> {
        let address = self
            .address
            .ok_or_else(|| anyhow!("QMI8658 read requested before initialization"))?;
        let status0 = self.read_register(address, STATUS0)?;
        let mut frame = [0_u8; 14];
        self.i2c
            .write_read(address, &[TEMPERATURE_L], &mut frame)
            .map_err(|error| anyhow!("QMI8658 motion frame read failed: {error:?}"))?;
        Ok(decode_motion_frame(frame, status0))
    }

    fn probe_address(&mut self) -> Result<u8> {
        for address in [QMI8658_ADDRESS_LOW, QMI8658_ADDRESS_HIGH] {
            let mut who_am_i = [0_u8; 1];
            if self
                .i2c
                .write_read(address, &[WHO_AM_I], &mut who_am_i)
                .is_ok()
                && who_am_i[0] == QMI8658_WHO_AM_I_VALUE
            {
                return Ok(address);
            }
        }
        bail!(
            "QMI8658 probe failed: expected WHO_AM_I 0x{QMI8658_WHO_AM_I_VALUE:02X} at 0x{QMI8658_ADDRESS_LOW:02X} or 0x{QMI8658_ADDRESS_HIGH:02X}"
        )
    }

    fn read_register(&mut self, address: u8, register: u8) -> Result<u8> {
        let mut value = [0_u8; 1];
        self.i2c
            .write_read(address, &[register], &mut value)
            .map_err(|error| anyhow!("QMI8658 read 0x{register:02X} failed: {error:?}"))?;
        Ok(value[0])
    }

    fn write_register(&mut self, address: u8, register: u8, value: u8) -> Result<()> {
        self.i2c
            .write(address, &[register, value])
            .map_err(|error| anyhow!("QMI8658 write 0x{register:02X} failed: {error:?}"))
    }
}

fn decode_motion_frame(frame: [u8; 14], status0: u8) -> ImuReading {
    let temperature_raw = i16::from_le_bytes([frame[0], frame[1]]);
    let acceleration_raw = [
        i16::from_le_bytes([frame[2], frame[3]]),
        i16::from_le_bytes([frame[4], frame[5]]),
        i16::from_le_bytes([frame[6], frame[7]]),
    ];
    let gyroscope_raw = [
        i16::from_le_bytes([frame[8], frame[9]]),
        i16::from_le_bytes([frame[10], frame[11]]),
        i16::from_le_bytes([frame[12], frame[13]]),
    ];

    let acceleration_mg_tenths = Axis3Tenths {
        x: i32::from(acceleration_raw[0]) * 10_000 / ACCEL_LSB_PER_G,
        y: i32::from(acceleration_raw[1]) * 10_000 / ACCEL_LSB_PER_G,
        z: i32::from(acceleration_raw[2]) * 10_000 / ACCEL_LSB_PER_G,
    };
    let gyroscope_dps_tenths = Axis3Tenths {
        x: i32::from(gyroscope_raw[0]) * 10 / GYRO_LSB_PER_DPS,
        y: i32::from(gyroscope_raw[1]) * 10 / GYRO_LSB_PER_DPS,
        z: i32::from(gyroscope_raw[2]) * 10 / GYRO_LSB_PER_DPS,
    };

    ImuReading {
        acceleration_mg_tenths,
        gyroscope_dps_tenths,
        temperature_tenths_c: (i32::from(temperature_raw) * 10 / 256) as i16,
        motion_magnitude_mg: motion_magnitude_mg(acceleration_mg_tenths),
        dominant_axis: dominant_axis(acceleration_mg_tenths),
        status0,
    }
}

fn motion_magnitude_mg(acceleration: Axis3Tenths) -> u32 {
    let x = i64::from(acceleration.x) / 10;
    let y = i64::from(acceleration.y) / 10;
    let z = i64::from(acceleration.z) / 10;
    integer_sqrt((x * x + y * y + z * z) as u64) as u32
}

fn dominant_axis(acceleration: Axis3Tenths) -> DominantAxis {
    let values = [acceleration.x, acceleration.y, acceleration.z];
    let mut index = 0_usize;
    let mut value = values[0];
    for (candidate_index, candidate) in values.iter().copied().enumerate().skip(1) {
        if candidate.abs() > value.abs() {
            index = candidate_index;
            value = candidate;
        }
    }
    if value == 0 {
        return DominantAxis::Unknown;
    }
    match (index, value.is_positive()) {
        (0, true) => DominantAxis::PositiveX,
        (0, false) => DominantAxis::NegativeX,
        (1, true) => DominantAxis::PositiveY,
        (1, false) => DominantAxis::NegativeY,
        (2, true) => DominantAxis::PositiveZ,
        (2, false) => DominantAxis::NegativeZ,
        _ => DominantAxis::Unknown,
    }
}

fn integer_sqrt(value: u64) -> u64 {
    if value < 2 {
        return value;
    }
    let mut x = value;
    let mut next = (x + value / x) / 2;
    while next < x {
        x = next;
        next = (x + value / x) / 2;
    }
    x
}

/// Render a signed fixed-point tenths value without floating point.
#[must_use]
pub fn format_tenths(value: i32) -> String {
    let magnitude = i64::from(value).abs();
    let sign = if value < 0 { "-" } else { "" };
    format!("{sign}{}.{:01}", magnitude / 10, magnitude % 10)
}

#[cfg(test)]
mod tests {
    use super::{
        decode_motion_frame, dominant_axis, format_tenths, integer_sqrt, Axis3Tenths, DominantAxis,
    };

    #[test]
    fn decodes_reference_profile_frame_without_floating_point() {
        let reading = decode_motion_frame(
            [
                0x00, 0x0A, // 10.0 C die temperature
                0x00, 0x10, // +4096 => +1000.0 mg X
                0x00, 0x00, // 0 mg Y
                0x00, 0x00, // 0 mg Z
                0x40, 0x00, // +64 => +1.0 dps X
                0x00, 0x00, // 0 dps Y
                0x00, 0x00, // 0 dps Z
            ],
            0x03,
        );
        assert_eq!(reading.temperature_tenths_c, 100);
        assert_eq!(reading.acceleration_mg_tenths.x, 10_000);
        assert_eq!(reading.gyroscope_dps_tenths.x, 10);
        assert_eq!(reading.motion_magnitude_mg, 1_000);
        assert_eq!(reading.dominant_axis, DominantAxis::PositiveX);
        assert_eq!(reading.status0, 0x03);
    }

    #[test]
    fn labels_signed_tenths() {
        assert_eq!(format_tenths(123), "12.3");
        assert_eq!(format_tenths(-7), "-0.7");
    }

    #[test]
    fn integer_sqrt_is_stable_for_motion_vectors() {
        assert_eq!(integer_sqrt(0), 0);
        assert_eq!(integer_sqrt(1_000_000), 1_000);
        assert_eq!(integer_sqrt(2_000_000), 1_414);
    }

    #[test]
    fn dominant_axis_uses_largest_absolute_acceleration() {
        assert_eq!(
            dominant_axis(Axis3Tenths {
                x: 15,
                y: -999,
                z: 200,
            }),
            DominantAxis::NegativeY
        );
    }
}
