//! Borrowed script run splitting for font fallback.
//!
//! This splitter does not shape, reorder, or normalize text. Empty and
//! whitespace-only input returns no runs. Neutral characters attach to nearby
//! strong script runs to avoid noisy one-character punctuation runs.

use super::script::{ScriptClass, classify_char};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TextRun<'a> {
    pub script: ScriptClass,
    pub text: &'a str,
}

#[derive(Clone, Copy, Debug)]
pub struct ScriptRunIter<'a> {
    input: &'a str,
    pos: usize,
}

pub fn split_script_runs(input: &str) -> ScriptRunIter<'_> {
    ScriptRunIter { input, pos: 0 }
}

impl<'a> Iterator for ScriptRunIter<'a> {
    type Item = TextRun<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.input.len() {
            return None;
        }

        let run_start = self.pos;
        let mut run_script = None;
        let mut run_end = self.pos;

        for (relative_idx, ch) in self.input[self.pos..].char_indices() {
            let idx = self.pos + relative_idx;
            let next_idx = idx + ch.len_utf8();
            let script = classify_char(ch);

            if script == ScriptClass::Common {
                if run_script.is_some() {
                    run_end = next_idx;
                }
                continue;
            }

            match run_script {
                None => {
                    run_script = Some(script);
                    run_end = next_idx;
                }
                Some(current) if current == script => {
                    run_end = next_idx;
                }
                Some(current) => {
                    self.pos = idx;
                    return Some(TextRun {
                        script: current,
                        text: &self.input[run_start..run_end],
                    });
                }
            }
        }

        match run_script {
            Some(script) => {
                self.pos = self.input.len();
                Some(TextRun {
                    script,
                    text: &self.input[run_start..run_end],
                })
            }
            None => {
                self.pos = self.input.len();
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::split_script_runs;
    use crate::rustmix_x4::text::ScriptClass;

    #[test]
    fn splits_latin_only_text() {
        let mut runs = split_script_runs("Rustmix reader");
        let run = runs.next().unwrap();
        assert_eq!(run.script, ScriptClass::Latin);
        assert_eq!(run.text, "Rustmix reader");
        assert_eq!(runs.next(), None);
    }

    #[test]
    fn detects_devanagari_script_for_hindi_text() {
        let mut runs = split_script_runs("नमस्ते दुनिया");
        let run = runs.next().unwrap();
        assert_eq!(run.script, ScriptClass::Devanagari);
        assert_eq!(run.text, "नमस्ते दुनिया");
        assert_eq!(runs.next(), None);
    }

    #[test]
    fn detects_devanagari_script_for_sanskrit_text() {
        let mut runs = split_script_runs("धर्मक्षेत्रे कुरुक्षेत्रे");
        let run = runs.next().unwrap();
        assert_eq!(run.script, ScriptClass::Devanagari);
        assert_eq!(run.text, "धर्मक्षेत्रे कुरुक्षेत्रे");
        assert_eq!(runs.next(), None);
    }

    #[test]
    fn detects_gujarati_script() {
        let mut runs = split_script_runs("નમસ્તે દુનિયા");
        let run = runs.next().unwrap();
        assert_eq!(run.script, ScriptClass::Gujarati);
        assert_eq!(run.text, "નમસ્તે દુનિયા");
        assert_eq!(runs.next(), None);
    }

    #[test]
    fn splits_mixed_latin_devanagari_gujarati_text() {
        let mut runs = split_script_runs("Rustmix नमस्ते નમસ્તે");
        let latin = runs.next().unwrap();
        let devanagari = runs.next().unwrap();
        let gujarati = runs.next().unwrap();
        assert_eq!(latin.script, ScriptClass::Latin);
        assert_eq!(latin.text, "Rustmix ");
        assert_eq!(devanagari.script, ScriptClass::Devanagari);
        assert_eq!(devanagari.text, "नमस्ते ");
        assert_eq!(gujarati.script, ScriptClass::Gujarati);
        assert_eq!(gujarati.text, "નમસ્તે");
        assert_eq!(runs.next(), None);
    }

    #[test]
    fn keeps_mantra_text_as_stable_script_runs() {
        let mut runs = split_script_runs("ॐ नमः शिवाय - Om Namah Shivaya");
        let devanagari = runs.next().unwrap();
        let latin = runs.next().unwrap();
        assert_eq!(devanagari.script, ScriptClass::Devanagari);
        assert_eq!(devanagari.text, "ॐ नमः शिवाय - ");
        assert_eq!(latin.script, ScriptClass::Latin);
        assert_eq!(latin.text, "Om Namah Shivaya");
        assert_eq!(runs.next(), None);
    }

    #[test]
    fn punctuation_and_space_do_not_create_excessive_runs() {
        let mut runs = split_script_runs("Rustmix, reader: नमस्ते!");
        assert_eq!(runs.next().unwrap().text, "Rustmix, reader: ");
        assert_eq!(runs.next().unwrap().text, "नमस्ते!");
        assert_eq!(runs.next(), None);
    }

    #[test]
    fn empty_and_whitespace_only_text_return_no_runs() {
        assert_eq!(split_script_runs("").count(), 0);
        assert_eq!(split_script_runs("   \n\t").count(), 0);
    }
}
