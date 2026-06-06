//! Native game rendering and refresh policy shared by SD-loaded applications.
//!
//! Scripts submit bounded drawing intent only. Rust owns framebuffer drawing,
//! dirty-region merging and game-level refresh intent. The shared native panel
//! coordinator owns the final e-paper refresh decision and one cleanup counter. The SSD1677 panel is
//! never exposed to removable-storage code.

pub mod canvas;
pub mod dirty_regions;
pub mod minesweeper;
pub mod motion_2048;
pub mod refresh_policy;
pub mod sokoban_tilt;
pub mod sudoku;
pub mod tilt_maze;
