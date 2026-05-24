//! Placeholder glyph run contracts.
//!
//! The current implementation deliberately does not shape Indic text on-device.
//! It gives all X4 apps a stable interface that can later render prepared
//! shaped runs and compact `.vfnt` glyph atlases.

use super::font_catalog::FontDescriptor;
use super::script::{ScriptClass, dominant_script};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GlyphId(pub u32);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GlyphPosition {
    pub x: i16,
    pub y: i16,
    pub advance_x: i16,
    pub advance_y: i16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GlyphRun<'a> {
    pub script: ScriptClass,
    pub font: FontDescriptor,
    pub source_text: &'a str,
    pub glyph_count: usize,
    pub prepared: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GlyphDrawRequest {
    pub glyph_id: GlyphId,
    pub position: GlyphPosition,
    pub font: FontDescriptor,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GlyphRenderError {
    MissingFont,
    MissingGlyph,
    UnsupportedPreparedRun,
    TargetRejectedDraw,
}

pub trait GlyphRenderer {
    fn draw_glyph(&mut self, request: GlyphDrawRequest) -> Result<(), GlyphRenderError>;
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PlaceholderGlyphRenderer {
    pub accepted_draws: usize,
}

impl GlyphRenderer for PlaceholderGlyphRenderer {
    fn draw_glyph(&mut self, _request: GlyphDrawRequest) -> Result<(), GlyphRenderError> {
        self.accepted_draws += 1;
        Ok(())
    }
}

pub fn shape_placeholder_run<'a>(text: &'a str, font: FontDescriptor) -> GlyphRun<'a> {
    GlyphRun {
        script: dominant_script(text),
        font,
        source_text: text,
        glyph_count: text.chars().count(),
        prepared: false,
    }
}

#[cfg(test)]
mod tests {
    use super::shape_placeholder_run;
    use crate::rustmix_x4::text::{ScriptClass, font_for_script};

    #[test]
    fn placeholder_run_preserves_cluster_order_input() {
        let font = font_for_script(ScriptClass::Devanagari);
        let run = shape_placeholder_run("ॐ नमः शिवाय", font);
        assert_eq!(run.source_text, "ॐ नमः शिवाय");
        assert_eq!(run.script, ScriptClass::Devanagari);
        assert!(!run.prepared);
    }
}
