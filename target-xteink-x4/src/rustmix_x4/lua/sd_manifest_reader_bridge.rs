//! Feature-gated bridge for reading known Lua app manifest text through a
//! storage abstraction and converting it into Rustmix Lua discovery/catalog
//! models.
//!
//! This is intentionally not a scanner and not an executor. It only reads the
//! known sample app manifest paths under `/RUSTMIX/APPS/`, feeds the text into
//! the accepted `rustmix-core` discovery model, and builds a dashboard catalog
//! model from the resulting registry. It does not touch the raw SD driver,
//! does not execute Lua, and does not wire entries into the visible dashboard.

use super::catalog_bridge::build_lua_dashboard_catalog_from_registry;
use rustmix_core::models::lua_app_catalog::LuaAppDashboardCatalogModel;
use rustmix_core::models::lua_app_discovery::{
    LuaAppDiscoveryInputModel, LuaAppDiscoveryOutcomeModel, discover_lua_apps_from_records,
};
use rustmix_core::models::lua_app_manifest::LUA_APP_MANIFEST_FILE;

/// Stable marker for the feature-gated Lua SD manifest reader bridge probe.
pub const LUA_SD_MANIFEST_READER_BRIDGE_MARKER: &str = "rustmix-lua-sd-manifest-reader-ok";

/// Stable identifier for this model-only bridge slice.
pub const LUA_SD_MANIFEST_READER_BRIDGE_ID: &str = "lua-sd-manifest-reader-bridge-v1";

/// Known logical app ids used by the first non-recursive bridge.
///
/// Physical SD folders are 8.3-safe uppercase paths such as `FLASHCRD`, but
/// discovery records continue to use stable logical app ids. Recursive SD
/// discovery remains a separate future deliverable.
pub const KNOWN_LUA_SAMPLE_APP_FOLDERS: [&str; 4] = [
    "daily_mantra",
    "calendar",
    "panchang",
    "daily_bhagvat_geeta_quote",
];

/// Known manifest paths read by this bridge.
pub const KNOWN_LUA_SAMPLE_MANIFEST_PATHS: [&str; 4] = [
    "/RUSTMIX/APPS/FLASHCRD/APP.TOM",
    "/RUSTMIX/APPS/CALENDAR/APP.TOM",
    "/RUSTMIX/APPS/PANCHANG/APP.TOM",
    "/RUSTMIX/APPS/EXAMPLE/APP.TOM",
];

/// Files expected for the accepted SD-only sample app contract.
///
/// The discovery model uses this list to confirm the manifest entry file exists.
/// This bridge does not scan directories yet, so the fixed sample app contract
/// is represented explicitly.
pub const KNOWN_LUA_SAMPLE_APP_FILES: [&str; 2] = [LUA_APP_MANIFEST_FILE, "main.lua"];

/// Read-only storage abstraction used by the bridge.
///
/// A future runtime slice may implement this trait with the existing Rustmix/Pulp
/// storage path. This contract deliberately does not mention `embedded-sdmmc`,
/// FAT internals, SPI, or any raw SD driver type.
pub trait LuaSdManifestTextSource<'a> {
    /// Returns manifest text for a known absolute manifest path.
    ///
    /// Returning `None` maps to the accepted discovery diagnostic for a missing
    /// manifest. The bridge never writes storage and never opens arbitrary paths.
    fn read_manifest_text(&mut self, absolute_manifest_path: &str) -> Option<&'a str>;
}

/// In-memory manifest source useful for model probes and host-side tests.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StaticLuaSdManifestRecord<'a> {
    pub absolute_manifest_path: &'static str,
    pub manifest_text: &'a str,
}

/// Static manifest source for probes that already have manifest text available.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StaticLuaSdManifestSource<'a> {
    records: &'a [StaticLuaSdManifestRecord<'a>],
}

impl<'a> StaticLuaSdManifestSource<'a> {
    pub const fn new(records: &'a [StaticLuaSdManifestRecord<'a>]) -> Self {
        Self { records }
    }
}

impl<'a> LuaSdManifestTextSource<'a> for StaticLuaSdManifestSource<'a> {
    fn read_manifest_text(&mut self, absolute_manifest_path: &str) -> Option<&'a str> {
        self.records
            .iter()
            .find(|record| record.absolute_manifest_path == absolute_manifest_path)
            .map(|record| record.manifest_text)
    }
}

/// Status of the SD manifest reader bridge probe.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LuaSdManifestReaderBridgeStatus {
    /// Known manifest paths were read through the abstraction and converted into
    /// discovery/catalog models. Discovery diagnostics may still be present for
    /// invalid sample app content.
    ModelReady,
    /// The bridge model detected an internal consistency issue.
    ModelInconsistent,
}

impl LuaSdManifestReaderBridgeStatus {
    pub const fn is_success(self) -> bool {
        matches!(self, Self::ModelReady)
    }
}

/// Model-only report for the SD manifest reader bridge.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LuaSdManifestReaderBridgeReport {
    pub marker: &'static str,
    pub bridge_id: &'static str,
    pub known_manifest_path_count: usize,
    pub manifest_read_count: usize,
    pub registry_app_count: usize,
    pub catalog_visible_app_count: usize,
    pub discovery_diagnostic_count: usize,
    pub status: LuaSdManifestReaderBridgeStatus,
}

impl LuaSdManifestReaderBridgeReport {
    pub const fn is_success(&self) -> bool {
        self.status.is_success()
    }
}

/// Full model outcome returned by the bridge.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LuaSdManifestReaderBridgeOutcome {
    pub discovery: LuaAppDiscoveryOutcomeModel,
    pub catalog: LuaAppDashboardCatalogModel,
    pub report: LuaSdManifestReaderBridgeReport,
}

/// Reads the fixed sample app manifest paths and builds discovery/catalog models.
///
/// This function is side-effect free except for calls into the provided storage
/// abstraction. It does not recursively scan `/RUSTMIX/APPS`, does not execute
/// Lua, and does not wire catalog entries into the dashboard.
pub fn read_known_lua_app_manifests<'a, S>(source: &mut S) -> LuaSdManifestReaderBridgeOutcome
where
    S: LuaSdManifestTextSource<'a>,
{
    let daily_mantra_manifest = source.read_manifest_text(KNOWN_LUA_SAMPLE_MANIFEST_PATHS[0]);
    let calendar_manifest = source.read_manifest_text(KNOWN_LUA_SAMPLE_MANIFEST_PATHS[1]);
    let panchang_manifest = source.read_manifest_text(KNOWN_LUA_SAMPLE_MANIFEST_PATHS[2]);
    let geeta_quote_manifest = source.read_manifest_text(KNOWN_LUA_SAMPLE_MANIFEST_PATHS[3]);

    let records = [
        LuaAppDiscoveryInputModel::new(
            KNOWN_LUA_SAMPLE_APP_FOLDERS[0],
            daily_mantra_manifest,
            &KNOWN_LUA_SAMPLE_APP_FILES,
        ),
        LuaAppDiscoveryInputModel::new(
            KNOWN_LUA_SAMPLE_APP_FOLDERS[1],
            calendar_manifest,
            &KNOWN_LUA_SAMPLE_APP_FILES,
        ),
        LuaAppDiscoveryInputModel::new(
            KNOWN_LUA_SAMPLE_APP_FOLDERS[2],
            panchang_manifest,
            &KNOWN_LUA_SAMPLE_APP_FILES,
        ),
        LuaAppDiscoveryInputModel::new(
            KNOWN_LUA_SAMPLE_APP_FOLDERS[3],
            geeta_quote_manifest,
            &KNOWN_LUA_SAMPLE_APP_FILES,
        ),
    ];

    let discovery = discover_lua_apps_from_records(&records);
    let catalog = build_lua_dashboard_catalog_from_registry(&discovery.registry);
    let manifest_read_count = [
        daily_mantra_manifest,
        calendar_manifest,
        panchang_manifest,
        geeta_quote_manifest,
    ]
    .iter()
    .filter(|manifest| manifest.is_some())
    .count();

    let report = LuaSdManifestReaderBridgeReport {
        marker: LUA_SD_MANIFEST_READER_BRIDGE_MARKER,
        bridge_id: LUA_SD_MANIFEST_READER_BRIDGE_ID,
        known_manifest_path_count: KNOWN_LUA_SAMPLE_MANIFEST_PATHS.len(),
        manifest_read_count,
        registry_app_count: discovery.registry.len(),
        catalog_visible_app_count: catalog
            .entries
            .iter()
            .filter(|entry| entry.is_visible())
            .count(),
        discovery_diagnostic_count: discovery.diagnostics.len(),
        status: if known_sample_manifest_paths_are_canonical() {
            LuaSdManifestReaderBridgeStatus::ModelReady
        } else {
            LuaSdManifestReaderBridgeStatus::ModelInconsistent
        },
    };

    LuaSdManifestReaderBridgeOutcome {
        discovery,
        catalog,
        report,
    }
}

/// Returns a report without any manifest records. This proves the bridge handles
/// missing SD sample manifests through diagnostics rather than panicking.
pub fn describe_empty_lua_sd_manifest_reader_bridge() -> LuaSdManifestReaderBridgeOutcome {
    let records: [StaticLuaSdManifestRecord<'_>; 0] = [];
    let mut source = StaticLuaSdManifestSource::new(&records);
    read_known_lua_app_manifests(&mut source)
}

/// Returns true when known sample paths remain fixed under `/RUSTMIX/APPS/`.
pub const fn known_sample_manifest_paths_are_canonical() -> bool {
    starts_with_rustmix_apps(KNOWN_LUA_SAMPLE_MANIFEST_PATHS[0])
        && starts_with_rustmix_apps(KNOWN_LUA_SAMPLE_MANIFEST_PATHS[1])
        && starts_with_rustmix_apps(KNOWN_LUA_SAMPLE_MANIFEST_PATHS[2])
        && starts_with_rustmix_apps(KNOWN_LUA_SAMPLE_MANIFEST_PATHS[3])
}

const fn starts_with_rustmix_apps(path: &str) -> bool {
    let bytes = path.as_bytes();
    let prefix = b"/RUSTMIX/APPS/";
    if bytes.len() <= prefix.len() {
        return false;
    }

    let mut index = 0;
    while index < prefix.len() {
        if bytes[index] != prefix[index] {
            return false;
        }
        index += 1;
    }
    true
}

/// Returns true when this bridge remains non-executing and model-only.
pub const fn lua_sd_manifest_reader_bridge_is_model_only() -> bool {
    true
}
