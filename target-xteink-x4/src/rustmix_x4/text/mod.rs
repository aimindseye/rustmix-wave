//! Shared text and font primitives for Rustmix X4 apps.
//!
//! This module is intentionally small and allocation-free. It provides the
//! contracts needed by reader, home, settings, and sleep-screen apps before a
//! real shaped glyph atlas renderer is wired in.

pub mod font_asset_reader;
pub mod font_assets;
pub mod font_catalog;
pub mod glyph_bitmap_renderer;
pub mod glyph_cache;
pub mod glyph_run;
pub mod glyph_run_renderer;
pub mod layout;
pub mod script;
pub mod text_run;

pub mod sd_font_selection;
pub mod static_font_assets;

pub mod sd_vfn_runtime;
