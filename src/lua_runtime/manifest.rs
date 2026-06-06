//! FAT 8.3-safe SD manifest model for scriptable applications.

use std::path::{Path, PathBuf};

pub const LUA_APP_MANIFEST_FILE: &str = "APP.TOM";
pub const LUA_APP_DEFAULT_ENTRY_FILE: &str = "MAIN.LUA";
pub const LUA_APP_MANIFEST_MAX_BYTES: usize = 4 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LuaAppKind {
    Game,
    App,
}

impl LuaAppKind {
    #[must_use]
    pub const fn marker(self) -> &'static str {
        match self {
            Self::Game => "game",
            Self::App => "app",
        }
    }

    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "game" => Some(Self::Game),
            "app" => Some(Self::App),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LuaAppManifest {
    pub id: String,
    pub name: String,
    pub kind: LuaAppKind,
    pub entry: String,
    pub version: String,
    pub input: Vec<String>,
}

impl LuaAppManifest {
    pub fn parse(source: &str) -> Result<Self, String> {
        if source.len() > LUA_APP_MANIFEST_MAX_BYTES {
            return Err(format!(
                "manifest exceeds {LUA_APP_MANIFEST_MAX_BYTES}-byte limit"
            ));
        }
        let mut id = None;
        let mut name = None;
        let mut kind = None;
        let mut entry = None;
        let mut version = None;
        let mut input = Vec::new();
        for raw_line in source.lines() {
            let line = raw_line.split('#').next().unwrap_or_default().trim();
            if line.is_empty() {
                continue;
            }
            let (key, value) = line
                .split_once('=')
                .ok_or_else(|| format!("manifest line is missing '=': {line}"))?;
            let key = key.trim();
            let value = value.trim();
            match key {
                "id" => id = Some(parse_quoted(value, key)?),
                "name" => name = Some(parse_quoted(value, key)?),
                "kind" => {
                    let value = parse_quoted(value, key)?;
                    kind = LuaAppKind::parse(&value)
                        .ok_or_else(|| format!("unsupported manifest kind: {value}"))
                        .map(Some)?;
                }
                "entry" => entry = Some(parse_quoted(value, key)?),
                "version" => version = Some(parse_quoted(value, key)?),
                "input" => input = parse_string_array(value, key)?,
                _ => return Err(format!("unsupported manifest key: {key}")),
            }
        }

        let manifest = Self {
            id: id.ok_or_else(|| "manifest id is required".to_string())?,
            name: name.ok_or_else(|| "manifest name is required".to_string())?,
            kind: kind.ok_or_else(|| "manifest kind is required".to_string())?,
            entry: entry.unwrap_or_else(|| LUA_APP_DEFAULT_ENTRY_FILE.to_string()),
            version: version.unwrap_or_else(|| "1.0".to_string()),
            input,
        };
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.id.is_empty() || self.id.len() > 32 {
            return Err("manifest id must contain 1..32 bytes".into());
        }
        if !self
            .id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
        {
            return Err("manifest id must use lowercase ASCII, digits or underscore".into());
        }
        if self.name.is_empty() || self.name.len() > 48 {
            return Err("manifest name must contain 1..48 bytes".into());
        }
        if !is_fat83_safe_file_name(&self.entry) {
            return Err(format!(
                "manifest entry is not FAT 8.3-safe: {}",
                self.entry
            ));
        }
        if self.entry.to_ascii_uppercase() != self.entry {
            return Err("manifest entry must use uppercase FAT 8.3 spelling".into());
        }
        if self.input.len() > 8 {
            return Err("manifest input list exceeds 8 entries".into());
        }
        for input in &self.input {
            if !matches!(input.as_str(), "rotary" | "select" | "back" | "imu") {
                return Err(format!("unsupported manifest input capability: {input}"));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LuaAppEntry {
    pub directory_name: String,
    pub directory: PathBuf,
    pub manifest: LuaAppManifest,
}

impl LuaAppEntry {
    #[must_use]
    pub fn entry_path(&self) -> PathBuf {
        self.directory.join(&self.manifest.entry)
    }
}

#[must_use]
pub fn is_fat83_safe_directory_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 8
        && name == name.to_ascii_uppercase()
        && name.bytes().all(is_fat83_char)
}

#[must_use]
pub fn is_fat83_safe_file_name(name: &str) -> bool {
    let Some((stem, extension)) = name.split_once('.') else {
        return false;
    };
    !stem.is_empty()
        && stem.len() <= 8
        && !extension.is_empty()
        && extension.len() <= 3
        && stem.bytes().all(is_fat83_char)
        && extension.bytes().all(is_fat83_char)
}

fn is_fat83_char(byte: u8) -> bool {
    byte.is_ascii_uppercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-')
}

fn parse_quoted(value: &str, key: &str) -> Result<String, String> {
    let value = value.trim();
    if value.len() < 2 || !value.starts_with('"') || !value.ends_with('"') {
        return Err(format!("manifest {key} must be a quoted string"));
    }
    let inner = &value[1..value.len() - 1];
    if inner.contains('"') || inner.contains('\n') || inner.contains('\r') {
        return Err(format!("manifest {key} contains unsupported characters"));
    }
    Ok(inner.to_string())
}

fn parse_string_array(value: &str, key: &str) -> Result<Vec<String>, String> {
    let value = value.trim();
    if value.len() < 2 || !value.starts_with('[') || !value.ends_with(']') {
        return Err(format!("manifest {key} must be a quoted-string array"));
    }
    let inner = value[1..value.len() - 1].trim();
    if inner.is_empty() {
        return Ok(Vec::new());
    }
    inner
        .split(',')
        .map(|item| parse_quoted(item.trim(), key))
        .collect()
}

pub fn read_manifest(directory: &Path) -> Result<LuaAppManifest, String> {
    let path = directory.join(LUA_APP_MANIFEST_FILE);
    let source =
        std::fs::read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    LuaAppManifest::parse(&source)
}

#[cfg(test)]
mod tests {
    use super::{
        is_fat83_safe_directory_name, is_fat83_safe_file_name, LuaAppKind, LuaAppManifest,
    };

    #[test]
    fn parses_bounded_fat83_manifest() {
        let manifest = LuaAppManifest::parse(
            r#"
                id = "hello_grid"
                name = "Hello Grid"
                kind = "game"
                entry = "MAIN.LUA"
                version = "1.0"
                input = ["rotary", "select", "back"]
            "#,
        )
        .unwrap();
        assert_eq!(manifest.id, "hello_grid");
        assert_eq!(manifest.kind, LuaAppKind::Game);
        assert_eq!(manifest.entry, "MAIN.LUA");
    }

    #[test]
    fn validates_fat83_paths() {
        assert!(is_fat83_safe_directory_name("HGRID"));
        assert!(!is_fat83_safe_directory_name("HelloGridLong"));
        assert!(is_fat83_safe_file_name("MAIN.LUA"));
        assert!(!is_fat83_safe_file_name("main.lua"));
    }
}
