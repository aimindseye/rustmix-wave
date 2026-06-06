//! Persistent global UI display preferences.
//!
//! Preferences are loaded from `/sdcard/RUSTMIX/DISPLAY.TXT` at boot. The UI
//! remains usable when the SD card or file is unavailable: Inter + Standard is
//! always the safe default. Changes are persisted best-effort by the runtime.

use std::{fs, path::Path};

use anyhow::{bail, Context, Result};

/// SD-backed global UI typography preference file.
pub const DISPLAY_CONFIG_PATH: &str = "/sdcard/RUSTMIX/DISPLAY.TXT";

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum UiFontFamily {
    #[default]
    Inter,
    AtkinsonHyperlegible,
}

impl UiFontFamily {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Inter => "Inter",
            Self::AtkinsonHyperlegible => "Atkinson Hyperlegible",
        }
    }

    #[must_use]
    pub const fn compact_label(self) -> &'static str {
        match self {
            Self::Inter => "Inter",
            Self::AtkinsonHyperlegible => "Atkinson",
        }
    }

    #[must_use]
    pub const fn marker(self) -> &'static str {
        match self {
            Self::Inter => "inter",
            Self::AtkinsonHyperlegible => "atkinson-hyperlegible",
        }
    }

    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Inter => Self::AtkinsonHyperlegible,
            Self::AtkinsonHyperlegible => Self::Inter,
        }
    }

    fn parse(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "inter" => Ok(Self::Inter),
            "atkinson-hyperlegible" | "atkinson_hyperlegible" | "atkinson" => {
                Ok(Self::AtkinsonHyperlegible)
            }
            other => bail!("unsupported font_family value {other:?}"),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum UiFontSize {
    Compact,
    #[default]
    Standard,
    Large,
}

impl UiFontSize {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Compact => "Compact",
            Self::Standard => "Standard",
            Self::Large => "Large",
        }
    }

    #[must_use]
    pub const fn marker(self) -> &'static str {
        match self {
            Self::Compact => "compact",
            Self::Standard => "standard",
            Self::Large => "large",
        }
    }

    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Compact => Self::Standard,
            Self::Standard => Self::Large,
            Self::Large => Self::Compact,
        }
    }

    fn parse(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "compact" => Ok(Self::Compact),
            "standard" => Ok(Self::Standard),
            "large" => Ok(Self::Large),
            other => bail!("unsupported font_size value {other:?}"),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DisplayPreferences {
    pub font_family: UiFontFamily,
    pub font_size: UiFontSize,
}

impl DisplayPreferences {
    pub fn cycle_font_family(&mut self) {
        self.font_family = self.font_family.next();
    }

    pub fn cycle_font_size(&mut self) {
        self.font_size = self.font_size.next();
    }

    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let text = fs::read_to_string(path)
            .with_context(|| format!("read display config {}", path.display()))?;
        Self::parse(&text)
    }

    pub fn parse(text: &str) -> Result<Self> {
        let mut preferences = Self::default();
        let mut saw_family = false;
        let mut saw_size = false;
        for (line_number, raw_line) in text.lines().enumerate() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (key, value) = line
                .split_once('=')
                .ok_or_else(|| anyhow::anyhow!("line {} must contain '='", line_number + 1))?;
            match key.trim() {
                "font_family" => {
                    if saw_family {
                        bail!("duplicate font_family entry");
                    }
                    preferences.font_family = UiFontFamily::parse(value)?;
                    saw_family = true;
                }
                "font_size" => {
                    if saw_size {
                        bail!("duplicate font_size entry");
                    }
                    preferences.font_size = UiFontSize::parse(value)?;
                    saw_size = true;
                }
                other => bail!("unsupported display config key {other:?}"),
            }
        }
        Ok(preferences)
    }

    pub fn save_to_path(self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        fs::write(path, self.serialized())
            .with_context(|| format!("write display config {}", path.display()))
    }

    #[must_use]
    pub fn serialized(self) -> String {
        format!(
            "# RustMix Wave UI typography\nfont_family={}\nfont_size={}\n",
            self.font_family.marker(),
            self.font_size.marker()
        )
    }

    #[must_use]
    pub const fn persistence_label(self) -> &'static str {
        "SD FILE"
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::{DisplayPreferences, UiFontFamily, UiFontSize};

    #[test]
    fn defaults_to_inter_standard() {
        let preferences = DisplayPreferences::default();
        assert_eq!(preferences.font_family, UiFontFamily::Inter);
        assert_eq!(preferences.font_size, UiFontSize::Standard);
        assert_eq!(preferences.persistence_label(), "SD FILE");
    }

    #[test]
    fn parses_and_serializes_supported_preferences() {
        let parsed =
            DisplayPreferences::parse("font_family=atkinson-hyperlegible\nfont_size=large\n")
                .unwrap();
        assert_eq!(parsed.font_family, UiFontFamily::AtkinsonHyperlegible);
        assert_eq!(parsed.font_size, UiFontSize::Large);
        assert!(parsed
            .serialized()
            .contains("font_family=atkinson-hyperlegible"));
        assert!(parsed.serialized().contains("font_size=large"));
    }

    #[test]
    fn saves_and_loads_display_file() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("rustmix-display-{nanos}.txt"));
        let preferences = DisplayPreferences {
            font_family: UiFontFamily::AtkinsonHyperlegible,
            font_size: UiFontSize::Compact,
        };
        preferences.save_to_path(&path).unwrap();
        assert_eq!(
            DisplayPreferences::load_from_path(&path).unwrap(),
            preferences
        );
        let _ = fs::remove_file(path);
    }

    #[test]
    fn rejects_unknown_keys_and_values() {
        assert!(DisplayPreferences::parse("font_family=comic-sans\n").is_err());
        assert!(DisplayPreferences::parse("font_size=huge\n").is_err());
        assert!(DisplayPreferences::parse("other=value\n").is_err());
    }
}
