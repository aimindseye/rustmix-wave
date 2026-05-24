pub mod board;
pub mod drivers;
pub mod error;
pub mod kernel;
pub mod ui;
pub mod util;

// Commit 2 sidecar shell.
// Keep this independent from board/runtime wiring for now.
pub mod app;
