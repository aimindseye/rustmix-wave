//! SD-loaded Lua game stub contract for Rustmix.
//!
//! This module adds a first Games-category Lua app pack without adding full game
//! logic yet. Each app follows the accepted deployment contract:
//!
//! - root: `/RUSTMIX/APPS`
//! - physical folders: uppercase 8.3-safe names
//! - manifest: `APP.TOM`
//! - entry script: `MAIN.LUA`
//! - logical app ids stay descriptive snake_case in the manifest

pub const LUA_GAMES_CATALOG_STUB_PACK_MARKER: &str = "rustmix-lua-games-catalog-stub-pack-ok";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LuaGameApp {
    pub folder: &'static str,
    pub logical_id: &'static str,
    pub display_name: &'static str,
    pub detail: &'static str,
    pub data_file: &'static str,
}

pub const LUA_GAME_APPS: [LuaGameApp; 7] = [
    LuaGameApp {
        folder: "SUDOKU",
        logical_id: "sudoku",
        display_name: "Sudoku",
        detail: "9x9 grid puzzle stub",
        data_file: "PUZZLES.TXT",
    },
    LuaGameApp {
        folder: "MINES",
        logical_id: "minesweeper",
        display_name: "Minesweeper",
        detail: "Reveal cells and avoid mines",
        data_file: "BOARD.TXT",
    },
    LuaGameApp {
        folder: "FREECELL",
        logical_id: "freecell",
        display_name: "FreeCell",
        detail: "Open-card solitaire puzzle",
        data_file: "CARDS.TXT",
    },
    LuaGameApp {
        folder: "MEMCARD",
        logical_id: "memory_cards",
        display_name: "Memory Cards",
        detail: "Match hidden card pairs",
        data_file: "CARDS.TXT",
    },
    LuaGameApp {
        folder: "SOLITAIR",
        logical_id: "solitaire",
        display_name: "Solitaire",
        detail: "Klondike-style card stub",
        data_file: "CARDS.TXT",
    },
    LuaGameApp {
        folder: "LUDO",
        logical_id: "ludo",
        display_name: "Ludo",
        detail: "Turn-based board game stub",
        data_file: "BOARD.TXT",
    },
    LuaGameApp {
        folder: "SNAKES",
        logical_id: "snakes_ladder",
        display_name: "Snakes and Ladder",
        detail: "Dice and ladder board stub",
        data_file: "BOARD.TXT",
    },
];

pub const fn lua_game_count() -> usize {
    LUA_GAME_APPS.len()
}

pub fn lua_game_app(index: usize) -> &'static LuaGameApp {
    &LUA_GAME_APPS[if index < LUA_GAME_APPS.len() {
        index
    } else {
        0
    }]
}

pub fn lua_game_title(index: usize) -> &'static str {
    lua_game_app(index).display_name
}

pub fn lua_game_detail(index: usize) -> &'static str {
    lua_game_app(index).detail
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LuaGameStubSource {
    SdLuaScript,
    BuiltInFallback,
    MissingManifest,
    MissingEntry,
    ManifestInvalidUtf8,
    ScriptInvalidUtf8,
    InvalidManifestContract,
}

impl LuaGameStubSource {
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
pub struct LuaGameText<const N: usize> {
    bytes: [u8; N],
    len: usize,
}

impl<const N: usize> LuaGameText<N> {
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
pub struct LuaGameStubScreen {
    pub source: LuaGameStubSource,
    pub title: LuaGameText<48>,
    pub line1: LuaGameText<144>,
    pub line2: LuaGameText<144>,
    pub line3: LuaGameText<144>,
    pub footer: LuaGameText<128>,
}

impl LuaGameStubScreen {
    pub fn fallback_for(index: usize) -> Self {
        let app = lua_game_app(index);
        let mut footer = LuaGameText::from_str("Folder: /RUSTMIX/APPS/");
        footer.push_str(app.folder);
        Self {
            source: LuaGameStubSource::BuiltInFallback,
            title: LuaGameText::from_str(app.display_name),
            line1: LuaGameText::from_str(app.detail),
            line2: LuaGameText::from_str("Upload APP.TOM + MAIN.LUA over Wi-Fi Transfer."),
            line3: LuaGameText::from_str("Use Back to return to Games."),
            footer,
        }
    }

    pub fn diagnostic(
        index: usize,
        source: LuaGameStubSource,
        primary: &str,
        remediation: &str,
    ) -> Self {
        let app = lua_game_app(index);
        let mut screen = Self::fallback_for(index);
        screen.source = source;
        screen.title.set(app.display_name);
        screen.line1.set(primary);
        screen.line2.set(remediation);
        screen.line3.set("Use Back to return to Games.");
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

impl Default for LuaGameStubScreen {
    fn default() -> Self {
        Self::fallback_for(0)
    }
}

pub fn build_game_stub_runtime(index: usize, manifest: &str, script: &str) -> LuaGameStubScreen {
    let app = lua_game_app(index);
    if !manifest_declares_id(manifest, app.logical_id) {
        return LuaGameStubScreen::diagnostic(
            index,
            LuaGameStubSource::InvalidManifestContract,
            "APP.TOM app id does not match folder",
            "Fix manifest id and reopen app",
        );
    }

    let mut screen = evaluate_game_stub_lua_subset(index, script);
    screen.source = LuaGameStubSource::SdLuaScript;
    screen.footer.set("Loaded from SD MAIN.LUA");
    screen
}

pub fn evaluate_game_stub_lua_subset(index: usize, script: &str) -> LuaGameStubScreen {
    let mut screen = LuaGameStubScreen::fallback_for(index);
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
    fn parses_game_stub_screen_from_manifest_and_script() {
        let manifest = "id = \"sudoku\"\nname = \"Sudoku\"";
        let script = "display_title = \"Sudoku\"\ndisplay_line1 = \"Loaded from SD\"";
        let screen = build_game_stub_runtime(0, manifest, script);
        assert_eq!(screen.source, LuaGameStubSource::SdLuaScript);
        assert_eq!(screen.title(), "Sudoku");
        assert_eq!(screen.line1(), "Loaded from SD");
    }

    #[test]
    fn rejects_manifest_id_mismatch() {
        let manifest = "id = \"minesweeper\"";
        let screen = build_game_stub_runtime(0, manifest, "display_title = \"Sudoku\"");
        assert_eq!(screen.source, LuaGameStubSource::InvalidManifestContract);
    }
}
