//! System font catalog and fallback selection for Rustmix X4.
//!
//! The catalog names system fonts by semantic role and script. Actual glyph
//! data can be supplied from firmware assets, SD card `.vfnt` files, or prepared
//! per-book/per-app glyph atlases.

use super::script::ScriptClass;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FontFamily {
    SystemLatin,
    SystemDevanagari,
    SystemGujarati,
    SystemMissingGlyph,
    Custom(&'static str),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FontWeight {
    Regular,
    Medium,
    Bold,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FontStyle {
    Normal,
    Italic,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FontRole {
    Ui,
    Reader,
    SleepScreen,
    Fallback,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FontDescriptor {
    pub key: &'static str,
    pub family: FontFamily,
    pub display_name: &'static str,
    pub script: ScriptClass,
    pub role: FontRole,
    pub asset_path: &'static str,
    pub point_size: u8,
    pub weight: FontWeight,
    pub style: FontStyle,
    pub system_font: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct FontFallbackChain<'a> {
    pub fonts: &'a [FontDescriptor],
}

impl<'a> FontFallbackChain<'a> {
    pub const fn new(fonts: &'a [FontDescriptor]) -> Self {
        Self { fonts }
    }

    pub fn select(self, script: ScriptClass) -> FontDescriptor {
        for font in self.fonts {
            if font.script == script {
                return *font;
            }
        }
        for font in self.fonts {
            if font.script == ScriptClass::Latin {
                return *font;
            }
        }
        SYSTEM_MISSING_GLYPH_FONT
    }
}

pub const SYSTEM_LATIN_FONT: FontDescriptor = FontDescriptor {
    key: "system-latin",
    family: FontFamily::SystemLatin,
    display_name: "Noto Sans",
    script: ScriptClass::Latin,
    role: FontRole::Fallback,
    asset_path: "/fonts/system/NotoSans-Regular.vfnt",
    point_size: 18,
    weight: FontWeight::Regular,
    style: FontStyle::Normal,
    system_font: true,
};

pub const SYSTEM_DEVANAGARI_FONT: FontDescriptor = FontDescriptor {
    key: "system-devanagari",
    family: FontFamily::SystemDevanagari,
    display_name: "Noto Sans Devanagari",
    script: ScriptClass::Devanagari,
    role: FontRole::Fallback,
    asset_path: "/fonts/system/NotoSansDevanagari-Regular.vfnt",
    point_size: 22,
    weight: FontWeight::Regular,
    style: FontStyle::Normal,
    system_font: true,
};

pub const SYSTEM_GUJARATI_FONT: FontDescriptor = FontDescriptor {
    key: "system-gujarati",
    family: FontFamily::SystemGujarati,
    display_name: "Noto Sans Gujarati",
    script: ScriptClass::Gujarati,
    role: FontRole::Fallback,
    asset_path: "/fonts/system/NotoSansGujarati-Regular.vfnt",
    point_size: 22,
    weight: FontWeight::Regular,
    style: FontStyle::Normal,
    system_font: true,
};

pub const SYSTEM_MISSING_GLYPH_FONT: FontDescriptor = FontDescriptor {
    key: "system-missing-glyph",
    family: FontFamily::SystemMissingGlyph,
    display_name: "Rustmix Missing Glyph",
    script: ScriptClass::Unknown,
    role: FontRole::Fallback,
    asset_path: "/fonts/system/MissingGlyph.vfnt",
    point_size: 18,
    weight: FontWeight::Regular,
    style: FontStyle::Normal,
    system_font: true,
};

pub const SYSTEM_FONT_CATALOG: &[FontDescriptor] = &[
    SYSTEM_LATIN_FONT,
    SYSTEM_DEVANAGARI_FONT,
    SYSTEM_GUJARATI_FONT,
    SYSTEM_MISSING_GLYPH_FONT,
];

pub const fn system_fallback_chain() -> FontFallbackChain<'static> {
    FontFallbackChain::new(SYSTEM_FONT_CATALOG)
}

pub fn font_for_script(script: ScriptClass) -> FontDescriptor {
    system_fallback_chain().select(script)
}

#[cfg(test)]
mod tests {
    use super::{FontFamily, font_for_script, system_fallback_chain};
    use crate::rustmix_x4::text::ScriptClass;

    #[test]
    fn selects_devanagari_font_for_hindi() {
        assert_eq!(
            font_for_script(ScriptClass::Devanagari).family,
            FontFamily::SystemDevanagari
        );
    }

    #[test]
    fn selects_gujarati_font_for_gujarati() {
        assert_eq!(
            font_for_script(ScriptClass::Gujarati).family,
            FontFamily::SystemGujarati
        );
    }

    #[test]
    fn fallback_chain_uses_latin_for_common_text() {
        assert_eq!(
            system_fallback_chain().select(ScriptClass::Common).family,
            FontFamily::SystemLatin
        );
    }
}
