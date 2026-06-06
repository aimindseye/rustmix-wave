//! Open-Meteo weather cache and bounded HTTP retrieval.
//!
//! JSON parsing and cache retention are hardware-independent. ESP-IDF HTTPS
//! wiring lives below `cfg(target_os = "espidf")`.

use crate::weather_config::{WeatherConfig, WEATHER_CONFIG_PATH};
use anyhow::{bail, Context, Result};

/// Maximum HTTP response accepted by the bounded weather client.
pub const MAX_WEATHER_RESPONSE_BYTES: usize = 8 * 1024;
/// HTTPS timeout used for one bounded provider request.
pub const WEATHER_HTTP_TIMEOUT_SECONDS: u64 = 15;
/// Delays used by the main-loop retry scheduler after transient failures.
pub const WEATHER_RETRY_DELAYS_SECONDS: [u64; 3] = [2, 5, 15];
/// Number of automatic retries allowed after the initial request.
pub const WEATHER_RETRY_LIMIT: usize = WEATHER_RETRY_DELAYS_SECONDS.len();

/// Provider-request failure classified for retry scheduling.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WeatherFetchError {
    Transport(String),
    HttpStatus(u16),
    InvalidResponse(String),
}

impl WeatherFetchError {
    #[must_use]
    pub const fn is_retryable(&self) -> bool {
        match self {
            Self::Transport(_) => true,
            Self::HttpStatus(status) => matches!(*status, 429 | 500 | 502 | 503 | 504),
            Self::InvalidResponse(_) => false,
        }
    }

    #[must_use]
    pub const fn category(&self) -> &'static str {
        match self {
            Self::Transport(_) => "transport",
            Self::HttpStatus(_) => "http-status",
            Self::InvalidResponse(_) => "invalid-response",
        }
    }
}

impl core::fmt::Display for WeatherFetchError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Transport(message) => write!(formatter, "{message}"),
            Self::HttpStatus(status) => {
                write!(formatter, "Open-Meteo returned HTTP status {status}")
            }
            Self::InvalidResponse(message) => write!(formatter, "{message}"),
        }
    }
}

impl std::error::Error for WeatherFetchError {}

/// Product-facing weather availability state.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum WeatherFetchState {
    Disabled,
    #[default]
    ConfigurationMissing,
    WaitingForNetwork,
    Fetching,
    Retrying,
    Ready,
    Stale,
    Failed,
}

impl WeatherFetchState {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Disabled => "DISABLED",
            Self::ConfigurationMissing => "NO CONFIG",
            Self::WaitingForNetwork => "WAIT WIFI",
            Self::Fetching => "FETCHING",
            Self::Retrying => "RETRYING",
            Self::Ready => "READY",
            Self::Stale => "STALE",
            Self::Failed => "FAILED",
        }
    }
}

/// One current-conditions payload in provider-native Fahrenheit and mph units.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CurrentConditions {
    pub observed_at: String,
    pub weather_code: u16,
    pub temperature_tenths_f: i16,
    pub apparent_temperature_tenths_f: i16,
    pub humidity_percent: u8,
    pub wind_speed_tenths_mph: u16,
}

impl CurrentConditions {
    #[must_use]
    pub fn temperature_label(&self) -> String {
        format_tenths(self.temperature_tenths_f, " F")
    }

    #[must_use]
    pub fn apparent_temperature_label(&self) -> String {
        format_tenths(self.apparent_temperature_tenths_f, " F")
    }

    #[must_use]
    pub fn wind_label(&self) -> String {
        format_tenths(self.wind_speed_tenths_mph as i16, " mph")
    }

    #[must_use]
    pub const fn condition_label(&self) -> &'static str {
        condition_label(self.weather_code)
    }
}

/// One daily forecast row.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DailyForecast {
    pub date: String,
    pub weather_code: u16,
    pub high_tenths_f: i16,
    pub low_tenths_f: i16,
    pub precipitation_probability_percent: Option<u8>,
}

impl DailyForecast {
    #[must_use]
    pub const fn condition_label(&self) -> &'static str {
        condition_label(self.weather_code)
    }

    #[must_use]
    pub fn compact_label(&self) -> String {
        format!(
            "{}  {:<12} H:{} L:{} POP:{}",
            self.date,
            self.condition_label(),
            format_tenths(self.high_tenths_f, "F"),
            format_tenths(self.low_tenths_f, "F"),
            self.precipitation_probability_percent
                .map_or_else(|| "--%".into(), |value| format!("{value}%"))
        )
    }
}

/// Successfully parsed provider response.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WeatherData {
    pub provider_timezone: String,
    pub current: CurrentConditions,
    pub forecast: Vec<DailyForecast>,
}

/// Cached rendering snapshot. Failed refreshes retain the last good payload.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WeatherSnapshot {
    pub state: WeatherFetchState,
    pub provider: String,
    pub location: String,
    pub provider_timezone: String,
    pub current: Option<CurrentConditions>,
    pub forecast: Vec<DailyForecast>,
    pub last_success: Option<String>,
    pub error: Option<String>,
}

impl Default for WeatherSnapshot {
    fn default() -> Self {
        Self {
            state: WeatherFetchState::ConfigurationMissing,
            provider: "open-meteo".into(),
            location: "--".into(),
            provider_timezone: "--".into(),
            current: None,
            forecast: Vec::new(),
            last_success: None,
            error: None,
        }
    }
}

impl WeatherSnapshot {
    #[must_use]
    pub fn provisioned(config: &WeatherConfig) -> Self {
        Self {
            state: WeatherFetchState::WaitingForNetwork,
            provider: config.provider.clone(),
            location: config.location.clone(),
            provider_timezone: config.timezone.clone(),
            ..Self::default()
        }
    }

    pub fn mark_fetching(&mut self) {
        self.state = WeatherFetchState::Fetching;
        self.error = None;
    }

    pub fn record_success(&mut self, data: WeatherData) {
        self.state = WeatherFetchState::Ready;
        self.last_success = Some(data.current.observed_at.clone());
        self.provider_timezone = data.provider_timezone;
        self.current = Some(data.current);
        self.forecast = data.forecast;
        self.error = None;
    }

    pub fn mark_retrying(&mut self, error: impl Into<String>) {
        self.state = WeatherFetchState::Retrying;
        self.error = Some(error.into());
    }

    pub fn record_failure(&mut self, error: impl Into<String>) {
        self.state = if self.current.is_some() {
            WeatherFetchState::Stale
        } else {
            WeatherFetchState::Failed
        };
        self.error = Some(error.into());
    }

    #[must_use]
    pub const fn home_badge(&self) -> &'static str {
        match self.state {
            WeatherFetchState::Ready => "WX OK",
            WeatherFetchState::Retrying => "RETRY",
            WeatherFetchState::Stale => "STALE",
            WeatherFetchState::Fetching => "FETCH",
            WeatherFetchState::WaitingForNetwork => "WAIT",
            WeatherFetchState::ConfigurationMissing => "NO CFG",
            WeatherFetchState::Disabled => "OFF",
            WeatherFetchState::Failed => "FAILED",
        }
    }

    #[must_use]
    pub fn current_summary(&self) -> String {
        self.current.as_ref().map_or_else(
            || "Weather unavailable".into(),
            |current| {
                format!(
                    "{}  {}",
                    current.condition_label(),
                    current.temperature_label()
                )
            },
        )
    }

    #[must_use]
    pub fn last_success_label(&self) -> &str {
        self.last_success.as_deref().unwrap_or("not fetched")
    }

    #[must_use]
    pub const fn config_path() -> &'static str {
        WEATHER_CONFIG_PATH
    }
}

/// Parse one Open-Meteo JSON body into a bounded cache payload.
///
/// This parser is intentionally field-specific and fixed-point. The Xtensa
/// LLVM backend used by the ESP32-S3 toolchain cannot reliably lower the
/// generic floating-point number visitor generated by a JSON framework for this
/// payload. Open-Meteo values are therefore decoded directly into integer
/// tenths without a generic JSON value tree or floating-point deserialization.
pub fn parse_open_meteo_response(body: &str) -> Result<WeatherData> {
    let current_object = object_field(body, "current")?;
    let daily_object = object_field(body, "daily")?;

    let current = CurrentConditions {
        observed_at: parse_json_string(object_field(current_object, "time")?)?,
        weather_code: parse_u16(
            object_field(current_object, "weather_code")?,
            "current.weather_code",
        )?,
        temperature_tenths_f: parse_signed_tenths(
            object_field(current_object, "temperature_2m")?,
            "current.temperature_2m",
        )?,
        apparent_temperature_tenths_f: parse_signed_tenths(
            object_field(current_object, "apparent_temperature")?,
            "current.apparent_temperature",
        )?,
        humidity_percent: parse_u8(
            object_field(current_object, "relative_humidity_2m")?,
            "current.relative_humidity_2m",
        )?
        .min(100),
        wind_speed_tenths_mph: parse_unsigned_tenths(
            object_field(current_object, "wind_speed_10m")?,
            "current.wind_speed_10m",
        )?,
    };

    let times = parse_array(object_field(daily_object, "time")?, parse_json_string)?;
    let weather_codes = parse_array(object_field(daily_object, "weather_code")?, |value| {
        parse_u16(value, "daily.weather_code")
    })?;
    let highs = parse_array(object_field(daily_object, "temperature_2m_max")?, |value| {
        parse_signed_tenths(value, "daily.temperature_2m_max")
    })?;
    let lows = parse_array(object_field(daily_object, "temperature_2m_min")?, |value| {
        parse_signed_tenths(value, "daily.temperature_2m_min")
    })?;
    let precipitation = parse_array(
        object_field(daily_object, "precipitation_probability_max")?,
        parse_optional_probability_percent,
    )?;

    let len = times.len();
    if len == 0 || len > 4 {
        bail!("Open-Meteo daily forecast must contain 1 to 4 days");
    }
    if weather_codes.len() != len
        || highs.len() != len
        || lows.len() != len
        || precipitation.len() != len
    {
        bail!("Open-Meteo daily arrays have inconsistent lengths");
    }

    let forecast = (0..len)
        .map(|index| DailyForecast {
            date: times[index].clone(),
            weather_code: weather_codes[index],
            high_tenths_f: highs[index],
            low_tenths_f: lows[index],
            precipitation_probability_percent: precipitation[index],
        })
        .collect();

    Ok(WeatherData {
        provider_timezone: parse_json_string(object_field(body, "timezone")?)?,
        current,
        forecast,
    })
}

#[must_use]
pub const fn condition_label(code: u16) -> &'static str {
    match code {
        0 => "Clear",
        1 | 2 => "Partly cloudy",
        3 => "Overcast",
        45 | 48 => "Fog",
        51 | 53 | 55 | 56 | 57 => "Drizzle",
        61 | 63 | 65 | 66 | 67 => "Rain",
        71 | 73 | 75 | 77 => "Snow",
        80 | 81 | 82 => "Showers",
        85 | 86 => "Snow showers",
        95 => "Thunderstorm",
        96 | 99 => "Thunder + hail",
        _ => "Unknown",
    }
}

fn format_tenths(value: i16, suffix: &str) -> String {
    let sign = if value < 0 { "-" } else { "" };
    let magnitude = i32::from(value).abs();
    format!("{sign}{}.{:01}{suffix}", magnitude / 10, magnitude % 10)
}

/// Advance past the JSON whitespace bytes accepted by RFC 8259.
///
/// Keep this helper small and target-independent: object and array parsing use
/// it before reading delimiters, keys and values.
fn skip_whitespace(bytes: &[u8], mut cursor: usize) -> usize {
    while let Some(byte) = bytes.get(cursor) {
        if !matches!(*byte, b' ' | b'\n' | b'\r' | b'\t') {
            break;
        }
        cursor += 1;
    }
    cursor
}

/// Return a named field from one JSON object without allocating a generic
/// provider tree. Only the current top-level object is searched.
fn object_field<'a>(object: &'a str, wanted: &str) -> Result<&'a str> {
    let bytes = object.as_bytes();
    let mut cursor = skip_whitespace(bytes, 0);
    if bytes.get(cursor) != Some(&b'{') {
        bail!("expected JSON object while reading field {wanted:?}");
    }
    cursor += 1;
    loop {
        cursor = skip_whitespace(bytes, cursor);
        match bytes.get(cursor) {
            Some(b'}') => bail!("missing Open-Meteo JSON field {wanted:?}"),
            Some(b'\"') => {}
            _ => bail!("invalid JSON object while reading field {wanted:?}"),
        }
        let key_start = cursor;
        cursor = skip_json_string(bytes, cursor)?;
        let key = parse_json_string(&object[key_start..cursor])?;
        cursor = skip_whitespace(bytes, cursor);
        if bytes.get(cursor) != Some(&b':') {
            bail!("expected ':' after JSON key {key:?}");
        }
        cursor += 1;
        cursor = skip_whitespace(bytes, cursor);
        let value_start = cursor;
        cursor = skip_json_value(bytes, cursor)?;
        if key == wanted {
            return Ok(object[value_start..cursor].trim());
        }
        cursor = skip_whitespace(bytes, cursor);
        match bytes.get(cursor) {
            Some(b',') => cursor += 1,
            Some(b'}') => bail!("missing Open-Meteo JSON field {wanted:?}"),
            _ => bail!("expected ',' or '}}' after JSON field {key:?}"),
        }
    }
}

fn parse_array<T>(array: &str, mut parse_element: impl FnMut(&str) -> Result<T>) -> Result<Vec<T>> {
    let bytes = array.as_bytes();
    let mut cursor = skip_whitespace(bytes, 0);
    if bytes.get(cursor) != Some(&b'[') {
        bail!("expected JSON array");
    }
    cursor += 1;
    let mut values = Vec::new();
    loop {
        cursor = skip_whitespace(bytes, cursor);
        if bytes.get(cursor) == Some(&b']') {
            return Ok(values);
        }
        let start = cursor;
        cursor = skip_json_value(bytes, cursor)?;
        values.push(parse_element(array[start..cursor].trim())?);
        cursor = skip_whitespace(bytes, cursor);
        match bytes.get(cursor) {
            Some(b',') => cursor += 1,
            Some(b']') => return Ok(values),
            _ => bail!("expected ',' or ']' in JSON array"),
        }
    }
}

fn skip_json_value(bytes: &[u8], cursor: usize) -> Result<usize> {
    let cursor = skip_whitespace(bytes, cursor);
    match bytes.get(cursor) {
        Some(b'\"') => skip_json_string(bytes, cursor),
        Some(b'{') => skip_nested(bytes, cursor, b'{', b'}'),
        Some(b'[') => skip_nested(bytes, cursor, b'[', b']'),
        Some(_) => {
            let mut end = cursor;
            while let Some(byte) = bytes.get(end) {
                if matches!(*byte, b',' | b'}' | b']' | b' ' | b'\n' | b'\r' | b'\t') {
                    break;
                }
                end += 1;
            }
            if end == cursor {
                bail!("expected JSON value");
            }
            Ok(end)
        }
        None => bail!("unexpected end of JSON input"),
    }
}

fn skip_nested(bytes: &[u8], cursor: usize, open: u8, close: u8) -> Result<usize> {
    let mut index = cursor;
    let mut depth = 0_u16;
    loop {
        match bytes.get(index) {
            Some(byte) if *byte == open => {
                depth = depth.checked_add(1).context("JSON nesting is too deep")?;
                index += 1;
            }
            Some(byte) if *byte == close => {
                depth = depth
                    .checked_sub(1)
                    .context("unexpected JSON closing delimiter")?;
                index += 1;
                if depth == 0 {
                    return Ok(index);
                }
            }
            Some(b'\"') => index = skip_json_string(bytes, index)?,
            Some(_) => index += 1,
            None => bail!("unterminated JSON collection"),
        }
    }
}

fn skip_json_string(bytes: &[u8], cursor: usize) -> Result<usize> {
    if bytes.get(cursor) != Some(&b'\"') {
        bail!("expected JSON string");
    }
    let mut index = cursor + 1;
    while let Some(byte) = bytes.get(index) {
        match *byte {
            b'\\' => {
                index += 2;
                if index > bytes.len() {
                    bail!("unterminated JSON escape");
                }
            }
            b'\"' => return Ok(index + 1),
            0x00..=0x1f => bail!("control byte in JSON string"),
            _ => index += 1,
        }
    }
    bail!("unterminated JSON string")
}

fn parse_json_string(value: &str) -> Result<String> {
    let bytes = value.trim().as_bytes();
    if bytes.first() != Some(&b'\"') || bytes.last() != Some(&b'\"') || bytes.len() < 2 {
        bail!("expected JSON string value");
    }
    let mut output = String::new();
    let mut index = 1;
    while index + 1 < bytes.len() {
        match bytes[index] {
            b'\\' => {
                index += 1;
                let escaped = *bytes.get(index).context("unterminated JSON escape")?;
                match escaped {
                    b'\"' => output.push('"'),
                    b'\\' => output.push('\\'),
                    b'/' => output.push('/'),
                    b'b' => output.push('\u{0008}'),
                    b'f' => output.push('\u{000c}'),
                    b'n' => output.push('\n'),
                    b'r' => output.push('\r'),
                    b't' => output.push('\t'),
                    b'u' => {
                        let digits = bytes
                            .get(index + 1..index + 5)
                            .context("short JSON unicode escape")?;
                        let digits =
                            core::str::from_utf8(digits).context("invalid JSON unicode escape")?;
                        let scalar = u32::from_str_radix(digits, 16)
                            .context("invalid JSON unicode escape")?;
                        let character =
                            char::from_u32(scalar).context("unsupported JSON unicode scalar")?;
                        output.push(character);
                        index += 4;
                    }
                    _ => bail!("unsupported JSON escape"),
                }
                index += 1;
            }
            byte if byte < 0x80 => {
                if byte < 0x20 {
                    bail!("control byte in JSON string");
                }
                output.push(char::from(byte));
                index += 1;
            }
            _ => {
                let remainder = core::str::from_utf8(&bytes[index..bytes.len() - 1])
                    .context("invalid UTF-8 in JSON string")?;
                let character = remainder
                    .chars()
                    .next()
                    .context("missing UTF-8 character")?;
                output.push(character);
                index += character.len_utf8();
            }
        }
    }
    Ok(output)
}

fn parse_u16(value: &str, field: &str) -> Result<u16> {
    value
        .trim()
        .parse::<u16>()
        .with_context(|| format!("{field} must be an unsigned integer"))
}

fn parse_u8(value: &str, field: &str) -> Result<u8> {
    value
        .trim()
        .parse::<u8>()
        .with_context(|| format!("{field} must be an unsigned integer"))
}

fn parse_signed_tenths(value: &str, field: &str) -> Result<i16> {
    let scaled = parse_decimal_tenths(value, field)?;
    i16::try_from(scaled).with_context(|| format!("{field} is outside the supported range"))
}

fn parse_unsigned_tenths(value: &str, field: &str) -> Result<u16> {
    let scaled = parse_decimal_tenths(value, field)?;
    u16::try_from(scaled).with_context(|| format!("{field} must be non-negative and in range"))
}

fn parse_optional_probability_percent(value: &str) -> Result<Option<u8>> {
    if value.trim() == "null" {
        return Ok(None);
    }
    let tenths = parse_decimal_tenths(value, "daily.precipitation_probability_max")?;
    let rounded = if tenths >= 0 {
        (tenths + 5) / 10
    } else {
        (tenths - 5) / 10
    };
    Ok(Some(
        u8::try_from(rounded.clamp(0, 100))
            .context("probability is outside the supported range")?,
    ))
}

/// Parse a provider decimal into tenths without generating target-side float
/// deserialization. Exponent notation is deliberately rejected because the
/// bounded Open-Meteo fields are ordinary fixed decimals.
fn parse_decimal_tenths(value: &str, field: &str) -> Result<i64> {
    let value = value.trim();
    if value.is_empty() || value.contains('e') || value.contains('E') {
        bail!("{field} must be a plain decimal number");
    }
    let (negative, unsigned) = value
        .strip_prefix('-')
        .map_or((false, value), |rest| (true, rest));
    let unsigned = unsigned.strip_prefix('+').unwrap_or(unsigned);
    let (whole, fraction) = unsigned.split_once('.').unwrap_or((unsigned, ""));
    if whole.is_empty()
        || !whole.bytes().all(|byte| byte.is_ascii_digit())
        || !fraction.bytes().all(|byte| byte.is_ascii_digit())
    {
        bail!("{field} must be a plain decimal number");
    }
    let whole = whole
        .parse::<i64>()
        .with_context(|| format!("{field} is too large"))?;
    let first = fraction
        .as_bytes()
        .first()
        .map_or(0_i64, |digit| i64::from(digit - b'0'));
    let should_round = fraction
        .as_bytes()
        .get(1)
        .is_some_and(|digit| *digit >= b'5');
    let magnitude = whole
        .checked_mul(10)
        .and_then(|scaled| scaled.checked_add(first))
        .and_then(|scaled| scaled.checked_add(if should_round { 1 } else { 0 }))
        .with_context(|| format!("{field} is too large"))?;
    Ok(if negative { -magnitude } else { magnitude })
}

#[cfg(target_os = "espidf")]
pub mod espidf {
    use std::{str, time::Duration};

    use embedded_svc::{
        http::{client::Client as HttpClient, Method},
        utils::io,
    };
    use esp_idf_svc::{
        http::client::{Configuration as HttpConfiguration, EspHttpConnection},
        sys,
    };

    use crate::{
        runtime_worker::{run_named_worker, NamedWorkerError},
        weather::{
            parse_open_meteo_response, WeatherData, WeatherFetchError, MAX_WEATHER_RESPONSE_BYTES,
            WEATHER_HTTP_TIMEOUT_SECONDS,
        },
        weather_config::WeatherConfig,
    };

    /// Stack budget for one short-lived HTTPS worker. The ESP-IDF TLS path and
    /// bounded response buffer no longer consume the firmware main task's
    /// deliberately small orchestration stack.
    pub const WEATHER_FETCH_WORKER_STACK_BYTES: usize = 64 * 1024;

    /// Fetch one bounded HTTPS payload on a short-lived dedicated worker. The
    /// main-loop retry policy remains synchronous and deterministic, while TLS
    /// certificate validation, response reads and JSON parsing receive an
    /// explicit stack budget independent from the firmware main task.
    pub fn fetch_open_meteo_on_worker(
        config: &WeatherConfig,
    ) -> Result<WeatherData, WeatherFetchError> {
        let config = config.clone();
        log::info!(
            "rustmix-wave=weather-fetch-worker status=starting stack-bytes={}",
            WEATHER_FETCH_WORKER_STACK_BYTES
        );
        let result = match run_named_worker(
            "weather-fetch",
            WEATHER_FETCH_WORKER_STACK_BYTES,
            move || fetch_open_meteo(&config),
        ) {
            Ok(data) => Ok(data),
            Err(NamedWorkerError::Operation(error)) => Err(error),
            Err(error) => {
                let error = WeatherFetchError::Transport(format!("weather fetch worker {error}"));
                log::warn!(
                    "rustmix-wave=weather-fetch-worker status=boundary-failed error={error}"
                );
                Err(error)
            }
        };
        match &result {
            Ok(data) => log::info!(
                "rustmix-wave=weather-fetch-worker status=completed forecast-days={}",
                data.forecast.len()
            ),
            Err(error) => log::warn!(
                "rustmix-wave=weather-fetch-worker status=failed classification={} error={error}",
                error.category()
            ),
        }
        result
    }

    /// Fetch one bounded HTTPS weather payload. A new client is constructed per
    /// request so a failed transport cannot poison later refresh attempts.
    #[inline(never)]
    pub fn fetch_open_meteo(config: &WeatherConfig) -> Result<WeatherData, WeatherFetchError> {
        let http_config = HttpConfiguration {
            crt_bundle_attach: Some(sys::esp_crt_bundle_attach),
            timeout: Some(Duration::from_secs(WEATHER_HTTP_TIMEOUT_SECONDS)),
            ..Default::default()
        };
        let connection = EspHttpConnection::new(&http_config).map_err(|error| {
            WeatherFetchError::Transport(format!("ESP HTTP connection init failed: {error}"))
        })?;
        let mut client = HttpClient::wrap(connection);
        let url = config.forecast_url();
        let headers = [("accept", "application/json")];
        let request = client
            .request(Method::Get, &url, &headers)
            .map_err(|error| {
                WeatherFetchError::Transport(format!("ESP HTTP request setup failed: {error}"))
            })?;
        let mut response = request.submit().map_err(|error| {
            WeatherFetchError::Transport(format!("ESP HTTP request failed: {error}"))
        })?;
        let status = response.status();
        if status != 200 {
            return Err(WeatherFetchError::HttpStatus(status));
        }
        let mut body = [0_u8; MAX_WEATHER_RESPONSE_BYTES];
        let bytes_read = io::try_read_full(&mut response, &mut body).map_err(|error| {
            WeatherFetchError::Transport(format!("ESP HTTP response read failed: {}", error.0))
        })?;
        if bytes_read == body.len() {
            return Err(WeatherFetchError::InvalidResponse(format!(
                "Open-Meteo response reached the {MAX_WEATHER_RESPONSE_BYTES}-byte limit"
            )));
        }
        let text = str::from_utf8(&body[..bytes_read]).map_err(|error| {
            WeatherFetchError::InvalidResponse(format!("Open-Meteo response is not UTF-8: {error}"))
        })?;
        parse_open_meteo_response(text).map_err(|error| {
            WeatherFetchError::InvalidResponse(format!(
                "Open-Meteo response parse failed: {error:#}"
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{
        condition_label, parse_open_meteo_response, WeatherFetchError, WeatherFetchState,
        WeatherSnapshot, WEATHER_RETRY_DELAYS_SECONDS, WEATHER_RETRY_LIMIT,
    };
    use crate::weather_config::WeatherConfig;

    const SAMPLE: &str = r#"{
      "timezone":"America/New_York",
      "current":{"time":"2026-06-03T16:30","temperature_2m":78.4,"relative_humidity_2m":61,"apparent_temperature":79.7,"weather_code":2,"wind_speed_10m":8.6},
      "daily":{"time":["2026-06-03","2026-06-04","2026-06-05","2026-06-06"],"weather_code":[2,61,0,95],"temperature_2m_max":[80.1,75.0,82.4,77.3],"temperature_2m_min":[64.2,62.5,65.1,63.8],"precipitation_probability_max":[20,80,5,70]}
    }"#;

    fn config() -> WeatherConfig {
        WeatherConfig::parse("location=Jersey City, NJ\nlatitude=40.7178\nlongitude=-74.0431\n")
            .unwrap()
    }

    #[test]
    fn parses_current_conditions_and_four_day_forecast() {
        let data = parse_open_meteo_response(SAMPLE).unwrap();
        assert_eq!(data.current.temperature_label(), "78.4 F");
        assert_eq!(data.current.apparent_temperature_label(), "79.7 F");
        assert_eq!(data.current.wind_label(), "8.6 mph");
        assert_eq!(data.current.condition_label(), "Partly cloudy");
        assert_eq!(data.forecast.len(), 4);
        assert_eq!(data.forecast[1].condition_label(), "Rain");
    }

    #[test]
    fn retains_cached_payload_when_refresh_fails() {
        let mut snapshot = WeatherSnapshot::provisioned(&config());
        snapshot.record_success(parse_open_meteo_response(SAMPLE).unwrap());
        snapshot.record_failure("temporary HTTP error");
        assert_eq!(snapshot.state, WeatherFetchState::Stale);
        assert_eq!(snapshot.current_summary(), "Partly cloudy  78.4 F");
        assert_eq!(snapshot.forecast.len(), 4);
    }

    #[test]
    fn retry_policy_is_bounded_and_classifies_transient_failures() {
        assert_eq!(WEATHER_RETRY_DELAYS_SECONDS, [2, 5, 15]);
        assert_eq!(WEATHER_RETRY_LIMIT, 3);
        assert!(WeatherFetchError::Transport("TLS EOF".into()).is_retryable());
        for status in [429, 500, 502, 503, 504] {
            assert!(WeatherFetchError::HttpStatus(status).is_retryable());
        }
        for status in [400, 401, 403, 404] {
            assert!(!WeatherFetchError::HttpStatus(status).is_retryable());
        }
        assert!(!WeatherFetchError::InvalidResponse("bad JSON".into()).is_retryable());
    }

    #[test]
    fn retrying_state_keeps_last_known_good_payload() {
        let mut snapshot = WeatherSnapshot::provisioned(&config());
        snapshot.record_success(parse_open_meteo_response(SAMPLE).unwrap());
        snapshot.mark_retrying("temporary transport failure");
        assert_eq!(snapshot.state, WeatherFetchState::Retrying);
        assert_eq!(snapshot.current_summary(), "Partly cloudy  78.4 F");
        assert_eq!(snapshot.forecast.len(), 4);
    }

    #[test]
    fn classifies_common_wmo_weather_codes() {
        assert_eq!(condition_label(0), "Clear");
        assert_eq!(condition_label(61), "Rain");
        assert_eq!(condition_label(95), "Thunderstorm");
        assert_eq!(condition_label(500), "Unknown");
    }

    #[test]
    fn fixed_point_parser_rounds_negative_and_positive_decimals() {
        assert_eq!(super::parse_decimal_tenths("78.45", "test").unwrap(), 785);
        assert_eq!(super::parse_decimal_tenths("-2.26", "test").unwrap(), -23);
        assert_eq!(super::parse_decimal_tenths("8", "test").unwrap(), 80);
    }

    #[test]
    fn fixed_point_parser_rejects_exponent_notation() {
        assert!(super::parse_decimal_tenths("1e3", "test").is_err());
    }

    #[test]
    fn skips_all_json_whitespace_bytes() {
        assert_eq!(super::skip_whitespace(b" \n\r\t{", 0), 4);
        assert_eq!(super::skip_whitespace(b"[", 0), 0);
        assert_eq!(super::skip_whitespace(b"  value", 2), 2);
    }

    #[test]
    fn parses_nullable_daily_probability_without_generic_json_tree() {
        let sample = SAMPLE.replace("[20,80,5,70]", "[20,null,5.6,100]");
        let data = parse_open_meteo_response(&sample).unwrap();
        assert_eq!(data.forecast[1].precipitation_probability_percent, None);
        assert_eq!(data.forecast[2].precipitation_probability_percent, Some(6));
    }
}
