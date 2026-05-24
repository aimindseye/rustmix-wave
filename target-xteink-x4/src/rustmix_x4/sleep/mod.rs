//! Sleep-screen interfaces for Rustmix X4.
//!
//! The reference build keeps sleep behavior neutral and folder-based: every
//! sleep screen is selected from valid BMP files in `/RUSTMIX/SLEEP`.
//! Legacy `/sleep.bmp`, daily-image, text-fallback, and no-redraw modes are
//! intentionally ignored by the first release firmware.

pub mod sleep_screen_mode;
