//! Feature-gated bridge from Rustmix Lua app models into the native dashboard catalog shape.
//!
//! This is a model bridge only. It does not scan the SD card, does not execute
//! Lua, and does not wire Lua apps into the visible dashboard. The native app
//! catalog remains authoritative; Lua catalog entries are optional additions
//! produced from already-discovered model records.

use crate::rustmix_x4::apps::app_catalog::{READER_APP, SystemAppDescriptor, SystemAppKind};
use rustmix_core::models::lua_app_catalog::{
    LuaAppDashboardCatalogModel, LuaDashboardCategoryModel, native_apps_are_authoritative,
};
use rustmix_core::models::lua_app_manifest::LuaAppRegistryModel;

/// Stable marker for the feature-gated Lua catalog bridge probe.
pub const LUA_CATALOG_BRIDGE_MARKER: &str = "rustmix-lua-catalog-bridge-ok";

/// Static identifier for this model-only bridge slice.
pub const LUA_CATALOG_BRIDGE_ID: &str = "lua-built-in-catalog-bridge-v1";

/// Target-side snapshot of a native dashboard app.
///
/// This intentionally references native app descriptors instead of replacing
/// them. The visible dashboard continues to use the native catalog until a later
/// explicit wiring slice.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeDashboardCatalogBridgeEntry {
    pub app_id: &'static str,
    pub display_label: &'static str,
    pub category: LuaDashboardCategoryModel,
    pub sort_order: u8,
    pub enabled: bool,
}

impl NativeDashboardCatalogBridgeEntry {
    pub const fn from_system_app(
        app: SystemAppDescriptor,
        category: LuaDashboardCategoryModel,
        sort_order: u8,
    ) -> Self {
        Self {
            app_id: app.id.stable_key(),
            display_label: app.name,
            category,
            sort_order,
            enabled: app.enabled,
        }
    }
}

/// Static native dashboard snapshot used by the bridge probe.
///
/// Keep this list aligned with `apps::app_catalog`; do not make Lua apps part of
/// this list. Lua apps remain optional catalog additions built from registry
/// model records.
pub const BUILTIN_NATIVE_DASHBOARD_CATALOG: [NativeDashboardCatalogBridgeEntry; 1] =
    [NativeDashboardCatalogBridgeEntry::from_system_app(
        READER_APP,
        LuaDashboardCategoryModel::Reader,
        0,
    )];

/// Maps current native app kinds into the dashboard categories understood by the
/// Lua catalog model.
pub const fn dashboard_category_for_native_kind(kind: SystemAppKind) -> LuaDashboardCategoryModel {
    match kind {
        SystemAppKind::Reading => LuaDashboardCategoryModel::Reader,
    }
}

/// Status of the model-only bridge probe.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LuaCatalogBridgeStatus {
    /// The bridge can convert registry records into catalog entries.
    ModelReady,
    /// The bridge detected a model-level consistency issue.
    ModelInconsistent,
}

impl LuaCatalogBridgeStatus {
    pub const fn is_success(self) -> bool {
        matches!(self, Self::ModelReady)
    }
}

/// Compact report returned by the feature-gated catalog bridge probe.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LuaCatalogBridgeReport {
    pub marker: &'static str,
    pub bridge_id: &'static str,
    pub native_app_count: usize,
    pub lua_app_count: usize,
    pub lua_diagnostic_count: usize,
    pub native_authoritative: bool,
    pub status: LuaCatalogBridgeStatus,
}

impl LuaCatalogBridgeReport {
    pub const fn is_success(&self) -> bool {
        self.status.is_success()
    }
}

/// Converts an already-discovered Lua app registry into dashboard catalog model
/// entries without touching the real dashboard.
pub fn build_lua_dashboard_catalog_from_registry(
    registry: &LuaAppRegistryModel,
) -> LuaAppDashboardCatalogModel {
    LuaAppDashboardCatalogModel::from_registry(registry)
}

/// Describes the current built-in/native catalog plus model-derived Lua catalog
/// additions.
///
/// This function is intentionally side-effect free. It does not emit logs by
/// itself, does not scan storage, and does not launch apps.
pub fn describe_lua_catalog_bridge(registry: &LuaAppRegistryModel) -> LuaCatalogBridgeReport {
    let lua_catalog = build_lua_dashboard_catalog_from_registry(registry);
    let native_count = BUILTIN_NATIVE_DASHBOARD_CATALOG
        .iter()
        .filter(|entry| entry.enabled)
        .count();
    let model_consistent = native_apps_are_authoritative()
        && native_count == BUILTIN_NATIVE_DASHBOARD_CATALOG.len()
        && native_snapshot_categories_match_native_kinds();

    LuaCatalogBridgeReport {
        marker: LUA_CATALOG_BRIDGE_MARKER,
        bridge_id: LUA_CATALOG_BRIDGE_ID,
        native_app_count: native_count,
        lua_app_count: lua_catalog
            .entries
            .iter()
            .filter(|entry| entry.is_visible())
            .count(),
        lua_diagnostic_count: lua_catalog.diagnostics.len(),
        native_authoritative: native_apps_are_authoritative(),
        status: if model_consistent {
            LuaCatalogBridgeStatus::ModelReady
        } else {
            LuaCatalogBridgeStatus::ModelInconsistent
        },
    }
}

/// Returns true when the static bridge snapshot maps the known native app kinds
/// to the expected dashboard categories.
pub const fn native_snapshot_categories_match_native_kinds() -> bool {
    category_code(BUILTIN_NATIVE_DASHBOARD_CATALOG[0].category)
        == category_code(dashboard_category_for_native_kind(READER_APP.kind))
}

const fn category_code(category: LuaDashboardCategoryModel) -> u8 {
    match category {
        LuaDashboardCategoryModel::Network => 0,
        LuaDashboardCategoryModel::Productivity => 1,
        LuaDashboardCategoryModel::Games => 2,
        LuaDashboardCategoryModel::Reader => 3,
        LuaDashboardCategoryModel::System => 4,
        LuaDashboardCategoryModel::Tools => 5,
    }
}

/// Returns true when this bridge remains a model-only probe.
pub const fn lua_catalog_bridge_is_model_only() -> bool {
    true
}
