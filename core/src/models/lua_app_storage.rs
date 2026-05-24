use heapless::String;
use serde::{Deserialize, Serialize};

use super::lua_app_manifest::{
    LUA_APP_ID_MAX, LuaAppCapabilityModel, LuaAppManifestModel, is_valid_lua_app_id,
};

pub const LUA_APP_STORAGE_ROOT: &str = "/RUSTMIX/APPS";
pub const LUA_APP_STATE_ROOT: &str = "/RUSTMIX/APPS/state";
pub const LUA_APP_CACHE_ROOT: &str = "/RUSTMIX/APPS/cache";
pub const LUA_APP_DATA_ROOT: &str = "/RUSTMIX/APPS/data";
pub const LUA_APP_STORAGE_RELATIVE_PATH_MAX: usize = 96;
pub const LUA_APP_STORAGE_ABSOLUTE_PATH_MAX: usize = 192;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LuaAppStorageRootModel {
    State,
    Cache,
    Data,
}

impl LuaAppStorageRootModel {
    pub const fn manifest_value(self) -> &'static str {
        match self {
            Self::State => "state",
            Self::Cache => "cache",
            Self::Data => "data",
        }
    }

    pub const fn base_path(self) -> &'static str {
        match self {
            Self::State => LUA_APP_STATE_ROOT,
            Self::Cache => LUA_APP_CACHE_ROOT,
            Self::Data => LUA_APP_DATA_ROOT,
        }
    }

    pub const fn is_read_only(self) -> bool {
        matches!(self, Self::Data)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LuaAppStorageAccessModel {
    Read,
    Write,
    List,
}

impl LuaAppStorageAccessModel {
    pub const fn manifest_value(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::List => "list",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LuaAppStorageOperationModel {
    ReadAppFile,
    ReadAppState,
    WriteAppState,
    ReadAppCache,
    WriteAppCache,
    ListAppData,
    Unsupported,
}

impl LuaAppStorageOperationModel {
    pub const fn host_api_name(self) -> &'static str {
        match self {
            Self::ReadAppFile => "read_app_file",
            Self::ReadAppState => "read_app_state",
            Self::WriteAppState => "write_app_state",
            Self::ReadAppCache => "read_app_cache",
            Self::WriteAppCache => "write_app_cache",
            Self::ListAppData => "list_app_data",
            Self::Unsupported => "unsupported",
        }
    }

    pub const fn required_capability(self) -> Option<LuaAppCapabilityModel> {
        match self {
            Self::ReadAppFile
            | Self::ReadAppState
            | Self::WriteAppState
            | Self::ReadAppCache
            | Self::WriteAppCache
            | Self::ListAppData => Some(LuaAppCapabilityModel::Storage),
            Self::Unsupported => None,
        }
    }

    pub const fn root(self) -> Option<LuaAppStorageRootModel> {
        match self {
            Self::ReadAppFile | Self::ListAppData => Some(LuaAppStorageRootModel::Data),
            Self::ReadAppState | Self::WriteAppState => Some(LuaAppStorageRootModel::State),
            Self::ReadAppCache | Self::WriteAppCache => Some(LuaAppStorageRootModel::Cache),
            Self::Unsupported => None,
        }
    }

    pub const fn access(self) -> Option<LuaAppStorageAccessModel> {
        match self {
            Self::ReadAppFile | Self::ReadAppState | Self::ReadAppCache => {
                Some(LuaAppStorageAccessModel::Read)
            }
            Self::WriteAppState | Self::WriteAppCache => Some(LuaAppStorageAccessModel::Write),
            Self::ListAppData => Some(LuaAppStorageAccessModel::List),
            Self::Unsupported => None,
        }
    }

    pub fn parse(value: &str) -> Result<Self, LuaAppStorageErrorModel> {
        match value.trim() {
            "read_app_file" => Ok(Self::ReadAppFile),
            "read_app_state" => Ok(Self::ReadAppState),
            "write_app_state" => Ok(Self::WriteAppState),
            "read_app_cache" => Ok(Self::ReadAppCache),
            "write_app_cache" => Ok(Self::WriteAppCache),
            "list_app_data" => Ok(Self::ListAppData),
            _ => Err(LuaAppStorageErrorModel::UnsupportedOperation),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LuaAppStorageErrorModel {
    UnsafePath,
    CapabilityDenied,
    ReadOnlyViolation,
    AppIdMismatch,
    UnsupportedOperation,
}

impl LuaAppStorageErrorModel {
    pub const fn diagnostic(self) -> &'static str {
        match self {
            Self::UnsafePath => "Lua app storage path is unsafe",
            Self::CapabilityDenied => "Lua app manifest does not declare storage capability",
            Self::ReadOnlyViolation => "Lua app attempted to write to a read-only storage root",
            Self::AppIdMismatch => "Lua app storage request app id does not match manifest id",
            Self::UnsupportedOperation => "Lua app storage operation is unsupported",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LuaAppStoragePathModel {
    pub app_id: String<LUA_APP_ID_MAX>,
    pub root: LuaAppStorageRootModel,
    pub access: LuaAppStorageAccessModel,
    pub relative_path: String<LUA_APP_STORAGE_RELATIVE_PATH_MAX>,
    pub absolute_path: String<LUA_APP_STORAGE_ABSOLUTE_PATH_MAX>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LuaAppStorageRequestModel {
    pub app_id: String<LUA_APP_ID_MAX>,
    pub operation: LuaAppStorageOperationModel,
    pub root: LuaAppStorageRootModel,
    pub access: LuaAppStorageAccessModel,
    pub relative_path: String<LUA_APP_STORAGE_RELATIVE_PATH_MAX>,
    pub absolute_path: String<LUA_APP_STORAGE_ABSOLUTE_PATH_MAX>,
    pub required_capability: LuaAppCapabilityModel,
}

pub fn is_safe_lua_app_storage_relative_path(path: &str) -> bool {
    let path = path.trim();
    if path.is_empty()
        || path.len() > LUA_APP_STORAGE_RELATIVE_PATH_MAX
        || path.starts_with('/')
        || path.contains('\\')
        || path.contains("//")
    {
        return false;
    }

    path.split('/').all(|segment| {
        !segment.is_empty()
            && segment != "."
            && segment != ".."
            && !segment.starts_with('.')
            && segment
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' || ch == '.')
    })
}

pub fn lua_app_storage_root_path(
    app_id: &str,
    root: LuaAppStorageRootModel,
) -> Result<String<LUA_APP_STORAGE_ABSOLUTE_PATH_MAX>, LuaAppStorageErrorModel> {
    let app_id = app_id.trim();
    if !is_valid_lua_app_id(app_id) {
        return Err(LuaAppStorageErrorModel::UnsafePath);
    }

    let mut out: String<LUA_APP_STORAGE_ABSOLUTE_PATH_MAX> = String::new();
    out.push_str(root.base_path())
        .map_err(|_| LuaAppStorageErrorModel::UnsafePath)?;
    out.push('/')
        .map_err(|_| LuaAppStorageErrorModel::UnsafePath)?;
    out.push_str(app_id)
        .map_err(|_| LuaAppStorageErrorModel::UnsafePath)?;
    Ok(out)
}

pub fn lua_app_storage_absolute_path(
    app_id: &str,
    root: LuaAppStorageRootModel,
    relative_path: &str,
) -> Result<String<LUA_APP_STORAGE_ABSOLUTE_PATH_MAX>, LuaAppStorageErrorModel> {
    if !is_safe_lua_app_storage_relative_path(relative_path) {
        return Err(LuaAppStorageErrorModel::UnsafePath);
    }

    let mut out = lua_app_storage_root_path(app_id, root)?;
    out.push('/')
        .map_err(|_| LuaAppStorageErrorModel::UnsafePath)?;
    out.push_str(relative_path.trim())
        .map_err(|_| LuaAppStorageErrorModel::UnsafePath)?;
    Ok(out)
}

pub fn validate_lua_app_storage_root_access(
    manifest: &LuaAppManifestModel,
    app_id: &str,
    root: LuaAppStorageRootModel,
    access: LuaAppStorageAccessModel,
    relative_path: &str,
) -> Result<LuaAppStoragePathModel, LuaAppStorageErrorModel> {
    let app_id = app_id.trim();
    if app_id != manifest.id.as_str() {
        return Err(LuaAppStorageErrorModel::AppIdMismatch);
    }
    if !manifest.has_capability(LuaAppCapabilityModel::Storage) {
        return Err(LuaAppStorageErrorModel::CapabilityDenied);
    }
    if root.is_read_only() && access == LuaAppStorageAccessModel::Write {
        return Err(LuaAppStorageErrorModel::ReadOnlyViolation);
    }
    if !is_safe_lua_app_storage_relative_path(relative_path) {
        return Err(LuaAppStorageErrorModel::UnsafePath);
    }

    Ok(LuaAppStoragePathModel {
        app_id: copy_string(app_id)?,
        root,
        access,
        relative_path: copy_string(relative_path.trim())?,
        absolute_path: lua_app_storage_absolute_path(app_id, root, relative_path)?,
    })
}

pub fn validate_lua_app_storage_operation(
    manifest: &LuaAppManifestModel,
    app_id: &str,
    operation: LuaAppStorageOperationModel,
    relative_path: &str,
) -> Result<LuaAppStorageRequestModel, LuaAppStorageErrorModel> {
    let root = operation
        .root()
        .ok_or(LuaAppStorageErrorModel::UnsupportedOperation)?;
    let access = operation
        .access()
        .ok_or(LuaAppStorageErrorModel::UnsupportedOperation)?;
    let required_capability = operation
        .required_capability()
        .ok_or(LuaAppStorageErrorModel::UnsupportedOperation)?;

    let path = validate_lua_app_storage_root_access(manifest, app_id, root, access, relative_path)?;

    Ok(LuaAppStorageRequestModel {
        app_id: path.app_id,
        operation,
        root,
        access,
        relative_path: path.relative_path,
        absolute_path: path.absolute_path,
        required_capability,
    })
}

fn copy_string<const N: usize>(value: &str) -> Result<String<N>, LuaAppStorageErrorModel> {
    let mut out: String<N> = String::new();
    out.push_str(value)
        .map_err(|_| LuaAppStorageErrorModel::UnsafePath)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::super::lua_app_manifest::{
        LuaAppCapabilityModel, LuaAppCategoryModel, LuaAppManifestModel, LuaAppTypeModel,
    };
    use super::*;

    fn app_with_capabilities(capabilities: &[LuaAppCapabilityModel]) -> LuaAppManifestModel {
        LuaAppManifestModel::new(
            "daily_mantra",
            "Daily Mantra",
            LuaAppCategoryModel::Tools,
            LuaAppTypeModel::Activity,
            "0.1.0",
            "main.lua",
            capabilities,
        )
        .expect("test app manifest is valid")
    }

    fn storage_app() -> LuaAppManifestModel {
        app_with_capabilities(&[LuaAppCapabilityModel::Storage])
    }

    #[test]
    fn roots_are_per_app_and_canonical() {
        assert_eq!(
            LuaAppStorageRootModel::State.base_path(),
            LUA_APP_STATE_ROOT
        );
        assert_eq!(
            LuaAppStorageRootModel::Cache.base_path(),
            LUA_APP_CACHE_ROOT
        );
        assert_eq!(LuaAppStorageRootModel::Data.base_path(), LUA_APP_DATA_ROOT);
        assert!(LuaAppStorageRootModel::Data.is_read_only());
        assert_eq!(
            lua_app_storage_root_path("daily_mantra", LuaAppStorageRootModel::State)
                .expect("state root resolves")
                .as_str(),
            "/RUSTMIX/APPS/state/daily_mantra"
        );
    }

    #[test]
    fn validates_safe_relative_paths() {
        assert!(is_safe_lua_app_storage_relative_path("mantras.txt"));
        assert!(is_safe_lua_app_storage_relative_path("2026/may.txt"));
        assert!(is_safe_lua_app_storage_relative_path("cache_01.bin"));
        assert!(!is_safe_lua_app_storage_relative_path(""));
        assert!(!is_safe_lua_app_storage_relative_path("/mantras.txt"));
        assert!(!is_safe_lua_app_storage_relative_path("../settings.txt"));
        assert!(!is_safe_lua_app_storage_relative_path(
            "data/../settings.txt"
        ));
        assert!(!is_safe_lua_app_storage_relative_path(".hidden/file.txt"));
        assert!(!is_safe_lua_app_storage_relative_path("folder/.hidden"));
        assert!(!is_safe_lua_app_storage_relative_path("bad name.txt"));
        assert!(!is_safe_lua_app_storage_relative_path("bad\\name.txt"));
    }

    #[test]
    fn operation_names_map_to_roots_access_and_storage_capability() {
        let read_file =
            LuaAppStorageOperationModel::parse("read_app_file").expect("read_app_file parses");
        assert_eq!(read_file.root(), Some(LuaAppStorageRootModel::Data));
        assert_eq!(read_file.access(), Some(LuaAppStorageAccessModel::Read));
        assert_eq!(
            read_file.required_capability(),
            Some(LuaAppCapabilityModel::Storage)
        );

        let write_state =
            LuaAppStorageOperationModel::parse("write_app_state").expect("write_app_state parses");
        assert_eq!(write_state.root(), Some(LuaAppStorageRootModel::State));
        assert_eq!(write_state.access(), Some(LuaAppStorageAccessModel::Write));

        let write_cache =
            LuaAppStorageOperationModel::parse("write_app_cache").expect("write_app_cache parses");
        assert_eq!(write_cache.root(), Some(LuaAppStorageRootModel::Cache));
        assert_eq!(write_cache.access(), Some(LuaAppStorageAccessModel::Write));

        let list_data =
            LuaAppStorageOperationModel::parse("list_app_data").expect("list_app_data parses");
        assert_eq!(list_data.root(), Some(LuaAppStorageRootModel::Data));
        assert_eq!(list_data.access(), Some(LuaAppStorageAccessModel::List));

        assert_eq!(
            LuaAppStorageOperationModel::parse("delete_app_state"),
            Err(LuaAppStorageErrorModel::UnsupportedOperation)
        );
    }

    #[test]
    fn resolves_storage_operation_to_sandbox_path() {
        let manifest = storage_app();
        let request = validate_lua_app_storage_operation(
            &manifest,
            "daily_mantra",
            LuaAppStorageOperationModel::ReadAppFile,
            "mantras.txt",
        )
        .expect("read_app_file is allowed");

        assert_eq!(request.operation, LuaAppStorageOperationModel::ReadAppFile);
        assert_eq!(request.root, LuaAppStorageRootModel::Data);
        assert_eq!(request.access, LuaAppStorageAccessModel::Read);
        assert_eq!(request.relative_path.as_str(), "mantras.txt");
        assert_eq!(
            request.absolute_path.as_str(),
            "/RUSTMIX/APPS/data/daily_mantra/mantras.txt"
        );
        assert_eq!(request.required_capability, LuaAppCapabilityModel::Storage);
    }

    #[test]
    fn state_and_cache_writes_resolve_to_writable_roots() {
        let manifest = storage_app();
        let state = validate_lua_app_storage_operation(
            &manifest,
            "daily_mantra",
            LuaAppStorageOperationModel::WriteAppState,
            "favorites.txt",
        )
        .expect("write_app_state is allowed");
        assert_eq!(state.root, LuaAppStorageRootModel::State);
        assert_eq!(
            state.absolute_path.as_str(),
            "/RUSTMIX/APPS/state/daily_mantra/favorites.txt"
        );

        let cache = validate_lua_app_storage_operation(
            &manifest,
            "daily_mantra",
            LuaAppStorageOperationModel::WriteAppCache,
            "render/page_001.bin",
        )
        .expect("write_app_cache is allowed");
        assert_eq!(cache.root, LuaAppStorageRootModel::Cache);
        assert_eq!(
            cache.absolute_path.as_str(),
            "/RUSTMIX/APPS/cache/daily_mantra/render/page_001.bin"
        );
    }

    #[test]
    fn list_app_data_is_read_only_data_access() {
        let manifest = storage_app();
        let request = validate_lua_app_storage_operation(
            &manifest,
            "daily_mantra",
            LuaAppStorageOperationModel::ListAppData,
            "2026",
        )
        .expect("list_app_data is allowed");
        assert_eq!(request.root, LuaAppStorageRootModel::Data);
        assert_eq!(request.access, LuaAppStorageAccessModel::List);
        assert_eq!(
            request.absolute_path.as_str(),
            "/RUSTMIX/APPS/data/daily_mantra/2026"
        );
    }

    #[test]
    fn denies_storage_without_capability() {
        let manifest = app_with_capabilities(&[LuaAppCapabilityModel::Display]);
        let err = validate_lua_app_storage_operation(
            &manifest,
            "daily_mantra",
            LuaAppStorageOperationModel::ReadAppState,
            "prefs.txt",
        )
        .expect_err("storage capability is required");
        assert_eq!(err, LuaAppStorageErrorModel::CapabilityDenied);
    }

    #[test]
    fn rejects_app_id_mismatch_and_unsafe_path() {
        let manifest = storage_app();
        assert_eq!(
            validate_lua_app_storage_operation(
                &manifest,
                "calendar",
                LuaAppStorageOperationModel::ReadAppState,
                "prefs.txt",
            ),
            Err(LuaAppStorageErrorModel::AppIdMismatch)
        );
        assert_eq!(
            validate_lua_app_storage_operation(
                &manifest,
                "daily_mantra",
                LuaAppStorageOperationModel::ReadAppState,
                "../prefs.txt",
            ),
            Err(LuaAppStorageErrorModel::UnsafePath)
        );
    }

    #[test]
    fn read_only_data_root_rejects_direct_write_access() {
        let manifest = storage_app();
        let err = validate_lua_app_storage_root_access(
            &manifest,
            "daily_mantra",
            LuaAppStorageRootModel::Data,
            LuaAppStorageAccessModel::Write,
            "mantras.txt",
        )
        .expect_err("data root is read-only");
        assert_eq!(err, LuaAppStorageErrorModel::ReadOnlyViolation);
    }

    #[test]
    fn unsupported_operation_is_diagnostic_specific() {
        let manifest = storage_app();
        let err = validate_lua_app_storage_operation(
            &manifest,
            "daily_mantra",
            LuaAppStorageOperationModel::Unsupported,
            "anything.txt",
        )
        .expect_err("unsupported operation is rejected");
        assert_eq!(err, LuaAppStorageErrorModel::UnsupportedOperation);
        assert_eq!(
            LuaAppStorageErrorModel::UnsupportedOperation.diagnostic(),
            "Lua app storage operation is unsupported"
        );
    }
}
