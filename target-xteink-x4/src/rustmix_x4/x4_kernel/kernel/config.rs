// system configuration: key=value text in _x4/SETTINGS.TXT
//
// SystemSettings and WifiConfig are kernel-owned configuration;
// the SettingsApp in apps/ provides the UI for editing them

pub const SETTINGS_FILE: &str = "SETTINGS.TXT";

// default sleep timeout in minutes
pub const DEFAULT_SLEEP_TIMEOUT: u16 = 10;

// maximum sleep timeout in minutes
pub const MAX_SLEEP_TIMEOUT: u16 = 120;

// increment step for sleep timeout adjustment
pub const SLEEP_TIMEOUT_STEP: u16 = 5;

// default ghost clear interval
pub const DEFAULT_GHOST_CLEAR: u8 = 10;

// minimum ghost clear interval
pub const MIN_GHOST_CLEAR: u8 = 5;

// maximum ghost clear interval
pub const MAX_GHOST_CLEAR: u8 = 100;

// increment step for ghost clear adjustment
pub const GHOST_CLEAR_STEP: u8 = 5;

// default font size index (0=XSmall, 1=Small, 2=Medium, 3=Large, 4=XLarge)
pub const DEFAULT_FONT_SIZE_IDX: u8 = 2;

// reading themes: named presets for margins, spacing, and overall feel.
// each theme bundles margin_h, margin_v, line_spacing_pct into one
// user-friendly selection instead of exposing raw pixel values.
//
// theme index is stored as a single u8 in SETTINGS.TXT:
//   0 = Compact   – narrow margins, tight spacing, max content
//   1 = Default   – balanced for most books
//   2 = Relaxed   – wider margins, looser spacing, easier on the eyes
//   3 = Spacious  – large margins, generous spacing, paperback feel

pub const NUM_READING_THEMES: u8 = 4;
pub const DEFAULT_READING_THEME: u8 = 1;

pub const DEFAULT_READER_SHOW_PROGRESS: bool = true;

pub const READER_BIONIC_MODE_COUNT: u8 = 3;
pub const DEFAULT_READER_BIONIC_MODE: u8 = 0;
pub const READER_BIONIC_MODE_LABELS: [&str; READER_BIONIC_MODE_COUNT as usize] =
    ["Off", "Light", "Medium"];

pub const READER_GUIDE_DOTS_MODE_COUNT: u8 = 3;
pub const DEFAULT_READER_GUIDE_DOTS_MODE: u8 = 0;
pub const READER_GUIDE_DOTS_MODE_LABELS: [&str; READER_GUIDE_DOTS_MODE_COUNT as usize] =
    ["Off", "Small", "Medium"];

pub const READER_ORIENTATION_COUNT: u8 = 4;
pub const DEFAULT_READER_ORIENTATION: u8 = 0;
pub const READER_ORIENTATION_LABELS: [&str; READER_ORIENTATION_COUNT as usize] =
    ["Portrait", "Inverted", "Landscape CW", "Landscape CCW"];

pub const fn reader_orientation_label(idx: u8) -> &'static str {
    match idx {
        1 => "Inverted",
        2 => "Landscape CW",
        3 => "Landscape CCW",
        _ => "Portrait",
    }
}

pub const DEFAULT_READER_SUNLIGHT_FADING_FIX: bool = false;

pub const PREPARED_FONT_PROFILE_COUNT: u8 = 3;
pub const DEFAULT_PREPARED_FONT_PROFILE: u8 = 1;
pub const PREPARED_FONT_PROFILE_LABELS: [&str; PREPARED_FONT_PROFILE_COUNT as usize] =
    ["Compact", "Balanced", "Large"];

pub const PREPARED_FALLBACK_POLICY_COUNT: u8 = 3;
pub const READER_FONT_SOURCE_COUNT: u8 = 4;
pub const READER_SD_FONT_ID_CAP: usize = 8;
pub const UI_FONT_SOURCE_COUNT: u8 = 3; // legacy persisted field; OS chrome is fixed to Inter
pub const DEFAULT_PREPARED_FALLBACK_POLICY: u8 = 0;
pub const PREPARED_FALLBACK_POLICY_LABELS: [&str; PREPARED_FALLBACK_POLICY_COUNT as usize] =
    ["Visible", "Latin", "Reject"];
pub const DEFAULT_DISPLAY_REFRESH_MODE: u8 = 1;
pub const DEFAULT_DISPLAY_INVERT_COLORS: bool = false;
pub const DEFAULT_DISPLAY_CONTRAST_HIGH: bool = false;

#[derive(Clone, Copy)]
pub struct ReadingTheme {
    pub name: &'static str,
    pub margin_h: u16,         // horizontal margin in pixels
    pub margin_v: u16,         // vertical margin (top offset) in pixels
    pub line_spacing_pct: u16, // line spacing as percentage (100 = font native)
}

pub const READING_THEMES: [ReadingTheme; NUM_READING_THEMES as usize] = [
    ReadingTheme {
        name: "Compact",
        margin_h: 8,
        margin_v: 0,
        line_spacing_pct: 100,
    },
    ReadingTheme {
        name: "Default",
        margin_h: 16,
        margin_v: 4,
        line_spacing_pct: 120,
    },
    ReadingTheme {
        name: "Relaxed",
        margin_h: 24,
        margin_v: 8,
        line_spacing_pct: 140,
    },
    ReadingTheme {
        name: "Spacious",
        margin_h: 40,
        margin_v: 12,
        line_spacing_pct: 160,
    },
];

// look up the active reading theme by index; falls back to Default
pub fn reading_theme(idx: u8) -> &'static ReadingTheme {
    let i = (idx as usize).min(READING_THEMES.len() - 1);
    &READING_THEMES[i]
}

#[derive(Clone, Copy)]
pub struct SystemSettings {
    // power settings
    pub sleep_timeout: u16,    // minutes idle before sleep; 0 = never
    pub ghost_clear_every: u8, // partial refreshes before forced full GC

    // font settings
    pub book_font_size_idx: u8, // 0 = XSmall, 1 = Small, 2 = Medium, 3 = Large, 4 = XLarge
    pub ui_font_size_idx: u8,
    pub ui_font_source: u8, // legacy persisted field; fixed UI chrome uses Inter

    // reading settings
    pub reading_theme: u8, // index into READING_THEMES
    pub reader_show_progress: bool,
    pub reader_sunlight_fading_fix: bool,
    pub reader_bionic_mode: u8,
    pub reader_guide_dots_mode: u8,
    pub reader_orientation: u8,
    pub prepared_font_profile: u8,
    pub prepared_fallback_policy: u8,
    pub reader_font_source: u8,
    pub reader_sd_font_slot: u8,
    pub reader_sd_font_id: [u8; READER_SD_FONT_ID_CAP],
    pub reader_sd_font_id_len: u8,

    // display preference preview settings; persisted, not applied to hardware
    pub display_refresh_mode: u8, // 0 = Full, 1 = Balanced, 2 = Fast
    pub display_invert_colors: bool,
    pub display_contrast_high: bool,

    // control settings
    pub swap_buttons: bool, // swap Back/Select with Left/Right physical buttons
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReaderPreferences {
    pub book_font: u8,
    pub reading_theme: u8,
    pub show_progress: bool,
    pub sunlight_fading_fix: bool,
    pub bionic_mode: u8,
    pub guide_dots_mode: u8,
    pub reader_orientation: u8,
    pub prepared_font_profile: u8,
    pub prepared_fallback_policy: u8,
    pub reader_font_source: u8,
    pub reader_sd_font_slot: u8,
    pub reader_sd_font_id: [u8; READER_SD_FONT_ID_CAP],
    pub reader_sd_font_id_len: u8,
}

impl Default for SystemSettings {
    fn default() -> Self {
        Self::defaults()
    }
}

impl SystemSettings {
    pub const fn defaults() -> Self {
        Self {
            sleep_timeout: DEFAULT_SLEEP_TIMEOUT,
            ghost_clear_every: DEFAULT_GHOST_CLEAR,
            book_font_size_idx: DEFAULT_FONT_SIZE_IDX,
            ui_font_size_idx: DEFAULT_FONT_SIZE_IDX,
            reading_theme: DEFAULT_READING_THEME,
            reader_show_progress: DEFAULT_READER_SHOW_PROGRESS,
            reader_sunlight_fading_fix: DEFAULT_READER_SUNLIGHT_FADING_FIX,
            reader_bionic_mode: DEFAULT_READER_BIONIC_MODE,
            reader_guide_dots_mode: DEFAULT_READER_GUIDE_DOTS_MODE,
            reader_orientation: DEFAULT_READER_ORIENTATION,
            prepared_font_profile: DEFAULT_PREPARED_FONT_PROFILE,
            prepared_fallback_policy: DEFAULT_PREPARED_FALLBACK_POLICY,
            reader_font_source: 0,
            ui_font_source: 1,
            reader_sd_font_slot: 0,
            reader_sd_font_id: [0; READER_SD_FONT_ID_CAP],
            reader_sd_font_id_len: 0,
            display_refresh_mode: DEFAULT_DISPLAY_REFRESH_MODE,
            display_invert_colors: DEFAULT_DISPLAY_INVERT_COLORS,
            display_contrast_high: DEFAULT_DISPLAY_CONTRAST_HIGH,
            swap_buttons: false,
        }
    }

    pub fn sanitize(&mut self) {
        self.sanitize_with_max_font(Self::DEFAULT_MAX_FONT_IDX);
    }

    pub fn sanitize_with_max_font(&mut self, max_font: u8) {
        self.sleep_timeout = self.sleep_timeout.min(MAX_SLEEP_TIMEOUT);
        self.ghost_clear_every = self
            .ghost_clear_every
            .clamp(MIN_GHOST_CLEAR, MAX_GHOST_CLEAR);
        self.book_font_size_idx = self.book_font_size_idx.min(max_font);
        self.ui_font_size_idx = self.ui_font_size_idx.min(max_font);
        self.reading_theme = self.reading_theme.min(NUM_READING_THEMES - 1);
        self.reader_bionic_mode = self.reader_bionic_mode.min(READER_BIONIC_MODE_COUNT - 1);
        self.reader_guide_dots_mode = self
            .reader_guide_dots_mode
            .min(READER_GUIDE_DOTS_MODE_COUNT - 1);
        self.reader_orientation = self.reader_orientation.min(READER_ORIENTATION_COUNT - 1);
        self.prepared_font_profile = self
            .prepared_font_profile
            .min(PREPARED_FONT_PROFILE_COUNT - 1);
        self.prepared_fallback_policy = self
            .prepared_fallback_policy
            .min(PREPARED_FALLBACK_POLICY_COUNT - 1);
        self.reader_font_source = self.reader_font_source.min(READER_FONT_SOURCE_COUNT - 1);
        self.reader_sd_font_slot = self.reader_sd_font_slot.min(READER_FONT_SOURCE_COUNT - 2);
        self.display_refresh_mode = self.display_refresh_mode.min(2);
    }

    pub fn reader_preferences(&self) -> ReaderPreferences {
        ReaderPreferences {
            book_font: self.book_font_size_idx,
            reading_theme: self.reading_theme,
            show_progress: self.reader_show_progress,
            sunlight_fading_fix: self.reader_sunlight_fading_fix,
            bionic_mode: self.reader_bionic_mode,
            guide_dots_mode: self.reader_guide_dots_mode,
            reader_orientation: self.reader_orientation,
            prepared_font_profile: self.prepared_font_profile,
            prepared_fallback_policy: self.prepared_fallback_policy,
            reader_font_source: self.reader_font_source,
            reader_sd_font_slot: self.reader_sd_font_slot,
            reader_sd_font_id: self.reader_sd_font_id,
            reader_sd_font_id_len: self.reader_sd_font_id_len,
        }
    }

    pub fn set_reader_preferences(&mut self, prefs: ReaderPreferences) {
        self.book_font_size_idx = prefs.book_font.min(Self::DEFAULT_MAX_FONT_IDX);
        self.reading_theme = prefs.reading_theme.min(NUM_READING_THEMES - 1);
        self.reader_show_progress = prefs.show_progress;
        self.reader_sunlight_fading_fix = prefs.sunlight_fading_fix;
        self.reader_bionic_mode = prefs.bionic_mode.min(READER_BIONIC_MODE_COUNT - 1);
        self.reader_guide_dots_mode = prefs.guide_dots_mode.min(READER_GUIDE_DOTS_MODE_COUNT - 1);
        self.reader_guide_dots_mode = prefs.guide_dots_mode.min(READER_GUIDE_DOTS_MODE_COUNT - 1);
        self.reader_guide_dots_mode = prefs.guide_dots_mode.min(READER_GUIDE_DOTS_MODE_COUNT - 1);
        self.reader_orientation = prefs.reader_orientation.min(READER_ORIENTATION_COUNT - 1);
        self.prepared_font_profile = prefs
            .prepared_font_profile
            .min(PREPARED_FONT_PROFILE_COUNT - 1);
        self.prepared_fallback_policy = prefs
            .prepared_fallback_policy
            .min(PREPARED_FALLBACK_POLICY_COUNT - 1);
        self.reader_font_source = prefs.reader_font_source.min(READER_FONT_SOURCE_COUNT - 1);
        self.reader_sd_font_slot = prefs.reader_sd_font_slot.min(READER_FONT_SOURCE_COUNT - 2);
        self.reader_sd_font_id = prefs.reader_sd_font_id;
        self.reader_sd_font_id_len = prefs.reader_sd_font_id_len.min(READER_SD_FONT_ID_CAP as u8);
    }

    pub fn reader_sd_font_id(&self) -> &[u8] {
        &self.reader_sd_font_id[..self.reader_sd_font_id_len as usize]
    }

    pub fn set_reader_sd_font_id(&mut self, value: &[u8]) {
        self.reader_sd_font_id = [0; READER_SD_FONT_ID_CAP];
        let mut n = 0usize;
        for b in value.iter().copied() {
            if n >= READER_SD_FONT_ID_CAP {
                break;
            }
            let up = b.to_ascii_uppercase();
            if up.is_ascii_uppercase() || up.is_ascii_digit() || up == b'_' {
                self.reader_sd_font_id[n] = up;
                n += 1;
            }
        }
        self.reader_sd_font_id_len = n as u8;
    }

    // reasonable default - override via sanitize_with_max_font
    const DEFAULT_MAX_FONT_IDX: u8 = 4;
}

pub const WIFI_SSID_CAP: usize = 32;
pub const WIFI_PASS_CAP: usize = 63;
pub const WIFI_PROFILE_COUNT: usize = 3;
pub const WIFI_PROFILE_NAME_CAP: usize = 12;

const WIFI_PROFILE_DEFAULT_NAMES: [&str; WIFI_PROFILE_COUNT] = ["Home", "Work", "Other"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WifiProfile {
    name: [u8; WIFI_PROFILE_NAME_CAP],
    name_len: u8,
    ssid: [u8; WIFI_SSID_CAP],
    ssid_len: u8,
    pass: [u8; WIFI_PASS_CAP],
    pass_len: u8,
}

impl WifiProfile {
    pub const fn empty() -> Self {
        Self {
            name: [0u8; WIFI_PROFILE_NAME_CAP],
            name_len: 0,
            ssid: [0u8; WIFI_SSID_CAP],
            ssid_len: 0,
            pass: [0u8; WIFI_PASS_CAP],
            pass_len: 0,
        }
    }

    pub fn name(&self) -> &str {
        core::str::from_utf8(&self.name[..self.name_len as usize]).unwrap_or("")
    }

    pub fn ssid(&self) -> &str {
        core::str::from_utf8(&self.ssid[..self.ssid_len as usize]).unwrap_or("")
    }

    pub fn password(&self) -> &str {
        core::str::from_utf8(&self.pass[..self.pass_len as usize]).unwrap_or("")
    }

    pub fn has_credentials(&self) -> bool {
        self.ssid_len > 0
    }

    fn name_bytes(&self) -> &[u8] {
        &self.name[..self.name_len as usize]
    }

    fn ssid_bytes(&self) -> &[u8] {
        &self.ssid[..self.ssid_len as usize]
    }

    fn pass_bytes(&self) -> &[u8] {
        &self.pass[..self.pass_len as usize]
    }

    fn set_name(&mut self, val: &[u8]) {
        self.name = [0u8; WIFI_PROFILE_NAME_CAP];
        let n = val.len().min(WIFI_PROFILE_NAME_CAP);
        self.name[..n].copy_from_slice(&val[..n]);
        self.name_len = n as u8;
    }

    fn set_ssid(&mut self, val: &[u8]) {
        self.ssid = [0u8; WIFI_SSID_CAP];
        let n = val.len().min(WIFI_SSID_CAP);
        self.ssid[..n].copy_from_slice(&val[..n]);
        self.ssid_len = n as u8;
    }

    fn set_pass(&mut self, val: &[u8]) {
        self.pass = [0u8; WIFI_PASS_CAP];
        let n = val.len().min(WIFI_PASS_CAP);
        self.pass[..n].copy_from_slice(&val[..n]);
        self.pass_len = n as u8;
    }
}

#[derive(Clone, Copy)]
pub struct WifiConfig {
    profiles: [WifiProfile; WIFI_PROFILE_COUNT],
    default_slot: u8,
}

impl WifiConfig {
    pub const fn empty() -> Self {
        Self {
            profiles: [WifiProfile::empty(); WIFI_PROFILE_COUNT],
            default_slot: 0,
        }
    }

    pub fn default_slot(&self) -> u8 {
        self.default_slot.min((WIFI_PROFILE_COUNT - 1) as u8)
    }

    pub fn set_default_slot(&mut self, slot: u8) {
        self.default_slot = slot.min((WIFI_PROFILE_COUNT - 1) as u8);
    }

    pub fn set_default_from_value(&mut self, val: &[u8]) {
        if let Some(idx) = parse_usize(val) {
            self.set_default_slot(idx as u8);
            return;
        }

        for idx in 0..WIFI_PROFILE_COUNT {
            if ascii_eq_ignore_case(self.profile_name(idx).as_bytes(), val) {
                self.set_default_slot(idx as u8);
                return;
            }
        }
    }

    fn active_profile(&self) -> &WifiProfile {
        &self.profiles[self.default_slot() as usize]
    }

    fn active_profile_mut(&mut self) -> &mut WifiProfile {
        let idx = self.default_slot() as usize;
        &mut self.profiles[idx]
    }

    pub fn ssid(&self) -> &str {
        self.active_profile().ssid()
    }

    pub fn password(&self) -> &str {
        self.active_profile().password()
    }

    pub fn has_credentials(&self) -> bool {
        self.active_profile().has_credentials()
    }

    pub fn profile_label(&self, slot: usize) -> &'static str {
        WIFI_PROFILE_DEFAULT_NAMES
            .get(slot)
            .copied()
            .unwrap_or("Wi-Fi")
    }

    pub fn profile_name(&self, slot: usize) -> &str {
        self.profiles
            .get(slot)
            .map(|p| p.name())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| self.profile_label(slot))
    }

    pub fn profile_ssid(&self, slot: usize) -> &str {
        self.profiles.get(slot).map(|p| p.ssid()).unwrap_or("")
    }

    pub fn profile_password(&self, slot: usize) -> &str {
        self.profiles.get(slot).map(|p| p.password()).unwrap_or("")
    }

    pub fn profile_has_credentials(&self, slot: usize) -> bool {
        self.profiles
            .get(slot)
            .map(|p| p.has_credentials())
            .unwrap_or(false)
    }

    pub fn set_profile_name_from_str(&mut self, slot: usize, val: &str) {
        self.set_profile_name(slot, val.as_bytes());
    }

    pub fn set_profile_credentials_from_str(&mut self, slot: usize, ssid: &str, password: &str) {
        self.set_profile_ssid(slot, ssid.as_bytes());
        self.set_profile_pass(slot, password.as_bytes());
    }

    pub fn set_ssid_from_str(&mut self, val: &str) {
        self.set_ssid(val.as_bytes());
    }

    pub fn set_password_from_str(&mut self, val: &str) {
        self.set_pass(val.as_bytes());
    }

    pub fn set_credentials_from_str(&mut self, ssid: &str, password: &str) {
        self.set_ssid_from_str(ssid);
        self.set_password_from_str(password);
    }

    fn set_profile_name(&mut self, slot: usize, val: &[u8]) {
        if let Some(profile) = self.profiles.get_mut(slot) {
            profile.set_name(val);
        }
    }

    fn set_profile_ssid(&mut self, slot: usize, val: &[u8]) {
        if let Some(profile) = self.profiles.get_mut(slot) {
            profile.set_ssid(val);
        }
    }

    fn set_profile_pass(&mut self, slot: usize, val: &[u8]) {
        if let Some(profile) = self.profiles.get_mut(slot) {
            profile.set_pass(val);
        }
    }

    fn set_ssid(&mut self, val: &[u8]) {
        self.active_profile_mut().set_ssid(val);
    }

    fn set_pass(&mut self, val: &[u8]) {
        self.active_profile_mut().set_pass(val);
    }

    fn profile_name_bytes_or_default(&self, slot: usize) -> &[u8] {
        let Some(profile) = self.profiles.get(slot) else {
            return b"Wi-Fi";
        };
        if profile.name_len == 0 {
            self.profile_label(slot).as_bytes()
        } else {
            profile.name_bytes()
        }
    }

    fn profile_ssid_bytes(&self, slot: usize) -> &[u8] {
        self.profiles
            .get(slot)
            .map(|p| p.ssid_bytes())
            .unwrap_or(b"")
    }

    fn profile_pass_bytes(&self, slot: usize) -> &[u8] {
        self.profiles
            .get(slot)
            .map(|p| p.pass_bytes())
            .unwrap_or(b"")
    }
}

fn ascii_eq_ignore_case(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right.iter())
        .all(|(&a, &b)| a.to_ascii_lowercase() == b.to_ascii_lowercase())
}

fn parse_usize(s: &[u8]) -> Option<usize> {
    if s.is_empty() {
        return None;
    }
    let mut val: usize = 0;
    for &b in s {
        if !b.is_ascii_digit() {
            return None;
        }
        val = val.checked_mul(10)?.checked_add((b - b'0') as usize)?;
    }
    Some(val)
}
fn trim(s: &[u8]) -> &[u8] {
    let mut start = 0;
    let mut end = s.len();
    while start < end && matches!(s[start], b' ' | b'\t' | b'\r') {
        start += 1;
    }
    while end > start && matches!(s[end - 1], b' ' | b'\t' | b'\r') {
        end -= 1;
    }
    &s[start..end]
}

fn parse_u16(s: &[u8]) -> Option<u16> {
    if s.is_empty() {
        return None;
    }
    let mut val: u16 = 0;
    for &b in s {
        if !b.is_ascii_digit() {
            return None;
        }
        val = val.checked_mul(10)?.checked_add((b - b'0') as u16)?;
    }
    Some(val)
}

fn parse_bool(s: &[u8]) -> Option<bool> {
    match s {
        b"1" | b"true" | b"on" => Some(true),
        b"0" | b"false" | b"off" => Some(false),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WifiProfileField {
    Name,
    Ssid,
    Pass,
}

fn parse_wifi_profile_key(key: &[u8]) -> Option<(usize, WifiProfileField)> {
    const PREFIX: &[u8] = b"wifi_profile_";
    if !key.starts_with(PREFIX) {
        return None;
    }

    let rest = &key[PREFIX.len()..];
    let sep = rest.iter().position(|&b| b == b'_')?;
    let slot = parse_usize(&rest[..sep])?;
    if slot >= WIFI_PROFILE_COUNT {
        return None;
    }

    let field = match &rest[sep + 1..] {
        b"name" | b"label" => WifiProfileField::Name,
        b"ssid" => WifiProfileField::Ssid,
        b"pass" | b"password" => WifiProfileField::Pass,
        _ => return None,
    };

    Some((slot, field))
}

fn apply_setting(key: &[u8], val: &[u8], s: &mut SystemSettings, w: &mut WifiConfig) {
    if let Some((slot, field)) = parse_wifi_profile_key(key) {
        match field {
            WifiProfileField::Name => w.set_profile_name(slot, val),
            WifiProfileField::Ssid => w.set_profile_ssid(slot, val),
            WifiProfileField::Pass => w.set_profile_pass(slot, val),
        }
        return;
    }

    match key {
        b"sleep_timeout" => {
            if let Some(v) = parse_u16(val) {
                s.sleep_timeout = v;
            }
        }
        b"ghost_clear" => {
            if let Some(v) = parse_u16(val) {
                s.ghost_clear_every = v as u8;
            }
        }
        b"book_font" => {
            if let Some(v) = parse_u16(val) {
                s.book_font_size_idx = v as u8;
            }
        }
        b"ui_font" => {
            if let Some(v) = parse_u16(val) {
                s.ui_font_size_idx = v as u8;
            }
        }
        b"ui_font_source" => {
            if let Some(v) = parse_u16(val) {
                s.ui_font_source = (v as u8).min(UI_FONT_SOURCE_COUNT - 1);
            }
        }
        b"reading_theme" => {
            if let Some(v) = parse_u16(val) {
                s.reading_theme = v as u8;
            }
        }
        b"reader_show_progress" => {
            if let Some(v) = parse_bool(val) {
                s.reader_show_progress = v;
            }
        }
        b"bionic_reading" | b"reader_bionic_mode" => {
            if let Some(v) = parse_u16(val) {
                s.reader_bionic_mode = v as u8;
            }
        }
        b"guide_dots" | b"reader_guide_dots_mode" => {
            if let Some(v) = parse_u16(val) {
                s.reader_guide_dots_mode = v as u8;
            }
        }
        b"sunlight_fading_fix" | b"reader_sunlight_fading_fix" => {
            if let Some(v) = parse_bool(val) {
                s.reader_sunlight_fading_fix = v;
            }
        }
        b"reader_orientation" | b"reading_orientation" => {
            if let Some(v) = parse_u16(val) {
                s.reader_orientation = v as u8;
            }
        }
        b"prepared_font_profile" => {
            if let Some(v) = parse_u16(val) {
                s.prepared_font_profile = v as u8;
            }
        }
        b"prepared_fallback_policy" => {
            if let Some(v) = parse_u16(val) {
                s.prepared_fallback_policy = v as u8;
            }
        }
        b"reader_font_source" => {
            if let Some(v) = parse_u16(val) {
                s.reader_font_source = (v as u8).min(READER_FONT_SOURCE_COUNT - 1);
            }
        }
        b"reader_sd_font_slot" => {
            if let Some(v) = parse_u16(val) {
                s.reader_sd_font_slot = (v as u8).min(READER_FONT_SOURCE_COUNT - 2);
            }
        }
        b"reader_sd_font_id" => s.set_reader_sd_font_id(val),
        b"show_progress" => {
            if let Some(v) = parse_bool(val) {
                s.reader_show_progress = v;
            }
        }
        b"display_refresh_mode" => {
            if let Some(v) = parse_u16(val) {
                s.display_refresh_mode = v as u8;
            }
        }
        b"display_invert_colors" => {
            if let Some(v) = parse_bool(val) {
                s.display_invert_colors = v;
            }
        }
        b"display_contrast_high" => {
            if let Some(v) = parse_bool(val) {
                s.display_contrast_high = v;
            }
        }
        b"swap_buttons" => {
            s.swap_buttons = val == b"1" || val == b"true";
        }
        b"wifi_ssid" => {
            w.set_default_slot(0);
            w.set_profile_name_from_str(0, "Home");
            w.set_profile_ssid(0, val);
        }
        b"wifi_pass" => {
            w.set_default_slot(0);
            w.set_profile_name_from_str(0, "Home");
            w.set_profile_pass(0, val);
        }
        _ => {}
    }
}

fn book_font_from_legacy_reader_font_pref(idx: u8) -> u8 {
    match idx {
        0 => 1,
        1 => 2,
        _ => 3,
    }
}

pub fn parse_settings_txt(data: &[u8], settings: &mut SystemSettings, wifi: &mut WifiConfig) {
    let mut saw_book_font = false;
    let mut pending_wifi_default: Option<&[u8]> = None;
    let mut saw_reading_theme = false;
    let mut legacy_reader_font_pref = None;
    let mut legacy_reader_theme_pref = None;

    for line in data.split(|&b| b == b'\n') {
        let line = trim(line);
        if line.is_empty() || line[0] == b'#' {
            continue;
        }
        if let Some(eq) = line.iter().position(|&b| b == b'=') {
            let key = trim(&line[..eq]);
            let val = trim(&line[eq + 1..]);
            match key {
                b"book_font" => saw_book_font = true,
                b"reading_theme" => saw_reading_theme = true,
                b"reader_font_pref" => legacy_reader_font_pref = parse_u16(val),
                b"wifi_default" => pending_wifi_default = Some(val),
                b"reader_line_spacing" | b"reader_margins" => {
                    legacy_reader_theme_pref = parse_u16(val)
                }
                _ => {}
            }
            apply_setting(key, val, settings, wifi);
        }
    }

    if !saw_book_font && let Some(v) = legacy_reader_font_pref {
        settings.book_font_size_idx = book_font_from_legacy_reader_font_pref(v as u8);
    }
    if !saw_reading_theme && let Some(v) = legacy_reader_theme_pref {
        settings.reading_theme = v as u8;
    }
    if let Some(v) = pending_wifi_default {
        wifi.set_default_from_value(v);
    }
}

struct TxtWriter<'a> {
    buf: &'a mut [u8],
    pos: usize,
}

impl<'a> TxtWriter<'a> {
    fn new(buf: &'a mut [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    fn put(&mut self, data: &[u8]) {
        let n = data.len().min(self.buf.len() - self.pos);
        self.buf[self.pos..self.pos + n].copy_from_slice(&data[..n]);
        self.pos += n;
    }

    fn put_u16(&mut self, val: u16) {
        if val == 0 {
            self.put(b"0");
            return;
        }
        let mut digits = [0u8; 5];
        let mut i = 5;
        let mut v = val;
        while v > 0 {
            i -= 1;
            digits[i] = b'0' + (v % 10) as u8;
            v /= 10;
        }
        self.put(&digits[i..5]);
    }

    fn kv_num(&mut self, key: &[u8], val: u16) {
        self.put(key);
        self.put(b"=");
        self.put_u16(val);
        self.put(b"\n");
    }

    fn kv_str(&mut self, key: &[u8], val: &[u8]) {
        self.put(key);
        self.put(b"=");
        self.put(val);
        self.put(b"\n");
    }
}

pub fn write_settings_txt(s: &SystemSettings, w: &WifiConfig, buf: &mut [u8]) -> usize {
    let mut wr = TxtWriter::new(buf);
    wr.put(b"# rustmix-os settings\n");
    wr.put(b"# lines starting with # are ignored\n\n");

    wr.put(b"# power settings\n");
    wr.kv_num(b"sleep_timeout", s.sleep_timeout);
    wr.kv_num(b"ghost_clear", s.ghost_clear_every as u16);

    wr.put(b"\n# font settings\n");
    wr.kv_num(b"book_font", s.book_font_size_idx as u16);
    wr.kv_num(b"ui_font", s.ui_font_size_idx as u16);

    wr.put(b"\n# reading settings (0=Compact, 1=Default, 2=Relaxed, 3=Spacious)\n");
    wr.kv_num(b"reading_theme", s.reading_theme as u16);

    wr.put(b"\n# reader preferences\n");
    wr.kv_num(b"show_progress", if s.reader_show_progress { 1 } else { 0 });
    wr.kv_num(b"bionic_reading", s.reader_bionic_mode as u16);
    wr.kv_num(b"guide_dots", s.reader_guide_dots_mode as u16);
    wr.kv_num(
        b"sunlight_fading_fix",
        if s.reader_sunlight_fading_fix { 1 } else { 0 },
    );
    wr.kv_num(b"reader_orientation", s.reader_orientation as u16);
    wr.kv_num(b"prepared_font_profile", s.prepared_font_profile as u16);
    wr.kv_num(
        b"prepared_fallback_policy",
        s.prepared_fallback_policy as u16,
    );
    wr.kv_num(b"reader_font_source", s.reader_font_source as u16);
    wr.kv_num(b"reader_sd_font_slot", s.reader_sd_font_slot as u16);
    wr.kv_str(b"reader_sd_font_id", s.reader_sd_font_id());

    wr.put(b"\n# display preferences (persisted only)\n");
    wr.kv_num(b"ui_font_source", s.ui_font_source as u16);
    wr.kv_num(b"display_refresh_mode", s.display_refresh_mode as u16);
    wr.kv_num(
        b"display_invert_colors",
        if s.display_invert_colors { 1 } else { 0 },
    );
    wr.kv_num(
        b"display_contrast_high",
        if s.display_contrast_high { 1 } else { 0 },
    );

    wr.put(b"\n# control settings\n");
    wr.kv_num(b"swap_buttons", if s.swap_buttons { 1 } else { 0 });

    wr.put(
        b"
# wifi credentials for upload mode
",
    );
    wr.put(
        b"# active/default profile used by Wi-Fi Transfer
",
    );
    wr.kv_str(
        b"wifi_ssid",
        w.profile_ssid_bytes(w.default_slot() as usize),
    );
    wr.kv_str(
        b"wifi_pass",
        w.profile_pass_bytes(w.default_slot() as usize),
    );
    wr.kv_num(b"wifi_default", w.default_slot() as u16);

    wr.put(
        b"
# saved Wi-Fi profiles stored in SETTINGS.TXT
",
    );
    const NAME_KEYS: [&[u8]; WIFI_PROFILE_COUNT] = [
        b"wifi_profile_0_name",
        b"wifi_profile_1_name",
        b"wifi_profile_2_name",
    ];
    const SSID_KEYS: [&[u8]; WIFI_PROFILE_COUNT] = [
        b"wifi_profile_0_ssid",
        b"wifi_profile_1_ssid",
        b"wifi_profile_2_ssid",
    ];
    const PASS_KEYS: [&[u8]; WIFI_PROFILE_COUNT] = [
        b"wifi_profile_0_pass",
        b"wifi_profile_1_pass",
        b"wifi_profile_2_pass",
    ];
    let mut idx = 0usize;
    while idx < WIFI_PROFILE_COUNT {
        wr.kv_str(NAME_KEYS[idx], w.profile_name_bytes_or_default(idx));
        wr.kv_str(SSID_KEYS[idx], w.profile_ssid_bytes(idx));
        wr.kv_str(PASS_KEYS[idx], w.profile_pass_bytes(idx));
        idx += 1;
    }
    wr.pos
}
