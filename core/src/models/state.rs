//! Rustmix-owned system state models for the current X4 runtime.
//!
//! These models are pure and hardware-free.  They document and test the file
//! formats already used by the active Pulp-derived runtime without moving SD,
//! display, input, Wi-Fi, or reader behavior out of that runtime yet.

use core::fmt::{self, Write};

use heapless::String;
use serde::{Deserialize, Serialize};

pub const X4_APP_DATA_DIR: &str = "RUSTMIX";
pub const X4_SETTINGS_FILE_NAME: &str = "SETTINGS.TXT";
pub const X4_SETTINGS_COMPAT_PATH: &str = "RUSTMIX/SETTINGS.TXT";
pub const X4_SLEEP_IMAGE_MODE_FILE: &str = "SLPMODE.TXT";
pub const X4_TIME_STATE_FILE: &str = "TIME.TXT";
pub const X4_FCACHE_ROOT: &str = "/FCACHE";
pub const X4_DEFAULT_FCACHE_TARGET: &str = "/FCACHE/15D1296A";

pub const DEFAULT_SLEEP_TIMEOUT_MINUTES: u16 = 10;
pub const MAX_SLEEP_TIMEOUT_MINUTES: u16 = 120;
pub const DEFAULT_GHOST_CLEAR_EVERY: u8 = 10;
pub const MIN_GHOST_CLEAR_EVERY: u8 = 5;
pub const MAX_GHOST_CLEAR_EVERY: u8 = 100;
pub const DEFAULT_FONT_SIZE_IDX: u8 = 2;
pub const MAX_FONT_SIZE_IDX: u8 = 4;
pub const DEFAULT_READING_THEME_IDX: u8 = 1;
pub const READING_THEME_COUNT: u8 = 4;
pub const DEFAULT_PREPARED_FONT_PROFILE: u8 = 1;
pub const PREPARED_FONT_PROFILE_COUNT: u8 = 3;
pub const DEFAULT_PREPARED_FALLBACK_POLICY: u8 = 0;
pub const PREPARED_FALLBACK_POLICY_COUNT: u8 = 3;
pub const DEFAULT_DISPLAY_REFRESH_MODE: u8 = 1;
pub const WIFI_TRANSFER_MIN_CHUNK_BYTES: u16 = 128;
pub const WIFI_TRANSFER_MAX_CHUNK_BYTES: u16 = 1536;
pub const WIFI_TRANSFER_DEFAULT_CHUNK_BYTES: u16 = 256;
pub const WIFI_TRANSFER_DEFAULT_CHUNK_DELAY_MS: u16 = 250;
pub const WIFI_TRANSFER_DEFAULT_FILE_DELAY_MS: u16 = 600;
pub const WIFI_TRANSFER_DEFAULT_MAX_RETRIES: u8 = 20;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReadingThemeModel {
    Compact,
    Default,
    Relaxed,
    Spacious,
}

impl ReadingThemeModel {
    pub const fn from_index(idx: u8) -> Self {
        match idx {
            0 => Self::Compact,
            2 => Self::Relaxed,
            3 => Self::Spacious,
            _ => Self::Default,
        }
    }

    pub const fn index(self) -> u8 {
        match self {
            Self::Compact => 0,
            Self::Default => 1,
            Self::Relaxed => 2,
            Self::Spacious => 3,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Compact => "Compact",
            Self::Default => "Default",
            Self::Relaxed => "Relaxed",
            Self::Spacious => "Spacious",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PreparedFontProfileModel {
    Compact,
    Balanced,
    Large,
}

impl PreparedFontProfileModel {
    pub const fn from_index(idx: u8) -> Self {
        match idx {
            0 => Self::Compact,
            2 => Self::Large,
            _ => Self::Balanced,
        }
    }

    pub const fn index(self) -> u8 {
        match self {
            Self::Compact => 0,
            Self::Balanced => 1,
            Self::Large => 2,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Compact => "Compact",
            Self::Balanced => "Balanced",
            Self::Large => "Large",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PreparedFallbackPolicyModel {
    Visible,
    Latin,
    Reject,
}

impl PreparedFallbackPolicyModel {
    pub const fn from_index(idx: u8) -> Self {
        match idx {
            1 => Self::Latin,
            2 => Self::Reject,
            _ => Self::Visible,
        }
    }

    pub const fn index(self) -> u8 {
        match self {
            Self::Visible => 0,
            Self::Latin => 1,
            Self::Reject => 2,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Visible => "Visible",
            Self::Latin => "Latin",
            Self::Reject => "Reject",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReaderOrientationModel {
    #[default]
    Portrait,
    Inverted,
    LandscapeCw,
    LandscapeCcw,
}

impl ReaderOrientationModel {
    pub const MODEL_COUNT: u8 = 4;
    pub const SELECTABLE_READER_UI_COUNT: u8 = 4;

    pub const fn from_index(idx: u8) -> Self {
        match idx {
            1 => Self::Inverted,
            2 => Self::LandscapeCw,
            3 => Self::LandscapeCcw,
            _ => Self::Portrait,
        }
    }

    pub const fn index(self) -> u8 {
        match self {
            Self::Portrait => 0,
            Self::Inverted => 1,
            Self::LandscapeCw => 2,
            Self::LandscapeCcw => 3,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Portrait => "Portrait",
            Self::Inverted => "Inverted",
            Self::LandscapeCw => "Landscape CW",
            Self::LandscapeCcw => "Landscape CCW",
        }
    }

    pub const fn is_selectable_in_reader_ui(self) -> bool {
        matches!(
            self,
            Self::Portrait | Self::Inverted | Self::LandscapeCw | Self::LandscapeCcw
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReaderPreferencesModel {
    pub book_font: u8,
    pub reading_theme: u8,
    pub show_progress: bool,
    pub reader_orientation: u8,
    pub prepared_font_profile: u8,
    pub prepared_fallback_policy: u8,
}

impl Default for ReaderPreferencesModel {
    fn default() -> Self {
        Self {
            book_font: DEFAULT_FONT_SIZE_IDX,
            reading_theme: DEFAULT_READING_THEME_IDX,
            show_progress: true,
            reader_orientation: 0,
            prepared_font_profile: DEFAULT_PREPARED_FONT_PROFILE,
            prepared_fallback_policy: DEFAULT_PREPARED_FALLBACK_POLICY,
        }
    }
}

impl ReaderPreferencesModel {
    pub fn sanitize(&mut self) {
        self.book_font = self.book_font.min(MAX_FONT_SIZE_IDX);
        self.reading_theme = self.reading_theme.min(READING_THEME_COUNT - 1);
        self.reader_orientation = self
            .reader_orientation
            .min(ReaderOrientationModel::MODEL_COUNT - 1);
        self.prepared_font_profile = self
            .prepared_font_profile
            .min(PREPARED_FONT_PROFILE_COUNT - 1);
        self.prepared_fallback_policy = self
            .prepared_fallback_policy
            .min(PREPARED_FALLBACK_POLICY_COUNT - 1);
    }

    pub const fn theme(self) -> ReadingThemeModel {
        ReadingThemeModel::from_index(self.reading_theme)
    }

    pub const fn orientation(self) -> ReaderOrientationModel {
        ReaderOrientationModel::from_index(self.reader_orientation)
    }

    pub const fn prepared_profile(self) -> PreparedFontProfileModel {
        PreparedFontProfileModel::from_index(self.prepared_font_profile)
    }

    pub const fn prepared_fallback(self) -> PreparedFallbackPolicyModel {
        PreparedFallbackPolicyModel::from_index(self.prepared_fallback_policy)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisplayPreferencesModel {
    pub refresh_mode: u8,
    pub invert_colors: bool,
    pub contrast_high: bool,
}

impl Default for DisplayPreferencesModel {
    fn default() -> Self {
        Self {
            refresh_mode: DEFAULT_DISPLAY_REFRESH_MODE,
            invert_colors: false,
            contrast_high: false,
        }
    }
}

impl DisplayPreferencesModel {
    pub fn sanitize(&mut self) {
        self.refresh_mode = self.refresh_mode.min(2);
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemSettingsModel {
    pub sleep_timeout_minutes: u16,
    pub ghost_clear_every: u8,
    pub ui_font: u8,
    pub reader: ReaderPreferencesModel,
    pub display: DisplayPreferencesModel,
    pub swap_buttons: bool,
    pub wifi_ssid: String<32>,
    pub wifi_password_configured: bool,
}

impl Default for SystemSettingsModel {
    fn default() -> Self {
        Self {
            sleep_timeout_minutes: DEFAULT_SLEEP_TIMEOUT_MINUTES,
            ghost_clear_every: DEFAULT_GHOST_CLEAR_EVERY,
            ui_font: DEFAULT_FONT_SIZE_IDX,
            reader: ReaderPreferencesModel::default(),
            display: DisplayPreferencesModel::default(),
            swap_buttons: false,
            wifi_ssid: String::new(),
            wifi_password_configured: false,
        }
    }
}

impl SystemSettingsModel {
    pub fn sanitize(&mut self) {
        self.sleep_timeout_minutes = self.sleep_timeout_minutes.min(MAX_SLEEP_TIMEOUT_MINUTES);
        self.ghost_clear_every = self
            .ghost_clear_every
            .clamp(MIN_GHOST_CLEAR_EVERY, MAX_GHOST_CLEAR_EVERY);
        self.ui_font = self.ui_font.min(MAX_FONT_SIZE_IDX);
        self.reader.sanitize();
        self.display.sanitize();
    }

    pub fn reader_preferences(&self) -> ReaderPreferencesModel {
        self.reader
    }

    pub fn set_reader_preferences(&mut self, prefs: ReaderPreferencesModel) {
        self.reader = prefs;
        self.reader.sanitize();
    }

    pub fn set_wifi_ssid(&mut self, ssid: &str) {
        self.wifi_ssid.clear();
        push_str(&mut self.wifi_ssid, ssid.trim());
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SleepImageModeModel {
    #[default]
    Static,
    Text,
    NoRedraw,
}

impl SleepImageModeModel {
    pub fn parse(data: &[u8]) -> Self {
        let Ok(text) = core::str::from_utf8(data) else {
            return Self::Static;
        };
        let trimmed = trim_ascii(text);
        if trimmed.eq_ignore_ascii_case("text") {
            Self::Text
        } else if trimmed.eq_ignore_ascii_case("off")
            || trimmed.eq_ignore_ascii_case("no-redraw")
            || trimmed.eq_ignore_ascii_case("no_redraw")
        {
            Self::NoRedraw
        } else {
            // Backward-compatible fallback: old daily/fast-daily/cached values
            // now render the neutral root /sleep.bmp static bitmap path.
            Self::Static
        }
    }

    pub const fn value(self) -> &'static str {
        match self {
            Self::Static => "static",
            Self::Text => "text",
            Self::NoRedraw => "no-redraw",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Static => "Static",
            Self::Text => "Text",
            Self::NoRedraw => "No Redraw",
        }
    }

    pub fn write_to(self, out: &mut [u8]) -> usize {
        write_bytes(out, self.value().as_bytes())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClockFreshnessModel {
    Live,
    Cached,
    Unsynced,
}

impl ClockFreshnessModel {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Live => "Live",
            Self::Cached => "Cached",
            Self::Unsynced => "Unsynced",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkTimeStateModel {
    pub timezone: String<32>,
    pub last_sync_unix: Option<u64>,
    pub last_sync_uptime_ms: Option<u64>,
    pub last_sync_ok: bool,
    pub last_sync_source: String<16>,
    pub last_sync_error: String<64>,
    pub last_sync_ip: String<16>,
    pub display_offset_minutes: i16,
}

impl Default for NetworkTimeStateModel {
    fn default() -> Self {
        let mut timezone = String::new();
        push_str(&mut timezone, "America/New_York");
        Self {
            timezone,
            last_sync_unix: None,
            last_sync_uptime_ms: None,
            last_sync_ok: false,
            last_sync_source: String::new(),
            last_sync_error: String::new(),
            last_sync_ip: String::new(),
            display_offset_minutes: -300,
        }
    }
}

impl NetworkTimeStateModel {
    pub fn synced(unix: u64, uptime_ms: u64, source: &str, ip: Option<&str>) -> Self {
        let mut state = Self {
            last_sync_unix: Some(unix),
            last_sync_uptime_ms: Some(uptime_ms),
            last_sync_ok: true,
            ..Self::default()
        };
        push_str(&mut state.last_sync_source, source);
        if let Some(ip) = ip {
            push_str(&mut state.last_sync_ip, ip);
        }
        state
    }

    pub fn with_retry_failure(mut self, error: &str, uptime_ms: u64) -> Self {
        self.last_sync_ok = false;
        self.last_sync_uptime_ms = Some(uptime_ms);
        self.last_sync_error.clear();
        push_str(&mut self.last_sync_error, error.trim());
        self
    }

    pub fn live_unix(&self, uptime_ms: u64) -> Option<u64> {
        if !self.last_sync_ok {
            return None;
        }
        let base = self.last_sync_unix?;
        let synced_at = self.last_sync_uptime_ms?;
        if uptime_ms < synced_at {
            return None;
        }
        Some(base.saturating_add((uptime_ms - synced_at) / 1000))
    }

    pub fn display_unix(&self, uptime_ms: u64) -> Option<u64> {
        self.live_unix(uptime_ms).or(self.last_sync_unix)
    }

    pub fn freshness(&self, uptime_ms: u64) -> ClockFreshnessModel {
        if self.live_unix(uptime_ms).is_some() {
            ClockFreshnessModel::Live
        } else if self.last_sync_unix.is_some() {
            ClockFreshnessModel::Cached
        } else {
            ClockFreshnessModel::Unsynced
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WifiTransferConfigModel {
    pub target_folder: String<64>,
    pub chunk_size_bytes: u16,
    pub delay_between_chunks_ms: u16,
    pub delay_between_files_ms: u16,
    pub max_retries: u8,
}

impl Default for WifiTransferConfigModel {
    fn default() -> Self {
        let mut target_folder = String::new();
        push_str(&mut target_folder, X4_DEFAULT_FCACHE_TARGET);
        Self {
            target_folder,
            chunk_size_bytes: WIFI_TRANSFER_DEFAULT_CHUNK_BYTES,
            delay_between_chunks_ms: WIFI_TRANSFER_DEFAULT_CHUNK_DELAY_MS,
            delay_between_files_ms: WIFI_TRANSFER_DEFAULT_FILE_DELAY_MS,
            max_retries: WIFI_TRANSFER_DEFAULT_MAX_RETRIES,
        }
    }
}

impl WifiTransferConfigModel {
    pub fn sanitize(&mut self) {
        self.chunk_size_bytes = self
            .chunk_size_bytes
            .clamp(WIFI_TRANSFER_MIN_CHUNK_BYTES, WIFI_TRANSFER_MAX_CHUNK_BYTES);
        self.delay_between_chunks_ms = self.delay_between_chunks_ms.min(2000);
        self.delay_between_files_ms = self.delay_between_files_ms.min(3000);
        self.max_retries = self.max_retries.clamp(1, 20);
        if !self.target_folder.starts_with('/') {
            let mut fixed = String::new();
            let _ = fixed.push('/');
            push_str(&mut fixed, self.target_folder.as_str());
            self.target_folder = fixed;
        }
    }

    pub const fn stores_password(&self) -> bool {
        false
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WifiTransferFailureModel {
    Idle,
    NetworkError,
    HttpError(u16),
    PartialChunk,
    StoppedByUser,
    InvalidTargetPath,
}

impl WifiTransferFailureModel {
    pub const fn retryable(self) -> bool {
        matches!(self, Self::NetworkError | Self::HttpError(500..=599))
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Idle => "Idle",
            Self::NetworkError => "Network error",
            Self::HttpError(_) => "HTTP error",
            Self::PartialChunk => "Partial chunk; resume required",
            Self::StoppedByUser => "Stopped by user",
            Self::InvalidTargetPath => "Invalid target path",
        }
    }
}

pub fn parse_settings_txt(data: &[u8]) -> SystemSettingsModel {
    let mut settings = SystemSettingsModel::default();
    let Ok(text) = core::str::from_utf8(data) else {
        return settings;
    };

    let mut saw_book_font = false;
    let mut saw_reading_theme = false;
    let mut legacy_reader_font_pref = None;
    let mut legacy_reader_theme_pref = None;

    for line in text.lines() {
        let line = trim_ascii(line);
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((raw_key, raw_value)) = line.split_once('=') else {
            continue;
        };
        let key = trim_ascii(raw_key);
        let value = trim_ascii(raw_value);
        match key {
            "sleep_timeout" => {
                if let Some(v) = parse_u16(value) {
                    settings.sleep_timeout_minutes = v;
                }
            }
            "ghost_clear" => {
                if let Some(v) = parse_u16(value) {
                    settings.ghost_clear_every = v as u8;
                }
            }
            "book_font" => {
                saw_book_font = true;
                if let Some(v) = parse_u16(value) {
                    settings.reader.book_font = v as u8;
                }
            }
            "ui_font" => {
                if let Some(v) = parse_u16(value) {
                    settings.ui_font = v as u8;
                }
            }
            "reading_theme" => {
                saw_reading_theme = true;
                if let Some(v) = parse_u16(value) {
                    settings.reader.reading_theme = v as u8;
                }
            }
            "show_progress" => {
                if let Some(v) = parse_bool(value) {
                    settings.reader.show_progress = v;
                }
            }
            "reader_orientation" | "reading_orientation" => {
                if let Some(v) = parse_u16(value) {
                    settings.reader.reader_orientation = v as u8;
                }
            }
            "prepared_font_profile" => {
                if let Some(v) = parse_u16(value) {
                    settings.reader.prepared_font_profile = v as u8;
                }
            }
            "prepared_fallback_policy" => {
                if let Some(v) = parse_u16(value) {
                    settings.reader.prepared_fallback_policy = v as u8;
                }
            }
            "display_refresh_mode" => {
                if let Some(v) = parse_u16(value) {
                    settings.display.refresh_mode = v as u8;
                }
            }
            "display_invert_colors" => {
                if let Some(v) = parse_bool(value) {
                    settings.display.invert_colors = v;
                }
            }
            "display_contrast_high" => {
                if let Some(v) = parse_bool(value) {
                    settings.display.contrast_high = v;
                }
            }
            "swap_buttons" => {
                if let Some(v) = parse_bool(value) {
                    settings.swap_buttons = v;
                }
            }
            "wifi_ssid" => settings.set_wifi_ssid(value),
            "wifi_pass" => settings.wifi_password_configured = !value.is_empty(),
            "reader_font_pref" => legacy_reader_font_pref = parse_u16(value),
            "reader_line_spacing" | "reader_margins" => legacy_reader_theme_pref = parse_u16(value),
            _ => {}
        }
    }

    if !saw_book_font && let Some(v) = legacy_reader_font_pref {
        settings.reader.book_font = legacy_font_to_book_font(v as u8);
    }
    if !saw_reading_theme && let Some(v) = legacy_reader_theme_pref {
        settings.reader.reading_theme = v as u8;
    }

    settings.sanitize();
    settings
}

pub fn write_reader_preferences_txt(prefs: &ReaderPreferencesModel, out: &mut [u8]) -> usize {
    let mut prefs = *prefs;
    prefs.sanitize();
    let mut writer = SliceWriter { out, pos: 0 };
    let _ = writeln!(writer, "book_font={}", prefs.book_font);
    let _ = writeln!(writer, "reading_theme={}", prefs.reading_theme);
    let _ = writeln!(
        writer,
        "show_progress={}",
        if prefs.show_progress { 1 } else { 0 }
    );
    let _ = writeln!(writer, "reader_orientation={}", prefs.reader_orientation);
    let _ = writeln!(
        writer,
        "prepared_font_profile={}",
        prefs.prepared_font_profile
    );
    let _ = writeln!(
        writer,
        "prepared_fallback_policy={}",
        prefs.prepared_fallback_policy
    );
    writer.pos
}

pub fn parse_time_txt(data: &[u8]) -> NetworkTimeStateModel {
    let mut state = NetworkTimeStateModel::default();
    let Ok(text) = core::str::from_utf8(data) else {
        return state;
    };
    for line in text.lines() {
        let Some((raw_key, raw_value)) = line.split_once('=') else {
            continue;
        };
        let key = trim_ascii(raw_key);
        let value = trim_ascii(raw_value);
        match key {
            "timezone" if !value.is_empty() => {
                state.timezone.clear();
                push_str(&mut state.timezone, value);
            }
            "last_sync_unix" => state.last_sync_unix = parse_u64(value),
            "last_sync_monotonic_ms" => state.last_sync_uptime_ms = parse_u64(value),
            "last_sync_ok" => state.last_sync_ok = value == "1",
            "last_sync_source" => {
                state.last_sync_source.clear();
                push_str(&mut state.last_sync_source, value);
            }
            "last_sync_error" => {
                state.last_sync_error.clear();
                push_str(&mut state.last_sync_error, value);
            }
            "last_sync_ip" => {
                state.last_sync_ip.clear();
                push_str(&mut state.last_sync_ip, value);
            }
            "display_offset_minutes" => {
                if let Some(v) = parse_i16(value) {
                    state.display_offset_minutes = v;
                }
            }
            _ => {}
        }
    }
    if state.last_sync_unix.is_none() {
        state.last_sync_ok = false;
    }
    state
}

pub fn write_time_txt(state: &NetworkTimeStateModel, out: &mut [u8]) -> usize {
    let mut writer = SliceWriter { out, pos: 0 };
    let _ = writeln!(writer, "timezone={}", state.timezone.as_str());
    write_optional_u64(&mut writer, "last_sync_unix", state.last_sync_unix);
    write_optional_u64(
        &mut writer,
        "last_sync_monotonic_ms",
        state.last_sync_uptime_ms,
    );
    let _ = writeln!(
        writer,
        "last_sync_ok={}",
        if state.last_sync_ok { 1 } else { 0 }
    );
    let _ = writeln!(
        writer,
        "last_sync_source={}",
        state.last_sync_source.as_str()
    );
    let _ = writeln!(writer, "last_sync_error={}", state.last_sync_error.as_str());
    let _ = writeln!(writer, "last_sync_ip={}", state.last_sync_ip.as_str());
    let _ = writeln!(
        writer,
        "display_offset_minutes={}",
        state.display_offset_minutes
    );
    writer.pos
}

fn write_optional_u64(writer: &mut SliceWriter<'_>, key: &str, value: Option<u64>) {
    if let Some(value) = value {
        let _ = writeln!(writer, "{}={}", key, value);
    } else {
        let _ = writeln!(writer, "{}=", key);
    }
}

fn legacy_font_to_book_font(idx: u8) -> u8 {
    match idx {
        0 => 1,
        1 => 2,
        _ => 3,
    }
}

fn parse_bool(value: &str) -> Option<bool> {
    if value.eq_ignore_ascii_case("1")
        || value.eq_ignore_ascii_case("true")
        || value.eq_ignore_ascii_case("on")
    {
        Some(true)
    } else if value.eq_ignore_ascii_case("0")
        || value.eq_ignore_ascii_case("false")
        || value.eq_ignore_ascii_case("off")
    {
        Some(false)
    } else {
        None
    }
}

fn parse_u16(value: &str) -> Option<u16> {
    value.parse().ok()
}

fn parse_u64(value: &str) -> Option<u64> {
    if value.is_empty() {
        None
    } else {
        value.parse().ok()
    }
}

fn parse_i16(value: &str) -> Option<i16> {
    value.parse().ok()
}

fn trim_ascii(value: &str) -> &str {
    value.trim_matches(|c| matches!(c, ' ' | '\t' | '\r' | '\n'))
}

fn write_bytes(out: &mut [u8], bytes: &[u8]) -> usize {
    let n = out.len().min(bytes.len());
    out[..n].copy_from_slice(&bytes[..n]);
    n
}

fn push_str<const N: usize>(out: &mut String<N>, value: &str) {
    let _ = out.push_str(value);
}

struct SliceWriter<'a> {
    out: &'a mut [u8],
    pos: usize,
}

impl Write for SliceWriter<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let bytes = s.as_bytes();
        let n = (self.out.len().saturating_sub(self.pos)).min(bytes.len());
        if n > 0 {
            self.out[self.pos..self.pos + n].copy_from_slice(&bytes[..n]);
            self.pos += n;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_parser_accepts_current_x4_keys() {
        let text = b"\
book_font=4\n\
ui_font=3\n\
reading_theme=2\n\
show_progress=0\n\
prepared_font_profile=2\n\
prepared_fallback_policy=1\n\
display_refresh_mode=2\n\
display_invert_colors=1\n\
display_contrast_high=true\n\
swap_buttons=on\n\
wifi_ssid=home-net\n\
wifi_pass=secret\n";
        let settings = parse_settings_txt(text);
        assert_eq!(settings.reader.book_font, 4);
        assert_eq!(settings.ui_font, 3);
        assert_eq!(settings.reader.theme(), ReadingThemeModel::Relaxed);
        assert!(!settings.reader.show_progress);
        assert_eq!(
            settings.reader.prepared_profile(),
            PreparedFontProfileModel::Large
        );
        assert_eq!(
            settings.reader.prepared_fallback(),
            PreparedFallbackPolicyModel::Latin
        );
        assert_eq!(settings.display.refresh_mode, 2);
        assert!(settings.display.invert_colors);
        assert!(settings.display.contrast_high);
        assert!(settings.swap_buttons);
        assert_eq!(settings.wifi_ssid.as_str(), "home-net");
        assert!(settings.wifi_password_configured);
    }

    #[test]
    fn legacy_reader_keys_map_to_current_reader_preferences() {
        let settings = parse_settings_txt(b"reader_font_pref=0\nreader_line_spacing=3\n");
        assert_eq!(settings.reader.book_font, 1);
        assert_eq!(settings.reader.reading_theme, 3);
    }

    #[test]
    fn settings_parser_sanitizes_out_of_range_values() {
        let settings = parse_settings_txt(
            b"sleep_timeout=999\nghost_clear=1\nbook_font=9\nprepared_font_profile=9\n",
        );
        assert_eq!(settings.sleep_timeout_minutes, MAX_SLEEP_TIMEOUT_MINUTES);
        assert_eq!(settings.ghost_clear_every, MIN_GHOST_CLEAR_EVERY);
        assert_eq!(settings.reader.book_font, MAX_FONT_SIZE_IDX);
        assert_eq!(
            settings.reader.prepared_font_profile,
            PREPARED_FONT_PROFILE_COUNT - 1
        );
    }

    #[test]
    fn reader_preferences_writer_uses_current_key_names() {
        let prefs = ReaderPreferencesModel {
            book_font: 3,
            reading_theme: 2,
            show_progress: false,
            prepared_font_profile: 1,
            prepared_fallback_policy: 2,
            reader_orientation: 1,
        };
        let mut buf = [0u8; 160];
        let n = write_reader_preferences_txt(&prefs, &mut buf);
        let text = core::str::from_utf8(&buf[..n]).unwrap();
        assert!(text.contains("book_font=3"));
        assert!(text.contains("reading_theme=2"));
        assert!(text.contains("show_progress=0"));
        assert!(text.contains("prepared_font_profile=1"));
        assert!(text.contains("prepared_fallback_policy=2"));
        assert!(text.contains("reader_orientation=1"));
    }

    #[test]
    fn sleep_mode_parser_matches_current_runtime_values() {
        assert_eq!(
            SleepImageModeModel::parse(b"static"),
            SleepImageModeModel::Static
        );
        assert_eq!(
            SleepImageModeModel::parse(b"daily\n"),
            SleepImageModeModel::Static
        );
        assert_eq!(
            SleepImageModeModel::parse(b"fast-daily"),
            SleepImageModeModel::Static
        );
        assert_eq!(
            SleepImageModeModel::parse(b"cached"),
            SleepImageModeModel::Static
        );
        assert_eq!(
            SleepImageModeModel::parse(b"text"),
            SleepImageModeModel::Text
        );
        assert_eq!(
            SleepImageModeModel::parse(b"off"),
            SleepImageModeModel::NoRedraw
        );
        assert_eq!(
            SleepImageModeModel::parse(b"no-redraw"),
            SleepImageModeModel::NoRedraw
        );
        let mut buf = [0u8; 16];
        let n = SleepImageModeModel::Static.write_to(&mut buf);
        assert_eq!(&buf[..n], b"static");
    }

    #[test]
    fn network_time_status_computes_live_cached_unsynced() {
        let unsynced = NetworkTimeStateModel::default();
        assert_eq!(unsynced.freshness(10_000), ClockFreshnessModel::Unsynced);

        let live = NetworkTimeStateModel::synced(1_700_000_000, 20_000, "ntp", Some("1.2.3.4"));
        assert_eq!(live.freshness(25_000), ClockFreshnessModel::Live);
        assert_eq!(live.live_unix(25_000), Some(1_700_000_005));

        let cached = live.with_retry_failure("timeout", 30_000);
        assert_eq!(cached.freshness(35_000), ClockFreshnessModel::Cached);
        assert_eq!(cached.display_unix(35_000), Some(1_700_000_000));
        assert_eq!(cached.last_sync_error.as_str(), "timeout");
    }

    #[test]
    fn network_time_parser_matches_current_time_file_keys() {
        let state = parse_time_txt(
            b"timezone=America/New_York\nlast_sync_unix=1700000000\nlast_sync_monotonic_ms=20000\nlast_sync_ok=1\nlast_sync_source=ntp\nlast_sync_ip=1.2.3.4\ndisplay_offset_minutes=-300\n",
        );
        assert_eq!(state.freshness(21_000), ClockFreshnessModel::Live);
        assert_eq!(state.live_unix(21_000), Some(1_700_000_001));
        assert_eq!(state.last_sync_ip.as_str(), "1.2.3.4");
    }

    #[test]
    fn wifi_transfer_model_preserves_large_fcache_defaults_without_password() {
        let mut cfg = WifiTransferConfigModel::default();
        assert_eq!(cfg.target_folder.as_str(), X4_DEFAULT_FCACHE_TARGET);
        assert_eq!(cfg.chunk_size_bytes, WIFI_TRANSFER_DEFAULT_CHUNK_BYTES);
        assert!(!cfg.stores_password());

        cfg.target_folder.clear();
        push_str(&mut cfg.target_folder, "FCACHE/BOOK");
        cfg.chunk_size_bytes = 9999;
        cfg.delay_between_chunks_ms = 9999;
        cfg.delay_between_files_ms = 9999;
        cfg.max_retries = 99;
        cfg.sanitize();
        assert_eq!(cfg.target_folder.as_str(), "/FCACHE/BOOK");
        assert_eq!(cfg.chunk_size_bytes, WIFI_TRANSFER_MAX_CHUNK_BYTES);
        assert_eq!(cfg.delay_between_chunks_ms, 2000);
        assert_eq!(cfg.delay_between_files_ms, 3000);
        assert_eq!(cfg.max_retries, 20);
    }

    #[test]
    fn transfer_failure_state_marks_retryable_errors() {
        assert!(WifiTransferFailureModel::NetworkError.retryable());
        assert!(WifiTransferFailureModel::HttpError(503).retryable());
        assert!(!WifiTransferFailureModel::HttpError(400).retryable());
        assert!(!WifiTransferFailureModel::PartialChunk.retryable());
    }
}
