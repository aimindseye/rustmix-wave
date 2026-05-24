//! SD-loaded Lua tools stub contract for Rustmix.
//!
//! Physical folders remain uppercase 8.3-safe under `/RUSTMIX/APPS`, while
//! logical app ids remain descriptive snake_case in APP.TOM.

pub const LUA_TOOLS_DICTIONARY_UNIT_CONVERTER_STUB_PACK_MARKER: &str =
    "rustmix-lua-tools-dictionary-unit-converter-stub-pack-ok";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LuaToolApp {
    pub folder: &'static str,
    pub logical_id: &'static str,
    pub display_name: &'static str,
    pub detail: &'static str,
    pub data_file: &'static str,
}

pub const LUA_TOOL_APPS: [LuaToolApp; 2] = [
    LuaToolApp {
        folder: "DICT",
        logical_id: "dictionary",
        display_name: "Dictionary",
        detail: "Offline prefix-shard word lookup",
        data_file: "INDEX.TXT",
    },
    LuaToolApp {
        folder: "UNITS",
        logical_id: "unit_converter",
        display_name: "Unit Converter",
        detail: "Offline units helper stub",
        data_file: "UNITS.TXT",
    },
];

pub const fn lua_tool_count() -> usize {
    LUA_TOOL_APPS.len()
}

pub fn lua_tool_app(index: usize) -> &'static LuaToolApp {
    &LUA_TOOL_APPS[if index < LUA_TOOL_APPS.len() {
        index
    } else {
        0
    }]
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LuaToolStubSource {
    SdLuaScript,
    BuiltInFallback,
    MissingManifest,
    MissingEntry,
    ManifestInvalidUtf8,
    ScriptInvalidUtf8,
    InvalidManifestContract,
}

impl LuaToolStubSource {
    pub const fn label(self) -> &'static str {
        match self {
            Self::SdLuaScript => "SD Lua",
            Self::BuiltInFallback => "Fallback",
            Self::MissingManifest => "Missing APP.TOM",
            Self::MissingEntry => "Missing MAIN.LUA",
            Self::ManifestInvalidUtf8 => "APP.TOM UTF-8 error",
            Self::ScriptInvalidUtf8 => "MAIN.LUA UTF-8 error",
            Self::InvalidManifestContract => "Bad APP.TOM",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LuaToolText<const N: usize> {
    bytes: [u8; N],
    len: usize,
}

impl<const N: usize> LuaToolText<N> {
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
pub struct LuaToolStubScreen {
    pub source: LuaToolStubSource,
    pub title: LuaToolText<48>,
    pub line1: LuaToolText<384>,
    pub line2: LuaToolText<384>,
    pub line3: LuaToolText<384>,
    pub footer: LuaToolText<192>,
}

impl LuaToolStubScreen {
    pub fn fallback_for(index: usize) -> Self {
        let app = lua_tool_app(index);
        let mut footer = LuaToolText::from_str("Folder: /RUSTMIX/APPS/");
        footer.push_str(app.folder);
        Self {
            source: LuaToolStubSource::BuiltInFallback,
            title: LuaToolText::from_str(app.display_name),
            line1: LuaToolText::from_str(app.detail),
            line2: LuaToolText::from_str("Upload APP.TOM + MAIN.LUA over Wi-Fi Transfer."),
            line3: LuaToolText::from_str("Back exits safely to Tools."),
            footer,
        }
    }

    pub fn diagnostic(
        index: usize,
        source: LuaToolStubSource,
        primary: &str,
        remediation: &str,
    ) -> Self {
        let app = lua_tool_app(index);
        let mut screen = Self::fallback_for(index);
        screen.source = source;
        screen.title.set(app.display_name);
        screen.line1.set(primary);
        screen.line2.set(remediation);
        screen.line3.set("Back exits safely to Tools.");
        screen.footer.set("Canonical root: /RUSTMIX/APPS");
        screen
    }

    pub fn title(&self) -> &str {
        self.title.as_str()
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

impl Default for LuaToolStubScreen {
    fn default() -> Self {
        Self::fallback_for(0)
    }
}

pub fn build_tool_stub_runtime(index: usize, manifest: &str, script: &str) -> LuaToolStubScreen {
    let app = lua_tool_app(index);
    if !manifest_declares_id(manifest, app.logical_id) {
        return LuaToolStubScreen::diagnostic(
            index,
            LuaToolStubSource::InvalidManifestContract,
            "APP.TOM app id does not match folder",
            "Fix manifest id and reopen app",
        );
    }

    let mut screen = evaluate_tool_stub_lua_subset(index, script);
    screen.source = LuaToolStubSource::SdLuaScript;
    screen.footer.set("Loaded from SD MAIN.LUA");
    screen
}

pub fn build_daily_bhagvat_geeta_quote_runtime(
    index: usize,
    manifest: &str,
    script: &str,
    quotes: &str,
    date_index: Option<usize>,
    date_label: Option<&str>,
) -> LuaToolStubScreen {
    let app = lua_tool_app(index);
    if !manifest_declares_id(manifest, app.logical_id) {
        return LuaToolStubScreen::diagnostic(
            index,
            LuaToolStubSource::InvalidManifestContract,
            "APP.TOM app id does not match folder",
            "Fix manifest id and reopen app",
        );
    }

    let mut screen = evaluate_tool_stub_lua_subset(index, script);
    screen.source = LuaToolStubSource::SdLuaScript;
    let Some(record) = select_quote_record(quotes, date_index.unwrap_or(0)) else {
        screen.line1.set("No quotes found in QUOTES.TXT");
        screen.line2.set("Add rows: number|reference|quote");
        screen.line3.set("Upload updated QUOTES.TXT");
        screen.footer.set("Missing quote data");
        return screen;
    };

    screen.title.set(app.display_name);
    set_wrapped_quote_lines(&mut screen, record.quote);
    screen.line3.set(record.reference);
    screen
        .footer
        .set(date_label.unwrap_or("Date & Time not synced; using first quote"));
    screen
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct GeetaQuoteRecord<'a> {
    reference: &'a str,
    quote: &'a str,
}

fn select_quote_record(quotes: &str, day_index: usize) -> Option<GeetaQuoteRecord<'_>> {
    let total = quotes.lines().filter_map(parse_quote_record).count();
    if total == 0 {
        return None;
    }
    let target = day_index % total;
    quotes.lines().filter_map(parse_quote_record).nth(target)
}

fn parse_quote_record(line: &str) -> Option<GeetaQuoteRecord<'_>> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }
    let mut parts = trimmed.splitn(3, '|');
    let _ordinal = parts.next()?.trim();
    let reference = parts.next()?.trim();
    let quote = parts.next()?.trim();
    if reference.is_empty() || quote.is_empty() {
        return None;
    }
    Some(GeetaQuoteRecord { reference, quote })
}

fn set_wrapped_quote_lines(screen: &mut LuaToolStubScreen, quote: &str) {
    const WRAP_CHARS: usize = 56;
    screen.line1.clear();
    screen.line2.clear();

    let mut line = 0usize;
    let mut line_len = 0usize;
    for word in quote.split_whitespace() {
        if word.is_empty() {
            continue;
        }
        let word_len = word.chars().count();
        let needs_space = line_len > 0;
        let next_len = line_len + word_len + if needs_space { 1 } else { 0 };
        if next_len > WRAP_CHARS && line == 0 {
            line = 1;
            line_len = 0;
        } else if next_len > WRAP_CHARS && line == 1 {
            append_ellipsis(&mut screen.line2);
            return;
        }

        let target = if line == 0 {
            &mut screen.line1
        } else {
            &mut screen.line2
        };
        if line_len > 0 {
            target.push_char(' ');
            line_len += 1;
        }
        target.push_str(word);
        line_len += word_len;
    }

    if screen.line2.as_str().is_empty() {
        screen.line2.set(" ");
    }
}

fn append_ellipsis<const N: usize>(text: &mut LuaToolText<N>) {
    if text.len + 3 <= N {
        text.push_str("...");
    }
}

pub fn evaluate_tool_stub_lua_subset(index: usize, script: &str) -> LuaToolStubScreen {
    let mut screen = LuaToolStubScreen::fallback_for(index);
    for line in script.lines() {
        if let Some((key, value)) = parse_lua_string_assignment(line) {
            match key {
                "display_title" | "title" => screen.title.set(value),
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

fn manifest_declares_id(manifest: &str, expected: &str) -> bool {
    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("id") {
            if let Some((key, value)) = parse_key_value(trimmed) {
                if key == "id" && value == expected {
                    return true;
                }
            }
        }
    }
    false
}

fn parse_key_value(line: &str) -> Option<(&str, &str)> {
    let line = line.split('#').next().unwrap_or("").trim();
    let (key, raw) = line.split_once('=')?;
    let key = key.trim();
    let value = unquote(raw.trim())?;
    Some((key, value))
}

fn parse_lua_string_assignment(line: &str) -> Option<(&str, &str)> {
    parse_key_value(line)
}

fn unquote(value: &str) -> Option<&str> {
    if value.len() < 2 {
        return None;
    }
    let bytes = value.as_bytes();
    let quote = bytes[0];
    if quote != b'\'' && quote != b'\"' {
        return None;
    }
    if bytes[value.len() - 1] != quote {
        return None;
    }
    Some(&value[1..value.len() - 1])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_dictionary_tool_stub_screen_from_manifest_and_script() {
        let manifest = "id = \"dictionary\"\nname = \"Dictionary\"";
        let script = "display_title = \"Dictionary\"\ndisplay_line1 = \"Loaded from SD\"";
        let screen = build_tool_stub_runtime(0, manifest, script);
        assert_eq!(screen.source, LuaToolStubSource::SdLuaScript);
        assert_eq!(screen.title(), "Dictionary");
        assert_eq!(screen.line1(), "Loaded from SD");
    }

    #[test]
    fn rejects_bad_tool_manifest_id() {
        let manifest = "id = \"wrong\"";
        let screen = build_tool_stub_runtime(1, manifest, "display_title = \"Unit Converter\"");
        assert_eq!(screen.source, LuaToolStubSource::InvalidManifestContract);
    }

    #[test]
    fn selects_geeta_quote_by_day_index() {
        let manifest = "id = \"daily_bhagvat_geeta_quote\"";
        let script = "display_title = \"Example Quote\"";
        let quotes = "1|2.47|First quote\n2|4.7|Second quote";
        let screen = build_daily_bhagvat_geeta_quote_runtime(
            2,
            manifest,
            script,
            quotes,
            Some(1),
            Some("Today"),
        );
        assert_eq!(screen.source, LuaToolStubSource::SdLuaScript);
        assert_eq!(screen.line1(), "Second quote");
        assert_eq!(screen.line3(), "4.7");
    }
}
