//! SD-backed Daily Mantra Lua app proof for Rustmix.
//!
//! This module keeps the safe Lua declaration subset as the fallback path.
//! The optional `lua-vm` feature can layer VM execution on top of this file.
//! The SD deployment path remains the 8.3-safe X4 layout:
//!
//! - `/RUSTMIX/APPS/MANTRA/APP.TOM`
//! - `/RUSTMIX/APPS/MANTRA/MAIN.LUA`
//! - `/RUSTMIX/APPS/MANTRA/MANTRAS.TXT`
//!
//! `MANTRAS.TXT` records use `|` as a field separator. The separator is never
//! rendered on screen.

/// Marker emitted/displayed by the first working Lua-backed app path.
pub const LUA_DAILY_MANTRA_APP_MARKER: &str = "rustmix-lua-daily-mantra-app-ok";

/// Marker for the SD runtime proof that loads all required app files.
pub const LUA_DAILY_MANTRA_SD_RUNTIME_MARKER: &str = "rustmix-lua-daily-mantra-sd-runtime-ok";

/// App id used by the accepted manifest/discovery model.
pub const LUA_DAILY_MANTRA_APP_ID: &str = "daily_mantra";

/// 8.3-safe physical app folder under `/RUSTMIX/APPS/` for X4 SD access.
pub const LUA_DAILY_MANTRA_APP_FOLDER: &str = "MANTRA";

/// Manifest file read from `/RUSTMIX/APPS/MANTRA/APP.TOM`.
pub const LUA_DAILY_MANTRA_MANIFEST_FILE: &str = "APP.TOM";

/// Entry file read from `/RUSTMIX/APPS/MANTRA/MAIN.LUA`.
pub const LUA_DAILY_MANTRA_ENTRY_FILE: &str = "MAIN.LUA";

/// Data file read from `/RUSTMIX/APPS/MANTRA/MANTRAS.TXT`.
pub const LUA_DAILY_MANTRA_MANTRAS_FILE: &str = "MANTRAS.TXT";

/// Canonical root for all Lua apps on the SD card.
pub const LUA_APPS_CANONICAL_ROOT: &str = "/RUSTMIX/APPS";

const TITLE_DEFAULT: &str = "Daily Mantra";
const SUBTITLE_DEFAULT: &str = "Day: Today";
const LINE1_DEFAULT: &str = "Mantra: Om Namah Shivaya";
const LINE2_DEFAULT: &str = "";
const LINE3_DEFAULT: &str = "";
const FOOTER_DEFAULT: &str = "";

/// Conservative wrap width for the current X4 bitmap-label Daily Mantra view.
pub const LUA_DAILY_MANTRA_WRAP_CHARS: usize = 52;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LuaDailyMantraSource {
    SdLuaScript,
    BuiltInFallback,
    MissingManifest,
    MissingEntry,
    MissingMantras,
    ManifestInvalidUtf8,
    ScriptInvalidUtf8,
    MantrasInvalidUtf8,
    InvalidManifestContract,
}

impl LuaDailyMantraSource {
    pub const fn label(self) -> &'static str {
        match self {
            Self::SdLuaScript => "SD Lua",
            Self::BuiltInFallback => "Fallback",
            Self::MissingManifest => "Missing APP.TOM",
            Self::MissingEntry => "Missing MAIN.LUA",
            Self::MissingMantras => "Missing MANTRAS.TXT",
            Self::ManifestInvalidUtf8 => "APP.TOM UTF-8 error",
            Self::ScriptInvalidUtf8 => "MAIN.LUA UTF-8 error",
            Self::MantrasInvalidUtf8 => "MANTRAS.TXT UTF-8 error",
            Self::InvalidManifestContract => "Bad APP.TOM",
        }
    }

    pub const fn is_sd_loaded(self) -> bool {
        matches!(self, Self::SdLuaScript)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LuaText<const N: usize> {
    bytes: [u8; N],
    len: usize,
}

impl<const N: usize> LuaText<N> {
    pub const fn empty() -> Self {
        Self {
            bytes: [0; N],
            len: 0,
        }
    }

    pub fn from_str(value: &str) -> Self {
        let mut text = Self::empty();
        text.set(value);
        text
    }

    pub fn clear(&mut self) {
        self.bytes = [0; N];
        self.len = 0;
    }

    pub fn len(&self) -> usize {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn set(&mut self, value: &str) {
        self.clear();
        self.push_str(value);
    }

    pub fn set_i32_line(&mut self, prefix: &str, value: i32) {
        self.clear();
        self.push_str(prefix);
        self.push_i32(value);
    }

    pub fn push_str(&mut self, value: &str) {
        for ch in value.chars() {
            self.push_char(ch);
        }
    }

    pub fn push_char(&mut self, value: char) {
        let mut buf = [0u8; 4];
        let encoded = value.encode_utf8(&mut buf);
        if self.len + encoded.len() <= N {
            self.bytes[self.len..self.len + encoded.len()].copy_from_slice(encoded.as_bytes());
            self.len += encoded.len();
        }
    }

    fn push_byte(&mut self, value: u8) {
        if self.len < N {
            self.bytes[self.len] = value;
            self.len += 1;
        }
    }

    fn push_i32(&mut self, value: i32) {
        if value == 0 {
            self.push_byte(b'0');
            return;
        }
        let mut number = if value < 0 {
            self.push_byte(b'-');
            (-(value as i64)) as u32
        } else {
            value as u32
        };
        let mut digits = [0u8; 10];
        let mut count = 0usize;
        while number > 0 && count < digits.len() {
            digits[count] = b'0' + (number % 10) as u8;
            number /= 10;
            count += 1;
        }
        while count > 0 {
            count -= 1;
            self.push_byte(digits[count]);
        }
    }

    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes[..self.len]).unwrap_or("")
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LuaDailyMantraScreen {
    pub source: LuaDailyMantraSource,
    pub title: LuaText<48>,
    pub subtitle: LuaText<96>,
    pub line1: LuaText<160>,
    pub line2: LuaText<160>,
    pub line3: LuaText<160>,
    pub footer: LuaText<112>,
}

impl LuaDailyMantraScreen {
    pub fn fallback() -> Self {
        Self {
            source: LuaDailyMantraSource::BuiltInFallback,
            title: LuaText::from_str(TITLE_DEFAULT),
            subtitle: LuaText::from_str(SUBTITLE_DEFAULT),
            line1: LuaText::from_str(LINE1_DEFAULT),
            line2: LuaText::from_str(LINE2_DEFAULT),
            line3: LuaText::from_str(LINE3_DEFAULT),
            footer: LuaText::from_str(FOOTER_DEFAULT),
        }
    }

    pub fn missing_manifest() -> Self {
        diagnostic_screen(
            LuaDailyMantraSource::MissingManifest,
            "Missing /RUSTMIX/APPS/MANTRA/APP.TOM",
            "Upload APP.TOM with Wi-Fi Transfer",
        )
    }
    pub fn missing_entry() -> Self {
        diagnostic_screen(
            LuaDailyMantraSource::MissingEntry,
            "Missing /RUSTMIX/APPS/MANTRA/MAIN.LUA",
            "Upload MAIN.LUA with Wi-Fi Transfer",
        )
    }
    pub fn missing_mantras() -> Self {
        diagnostic_screen(
            LuaDailyMantraSource::MissingMantras,
            "Missing /RUSTMIX/APPS/MANTRA/MANTRAS.TXT",
            "Upload MANTRAS.TXT with Wi-Fi Transfer",
        )
    }
    pub fn invalid_manifest_utf8() -> Self {
        diagnostic_screen(
            LuaDailyMantraSource::ManifestInvalidUtf8,
            "APP.TOM is not valid UTF-8",
            "Replace APP.TOM on SD card",
        )
    }
    pub fn invalid_script_utf8() -> Self {
        diagnostic_screen(
            LuaDailyMantraSource::ScriptInvalidUtf8,
            "MAIN.LUA is not valid UTF-8",
            "Replace MAIN.LUA on SD card",
        )
    }
    pub fn invalid_mantras_utf8() -> Self {
        diagnostic_screen(
            LuaDailyMantraSource::MantrasInvalidUtf8,
            "MANTRAS.TXT is not valid UTF-8",
            "Replace MANTRAS.TXT on SD card",
        )
    }
    pub fn invalid_manifest_contract() -> Self {
        diagnostic_screen(
            LuaDailyMantraSource::InvalidManifestContract,
            "APP.TOM must declare id = \"daily_mantra\"",
            "Fix manifest and reopen the app",
        )
    }

    pub fn title(&self) -> &str {
        self.title.as_str()
    }
    pub fn subtitle(&self) -> &str {
        self.subtitle.as_str()
    }
    pub fn line1(&self) -> &str {
        self.line1.as_str()
    }
    pub fn line2(&self) -> &str {
        self.line2.as_str()
    }
    pub fn line3(&self) -> &str {
        self.line3.as_str()
    }
    pub fn footer(&self) -> &str {
        self.footer.as_str()
    }
}

impl Default for LuaDailyMantraScreen {
    fn default() -> Self {
        Self::fallback()
    }
}

fn diagnostic_screen(
    source: LuaDailyMantraSource,
    primary: &str,
    remediation: &str,
) -> LuaDailyMantraScreen {
    let mut screen = LuaDailyMantraScreen::fallback();
    screen.source = source;
    screen.subtitle.set("Lua app SD files are incomplete");
    screen.line1.set(primary);
    screen.line2.set(remediation);
    screen.line3.set("Back exits safely to Tools");
    screen.footer.set("Canonical root: /RUSTMIX/APPS");
    screen
}

/// Builds the SD-backed Daily Mantra screen from all required app files.
pub fn build_daily_mantra_sd_runtime(
    manifest: &str,
    script: &str,
    mantras: &str,
) -> LuaDailyMantraScreen {
    build_daily_mantra_sd_runtime_for_day(manifest, script, mantras, None)
}

/// Builds the Daily Mantra screen and optionally selects a specific weekday record.
///
/// The VM bridge uses this with a VM-derived weekday, for example `Monday`, so
/// `Day: Monday` is shown even when older SD files have a non-weekday first row.
pub fn build_daily_mantra_sd_runtime_for_day(
    manifest: &str,
    script: &str,
    mantras: &str,
    requested_day: Option<&str>,
) -> LuaDailyMantraScreen {
    if !manifest_declares_daily_mantra(manifest) {
        return LuaDailyMantraScreen::invalid_manifest_contract();
    }

    let mut screen = evaluate_daily_mantra_lua_subset(script);
    screen.source = LuaDailyMantraSource::SdLuaScript;
    screen.title.set("Daily Mantra");

    let requested = requested_day.and_then(normalize_weekday);
    let selected = if let Some(day) = requested {
        daily_mantra_record_for_day(mantras, day).or_else(|| first_daily_mantra_record(mantras))
    } else {
        first_daily_mantra_record(mantras)
    };

    if let Some(record) = selected {
        let display_day = requested.unwrap_or(record.day);
        set_day_line(&mut screen, display_day);
        set_wrapped_mantra_lines(&mut screen, record);
    } else {
        set_day_line(&mut screen, requested.unwrap_or("Today"));
        screen.line1.set("Mantra: No mantra data found.");
        screen.line2.clear();
        screen.line3.clear();
    }

    screen.footer.clear();
    screen
}

fn manifest_declares_daily_mantra(manifest: &str) -> bool {
    manifest.lines().any(|line| {
        let trimmed = line.trim();
        trimmed == "id = \"daily_mantra\"" || trimmed == "id=\"daily_mantra\""
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LuaDailyMantraRecord<'a> {
    pub day: &'static str,
    parts: [&'a str; 3],
    part_count: usize,
}

fn first_daily_mantra_record(text: &str) -> Option<LuaDailyMantraRecord<'_>> {
    text.lines().find_map(parse_daily_mantra_record)
}

fn daily_mantra_record_for_day<'a>(
    text: &'a str,
    day: &'static str,
) -> Option<LuaDailyMantraRecord<'a>> {
    text.lines()
        .filter_map(parse_daily_mantra_record)
        .find(|record| record.day == day)
}

pub fn parse_daily_mantra_record(line: &str) -> Option<LuaDailyMantraRecord<'_>> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("--") {
        return None;
    }

    let mut fields = [""; 4];
    let mut field_count = 0usize;
    for field in trimmed.split('|') {
        if field_count >= fields.len() {
            break;
        }
        fields[field_count] = field.trim();
        field_count += 1;
    }
    if field_count == 0 {
        return None;
    }

    let mut start = 0usize;
    let day = if let Some(day) = normalize_weekday(fields[0]) {
        start = 1;
        day
    } else {
        "Today"
    };

    let mut parts = [""; 3];
    let mut part_count = 0usize;
    let mut index = start;
    while index < field_count && part_count < parts.len() {
        if !fields[index].is_empty() {
            parts[part_count] = fields[index];
            part_count += 1;
        }
        index += 1;
    }

    if part_count == 0 {
        None
    } else {
        Some(LuaDailyMantraRecord {
            day,
            parts,
            part_count,
        })
    }
}

pub fn normalize_weekday(value: &str) -> Option<&'static str> {
    let value = value.trim();
    if value.eq_ignore_ascii_case("monday") || value.eq_ignore_ascii_case("mon") {
        Some("Monday")
    } else if value.eq_ignore_ascii_case("tuesday") || value.eq_ignore_ascii_case("tue") {
        Some("Tuesday")
    } else if value.eq_ignore_ascii_case("wednesday") || value.eq_ignore_ascii_case("wed") {
        Some("Wednesday")
    } else if value.eq_ignore_ascii_case("thursday") || value.eq_ignore_ascii_case("thu") {
        Some("Thursday")
    } else if value.eq_ignore_ascii_case("friday") || value.eq_ignore_ascii_case("fri") {
        Some("Friday")
    } else if value.eq_ignore_ascii_case("saturday") || value.eq_ignore_ascii_case("sat") {
        Some("Saturday")
    } else if value.eq_ignore_ascii_case("sunday") || value.eq_ignore_ascii_case("sun") {
        Some("Sunday")
    } else {
        None
    }
}

fn set_day_line(screen: &mut LuaDailyMantraScreen, day: &str) {
    screen.subtitle.clear();
    screen.subtitle.push_str("Day: ");
    screen.subtitle.push_str(day);
}

fn set_wrapped_mantra_lines(screen: &mut LuaDailyMantraScreen, record: LuaDailyMantraRecord<'_>) {
    let mut combined = LuaText::<256>::empty();
    let mut index = 0usize;
    while index < record.part_count {
        let part = record.parts[index].trim();
        if !part.is_empty() {
            if !combined.is_empty() {
                combined.push_str(" - ");
            }
            combined.push_str(part);
        }
        index += 1;
    }

    screen.line1.set("Mantra:");
    screen.line2.clear();
    screen.line3.clear();

    let mut line_index = 0usize;
    for word in combined.as_str().split_whitespace() {
        if word.is_empty() {
            continue;
        }
        if try_append_word(
            line_mut(screen, line_index),
            word,
            LUA_DAILY_MANTRA_WRAP_CHARS,
        ) {
            continue;
        }
        if line_index < 2 {
            line_index += 1;
            if !try_append_word(
                line_mut(screen, line_index),
                word,
                LUA_DAILY_MANTRA_WRAP_CHARS,
            ) {
                append_ellipsis(line_mut(screen, line_index), LUA_DAILY_MANTRA_WRAP_CHARS);
                break;
            }
        } else {
            append_ellipsis(line_mut(screen, line_index), LUA_DAILY_MANTRA_WRAP_CHARS);
            break;
        }
    }
}

fn line_mut(screen: &mut LuaDailyMantraScreen, index: usize) -> &mut LuaText<160> {
    match index {
        0 => &mut screen.line1,
        1 => &mut screen.line2,
        _ => &mut screen.line3,
    }
}

fn try_append_word<const N: usize>(line: &mut LuaText<N>, word: &str, limit: usize) -> bool {
    let needs_space = !line.is_empty();
    let extra = if needs_space { 1 } else { 0 };
    if line.len() + extra + word.len() <= limit {
        if needs_space {
            line.push_char(' ');
        }
        line.push_str(word);
        return true;
    }
    if line.is_empty() {
        for ch in word.chars() {
            if line.len() + ch.len_utf8() > limit {
                append_ellipsis(line, limit);
                return false;
            }
            line.push_char(ch);
        }
    }
    false
}

fn append_ellipsis<const N: usize>(line: &mut LuaText<N>, limit: usize) {
    if line.len() + 3 <= limit {
        line.push_str("...");
    }
}

/// Evaluates the first app's safe Lua declaration subset.
pub fn evaluate_daily_mantra_lua_subset(script: &str) -> LuaDailyMantraScreen {
    let mut screen = LuaDailyMantraScreen::fallback();
    screen.source = LuaDailyMantraSource::SdLuaScript;
    for line in script.lines() {
        if let Some((key, value)) = parse_lua_string_assignment(line) {
            match key {
                "display_title" | "title" => screen.title.set(value),
                "display_subtitle" | "subtitle" => screen.subtitle.set(value),
                "display_line1" | "line1" => screen.line1.set(value),
                "display_line2" | "line2" => screen.line2.set(value),
                "display_line3" | "line3" => screen.line3.set(value),
                "display_footer" | "footer" => screen.footer.set(value),
                _ => {}
            }
        }
    }
    screen
}

pub fn parse_lua_string_assignment(line: &str) -> Option<(&str, &str)> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with("--") || trimmed.starts_with("local function") {
        return None;
    }
    let eq = trimmed.find('=')?;
    let key = trimmed[..eq].trim().trim_start_matches("local ").trim();
    if key.is_empty() || key.contains(' ') || key.contains('.') || key.contains('(') {
        return None;
    }
    let mut value = trimmed[eq + 1..].trim();
    if value.ends_with(',') {
        value = value[..value.len() - 1].trim_end();
    }
    if let Some(comment_pos) = value.find(" --") {
        value = value[..comment_pos].trim_end();
    }
    if !(value.starts_with('"') && value.ends_with('"') && value.len() >= 2) {
        return None;
    }
    Some((key, &value[1..value.len() - 1]))
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str = r#"
id = "daily_mantra"
name = "Daily Mantra"
category = "Tools"
type = "activity"
version = "0.1.0"
entry = "MAIN.LUA"
capabilities = ["display", "input", "storage", "time"]
"#;

    #[test]
    fn parses_weekday_and_separator_without_rendering_pipe() {
        let record = parse_daily_mantra_record(
            "Monday|Om Namah Shivaya|A steady mind turns every page into practice.",
        )
        .unwrap();
        assert_eq!(record.day, "Monday");
        assert_eq!(record.parts[0], "Om Namah Shivaya");
        assert_eq!(
            record.parts[1],
            "A steady mind turns every page into practice."
        );
    }

    #[test]
    fn separator_without_weekday_defaults_to_today() {
        let screen = build_daily_mantra_sd_runtime(
            MANIFEST,
            "display_title = \"Old Lua Title\"\n",
            "Om Namah Shivaya|A steady mind turns every page into practice.\n",
        );
        assert_eq!(screen.title(), "Daily Mantra");
        assert_eq!(screen.subtitle(), "Day: Today");
        assert!(!screen.line1().contains('|'));
        assert!(!screen.line2().contains('|'));
        assert!(!screen.line3().contains('|'));
    }

    #[test]
    fn sd_runtime_uses_day_and_wrapped_mantra_data() {
        let screen = build_daily_mantra_sd_runtime(
            MANIFEST,
            "display_title = \"Lua Daily Mantra\"\nvm_expression = \"return 108 + 0\"\n",
            "Tuesday|Om Shanti Shanti Shanti|Peace in thought, word, and action.\n",
        );
        assert_eq!(screen.source, LuaDailyMantraSource::SdLuaScript);
        assert_eq!(screen.source.label(), "SD Lua");
        assert_eq!(screen.title(), "Daily Mantra");
        assert_eq!(screen.subtitle(), "Day: Tuesday");
        assert!(screen.line1().starts_with("Mantra: Om Shanti"));
        assert!(!screen.line1().contains('|'));
        assert!(screen.footer().is_empty());
    }

    #[test]
    fn requested_weekday_selects_matching_record() {
        let screen = build_daily_mantra_sd_runtime_for_day(
            MANIFEST,
            "display_title = \"Lua Daily Mantra\"\n",
            "Om Namah Shivaya|A steady mind turns every page into practice.\nMonday|Om Namah Shivaya|A steady mind turns every page into practice.\nTuesday|Om Shanti|Peace.\n",
            Some("Monday"),
        );
        assert_eq!(screen.subtitle(), "Day: Monday");
        assert!(screen.line1().contains("Om Namah Shivaya"));
        assert!(!screen.line1().contains('|'));
    }

    #[test]
    fn invalid_manifest_produces_diagnostic_screen() {
        let screen = build_daily_mantra_sd_runtime("id = \"bad\"", "", "Om");
        assert_eq!(screen.source, LuaDailyMantraSource::InvalidManifestContract);
        assert!(screen.line1().contains("id = \"daily_mantra\""));
    }

    #[test]
    fn empty_mantra_file_is_explicit_on_screen() {
        let screen = build_daily_mantra_sd_runtime(MANIFEST, "", "\n# comment\n-- lua comment\n");
        assert_eq!(screen.source, LuaDailyMantraSource::SdLuaScript);
        assert_eq!(screen.subtitle(), "Day: Today");
        assert_eq!(screen.line1(), "Mantra: No mantra data found.");
    }
}
