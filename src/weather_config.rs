//! SD-card weather provisioning configuration.
//!
//! Weather retrieval is intentionally separate from Wi-Fi credentials. The
//! Open-Meteo provider does not require an API key for this milestone.

use std::{collections::BTreeMap, fs, path::Path};

use anyhow::{bail, Context, Result};

/// Read-only weather provisioning file consumed at boot.
pub const WEATHER_CONFIG_PATH: &str = "/sdcard/RUSTMIX/WEATHER.TXT";
/// Provider selected for the first weather milestone.
pub const DEFAULT_WEATHER_PROVIDER: &str = "open-meteo";
/// Product-facing fallback location label.
pub const DEFAULT_LOCATION_LABEL: &str = "Configured location";
/// Default bounded weather-refresh interval.
pub const DEFAULT_REFRESH_MINUTES: u64 = 30;
/// Lower refresh bound protects the free provider and the e-paper workflow.
pub const MIN_REFRESH_MINUTES: u64 = 15;
/// Upper refresh bound keeps stale data visible without excessive polling.
pub const MAX_REFRESH_MINUTES: u64 = 360;

/// Validated boot-time weather configuration.
#[derive(Clone, Debug, PartialEq)]
pub struct WeatherConfig {
    pub provider: String,
    pub location: String,
    pub latitude: f64,
    pub longitude: f64,
    pub timezone: String,
    pub refresh_minutes: u64,
}

impl WeatherConfig {
    /// Load and validate the read-only boot configuration.
    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let contents = fs::read_to_string(path)
            .with_context(|| format!("unable to read {}", path.display()))?;
        Self::parse(&contents)
            .with_context(|| format!("invalid weather configuration in {}", path.display()))
    }

    /// Parse the intentionally small `key=value` configuration format.
    pub fn parse(contents: &str) -> Result<Self> {
        let mut values = BTreeMap::<String, String>::new();
        for (line_number, raw_line) in contents.lines().enumerate() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (key, value) = line
                .split_once('=')
                .ok_or_else(|| anyhow::anyhow!("line {} must use key=value", line_number + 1))?;
            let key = key.trim();
            let value = value.trim();
            if !matches!(
                key,
                "provider" | "location" | "latitude" | "longitude" | "timezone" | "refresh_minutes"
            ) {
                bail!("line {} uses unsupported key {key:?}", line_number + 1);
            }
            if values.insert(key.into(), value.into()).is_some() {
                bail!("line {} repeats key {key:?}", line_number + 1);
            }
        }

        let provider = values
            .get("provider")
            .cloned()
            .unwrap_or_else(|| DEFAULT_WEATHER_PROVIDER.into());
        if provider != DEFAULT_WEATHER_PROVIDER {
            bail!("provider must be open-meteo in this milestone");
        }

        let location = values
            .get("location")
            .cloned()
            .unwrap_or_else(|| DEFAULT_LOCATION_LABEL.into());
        if location.is_empty() || location.len() > 40 {
            bail!("location must contain 1 to 40 UTF-8 bytes");
        }

        let latitude = required(&values, "latitude")?
            .parse::<f64>()
            .context("latitude must be a decimal number")?;
        if !(-90.0..=90.0).contains(&latitude) {
            bail!("latitude must be between -90 and 90 degrees");
        }

        let longitude = required(&values, "longitude")?
            .parse::<f64>()
            .context("longitude must be a decimal number")?;
        if !(-180.0..=180.0).contains(&longitude) {
            bail!("longitude must be between -180 and 180 degrees");
        }

        let timezone = values
            .get("timezone")
            .cloned()
            .unwrap_or_else(|| "America/New_York".into());
        if !matches!(timezone.as_str(), "America/New_York" | "UTC") {
            bail!("timezone must be America/New_York or UTC in this milestone");
        }

        let refresh_minutes = values
            .get("refresh_minutes")
            .map(String::as_str)
            .unwrap_or("30")
            .parse::<u64>()
            .context("refresh_minutes must be an integer")?;
        if !(MIN_REFRESH_MINUTES..=MAX_REFRESH_MINUTES).contains(&refresh_minutes) {
            bail!(
                "refresh_minutes must be between {MIN_REFRESH_MINUTES} and {MAX_REFRESH_MINUTES}"
            );
        }

        Ok(Self {
            provider,
            location,
            latitude,
            longitude,
            timezone,
            refresh_minutes,
        })
    }

    /// Construct one bounded Open-Meteo request URL. The provider returns
    /// Fahrenheit and mph values directly so rendering does not mix units.
    #[must_use]
    pub fn forecast_url(&self) -> String {
        format!(
            "https://api.open-meteo.com/v1/forecast?latitude={:.4}&longitude={:.4}&current=temperature_2m,relative_humidity_2m,apparent_temperature,weather_code,wind_speed_10m&daily=weather_code,temperature_2m_max,temperature_2m_min,precipitation_probability_max&temperature_unit=fahrenheit&wind_speed_unit=mph&timezone={}&forecast_days=4",
            self.latitude, self.longitude, self.timezone
        )
    }
}

fn required<'a>(values: &'a BTreeMap<String, String>, key: &str) -> Result<&'a str> {
    values
        .get(key)
        .map(String::as_str)
        .ok_or_else(|| anyhow::anyhow!("missing required key {key:?}"))
}

#[cfg(test)]
mod tests {
    use super::{WeatherConfig, DEFAULT_REFRESH_MINUTES, DEFAULT_WEATHER_PROVIDER};

    #[test]
    fn parses_open_meteo_configuration_with_safe_defaults() {
        let config = WeatherConfig::parse(
            "location=Jersey City, NJ\nlatitude=40.7178\nlongitude=-74.0431\ntimezone=America/New_York\n",
        )
        .unwrap();
        assert_eq!(config.provider, DEFAULT_WEATHER_PROVIDER);
        assert_eq!(config.refresh_minutes, DEFAULT_REFRESH_MINUTES);
        assert!(config
            .forecast_url()
            .contains("temperature_unit=fahrenheit"));
        assert!(config.forecast_url().contains("forecast_days=4"));
    }

    #[test]
    fn accepts_utc_and_explicit_refresh_interval() {
        let config = WeatherConfig::parse(
            "provider=open-meteo\nlatitude=0\nlongitude=0\ntimezone=UTC\nrefresh_minutes=60\n",
        )
        .unwrap();
        assert_eq!(config.timezone, "UTC");
        assert_eq!(config.refresh_minutes, 60);
    }

    #[test]
    fn rejects_unknown_provider_invalid_coordinates_and_aggressive_refresh() {
        assert!(WeatherConfig::parse("provider=other\nlatitude=0\nlongitude=0\n").is_err());
        assert!(WeatherConfig::parse("latitude=91\nlongitude=0\n").is_err());
        assert!(WeatherConfig::parse("latitude=0\nlongitude=-181\n").is_err());
        assert!(WeatherConfig::parse("latitude=0\nlongitude=0\nrefresh_minutes=5\n").is_err());
    }
}
