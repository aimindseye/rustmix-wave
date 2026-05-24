//! Unicode script detection used by shared font fallback.
//!
//! This is not a shaping engine. It only classifies text so callers can select
//! a suitable system font or request prepared glyph runs from host tooling.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScriptClass {
    Latin,
    Devanagari,
    Gujarati,
    Common,
    Unknown,
}

impl ScriptClass {
    pub const fn needs_complex_shaping(self) -> bool {
        matches!(self, Self::Devanagari | Self::Gujarati)
    }

    pub const fn system_font_key(self) -> &'static str {
        match self {
            Self::Latin => "system-latin",
            Self::Devanagari => "system-devanagari",
            Self::Gujarati => "system-gujarati",
            Self::Common => "system-latin",
            Self::Unknown => "system-missing-glyph",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScriptRun {
    pub script: ScriptClass,
    pub byte_start: usize,
    pub byte_end: usize,
}

pub const fn classify_char(ch: char) -> ScriptClass {
    let code = ch as u32;

    if is_devanagari_codepoint(code) {
        return ScriptClass::Devanagari;
    }
    if is_gujarati_codepoint(code) {
        return ScriptClass::Gujarati;
    }
    if is_latin_codepoint(code) {
        return ScriptClass::Latin;
    }
    if is_common_codepoint(code) {
        return ScriptClass::Common;
    }

    ScriptClass::Unknown
}

const fn is_latin_codepoint(code: u32) -> bool {
    matches!(
        code,
        0x0041..=0x005A
            | 0x0061..=0x007A
            | 0x00C0..=0x00FF
            | 0x0100..=0x017F
            | 0x0180..=0x024F
    )
}

const fn is_devanagari_codepoint(code: u32) -> bool {
    matches!(
        code,
        0x0900..=0x097F
            | 0x1CD0..=0x1CFF
            | 0xA8E0..=0xA8FF
            | 0x11B00..=0x11B5F
    )
}

const fn is_gujarati_codepoint(code: u32) -> bool {
    matches!(code, 0x0A80..=0x0AFF)
}

const fn is_common_codepoint(code: u32) -> bool {
    matches!(
        code,
        0x0000..=0x0040
            | 0x005B..=0x0060
            | 0x007B..=0x00BF
            | 0x2000..=0x206F
            | 0x20A0..=0x20CF
            | 0x2100..=0x214F
    )
}

pub fn dominant_script(text: &str) -> ScriptClass {
    let mut latin = 0usize;
    let mut devanagari = 0usize;
    let mut gujarati = 0usize;
    let mut unknown = 0usize;

    for ch in text.chars() {
        match classify_char(ch) {
            ScriptClass::Latin => latin += 1,
            ScriptClass::Devanagari => devanagari += 1,
            ScriptClass::Gujarati => gujarati += 1,
            ScriptClass::Unknown => unknown += 1,
            ScriptClass::Common => {}
        }
    }

    if devanagari >= latin && devanagari >= gujarati && devanagari > 0 {
        ScriptClass::Devanagari
    } else if gujarati >= latin && gujarati > 0 {
        ScriptClass::Gujarati
    } else if latin > 0 {
        ScriptClass::Latin
    } else if unknown > 0 {
        ScriptClass::Unknown
    } else {
        ScriptClass::Common
    }
}

pub fn first_script_run(text: &str) -> Option<ScriptRun> {
    let mut run_script = None;
    let mut run_start = 0usize;
    let mut run_end = 0usize;

    for (idx, ch) in text.char_indices() {
        let script = classify_char(ch);
        if script == ScriptClass::Common {
            if run_script.is_some() {
                run_end = idx + ch.len_utf8();
            }
            continue;
        }

        match run_script {
            None => {
                run_script = Some(script);
                run_start = idx;
                run_end = idx + ch.len_utf8();
            }
            Some(current) if current == script => {
                run_end = idx + ch.len_utf8();
            }
            Some(current) => {
                return Some(ScriptRun {
                    script: current,
                    byte_start: run_start,
                    byte_end: run_end,
                });
            }
        }
    }

    run_script.map(|script| ScriptRun {
        script,
        byte_start: run_start,
        byte_end: run_end,
    })
}

#[cfg(test)]
mod tests {
    use super::{ScriptClass, classify_char, dominant_script, first_script_run};

    #[test]
    fn detects_devanagari_script_for_hindi_text() {
        assert_eq!(dominant_script("ॐ नमः शिवाय"), ScriptClass::Devanagari);
    }

    #[test]
    fn detects_devanagari_script_for_sanskrit_text() {
        assert_eq!(dominant_script("धर्मक्षेत्रे कुरुक्षेत्रे"), ScriptClass::Devanagari);
    }

    #[test]
    fn detects_gujarati_script() {
        assert_eq!(dominant_script("નમસ્તે દુનિયા"), ScriptClass::Gujarati);
    }

    #[test]
    fn detects_latin_script_for_english_text() {
        assert_eq!(dominant_script("Om Namah Shivaya"), ScriptClass::Latin);
    }

    #[test]
    fn om_symbol_is_devanagari_for_font_selection() {
        assert_eq!(classify_char('ॐ'), ScriptClass::Devanagari);
    }

    #[test]
    fn mixed_text_returns_first_non_common_run() {
        let run = first_script_run("Rustmix नमस्ते નમસ્તે").unwrap();
        assert_eq!(run.script, ScriptClass::Latin);
        assert_eq!(
            &"Rustmix नमस्ते નમસ્તે"[run.byte_start..run.byte_end],
            "Rustmix "
        );
    }
}
