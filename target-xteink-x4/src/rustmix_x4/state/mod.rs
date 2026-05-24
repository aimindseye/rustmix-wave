//! Rustmix X4 state boundary modules.
//!
//! The current implementation adds only the progress-state I/O adapter.
//! If this module already exists in your local tree, merge the `pub mod` line
//! instead of overwriting unrelated local state modules.

pub mod progress_state_io_adapter;

pub mod theme_state_io_adapter;

pub mod metadata_state_io_adapter;

pub mod bookmark_state_io_adapter;

pub mod state_registry_adapter;
