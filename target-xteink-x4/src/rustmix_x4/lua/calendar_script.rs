//! SD-backed Calendar Lua app renderer for Rustmix.
//!
//! This module intentionally keeps the current constrained Lua declaration
//! subset plus the feature-gated VM event-selection proof. Calendar app data is
//! read from `/RUSTMIX/APPS/CALENDAR` using 8.3-safe physical names.
//!
//! Required SD files:
//!
//! - `/RUSTMIX/APPS/CALENDAR/APP.TOM`
//! - `/RUSTMIX/APPS/CALENDAR/MAIN.LUA`
//! - `/RUSTMIX/APPS/CALENDAR/EVENTS.TXT`

/// Marker emitted/displayed by the Calendar Lua-backed app path.
pub const LUA_CALENDAR_APP_MARKER: &str = "rustmix-lua-calendar-app-ok";

/// Marker for the SD runtime proof that loads all required Calendar app files.
pub const LUA_CALENDAR_SD_RUNTIME_MARKER: &str = "rustmix-lua-calendar-sd-runtime-ok";

/// Optional VM marker for event-selection expression support.
pub const LUA_CALENDAR_VM_SELECTION_MARKER: &str = "rustmix-lua-calendar-vm-selection-ok";

/// Logical app id used by the accepted manifest/discovery model.
pub const LUA_CALENDAR_APP_ID: &str = "calendar";

/// 8.3-safe physical app folder under `/RUSTMIX/APPS/` for X4 SD access.
pub const LUA_CALENDAR_APP_FOLDER: &str = "CALENDAR";

/// Manifest file read from `/RUSTMIX/APPS/CALENDAR/APP.TOM`.
pub const LUA_CALENDAR_MANIFEST_FILE: &str = "APP.TOM";

/// Entry file read from `/RUSTMIX/APPS/CALENDAR/MAIN.LUA`.
pub const LUA_CALENDAR_ENTRY_FILE: &str = "MAIN.LUA";

/// Data file read from `/RUSTMIX/APPS/CALENDAR/EVENTS.TXT`.
pub const LUA_CALENDAR_EVENTS_FILE: &str = "EVENTS.TXT";

/// Optional American calendar / U.S. federal holiday file read from
/// `/RUSTMIX/APPS/CALENDAR/US2026.TXT`.
pub const LUA_CALENDAR_AMERICAN_EVENTS_FILE: &str = "US2026.TXT";

/// Optional Hindu calendar event file read from
/// `/RUSTMIX/APPS/CALENDAR/HINDU26.TXT`.
pub const LUA_CALENDAR_HINDU_EVENTS_FILE: &str = "HINDU26.TXT";

/// Marker for the combined native-month-grid + SD Lua events Calendar view.
pub const LUA_CALENDAR_COMBINED_MARKER: &str = "combined-calendar-lua-app-rustmix-ok";

const TITLE_DEFAULT: &str = "Calendar";
const SUBTITLE_DEFAULT: &str = "Date: Today";
const LINE1_DEFAULT: &str = "Event: No event loaded yet";
const LINE2_DEFAULT: &str = "";
const LINE3_DEFAULT: &str = "";
const FOOTER_DEFAULT: &str = "Loaded from /RUSTMIX/APPS/CALENDAR";

/// Conservative wrap width for the current X4 bitmap-label Calendar view.
pub const LUA_CALENDAR_WRAP_CHARS: usize = 44;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LuaCalendarSource {
    SdLuaScript,
    BuiltInFallback,
    MissingManifest,
    MissingEntry,
    MissingEvents,
    ManifestInvalidUtf8,
    ScriptInvalidUtf8,
    EventsInvalidUtf8,
    InvalidManifestContract,
}

impl LuaCalendarSource {
    pub const fn label(self) -> &'static str {
        match self {
            Self::SdLuaScript => "SD Lua",
            Self::BuiltInFallback => "Fallback",
            Self::MissingManifest => "Missing APP.TOM",
            Self::MissingEntry => "Missing MAIN.LUA",
            Self::MissingEvents => "Missing EVENTS.TXT",
            Self::ManifestInvalidUtf8 => "APP.TOM UTF-8 error",
            Self::ScriptInvalidUtf8 => "MAIN.LUA UTF-8 error",
            Self::EventsInvalidUtf8 => "EVENTS.TXT UTF-8 error",
            Self::InvalidManifestContract => "Bad APP.TOM",
        }
    }

    pub const fn is_sd_loaded(self) -> bool {
        matches!(self, Self::SdLuaScript)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LuaCalendarText<const N: usize> {
    bytes: [u8; N],
    len: usize,
}

impl<const N: usize> LuaCalendarText<N> {
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
pub struct LuaCalendarScreen {
    pub source: LuaCalendarSource,
    pub title: LuaCalendarText<48>,
    pub subtitle: LuaCalendarText<96>,
    pub line1: LuaCalendarText<160>,
    pub line2: LuaCalendarText<160>,
    pub line3: LuaCalendarText<160>,
    pub footer: LuaCalendarText<112>,
}

impl LuaCalendarScreen {
    pub fn fallback() -> Self {
        Self {
            source: LuaCalendarSource::BuiltInFallback,
            title: LuaCalendarText::from_str(TITLE_DEFAULT),
            subtitle: LuaCalendarText::from_str(SUBTITLE_DEFAULT),
            line1: LuaCalendarText::from_str(LINE1_DEFAULT),
            line2: LuaCalendarText::from_str(LINE2_DEFAULT),
            line3: LuaCalendarText::from_str(LINE3_DEFAULT),
            footer: LuaCalendarText::from_str(FOOTER_DEFAULT),
        }
    }

    pub fn missing_manifest() -> Self {
        diagnostic_screen(
            LuaCalendarSource::MissingManifest,
            "Missing /RUSTMIX/APPS/CALENDAR/APP.TOM",
            "Upload APP.TOM with Wi-Fi Transfer",
        )
    }
    pub fn missing_entry() -> Self {
        diagnostic_screen(
            LuaCalendarSource::MissingEntry,
            "Missing /RUSTMIX/APPS/CALENDAR/MAIN.LUA",
            "Upload MAIN.LUA with Wi-Fi Transfer",
        )
    }
    pub fn missing_events() -> Self {
        diagnostic_screen(
            LuaCalendarSource::MissingEvents,
            "Missing /RUSTMIX/APPS/CALENDAR/EVENTS.TXT",
            "Upload EVENTS.TXT with Wi-Fi Transfer",
        )
    }
    pub fn invalid_manifest_utf8() -> Self {
        diagnostic_screen(
            LuaCalendarSource::ManifestInvalidUtf8,
            "APP.TOM is not valid UTF-8",
            "Replace APP.TOM on SD card",
        )
    }
    pub fn invalid_script_utf8() -> Self {
        diagnostic_screen(
            LuaCalendarSource::ScriptInvalidUtf8,
            "MAIN.LUA is not valid UTF-8",
            "Replace MAIN.LUA on SD card",
        )
    }
    pub fn invalid_events_utf8() -> Self {
        diagnostic_screen(
            LuaCalendarSource::EventsInvalidUtf8,
            "EVENTS.TXT is not valid UTF-8",
            "Replace EVENTS.TXT on SD card",
        )
    }
    pub fn invalid_manifest_contract() -> Self {
        diagnostic_screen(
            LuaCalendarSource::InvalidManifestContract,
            "APP.TOM must declare id = \"calendar\"",
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

impl Default for LuaCalendarScreen {
    fn default() -> Self {
        Self::fallback()
    }
}

fn diagnostic_screen(
    source: LuaCalendarSource,
    primary: &str,
    remediation: &str,
) -> LuaCalendarScreen {
    let mut screen = LuaCalendarScreen::fallback();
    screen.source = source;
    screen.subtitle.set("Lua app SD files are incomplete");
    screen.line1.set(primary);
    screen.line2.set(remediation);
    screen.line3.clear();
    screen.footer.set("Canonical root: /RUSTMIX/APPS");
    screen
}

/// Build the Calendar screen from SD file contents.
///
/// This intentionally ignores visual text assignments in MAIN.LUA for the event
/// body. `EVENTS.TXT` is authoritative for date/event display so old SD scripts
/// cannot accidentally show raw `|` separators.
pub fn build_calendar_sd_runtime(manifest: &str, script: &str, events: &str) -> LuaCalendarScreen {
    if !manifest_declares_calendar(manifest) {
        return LuaCalendarScreen::invalid_manifest_contract();
    }

    let mut screen = LuaCalendarScreen::fallback();
    screen.source = LuaCalendarSource::SdLuaScript;
    screen.title.set("Calendar");

    let selected_index = calendar_selected_event_index(script).unwrap_or(0);
    if let Some(event) = nth_calendar_record(events, selected_index) {
        set_date_line(&mut screen, event.date);
        set_wrapped_event(&mut screen, event.title, event.detail);
    } else if first_non_empty_line(events).is_some() {
        screen.subtitle.set("Date: Today");
        screen.line1.set("Event: No matching event found");
        screen.line2.set("");
    } else {
        screen.subtitle.set("Date: Today");
        screen.line1.set("Event: EVENTS.TXT is present but empty");
        screen.line2.set("");
    }

    screen.line3.clear();
    screen.footer.set("Loaded: APP.TOM + MAIN.LUA + EVENTS.TXT");
    screen
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CalendarEventKind {
    American,
    Hindu,
    General,
}

/// Build the combined Calendar screen used by the unified native/Lua app.
///
/// The native Rust side draws the month grid. This model supplies the compact
/// event lines displayed below that grid. The event contract is SD-data only:
/// `EVENTS.TXT` remains the legacy/general file, while `US2026.TXT` and
/// `HINDU26.TXT` can be uploaded beside it for yearly American and Hindu rows.
pub fn build_combined_calendar_sd_runtime(
    manifest: &str,
    script: &str,
    events: &str,
    today: Option<(i32, u8, u8)>,
) -> LuaCalendarScreen {
    if !manifest_declares_calendar(manifest) {
        return LuaCalendarScreen::invalid_manifest_contract();
    }

    let mut screen = LuaCalendarScreen::fallback();
    screen.source = LuaCalendarSource::SdLuaScript;
    screen.title.set("Calendar");

    if let Some((year, month, day)) = today {
        set_combined_calendar_subtitle(&mut screen, year, month, day);
        set_upcoming_calendar_events(&mut screen, events, year, month, day);
    } else {
        screen.subtitle.set("Calendar + Events");
        let selected_index = calendar_selected_event_index(script).unwrap_or(0);
        if let Some(event) = nth_calendar_record(events, selected_index) {
            set_wrapped_event(&mut screen, event.title, event.detail);
            screen.line3.set("Sync Date & Time for today's events");
        } else {
            screen.line1.set("Sync Date & Time to select events");
            screen.line2.set("Upload US2026.TXT and HINDU26.TXT");
            screen.line3.set("Optional: EVENTS.TXT for personal events");
        }
    }

    screen.footer.set("SD events: US2026.TXT + HINDU26.TXT");
    screen
}

fn set_combined_calendar_subtitle(screen: &mut LuaCalendarScreen, year: i32, month: u8, day: u8) {
    screen.subtitle.set("Events from ");
    push_yyyy_mm_dd(&mut screen.subtitle, year, month, day);
}

fn set_upcoming_calendar_events(
    screen: &mut LuaCalendarScreen,
    events: &str,
    year: i32,
    month: u8,
    day: u8,
) {
    let mut american = LuaCalendarText::<160>::empty();
    let mut hindu = LuaCalendarText::<160>::empty();
    let mut general = LuaCalendarText::<160>::empty();
    let mut has_american = false;
    let mut has_hindu = false;
    let mut has_general = false;

    for raw in events.lines() {
        let Some(record) = parse_combined_calendar_record(raw) else {
            continue;
        };
        let Some((event_year, event_month, event_day)) = parse_calendar_ymd(record.date) else {
            continue;
        };
        if compare_ymd(event_year, event_month, event_day, year, month, day) < 0 {
            continue;
        }

        match record.kind {
            CalendarEventKind::American if !has_american => {
                set_compact_event_line(
                    &mut american,
                    event_month,
                    event_day,
                    "US",
                    record.title,
                    record.detail,
                );
                has_american = true;
            }
            CalendarEventKind::Hindu if !has_hindu => {
                set_compact_event_line(
                    &mut hindu,
                    event_month,
                    event_day,
                    "Hindu",
                    record.title,
                    record.detail,
                );
                has_hindu = true;
            }
            CalendarEventKind::General if !has_general => {
                set_compact_event_line(
                    &mut general,
                    event_month,
                    event_day,
                    "Event",
                    record.title,
                    record.detail,
                );
                has_general = true;
            }
            _ => {}
        }

        if has_american && has_hindu && has_general {
            break;
        }
    }

    screen.line1.clear();
    screen.line2.clear();
    screen.line3.clear();

    if has_american {
        screen.line1 = american;
    } else if has_general {
        screen.line1 = general;
        has_general = false;
    } else {
        screen.line1.set("US: no upcoming row in US2026.TXT");
    }

    if has_hindu {
        screen.line2 = hindu;
    } else if has_general {
        screen.line2 = general;
        has_general = false;
    } else {
        screen.line2.set("Hindu: no upcoming row in HINDU26.TXT");
    }

    if has_general {
        screen.line3 = general;
    } else {
        screen.line3.set("EVENTS.TXT can add personal events");
    }
}

struct CombinedCalendarRecord<'a> {
    date: &'a str,
    kind: CalendarEventKind,
    title: &'a str,
    detail: &'a str,
}

fn parse_combined_calendar_record(line: &str) -> Option<CombinedCalendarRecord<'_>> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }

    let mut parts = trimmed.split('|').map(str::trim);
    let date = parts.next()?;
    let first = parts.next().unwrap_or("");
    let second = parts.next().unwrap_or("");
    let third = parts.next().unwrap_or("");

    if first.is_empty() {
        return None;
    }

    let kind = classify_calendar_event_kind(first);
    let (title, detail) = if third.is_empty() {
        (first, second)
    } else {
        (second, third)
    };

    Some(CombinedCalendarRecord {
        date,
        kind,
        title,
        detail,
    })
}

fn classify_calendar_event_kind(value: &str) -> CalendarEventKind {
    let value = value.trim();
    if value.eq_ignore_ascii_case("US")
        || value.eq_ignore_ascii_case("USA")
        || value.eq_ignore_ascii_case("American")
        || value.eq_ignore_ascii_case("Federal")
    {
        CalendarEventKind::American
    } else if value.eq_ignore_ascii_case("Hindu")
        || value.eq_ignore_ascii_case("Vrat")
        || value.eq_ignore_ascii_case("Panchang")
    {
        CalendarEventKind::Hindu
    } else {
        CalendarEventKind::General
    }
}

fn parse_calendar_ymd(value: &str) -> Option<(i32, u8, u8)> {
    let bytes = value.as_bytes();
    if bytes.len() < 10 || bytes.get(4) != Some(&b'-') || bytes.get(7) != Some(&b'-') {
        return None;
    }
    let year = parse_i32_digits(&bytes[0..4])?;
    let month = parse_u8_digits(&bytes[5..7])?;
    let day = parse_u8_digits(&bytes[8..10])?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    Some((year, month, day))
}

fn parse_i32_digits(bytes: &[u8]) -> Option<i32> {
    let mut value = 0i32;
    for byte in bytes {
        if !byte.is_ascii_digit() {
            return None;
        }
        value = value
            .saturating_mul(10)
            .saturating_add(i32::from(byte.saturating_sub(b'0')));
    }
    Some(value)
}

fn parse_u8_digits(bytes: &[u8]) -> Option<u8> {
    let mut value = 0u8;
    for byte in bytes {
        if !byte.is_ascii_digit() {
            return None;
        }
        value = value
            .saturating_mul(10)
            .saturating_add(byte.saturating_sub(b'0'));
    }
    Some(value)
}

fn compare_ymd(
    left_year: i32,
    left_month: u8,
    left_day: u8,
    right_year: i32,
    right_month: u8,
    right_day: u8,
) -> i8 {
    if left_year != right_year {
        return if left_year < right_year { -1 } else { 1 };
    }
    if left_month != right_month {
        return if left_month < right_month { -1 } else { 1 };
    }
    if left_day != right_day {
        return if left_day < right_day { -1 } else { 1 };
    }
    0
}

fn set_compact_event_line(
    line: &mut LuaCalendarText<160>,
    month: u8,
    day: u8,
    label: &str,
    title: &str,
    detail: &str,
) {
    line.clear();
    push_two_digits(line, month);
    line.push_char('/');
    push_two_digits(line, day);
    line.push_char(' ');
    line.push_str(label);
    line.push_str(": ");
    line.push_str(title);
    if !detail.is_empty() {
        line.push_str(" - ");
        line.push_str(detail);
    }
}

fn push_yyyy_mm_dd<const N: usize>(line: &mut LuaCalendarText<N>, year: i32, month: u8, day: u8) {
    line.push_i32(year);
    line.push_char('-');
    push_two_digits(line, month);
    line.push_char('-');
    push_two_digits(line, day);
}

fn push_two_digits<const N: usize>(line: &mut LuaCalendarText<N>, value: u8) {
    line.push_byte(b'0' + (value / 10));
    line.push_byte(b'0' + (value % 10));
}

fn manifest_declares_calendar(manifest: &str) -> bool {
    manifest.lines().any(|line| {
        let trimmed = line.trim();
        trimmed == "id = \"calendar\"" || trimmed == "id=\"calendar\""
    })
}

fn calendar_selected_event_index(script: &str) -> Option<usize> {
    #[cfg(feature = "lua-vm")]
    {
        let expr = extract_calendar_vm_event_index(script)?;
        let mut vm = rustmix_lua_vm::LuaVm::new();
        match vm.eval_i32(expr) {
            Ok(value) if value > 0 => return Some((value as usize).saturating_sub(1)),
            _ => return None,
        }
    }
    #[cfg(not(feature = "lua-vm"))]
    {
        let _ = script;
        None
    }
}

pub fn extract_calendar_vm_event_index(script: &str) -> Option<&str> {
    for line in script.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(expr) = trimmed.strip_prefix("-- rustmix-vm-event:") {
            let expr = expr.trim();
            if !expr.is_empty() {
                return Some(expr);
            }
        }
        if let Some(expr) = parse_vm_assignment(trimmed, "vm_event_index_expression") {
            return Some(expr);
        }
    }
    None
}

fn parse_vm_assignment<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let rest = line.strip_prefix(key)?.trim_start();
    let rest = rest.strip_prefix('=')?.trim_start();
    let value = strip_lua_trailing_comment(rest)
        .trim_end_matches(',')
        .trim();
    if value.len() < 2 || !value.starts_with('"') || !value.ends_with('"') {
        return None;
    }
    let inner = &value[1..value.len() - 1];
    if inner.trim().is_empty() {
        None
    } else {
        Some(inner.trim())
    }
}

fn strip_lua_trailing_comment(value: &str) -> &str {
    match value.find(" --") {
        Some(pos) => value[..pos].trim_end(),
        None => value,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CalendarRecord<'a> {
    pub date: &'a str,
    pub title: &'a str,
    pub detail: &'a str,
}

fn nth_calendar_record(text: &str, index: usize) -> Option<CalendarRecord<'_>> {
    let mut seen = 0usize;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("--") {
            continue;
        }
        if let Some(record) = parse_calendar_record(line) {
            if seen == index {
                return Some(record);
            }
            seen += 1;
        }
    }
    None
}

fn parse_calendar_record(line: &str) -> Option<CalendarRecord<'_>> {
    let mut parts = line.splitn(3, '|');
    let date = parts.next()?.trim();
    let title = parts.next().unwrap_or("").trim();
    let detail = parts.next().unwrap_or("").trim();
    if date.is_empty() || title.is_empty() {
        None
    } else {
        Some(CalendarRecord {
            date,
            title,
            detail,
        })
    }
}

fn first_non_empty_line(text: &str) -> Option<&str> {
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#') && !line.starts_with("--"))
}

fn set_date_line(screen: &mut LuaCalendarScreen, date: &str) {
    screen.subtitle.set("Date: ");
    screen.subtitle.push_str(date);
}

fn set_wrapped_event(screen: &mut LuaCalendarScreen, title: &str, detail: &str) {
    let mut combined = LuaCalendarText::<220>::empty();
    combined.push_str("Event: ");
    combined.push_str(title);
    if !detail.trim().is_empty() {
        combined.push_str(" - ");
        combined.push_str(detail.trim());
    }
    wrap_two_lines(combined.as_str(), &mut screen.line1, &mut screen.line2);
}

fn wrap_two_lines(text: &str, line1: &mut LuaCalendarText<160>, line2: &mut LuaCalendarText<160>) {
    let trimmed = text.trim();
    line1.clear();
    line2.clear();
    if trimmed.chars().count() <= LUA_CALENDAR_WRAP_CHARS {
        line1.set(trimmed);
        return;
    }

    let mut split_byte = trimmed.len();
    let mut char_count = 0usize;
    for (idx, ch) in trimmed.char_indices() {
        if char_count >= LUA_CALENDAR_WRAP_CHARS {
            split_byte = idx;
            break;
        }
        char_count += 1;
        split_byte = idx + ch.len_utf8();
    }

    let prefix = &trimmed[..split_byte];
    let split = prefix.rfind(' ').unwrap_or(split_byte).max(1);
    line1.set(trimmed[..split].trim_end());
    line2.set(trimmed[split..].trim_start());
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str = r#"id = "calendar"
name = "Calendar"
category = "Productivity"
type = "activity"
version = "0.1.0"
entry = "MAIN.LUA"
capabilities = ["display", "input", "storage", "time"]
"#;

    #[test]
    fn sd_runtime_uses_events_txt_for_date_and_event_layout() {
        let script = r#"
display_title = "Old title from script"
display_line1 = "This must not override EVENTS.TXT"
"#;
        let screen = build_calendar_sd_runtime(
            MANIFEST,
            script,
            "2026-05-11|Rustmix Lua app milestone|Calendar app follow-up under Productivity\n",
        );
        assert_eq!(screen.source, LuaCalendarSource::SdLuaScript);
        assert_eq!(screen.title(), "Calendar");
        assert_eq!(screen.subtitle(), "Date: 2026-05-11");
        assert!(
            screen
                .line1()
                .starts_with("Event: Rustmix Lua app milestone")
        );
        assert!(screen.line2().contains("Productivity") || screen.line1().contains("Productivity"));
        assert!(!screen.line1().contains('|'));
        assert!(!screen.line2().contains('|'));
        assert_eq!(screen.line3(), "");
    }

    #[test]
    fn invalid_manifest_produces_diagnostic_screen() {
        let screen = build_calendar_sd_runtime("id = \"bad\"", "", "2026-01-01|New Year");
        assert_eq!(screen.source, LuaCalendarSource::InvalidManifestContract);
        assert!(screen.line1().contains("id = \"calendar\""));
    }

    #[test]
    fn extracts_vm_event_index_expression() {
        assert_eq!(
            extract_calendar_vm_event_index("vm_event_index_expression = \"return 2 + 0\""),
            Some("return 2 + 0")
        );
        assert_eq!(
            extract_calendar_vm_event_index("-- rustmix-vm-event: return 3"),
            Some("return 3")
        );
    }
}
