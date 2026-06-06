//! SD Lua runtime foundation with a Rust-owned native canvas.
//!
//! v0.18.6 preserves the static `ui.*` subset and adds bounded Sudoku,
//! Minesweeper, Tilt Maze, Motion 2048 and Sokoban Tilt bridges. Mutable game
//! state, motion latching and panel ownership remain in Rust; unrestricted Lua
//! VM callbacks stay deferred.

use std::path::PathBuf;

use crate::{
    buttons::ButtonEvent,
    games::{
        canvas::NativeGameCanvas,
        refresh_policy::{GameRefreshPlan, GameRefreshPolicy, RefreshTrigger},
    },
    imu_events::ImuDetectedEvent,
};

pub mod bootstrap;
pub mod catalog;
pub mod event_bridge;
pub mod loader;
pub mod manifest;

use catalog::{LuaAppCatalog, LUA_APPS_DIRECTORY};
use event_bridge::LuaEventBridge;
use loader::open_entry_on_worker;
use manifest::LuaAppEntry;

pub const LUA_CATALOG_PAGE_SIZE: usize = 6;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LuaAppSession {
    pub entry: LuaAppEntry,
    pub source_bytes: usize,
    pub canvas: NativeGameCanvas,
    pub refresh_plan: GameRefreshPlan,
    pub event_bridge: LuaEventBridge,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LuaRuntimeUiState {
    pub catalog: LuaAppCatalog,
    pub selected: usize,
    pub session: Option<LuaAppSession>,
    pub error: Option<String>,
    diagnostics: Vec<String>,
}

impl Default for LuaRuntimeUiState {
    fn default() -> Self {
        Self {
            catalog: LuaAppCatalog::unavailable(LUA_APPS_DIRECTORY, "catalog has not been scanned"),
            selected: 0,
            session: None,
            error: None,
            diagnostics: Vec::new(),
        }
    }
}

impl LuaRuntimeUiState {
    pub fn refresh_catalog(&mut self, mounted: bool) {
        self.catalog = LuaAppCatalog::scan(LUA_APPS_DIRECTORY, mounted);
        self.selected = self
            .selected
            .min(self.catalog.entries.len().saturating_sub(1));
        self.push_diagnostic(format!(
            "rustmix-wave=lua-app-scan status={} root={} apps={} raw={} rejected={} warning={}",
            if self.catalog.is_available() {
                "completed"
            } else {
                "unavailable"
            },
            self.catalog.root.display(),
            self.catalog.entries.len(),
            self.catalog.raw_entries,
            self.catalog.rejected_entries,
            self.catalog.warning.as_deref().unwrap_or("none")
        ));
    }

    pub fn refresh_catalog_from_root(&mut self, root: impl Into<PathBuf>, mounted: bool) {
        self.catalog = LuaAppCatalog::scan(root, mounted);
        self.selected = self
            .selected
            .min(self.catalog.entries.len().saturating_sub(1));
    }

    #[must_use]
    pub fn selected_entry(&self) -> Option<&LuaAppEntry> {
        self.catalog.entries.get(self.selected)
    }

    pub fn apply_catalog_button(&mut self, event: ButtonEvent) -> bool {
        if self.catalog.entries.is_empty() {
            return false;
        }
        match event {
            ButtonEvent::Up => {
                self.selected = self
                    .selected
                    .checked_sub(1)
                    .unwrap_or(self.catalog.entries.len() - 1);
                false
            }
            ButtonEvent::Down => {
                self.selected = (self.selected + 1) % self.catalog.entries.len();
                false
            }
            ButtonEvent::Select => self.open_selected(),
        }
    }

    pub fn open_selected(&mut self) -> bool {
        self.error = None;
        let Some(entry) = self.selected_entry().cloned() else {
            self.error = Some("No SD Lua application is selected".into());
            return false;
        };
        self.push_diagnostic(format!(
            "rustmix-wave=lua-app-open id={} status=starting runtime=bootstrap-static entry={}",
            entry.manifest.id, entry.manifest.entry
        ));
        let entry_id = entry.manifest.id.clone();
        match self.open_entry(entry) {
            Ok(session) => {
                let regions = session.canvas.dirty().regions().len();
                let command_count = session.canvas.commands().len();
                self.push_diagnostic(format!(
                    "rustmix-wave=lua-canvas-frame commands={command_count} dirty-regions={regions} transport=existing-fullscreen-partial"
                ));
                self.push_diagnostic(format!(
                    "rustmix-wave=lua-app-open id={} status=ready runtime=bootstrap-static source-bytes={} commands={command_count}",
                    session.entry.manifest.id, session.source_bytes
                ));
                self.session = Some(session);
                true
            }
            Err(error) => {
                self.push_diagnostic(format!(
                    "rustmix-wave=lua-runtime-error id={} error={}",
                    entry_id,
                    sanitize_marker(&error)
                ));
                self.error = Some(error);
                self.session = None;
                false
            }
        }
    }

    fn open_entry(&mut self, entry: LuaAppEntry) -> Result<LuaAppSession, String> {
        open_entry_on_worker(entry)
    }

    pub fn apply_game_button(&mut self, event: ButtonEvent) -> bool {
        let outcome = {
            let Some(session) = self.session.as_mut() else {
                return false;
            };
            match session
                .event_bridge
                .apply_button(event, &mut session.canvas)
            {
                Ok(Some(result)) => {
                    session.refresh_plan = GameRefreshPolicy::plan(
                        session.canvas.dirty(),
                        RefreshTrigger::ScriptFrame,
                    );
                    Ok(Some((session.entry.manifest.id.clone(), result)))
                }
                Ok(None) => Ok(None),
                Err(error) => Err((session.entry.manifest.id.clone(), error)),
            }
        };
        match outcome {
            Ok(Some((id, result))) => {
                self.push_diagnostic(format!(
                    "rustmix-wave=lua-event-bridge id={id} bridge={} event={} outcome={} row={} column={} mode={} axis={} {} completed={} dirty-regions={} refresh=partial-fullscreen transport=existing-fullscreen-partial",
                    result.bridge_marker(),
                    button_marker(event),
                    result.reason(),
                    result.row() + 1,
                    result.column() + 1,
                    result.mode_marker(),
                    result.axis_marker(),
                    result.detail_marker(),
                    result.completed(),
                    result.dirty_regions_len(),
                ));
                true
            }
            Ok(None) => false,
            Err((id, error)) => {
                self.push_diagnostic(format!(
                    "rustmix-wave=lua-runtime-error id={id} error={}",
                    sanitize_marker(&error)
                ));
                self.error = Some(error);
                false
            }
        }
    }

    pub fn apply_game_boot_short_press(&mut self) -> bool {
        let outcome = {
            let Some(session) = self.session.as_mut() else {
                return false;
            };
            match session
                .event_bridge
                .apply_boot_short_press(&mut session.canvas)
            {
                Ok(Some(result)) => {
                    session.refresh_plan = GameRefreshPolicy::plan(
                        session.canvas.dirty(),
                        RefreshTrigger::ScriptFrame,
                    );
                    Ok(Some((session.entry.manifest.id.clone(), result)))
                }
                Ok(None) => Ok(None),
                Err(error) => Err((session.entry.manifest.id.clone(), error)),
            }
        };
        match outcome {
            Ok(Some((id, result))) => {
                self.push_diagnostic(format!(
                    "rustmix-wave=lua-event-bridge id={id} bridge={} event=boot-short outcome={} row={} column={} mode={} axis={} {} completed={} dirty-regions={} refresh=partial-fullscreen transport=existing-fullscreen-partial",
                    result.bridge_marker(),
                    result.reason(),
                    result.row() + 1,
                    result.column() + 1,
                    result.mode_marker(),
                    result.axis_marker(),
                    result.detail_marker(),
                    result.completed(),
                    result.dirty_regions_len(),
                ));
                true
            }
            Ok(None) => false,
            Err((id, error)) => {
                self.push_diagnostic(format!(
                    "rustmix-wave=lua-runtime-error id={id} error={}",
                    sanitize_marker(&error)
                ));
                self.error = Some(error);
                false
            }
        }
    }

    #[must_use]
    pub fn needs_imu_events(&self) -> bool {
        self.session
            .as_ref()
            .is_some_and(|session| session.event_bridge.needs_imu_events())
    }

    pub fn apply_game_motion_event(&mut self, event: ImuDetectedEvent) -> bool {
        let outcome = {
            let Some(session) = self.session.as_mut() else {
                return false;
            };
            match session
                .event_bridge
                .apply_motion_event(event, &mut session.canvas)
            {
                Ok(Some(result)) => {
                    session.refresh_plan = GameRefreshPolicy::plan(
                        session.canvas.dirty(),
                        RefreshTrigger::ScriptFrame,
                    );
                    Ok(Some((session.entry.manifest.id.clone(), result)))
                }
                Ok(None) => Ok(None),
                Err(error) => Err((session.entry.manifest.id.clone(), error)),
            }
        };
        match outcome {
            Ok(Some((id, result))) => {
                self.push_diagnostic(format!(
                    "rustmix-wave=lua-motion-event-bridge id={id} bridge={} outcome={} row={} column={} mode={} axis={} {} completed={} dirty-regions={} refresh=partial-fullscreen transport=existing-fullscreen-partial",
                    result.bridge_marker(), result.reason(), result.row() + 1, result.column() + 1,
                    result.mode_marker(), result.axis_marker(), result.detail_marker(), result.completed(), result.dirty_regions_len(),
                ));
                true
            }
            Ok(None) => false,
            Err((id, error)) => {
                self.push_diagnostic(format!(
                    "rustmix-wave=lua-runtime-error id={id} error={}",
                    sanitize_marker(&error)
                ));
                self.error = Some(error);
                false
            }
        }
    }

    pub fn close_session(&mut self) {
        if let Some(session) = self.session.take() {
            self.push_diagnostic(format!(
                "rustmix-wave=lua-app-close id={} status=released",
                session.entry.manifest.id
            ));
        }
        self.error = None;
    }

    pub fn take_diagnostics(&mut self) -> Vec<String> {
        core::mem::take(&mut self.diagnostics)
    }

    fn push_diagnostic(&mut self, line: String) {
        self.diagnostics.push(line);
    }
}

fn button_marker(event: ButtonEvent) -> &'static str {
    match event {
        ButtonEvent::Up => "up",
        ButtonEvent::Select => "select",
        ButtonEvent::Down => "down",
    }
}

fn sanitize_marker(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_whitespace() {
                '-'
            } else {
                character
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::buttons::ButtonEvent;

    use super::LuaRuntimeUiState;

    fn temp_directory() -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("rustmix-lua-runtime-{nonce}"))
    }

    #[test]
    fn opens_sd_bootstrap_app_into_native_dirty_canvas() {
        let root = temp_directory();
        let app = root.join("HGRID");
        std::fs::create_dir_all(&app).unwrap();
        std::fs::write(
            app.join("APP.TOM"),
            "id=\"hello_grid\"\nname=\"Hello Grid\"\nkind=\"game\"\nentry=\"MAIN.LUA\"\n",
        )
        .unwrap();
        std::fs::write(
            app.join("MAIN.LUA"),
            "ui.clear()\nui.grid(80, 220, 4, 4, 64, 64)\nui.request_refresh()\n",
        )
        .unwrap();
        let mut runtime = LuaRuntimeUiState::default();
        runtime.refresh_catalog_from_root(&root, true);
        assert!(runtime.apply_catalog_button(ButtonEvent::Select));
        assert!(runtime.session.is_some());
        assert!(!runtime.take_diagnostics().is_empty());
        std::fs::remove_dir_all(root).unwrap();
    }
}
