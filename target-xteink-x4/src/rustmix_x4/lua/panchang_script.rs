//! SD-backed Panchang Lua app for Rustmix.
//!
//! This app follows the accepted Lua deployment contract:
//!
//! - physical folder: `/RUSTMIX/APPS/PANCHANG`
//! - logical app id: `panchang`
//! - manifest: `APP.TOM`
//! - script: `MAIN.LUA`
//! - precomputed data: `DATA/Y2026.TXT`
//!
//! It intentionally uses precomputed data rather than doing astronomical
//! calculations on the ESP32-C3. A small VM expression can select a record when
//! built with `lua-vm`; otherwise the first record is used.

pub const LUA_PANCHANG_APP_MARKER: &str = "rustmix-lua-panchang-app-ok";
pub const LUA_PANCHANG_SD_RUNTIME_MARKER: &str = "rustmix-lua-panchang-sd-runtime-ok";
pub const LUA_PANCHANG_VM_SELECTION_MARKER: &str = "rustmix-lua-panchang-vm-selection-ok";

pub const LUA_PANCHANG_APP_ID: &str = "panchang";
pub const LUA_PANCHANG_APP_FOLDER: &str = "PANCHANG";
pub const LUA_PANCHANG_MANIFEST_FILE: &str = "APP.TOM";
pub const LUA_PANCHANG_ENTRY_FILE: &str = "MAIN.LUA";
pub const LUA_PANCHANG_DATA_DIR: &str = "DATA";
pub const LUA_PANCHANG_DATA_FILE: &str = "Y2026.TXT";

const TITLE_DEFAULT: &str = "Panchang";
const SUBTITLE_DEFAULT: &str = "Date: Today";
const LINE1_DEFAULT: &str = "Tithi: Not loaded";
const LINE2_DEFAULT: &str = "Nakshatra: Not loaded";
const LINE3_DEFAULT: &str = "Back returns to Productivity";
const FOOTER_DEFAULT: &str = "Loaded from /RUSTMIX/APPS/PANCHANG";

pub const LUA_PANCHANG_WRAP_CHARS: usize = 52;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LuaPanchangSource {
    SdLuaScript,
    BuiltInFallback,
    MissingManifest,
    MissingEntry,
    MissingData,
    ManifestInvalidUtf8,
    ScriptInvalidUtf8,
    DataInvalidUtf8,
    InvalidManifestContract,
}

impl LuaPanchangSource {
    pub const fn label(self) -> &'static str {
        match self {
            Self::SdLuaScript => "SD Lua",
            Self::BuiltInFallback => "Fallback",
            Self::MissingManifest => "Missing APP.TOM",
            Self::MissingEntry => "Missing MAIN.LUA",
            Self::MissingData => "Missing nested Y2026.TXT",
            Self::ManifestInvalidUtf8 => "APP.TOM UTF-8 error",
            Self::ScriptInvalidUtf8 => "MAIN.LUA UTF-8 error",
            Self::DataInvalidUtf8 => "Y2026.TXT UTF-8 error",
            Self::InvalidManifestContract => "Bad APP.TOM",
        }
    }

    pub const fn is_sd_loaded(self) -> bool {
        matches!(self, Self::SdLuaScript)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LuaPanchangText<const N: usize> {
    bytes: [u8; N],
    len: usize,
}

impl<const N: usize> LuaPanchangText<N> {
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

    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes[..self.len]).unwrap_or("")
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LuaPanchangScreen {
    pub source: LuaPanchangSource,
    pub title: LuaPanchangText<48>,
    pub subtitle: LuaPanchangText<96>,
    pub line1: LuaPanchangText<160>,
    pub line2: LuaPanchangText<160>,
    pub line3: LuaPanchangText<160>,
    pub footer: LuaPanchangText<112>,
}

impl LuaPanchangScreen {
    pub fn fallback() -> Self {
        Self {
            source: LuaPanchangSource::BuiltInFallback,
            title: LuaPanchangText::from_str(TITLE_DEFAULT),
            subtitle: LuaPanchangText::from_str(SUBTITLE_DEFAULT),
            line1: LuaPanchangText::from_str(LINE1_DEFAULT),
            line2: LuaPanchangText::from_str(LINE2_DEFAULT),
            line3: LuaPanchangText::from_str(LINE3_DEFAULT),
            footer: LuaPanchangText::from_str(FOOTER_DEFAULT),
        }
    }

    pub fn missing_manifest() -> Self {
        diagnostic_screen(
            LuaPanchangSource::MissingManifest,
            "Missing /RUSTMIX/APPS/PANCHANG/APP.TOM",
            "Upload APP.TOM with Wi-Fi Transfer",
        )
    }
    pub fn missing_entry() -> Self {
        diagnostic_screen(
            LuaPanchangSource::MissingEntry,
            "Missing /RUSTMIX/APPS/PANCHANG/MAIN.LUA",
            "Upload MAIN.LUA with Wi-Fi Transfer",
        )
    }
    pub fn missing_data() -> Self {
        diagnostic_screen(
            LuaPanchangSource::MissingData,
            "Missing /RUSTMIX/APPS/PANCHANG/DATA/Y2026.TXT",
            "Upload Y2026.TXT with Wi-Fi Transfer",
        )
    }
    pub fn invalid_manifest_utf8() -> Self {
        diagnostic_screen(
            LuaPanchangSource::ManifestInvalidUtf8,
            "APP.TOM is not valid UTF-8",
            "Replace APP.TOM on SD card",
        )
    }
    pub fn invalid_script_utf8() -> Self {
        diagnostic_screen(
            LuaPanchangSource::ScriptInvalidUtf8,
            "MAIN.LUA is not valid UTF-8",
            "Replace MAIN.LUA on SD card",
        )
    }
    pub fn invalid_data_utf8() -> Self {
        diagnostic_screen(
            LuaPanchangSource::DataInvalidUtf8,
            "Y2026.TXT is not valid UTF-8",
            "Replace Y2026.TXT on SD card",
        )
    }
    pub fn invalid_manifest_contract() -> Self {
        diagnostic_screen(
            LuaPanchangSource::InvalidManifestContract,
            "APP.TOM must declare id = \"panchang\"",
            "Fix manifest and reopen app",
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

impl Default for LuaPanchangScreen {
    fn default() -> Self {
        Self::fallback()
    }
}

fn diagnostic_screen(
    source: LuaPanchangSource,
    primary: &str,
    remediation: &str,
) -> LuaPanchangScreen {
    let mut screen = LuaPanchangScreen::fallback();
    screen.source = source;
    screen.subtitle.set("Lua app SD files are incomplete");
    screen.line1.set(primary);
    screen.line2.set(remediation);
    screen.line3.set("Back returns to Productivity");
    screen.footer.set("Canonical root: /RUSTMIX/APPS");
    screen
}

pub fn build_panchang_sd_runtime(manifest: &str, script: &str, data: &str) -> LuaPanchangScreen {
    if !manifest_declares_panchang(manifest) {
        return LuaPanchangScreen::invalid_manifest_contract();
    }

    let mut screen = evaluate_panchang_lua_subset(script);
    screen.source = LuaPanchangSource::SdLuaScript;
    screen.title.set("Panchang");

    let selected_index = panchang_selected_record_index(script).unwrap_or(0);
    if let Some(record) = nth_panchang_record(data, selected_index) {
        set_date_line(&mut screen, record.date);
        set_panchang_lines(&mut screen, record.tithi, record.nakshatra, record.note);
    } else if first_non_empty_line(data).is_some() {
        screen.subtitle.set("Date: Today");
        screen.line1.set("Tithi: No matching record found");
        screen.line2.set("Nakshatra: -");
    } else {
        screen.subtitle.set("Date: Today");
        screen.line1.set("Tithi: DATA/Y2026.TXT is empty");
        screen.line2.set("Nakshatra: -");
    }

    screen
        .footer
        .set("Loaded: APP.TOM + MAIN.LUA + DATA/Y2026.TXT");
    screen
}

fn manifest_declares_panchang(manifest: &str) -> bool {
    manifest.lines().any(|line| {
        let trimmed = line.trim();
        trimmed == "id = \"panchang\"" || trimmed == "id=\"panchang\""
    })
}

pub fn evaluate_panchang_lua_subset(script: &str) -> LuaPanchangScreen {
    let mut screen = LuaPanchangScreen::fallback();
    screen.source = LuaPanchangSource::SdLuaScript;
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

fn panchang_selected_record_index(script: &str) -> Option<usize> {
    #[cfg(feature = "lua-vm")]
    {
        let expr = extract_panchang_vm_record_index(script)?;
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

pub fn extract_panchang_vm_record_index(script: &str) -> Option<&str> {
    for line in script.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(expr) = trimmed.strip_prefix("-- rustmix-vm-panchang:") {
            let expr = expr.trim();
            if !expr.is_empty() {
                return Some(expr);
            }
        }
        if let Some(expr) = parse_vm_assignment(trimmed, "vm_panchang_index_expression") {
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
pub struct PanchangRecord<'a> {
    pub date: &'a str,
    pub tithi: &'a str,
    pub nakshatra: &'a str,
    pub note: &'a str,
}

fn nth_panchang_record(text: &str, index: usize) -> Option<PanchangRecord<'_>> {
    let mut seen = 0usize;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("--") {
            continue;
        }
        if seen == index {
            return parse_panchang_record(line);
        }
        seen += 1;
    }
    None
}

fn parse_panchang_record(line: &str) -> Option<PanchangRecord<'_>> {
    let mut parts = line.splitn(4, '|');
    let date = parts.next()?.trim();
    let tithi = parts.next().unwrap_or("").trim();
    let nakshatra = parts.next().unwrap_or("").trim();
    let note = parts.next().unwrap_or("").trim();
    if date.is_empty() || tithi.is_empty() {
        None
    } else {
        Some(PanchangRecord {
            date,
            tithi,
            nakshatra,
            note,
        })
    }
}

fn first_non_empty_line(text: &str) -> Option<&str> {
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#') && !line.starts_with("--"))
}

fn set_date_line(screen: &mut LuaPanchangScreen, date: &str) {
    screen.subtitle.set("Date: ");
    screen.subtitle.push_str(date);
}

fn set_panchang_lines(screen: &mut LuaPanchangScreen, tithi: &str, nakshatra: &str, note: &str) {
    screen.line1.set("Tithi: ");
    screen.line1.push_str(tithi);
    screen.line2.set("Nakshatra: ");
    screen.line2.push_str(nakshatra);
    if note.trim().is_empty() {
        screen.line3.set("Back returns to Productivity");
    } else {
        let mut combined = LuaPanchangText::<220>::empty();
        combined.push_str("Note: ");
        combined.push_str(note.trim());
        wrap_one_line(combined.as_str(), &mut screen.line3);
    }
}

fn wrap_one_line(text: &str, line: &mut LuaPanchangText<160>) {
    let trimmed = text.trim();
    line.clear();
    if trimmed.chars().count() <= LUA_PANCHANG_WRAP_CHARS {
        line.set(trimmed);
        return;
    }
    let mut split_byte = 0usize;
    let mut char_count = 0usize;
    for (idx, ch) in trimmed.char_indices() {
        if char_count >= LUA_PANCHANG_WRAP_CHARS {
            break;
        }
        split_byte = idx + ch.len_utf8();
        char_count += 1;
    }
    let prefix = &trimmed[..split_byte];
    let split = prefix.rfind(' ').unwrap_or(split_byte).max(1);
    line.set(trimmed[..split].trim_end());
}

fn parse_lua_string_assignment(line: &str) -> Option<(&str, &str)> {
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

    const MANIFEST: &str = r#"id = \"panchang\"
name = \"Panchang\"
category = \"Tools\"
type = \"activity\"
version = \"0.1.0\"
entry = \"MAIN.LUA\"
capabilities = [\"display\", \"input\", \"storage\", \"time\"]
"#;

    #[test]
    fn sd_runtime_uses_precomputed_panchang_data() {
        let screen = build_panchang_sd_runtime(
            MANIFEST,
            "display_title = \"Lua Panchang\"",
            "2026-05-11|Krishna Ashtami|Shravana|Good day for reading and reflection.\n",
        );
        assert_eq!(screen.source, LuaPanchangSource::SdLuaScript);
        assert_eq!(screen.title(), "Panchang");
        assert_eq!(screen.subtitle(), "Date: 2026-05-11");
        assert!(screen.line1().contains("Krishna Ashtami"));
        assert!(screen.line2().contains("Shravana"));
    }

    #[test]
    fn invalid_manifest_produces_diagnostic_screen() {
        let screen = build_panchang_sd_runtime("id = \"bad\"", "", "2026-01-01|Tithi");
        assert_eq!(screen.source, LuaPanchangSource::InvalidManifestContract);
        assert!(screen.line1().contains("id = \"panchang\""));
    }

    #[test]
    fn extracts_vm_panchang_index_expression() {
        assert_eq!(
            extract_panchang_vm_record_index("vm_panchang_index_expression = \"return 2 + 0\""),
            Some("return 2 + 0")
        );
        assert_eq!(
            extract_panchang_vm_record_index("-- rustmix-vm-panchang: return 3"),
            Some("return 3")
        );
    }
}
