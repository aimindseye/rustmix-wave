use heapless::{String, Vec};
use serde::{Deserialize, Serialize};

use super::lua_app_manifest::{
    LUA_APP_MANIFEST_FILE, LUA_APP_PATH_MAX, LUA_APP_REGISTRY_MAX, LUA_APPS_ROOT,
    LuaAppManifestErrorModel, LuaAppRegistryModel, LuaDiscoveredAppModel, parse_lua_app_manifest,
};

pub const LUA_APP_DISCOVERY_DIAGNOSTICS_MAX: usize = LUA_APP_REGISTRY_MAX;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LuaAppDiscoveryDiagnosticKindModel {
    MissingManifest,
    InvalidManifest,
    MissingEntryFile,
    DuplicateAppId,
    UnsafeFolderPath,
    UnsupportedCategory,
    UnsupportedType,
    UnsupportedCapability,
    AppIdFolderMismatch,
    RegistryFull,
}

impl LuaAppDiscoveryDiagnosticKindModel {
    pub const fn diagnostic(self) -> &'static str {
        match self {
            Self::MissingManifest => "app.toml is missing",
            Self::InvalidManifest => "app.toml is invalid",
            Self::MissingEntryFile => "manifest entry file is missing",
            Self::DuplicateAppId => "app id was discovered more than once",
            Self::UnsafeFolderPath => "app folder path is unsafe",
            Self::UnsupportedCategory => "app category is not supported",
            Self::UnsupportedType => "app type is not supported",
            Self::UnsupportedCapability => "app capability is not supported",
            Self::AppIdFolderMismatch => "app id must match the app folder name",
            Self::RegistryFull => "Lua app registry is full",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LuaAppDiscoveryDiagnosticModel {
    pub app_folder: String<LUA_APP_PATH_MAX>,
    pub kind: LuaAppDiscoveryDiagnosticKindModel,
    pub manifest_error: Option<LuaAppManifestErrorModel>,
}

impl LuaAppDiscoveryDiagnosticModel {
    pub fn new(
        app_folder: &str,
        kind: LuaAppDiscoveryDiagnosticKindModel,
        manifest_error: Option<LuaAppManifestErrorModel>,
    ) -> Self {
        Self {
            app_folder: copy_discovery_string(app_folder.trim()).unwrap_or_else(|_| String::new()),
            kind,
            manifest_error,
        }
    }

    pub fn message(&self) -> &'static str {
        self.kind.diagnostic()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LuaAppDiscoveryOutcomeModel {
    pub registry: LuaAppRegistryModel,
    pub diagnostics: Vec<LuaAppDiscoveryDiagnosticModel, LUA_APP_DISCOVERY_DIAGNOSTICS_MAX>,
}

impl LuaAppDiscoveryOutcomeModel {
    pub fn new() -> Self {
        Self {
            registry: LuaAppRegistryModel::new(),
            diagnostics: Vec::new(),
        }
    }

    pub fn has_errors(&self) -> bool {
        !self.diagnostics.is_empty()
    }

    pub fn add_diagnostic(
        &mut self,
        app_folder: &str,
        kind: LuaAppDiscoveryDiagnosticKindModel,
        manifest_error: Option<LuaAppManifestErrorModel>,
    ) {
        let diagnostic = LuaAppDiscoveryDiagnosticModel::new(app_folder, kind, manifest_error);
        let _ = self.diagnostics.push(diagnostic);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LuaAppDiscoveryInputModel<'a> {
    pub app_folder: &'a str,
    pub manifest_text: Option<&'a str>,
    pub files: &'a [&'a str],
}

impl<'a> LuaAppDiscoveryInputModel<'a> {
    pub const fn new(
        app_folder: &'a str,
        manifest_text: Option<&'a str>,
        files: &'a [&'a str],
    ) -> Self {
        Self {
            app_folder,
            manifest_text,
            files,
        }
    }
}

pub fn discover_lua_apps_from_records(
    records: &[LuaAppDiscoveryInputModel<'_>],
) -> LuaAppDiscoveryOutcomeModel {
    let mut outcome = LuaAppDiscoveryOutcomeModel::new();

    for record in records {
        let app_folder = record.app_folder.trim();
        if !is_safe_lua_app_folder_path(app_folder) {
            outcome.add_diagnostic(
                app_folder,
                LuaAppDiscoveryDiagnosticKindModel::UnsafeFolderPath,
                Some(LuaAppManifestErrorModel::InvalidPath),
            );
            continue;
        }

        let Some(manifest_text) = record.manifest_text else {
            outcome.add_diagnostic(
                app_folder,
                LuaAppDiscoveryDiagnosticKindModel::MissingManifest,
                Some(LuaAppManifestErrorModel::MissingField),
            );
            continue;
        };

        let manifest = match parse_lua_app_manifest(manifest_text) {
            Ok(manifest) => manifest,
            Err(error) => {
                outcome.add_diagnostic(
                    app_folder,
                    discovery_kind_for_manifest_error(error),
                    Some(error),
                );
                continue;
            }
        };

        if manifest.id.as_str() != app_folder {
            outcome.add_diagnostic(
                app_folder,
                LuaAppDiscoveryDiagnosticKindModel::AppIdFolderMismatch,
                Some(LuaAppManifestErrorModel::InvalidId),
            );
            continue;
        }

        if !record
            .files
            .iter()
            .any(|file| file.trim() == manifest.entry.as_str())
        {
            outcome.add_diagnostic(
                app_folder,
                LuaAppDiscoveryDiagnosticKindModel::MissingEntryFile,
                Some(LuaAppManifestErrorModel::InvalidEntry),
            );
            continue;
        }

        let Some(app_dir) = lua_app_absolute_dir(app_folder) else {
            outcome.add_diagnostic(
                app_folder,
                LuaAppDiscoveryDiagnosticKindModel::UnsafeFolderPath,
                Some(LuaAppManifestErrorModel::InvalidPath),
            );
            continue;
        };
        let Some(manifest_path) = lua_app_manifest_path(app_folder) else {
            outcome.add_diagnostic(
                app_folder,
                LuaAppDiscoveryDiagnosticKindModel::UnsafeFolderPath,
                Some(LuaAppManifestErrorModel::InvalidPath),
            );
            continue;
        };

        let discovered =
            match LuaDiscoveredAppModel::new(app_dir.as_str(), manifest_path.as_str(), manifest) {
                Ok(discovered) => discovered,
                Err(error) => {
                    outcome.add_diagnostic(
                        app_folder,
                        LuaAppDiscoveryDiagnosticKindModel::UnsafeFolderPath,
                        Some(error),
                    );
                    continue;
                }
            };

        if let Err(error) = outcome.registry.add(discovered) {
            outcome.add_diagnostic(
                app_folder,
                discovery_kind_for_registry_error(error),
                Some(error),
            );
        }
    }

    outcome
}

pub fn is_safe_lua_app_folder_path(path: &str) -> bool {
    let path = path.trim();
    if path.is_empty()
        || path.len() > LUA_APP_PATH_MAX
        || path.starts_with('/')
        || path.contains('/')
        || path.contains('\\')
        || path == "."
        || path == ".."
        || path == "state"
        || path == "cache"
        || path == "lib"
    {
        return false;
    }

    let mut chars = path.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_lowercase() {
        return false;
    }

    chars.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' || ch == '-')
}

pub fn lua_app_absolute_dir(app_folder: &str) -> Option<String<LUA_APP_PATH_MAX>> {
    if !is_safe_lua_app_folder_path(app_folder) {
        return None;
    }

    let mut path: String<LUA_APP_PATH_MAX> = String::new();
    path.push_str(LUA_APPS_ROOT).ok()?;
    path.push('/').ok()?;
    path.push_str(app_folder.trim()).ok()?;
    Some(path)
}

pub fn lua_app_manifest_path(app_folder: &str) -> Option<String<LUA_APP_PATH_MAX>> {
    let mut path = lua_app_absolute_dir(app_folder)?;
    path.push('/').ok()?;
    path.push_str(LUA_APP_MANIFEST_FILE).ok()?;
    Some(path)
}

fn discovery_kind_for_manifest_error(
    error: LuaAppManifestErrorModel,
) -> LuaAppDiscoveryDiagnosticKindModel {
    match error {
        LuaAppManifestErrorModel::InvalidCategory => {
            LuaAppDiscoveryDiagnosticKindModel::UnsupportedCategory
        }
        LuaAppManifestErrorModel::InvalidType => {
            LuaAppDiscoveryDiagnosticKindModel::UnsupportedType
        }
        LuaAppManifestErrorModel::InvalidCapability => {
            LuaAppDiscoveryDiagnosticKindModel::UnsupportedCapability
        }
        _ => LuaAppDiscoveryDiagnosticKindModel::InvalidManifest,
    }
}

fn discovery_kind_for_registry_error(
    error: LuaAppManifestErrorModel,
) -> LuaAppDiscoveryDiagnosticKindModel {
    match error {
        LuaAppManifestErrorModel::DuplicateAppId => {
            LuaAppDiscoveryDiagnosticKindModel::DuplicateAppId
        }
        LuaAppManifestErrorModel::RegistryFull => LuaAppDiscoveryDiagnosticKindModel::RegistryFull,
        LuaAppManifestErrorModel::InvalidCategory => {
            LuaAppDiscoveryDiagnosticKindModel::UnsupportedCategory
        }
        LuaAppManifestErrorModel::InvalidType => {
            LuaAppDiscoveryDiagnosticKindModel::UnsupportedType
        }
        LuaAppManifestErrorModel::InvalidCapability => {
            LuaAppDiscoveryDiagnosticKindModel::UnsupportedCapability
        }
        LuaAppManifestErrorModel::InvalidPath => {
            LuaAppDiscoveryDiagnosticKindModel::UnsafeFolderPath
        }
        _ => LuaAppDiscoveryDiagnosticKindModel::InvalidManifest,
    }
}

fn copy_discovery_string<const N: usize>(value: &str) -> Result<String<N>, ()> {
    let mut out: String<N> = String::new();
    out.push_str(value).map_err(|_| ())?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{LuaAppCapabilityModel, LuaAppCategoryModel};

    const DAILY_MANTRA: &str = r#"
        id = "daily_mantra"
        name = "Daily Mantra"
        category = "Tools"
        type = "activity"
        version = "0.1.0"
        entry = "main.lua"
        capabilities = ["display", "input", "storage", "time"]
    "#;

    const CALENDAR: &str = r#"
        id = "calendar"
        name = "Calendar"
        category = "Productivity"
        type = "activity"
        version = "0.1.0"
        entry = "main.lua"
        capabilities = ["display", "input", "storage", "time"]
    "#;

    fn files() -> &'static [&'static str] {
        &["app.toml", "main.lua"]
    }

    #[test]
    fn discovers_valid_sd_apps_into_registry() {
        let records = [
            LuaAppDiscoveryInputModel::new("daily_mantra", Some(DAILY_MANTRA), files()),
            LuaAppDiscoveryInputModel::new("calendar", Some(CALENDAR), files()),
        ];

        let outcome = discover_lua_apps_from_records(&records);

        assert!(!outcome.has_errors());
        assert_eq!(outcome.registry.len(), 2);
        assert_eq!(
            outcome
                .registry
                .count_for_category(LuaAppCategoryModel::Tools),
            1
        );
        let daily = outcome
            .registry
            .find_by_id("daily_mantra")
            .expect("daily mantra app exists");
        assert_eq!(daily.app_dir.as_str(), "/RUSTMIX/APPS/daily_mantra");
        assert_eq!(
            daily.manifest_path.as_str(),
            "/RUSTMIX/APPS/daily_mantra/app.toml"
        );
        assert!(
            daily
                .manifest
                .has_capability(LuaAppCapabilityModel::Display)
        );
    }

    #[test]
    fn missing_manifest_is_reported_without_registry_entry() {
        let records = [LuaAppDiscoveryInputModel::new(
            "daily_mantra",
            None,
            &["main.lua"],
        )];

        let outcome = discover_lua_apps_from_records(&records);

        assert_eq!(outcome.registry.len(), 0);
        assert_eq!(outcome.diagnostics.len(), 1);
        assert_eq!(
            outcome.diagnostics[0].kind,
            LuaAppDiscoveryDiagnosticKindModel::MissingManifest
        );
    }

    #[test]
    fn unsafe_folder_path_is_reported() {
        let records = [LuaAppDiscoveryInputModel::new(
            "../daily_mantra",
            Some(DAILY_MANTRA),
            files(),
        )];

        let outcome = discover_lua_apps_from_records(&records);

        assert_eq!(outcome.registry.len(), 0);
        assert_eq!(outcome.diagnostics.len(), 1);
        assert_eq!(
            outcome.diagnostics[0].kind,
            LuaAppDiscoveryDiagnosticKindModel::UnsafeFolderPath
        );
        assert!(!is_safe_lua_app_folder_path("state"));
        assert!(!is_safe_lua_app_folder_path("DailyMantra"));
        assert!(is_safe_lua_app_folder_path("daily_mantra"));
    }

    #[test]
    fn invalid_manifest_values_map_to_specific_diagnostics() {
        let unsupported_category = DAILY_MANTRA.replace("Tools", "Fun");
        let unsupported_type = DAILY_MANTRA.replace("activity", "widget");
        let unsupported_capability = DAILY_MANTRA.replace("time", "bluetooth");
        let records = [
            LuaAppDiscoveryInputModel::new(
                "daily_mantra",
                Some(unsupported_category.as_str()),
                files(),
            ),
            LuaAppDiscoveryInputModel::new("calendar", Some(unsupported_type.as_str()), files()),
            LuaAppDiscoveryInputModel::new(
                "panchang",
                Some(unsupported_capability.as_str()),
                files(),
            ),
        ];

        let outcome = discover_lua_apps_from_records(&records);

        assert_eq!(outcome.registry.len(), 0);
        assert_eq!(outcome.diagnostics.len(), 3);
        assert_eq!(
            outcome.diagnostics[0].kind,
            LuaAppDiscoveryDiagnosticKindModel::UnsupportedCategory
        );
        assert_eq!(
            outcome.diagnostics[1].kind,
            LuaAppDiscoveryDiagnosticKindModel::UnsupportedType
        );
        assert_eq!(
            outcome.diagnostics[2].kind,
            LuaAppDiscoveryDiagnosticKindModel::UnsupportedCapability
        );
    }

    #[test]
    fn missing_entry_file_is_reported() {
        let records = [LuaAppDiscoveryInputModel::new(
            "daily_mantra",
            Some(DAILY_MANTRA),
            &["app.toml", "notes.lua"],
        )];

        let outcome = discover_lua_apps_from_records(&records);

        assert_eq!(outcome.registry.len(), 0);
        assert_eq!(outcome.diagnostics.len(), 1);
        assert_eq!(
            outcome.diagnostics[0].kind,
            LuaAppDiscoveryDiagnosticKindModel::MissingEntryFile
        );
    }

    #[test]
    fn duplicate_app_id_is_reported() {
        let records = [
            LuaAppDiscoveryInputModel::new("daily_mantra", Some(DAILY_MANTRA), files()),
            LuaAppDiscoveryInputModel::new("daily_mantra", Some(DAILY_MANTRA), files()),
        ];

        let outcome = discover_lua_apps_from_records(&records);

        assert_eq!(outcome.registry.len(), 1);
        assert_eq!(outcome.diagnostics.len(), 1);
        assert_eq!(
            outcome.diagnostics[0].kind,
            LuaAppDiscoveryDiagnosticKindModel::DuplicateAppId
        );
    }

    #[test]
    fn app_id_must_match_folder_name() {
        let records = [LuaAppDiscoveryInputModel::new(
            "daily_mantra_copy",
            Some(DAILY_MANTRA),
            files(),
        )];

        let outcome = discover_lua_apps_from_records(&records);

        assert_eq!(outcome.registry.len(), 0);
        assert_eq!(outcome.diagnostics.len(), 1);
        assert_eq!(
            outcome.diagnostics[0].kind,
            LuaAppDiscoveryDiagnosticKindModel::AppIdFolderMismatch
        );
    }

    #[test]
    fn path_builders_keep_canonical_apps_root() {
        assert_eq!(
            lua_app_absolute_dir("panchang")
                .expect("folder path")
                .as_str(),
            "/RUSTMIX/APPS/panchang"
        );
        assert_eq!(
            lua_app_manifest_path("panchang")
                .expect("manifest path")
                .as_str(),
            "/RUSTMIX/APPS/panchang/app.toml"
        );
        assert!(lua_app_absolute_dir("../panchang").is_none());
    }
}
