use heapless::{String, Vec};
use serde::{Deserialize, Serialize};

pub const LUA_APPS_ROOT: &str = "/RUSTMIX/APPS";
pub const LUA_APP_MANIFEST_FILE: &str = "app.toml";
pub const LUA_APP_ID_MAX: usize = 32;
pub const LUA_APP_NAME_MAX: usize = 48;
pub const LUA_APP_VERSION_MAX: usize = 16;
pub const LUA_APP_ENTRY_MAX: usize = 48;
pub const LUA_APP_CAPABILITIES_MAX: usize = 8;
pub const LUA_APP_REGISTRY_MAX: usize = 32;
pub const LUA_APP_PATH_MAX: usize = 128;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LuaAppCategoryModel {
    Network,
    Productivity,
    Games,
    Reader,
    System,
    Tools,
}

impl LuaAppCategoryModel {
    pub const fn manifest_value(self) -> &'static str {
        match self {
            Self::Network => "Network",
            Self::Productivity => "Productivity",
            Self::Games => "Games",
            Self::Reader => "Reader",
            Self::System => "System",
            Self::Tools => "Tools",
        }
    }

    pub const fn dashboard_label(self) -> &'static str {
        self.manifest_value()
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "Network" | "network" => Some(Self::Network),
            "Productivity" | "productivity" => Some(Self::Productivity),
            "Games" | "games" => Some(Self::Games),
            "Reader" | "reader" => Some(Self::Reader),
            "System" | "system" => Some(Self::System),
            "Tools" | "tools" => Some(Self::Tools),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LuaAppTypeModel {
    Activity,
    Service,
    ReaderTool,
}

impl LuaAppTypeModel {
    pub const fn manifest_value(self) -> &'static str {
        match self {
            Self::Activity => "activity",
            Self::Service => "service",
            Self::ReaderTool => "reader-tool",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "activity" => Some(Self::Activity),
            "service" => Some(Self::Service),
            "reader-tool" => Some(Self::ReaderTool),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LuaAppCapabilityModel {
    Display,
    Input,
    Storage,
    Time,
    Network,
    Settings,
}

impl LuaAppCapabilityModel {
    pub const fn manifest_value(self) -> &'static str {
        match self {
            Self::Display => "display",
            Self::Input => "input",
            Self::Storage => "storage",
            Self::Time => "time",
            Self::Network => "network",
            Self::Settings => "settings",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "display" => Some(Self::Display),
            "input" => Some(Self::Input),
            "storage" => Some(Self::Storage),
            "time" => Some(Self::Time),
            "network" => Some(Self::Network),
            "settings" => Some(Self::Settings),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LuaAppManifestErrorModel {
    EmptyManifest,
    MalformedLine,
    MalformedString,
    MalformedArray,
    UnknownKey,
    DuplicateKey,
    MissingField,
    InvalidId,
    InvalidName,
    InvalidCategory,
    InvalidType,
    InvalidVersion,
    InvalidEntry,
    InvalidCapability,
    DuplicateCapability,
    FieldTooLong,
    TooManyCapabilities,
    RegistryFull,
    DuplicateAppId,
    InvalidPath,
}

impl LuaAppManifestErrorModel {
    pub const fn diagnostic(self) -> &'static str {
        match self {
            Self::EmptyManifest => "manifest is empty",
            Self::MalformedLine => "manifest line must use key = value syntax",
            Self::MalformedString => "manifest string value must be quoted",
            Self::MalformedArray => "manifest array value is malformed",
            Self::UnknownKey => "manifest key is not supported",
            Self::DuplicateKey => "manifest key appears more than once",
            Self::MissingField => "manifest is missing a required field",
            Self::InvalidId => "app id is invalid",
            Self::InvalidName => "app name is invalid",
            Self::InvalidCategory => "app category is not allowed",
            Self::InvalidType => "app type is not allowed",
            Self::InvalidVersion => "app version is invalid",
            Self::InvalidEntry => "entry file must be a safe relative .lua path",
            Self::InvalidCapability => "app capability is not allowed",
            Self::DuplicateCapability => "app capability appears more than once",
            Self::FieldTooLong => "manifest field exceeds Rustmix limits",
            Self::TooManyCapabilities => "manifest declares too many capabilities",
            Self::RegistryFull => "Lua app registry is full",
            Self::DuplicateAppId => "Lua app id is already registered",
            Self::InvalidPath => "discovered app path is invalid",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LuaAppManifestModel {
    pub id: String<LUA_APP_ID_MAX>,
    pub name: String<LUA_APP_NAME_MAX>,
    pub category: LuaAppCategoryModel,
    pub app_type: LuaAppTypeModel,
    pub version: String<LUA_APP_VERSION_MAX>,
    pub entry: String<LUA_APP_ENTRY_MAX>,
    pub capabilities: Vec<LuaAppCapabilityModel, LUA_APP_CAPABILITIES_MAX>,
}

impl LuaAppManifestModel {
    pub fn new(
        id: &str,
        name: &str,
        category: LuaAppCategoryModel,
        app_type: LuaAppTypeModel,
        version: &str,
        entry: &str,
        capabilities: &[LuaAppCapabilityModel],
    ) -> Result<Self, LuaAppManifestErrorModel> {
        if !is_valid_lua_app_id(id) {
            return Err(LuaAppManifestErrorModel::InvalidId);
        }
        if name.trim().is_empty() {
            return Err(LuaAppManifestErrorModel::InvalidName);
        }
        if !is_valid_lua_app_version(version) {
            return Err(LuaAppManifestErrorModel::InvalidVersion);
        }
        if !is_safe_lua_entry_path(entry) {
            return Err(LuaAppManifestErrorModel::InvalidEntry);
        }

        let mut capability_vec: Vec<LuaAppCapabilityModel, LUA_APP_CAPABILITIES_MAX> = Vec::new();
        for capability in capabilities {
            if capability_vec.iter().any(|existing| existing == capability) {
                return Err(LuaAppManifestErrorModel::DuplicateCapability);
            }
            capability_vec
                .push(*capability)
                .map_err(|_| LuaAppManifestErrorModel::TooManyCapabilities)?;
        }

        Ok(Self {
            id: copy_string(id)?,
            name: copy_string(name.trim())?,
            category,
            app_type,
            version: copy_string(version.trim())?,
            entry: copy_string(entry.trim())?,
            capabilities: capability_vec,
        })
    }

    pub fn has_capability(&self, capability: LuaAppCapabilityModel) -> bool {
        self.capabilities
            .iter()
            .any(|existing| *existing == capability)
    }

    pub const fn is_sd_loaded_optional_app(&self) -> bool {
        true
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LuaDiscoveredAppModel {
    pub app_dir: String<LUA_APP_PATH_MAX>,
    pub manifest_path: String<LUA_APP_PATH_MAX>,
    pub manifest: LuaAppManifestModel,
}

impl LuaDiscoveredAppModel {
    pub fn new(
        app_dir: &str,
        manifest_path: &str,
        manifest: LuaAppManifestModel,
    ) -> Result<Self, LuaAppManifestErrorModel> {
        if app_dir.trim().is_empty() || manifest_path.trim().is_empty() {
            return Err(LuaAppManifestErrorModel::InvalidPath);
        }
        if has_path_traversal(app_dir) || has_path_traversal(manifest_path) {
            return Err(LuaAppManifestErrorModel::InvalidPath);
        }
        Ok(Self {
            app_dir: copy_string(app_dir.trim())?,
            manifest_path: copy_string(manifest_path.trim())?,
            manifest,
        })
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LuaAppRegistryModel {
    pub apps: Vec<LuaDiscoveredAppModel, LUA_APP_REGISTRY_MAX>,
}

impl LuaAppRegistryModel {
    pub fn new() -> Self {
        Self { apps: Vec::new() }
    }

    pub fn add(&mut self, app: LuaDiscoveredAppModel) -> Result<(), LuaAppManifestErrorModel> {
        if self.find_by_id(app.manifest.id.as_str()).is_some() {
            return Err(LuaAppManifestErrorModel::DuplicateAppId);
        }
        self.apps
            .push(app)
            .map_err(|_| LuaAppManifestErrorModel::RegistryFull)
    }

    pub fn len(&self) -> usize {
        self.apps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.apps.is_empty()
    }

    pub fn find_by_id(&self, id: &str) -> Option<&LuaDiscoveredAppModel> {
        self.apps.iter().find(|app| app.manifest.id.as_str() == id)
    }

    pub fn count_for_category(&self, category: LuaAppCategoryModel) -> usize {
        self.apps
            .iter()
            .filter(|app| app.manifest.category == category)
            .count()
    }
}

#[derive(Default)]
struct ManifestBuilder {
    id: Option<String<LUA_APP_ID_MAX>>,
    name: Option<String<LUA_APP_NAME_MAX>>,
    category: Option<LuaAppCategoryModel>,
    app_type: Option<LuaAppTypeModel>,
    version: Option<String<LUA_APP_VERSION_MAX>>,
    entry: Option<String<LUA_APP_ENTRY_MAX>>,
    capabilities: Option<Vec<LuaAppCapabilityModel, LUA_APP_CAPABILITIES_MAX>>,
}

impl ManifestBuilder {
    fn build(self) -> Result<LuaAppManifestModel, LuaAppManifestErrorModel> {
        let id = self.id.ok_or(LuaAppManifestErrorModel::MissingField)?;
        let name = self.name.ok_or(LuaAppManifestErrorModel::MissingField)?;
        let category = self
            .category
            .ok_or(LuaAppManifestErrorModel::MissingField)?;
        let app_type = self
            .app_type
            .ok_or(LuaAppManifestErrorModel::MissingField)?;
        let version = self.version.ok_or(LuaAppManifestErrorModel::MissingField)?;
        let entry = self.entry.ok_or(LuaAppManifestErrorModel::MissingField)?;
        let capabilities = self
            .capabilities
            .ok_or(LuaAppManifestErrorModel::MissingField)?;

        LuaAppManifestModel::new(
            id.as_str(),
            name.as_str(),
            category,
            app_type,
            version.as_str(),
            entry.as_str(),
            capabilities.as_slice(),
        )
    }
}

pub fn parse_lua_app_manifest(
    input: &str,
) -> Result<LuaAppManifestModel, LuaAppManifestErrorModel> {
    if input.trim().is_empty() {
        return Err(LuaAppManifestErrorModel::EmptyManifest);
    }

    let mut builder = ManifestBuilder::default();
    let mut saw_field = false;

    for raw_line in input.lines() {
        let line = strip_manifest_comment(raw_line).trim();
        if line.is_empty() {
            continue;
        }
        saw_field = true;

        let (key, raw_value) = line
            .split_once('=')
            .ok_or(LuaAppManifestErrorModel::MalformedLine)?;
        let key = key.trim();
        let raw_value = raw_value.trim();

        match key {
            "id" => {
                if builder.id.is_some() {
                    return Err(LuaAppManifestErrorModel::DuplicateKey);
                }
                let value = parse_manifest_string(raw_value)?;
                if !is_valid_lua_app_id(value) {
                    return Err(LuaAppManifestErrorModel::InvalidId);
                }
                builder.id = Some(copy_string(value)?);
            }
            "name" => {
                if builder.name.is_some() {
                    return Err(LuaAppManifestErrorModel::DuplicateKey);
                }
                let value = parse_manifest_string(raw_value)?;
                if value.trim().is_empty() {
                    return Err(LuaAppManifestErrorModel::InvalidName);
                }
                builder.name = Some(copy_string(value.trim())?);
            }
            "category" => {
                if builder.category.is_some() {
                    return Err(LuaAppManifestErrorModel::DuplicateKey);
                }
                let value = parse_manifest_string(raw_value)?;
                builder.category = Some(
                    LuaAppCategoryModel::parse(value)
                        .ok_or(LuaAppManifestErrorModel::InvalidCategory)?,
                );
            }
            "type" => {
                if builder.app_type.is_some() {
                    return Err(LuaAppManifestErrorModel::DuplicateKey);
                }
                let value = parse_manifest_string(raw_value)?;
                builder.app_type = Some(
                    LuaAppTypeModel::parse(value).ok_or(LuaAppManifestErrorModel::InvalidType)?,
                );
            }
            "version" => {
                if builder.version.is_some() {
                    return Err(LuaAppManifestErrorModel::DuplicateKey);
                }
                let value = parse_manifest_string(raw_value)?;
                if !is_valid_lua_app_version(value) {
                    return Err(LuaAppManifestErrorModel::InvalidVersion);
                }
                builder.version = Some(copy_string(value.trim())?);
            }
            "entry" => {
                if builder.entry.is_some() {
                    return Err(LuaAppManifestErrorModel::DuplicateKey);
                }
                let value = parse_manifest_string(raw_value)?;
                if !is_safe_lua_entry_path(value) {
                    return Err(LuaAppManifestErrorModel::InvalidEntry);
                }
                builder.entry = Some(copy_string(value.trim())?);
            }
            "capabilities" => {
                if builder.capabilities.is_some() {
                    return Err(LuaAppManifestErrorModel::DuplicateKey);
                }
                builder.capabilities = Some(parse_capability_array(raw_value)?);
            }
            _ => return Err(LuaAppManifestErrorModel::UnknownKey),
        }
    }

    if !saw_field {
        return Err(LuaAppManifestErrorModel::EmptyManifest);
    }

    builder.build()
}

fn parse_capability_array(
    raw_value: &str,
) -> Result<Vec<LuaAppCapabilityModel, LUA_APP_CAPABILITIES_MAX>, LuaAppManifestErrorModel> {
    let value = raw_value.trim();
    if !value.starts_with('[') || !value.ends_with(']') {
        return Err(LuaAppManifestErrorModel::MalformedArray);
    }

    let inner = value[1..value.len() - 1].trim();
    let mut capabilities: Vec<LuaAppCapabilityModel, LUA_APP_CAPABILITIES_MAX> = Vec::new();
    if inner.is_empty() {
        return Ok(capabilities);
    }

    for item in inner.split(',') {
        let item_value = parse_manifest_string(item.trim())?;
        let capability = LuaAppCapabilityModel::parse(item_value)
            .ok_or(LuaAppManifestErrorModel::InvalidCapability)?;
        if capabilities.iter().any(|existing| *existing == capability) {
            return Err(LuaAppManifestErrorModel::DuplicateCapability);
        }
        capabilities
            .push(capability)
            .map_err(|_| LuaAppManifestErrorModel::TooManyCapabilities)?;
    }

    Ok(capabilities)
}

fn parse_manifest_string(raw_value: &str) -> Result<&str, LuaAppManifestErrorModel> {
    let value = raw_value.trim();
    if value.len() < 2 || !value.starts_with('"') || !value.ends_with('"') {
        return Err(LuaAppManifestErrorModel::MalformedString);
    }
    let unquoted = &value[1..value.len() - 1];
    if unquoted.contains('"') {
        return Err(LuaAppManifestErrorModel::MalformedString);
    }
    Ok(unquoted)
}

fn strip_manifest_comment(line: &str) -> &str {
    let mut in_quotes = false;
    for (idx, ch) in line.char_indices() {
        if ch == '"' {
            in_quotes = !in_quotes;
        }
        if ch == '#' && !in_quotes {
            return &line[..idx];
        }
    }
    line
}

pub fn is_valid_lua_app_id(id: &str) -> bool {
    let id = id.trim();
    if id.is_empty() || id.len() > LUA_APP_ID_MAX {
        return false;
    }

    let mut chars = id.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
        return false;
    }

    chars.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' || ch == '-')
}

pub fn is_valid_lua_app_version(version: &str) -> bool {
    let version = version.trim();
    !version.is_empty()
        && version.len() <= LUA_APP_VERSION_MAX
        && version
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '.' || ch == '-' || ch == '+')
}

pub fn is_safe_lua_entry_path(entry: &str) -> bool {
    let entry = entry.trim();
    if entry.is_empty()
        || entry.len() > LUA_APP_ENTRY_MAX
        || entry.starts_with('/')
        || entry.contains('\\')
        || entry.contains("//")
        || !entry.ends_with(".lua")
        || has_path_traversal(entry)
    {
        return false;
    }

    entry.split('/').all(|segment| {
        !segment.is_empty()
            && segment != "."
            && segment != ".."
            && segment
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' || ch == '.')
    })
}

fn has_path_traversal(path: &str) -> bool {
    path.split('/').any(|segment| segment == "..")
}

fn copy_string<const N: usize>(value: &str) -> Result<String<N>, LuaAppManifestErrorModel> {
    let mut out: String<N> = String::new();
    out.push_str(value)
        .map_err(|_| LuaAppManifestErrorModel::FieldTooLong)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_LUA_MANIFEST: &str = r#"
        # Rustmix SD-loaded optional app
        id = "daily_notes"
        name = "Daily Notes"
        category = "Tools"
        type = "activity"
        version = "0.1.0"
        entry = "main.lua"
        capabilities = ["display", "input", "storage", "time"]
    "#;

    fn sample_manifest(id: &str, category: LuaAppCategoryModel) -> LuaAppManifestModel {
        LuaAppManifestModel::new(
            id,
            "Sample App",
            category,
            LuaAppTypeModel::Activity,
            "0.1.0",
            "main.lua",
            &[LuaAppCapabilityModel::Display, LuaAppCapabilityModel::Input],
        )
        .expect("sample manifest is valid")
    }

    #[test]
    fn parses_daily_notes_manifest() {
        let manifest = parse_lua_app_manifest(SAMPLE_LUA_MANIFEST).expect("manifest parses");
        assert_eq!(manifest.id.as_str(), "daily_notes");
        assert_eq!(manifest.name.as_str(), "Daily Notes");
        assert_eq!(manifest.category, LuaAppCategoryModel::Tools);
        assert_eq!(manifest.app_type, LuaAppTypeModel::Activity);
        assert_eq!(manifest.version.as_str(), "0.1.0");
        assert_eq!(manifest.entry.as_str(), "main.lua");
        assert!(manifest.has_capability(LuaAppCapabilityModel::Display));
        assert!(manifest.has_capability(LuaAppCapabilityModel::Time));
        assert!(!manifest.has_capability(LuaAppCapabilityModel::Network));
        assert!(manifest.is_sd_loaded_optional_app());
    }

    #[test]
    fn parses_allowed_categories_app_types_and_capabilities() {
        assert_eq!(
            LuaAppCategoryModel::parse("Network"),
            Some(LuaAppCategoryModel::Network)
        );
        assert_eq!(
            LuaAppCategoryModel::parse("Productivity"),
            Some(LuaAppCategoryModel::Productivity)
        );
        assert_eq!(
            LuaAppCategoryModel::parse("Games"),
            Some(LuaAppCategoryModel::Games)
        );
        assert_eq!(
            LuaAppCategoryModel::parse("Reader"),
            Some(LuaAppCategoryModel::Reader)
        );
        assert_eq!(
            LuaAppCategoryModel::parse("System"),
            Some(LuaAppCategoryModel::System)
        );
        assert_eq!(
            LuaAppCategoryModel::parse("Tools"),
            Some(LuaAppCategoryModel::Tools)
        );
        assert_eq!(
            LuaAppTypeModel::parse("activity"),
            Some(LuaAppTypeModel::Activity)
        );
        assert_eq!(
            LuaAppTypeModel::parse("service"),
            Some(LuaAppTypeModel::Service)
        );
        assert_eq!(
            LuaAppTypeModel::parse("reader-tool"),
            Some(LuaAppTypeModel::ReaderTool)
        );
        assert_eq!(
            LuaAppCapabilityModel::parse("settings"),
            Some(LuaAppCapabilityModel::Settings)
        );
    }

    #[test]
    fn rejects_unknown_category_type_and_capability() {
        let invalid_category = SAMPLE_LUA_MANIFEST.replace("Tools", "Fun");
        assert_eq!(
            parse_lua_app_manifest(invalid_category.as_str()).unwrap_err(),
            LuaAppManifestErrorModel::InvalidCategory
        );

        let invalid_type = SAMPLE_LUA_MANIFEST.replace("activity", "widget");
        assert_eq!(
            parse_lua_app_manifest(invalid_type.as_str()).unwrap_err(),
            LuaAppManifestErrorModel::InvalidType
        );

        let invalid_capability = SAMPLE_LUA_MANIFEST.replace("time", "bluetooth");
        assert_eq!(
            parse_lua_app_manifest(invalid_capability.as_str()).unwrap_err(),
            LuaAppManifestErrorModel::InvalidCapability
        );
    }

    #[test]
    fn rejects_duplicate_capability_and_duplicate_key() {
        let duplicate_capability = SAMPLE_LUA_MANIFEST.replace(
            "\"display\", \"input\", \"storage\", \"time\"",
            "\"display\", \"input\", \"display\"",
        );
        assert_eq!(
            parse_lua_app_manifest(duplicate_capability.as_str()).unwrap_err(),
            LuaAppManifestErrorModel::DuplicateCapability
        );

        let duplicate_key = r#"
            id = "daily_notes"
            id = "daily_notes_2"
            name = "Daily Notes"
            category = "Tools"
            type = "activity"
            version = "0.1.0"
            entry = "main.lua"
            capabilities = ["display"]
        "#;
        assert_eq!(
            parse_lua_app_manifest(duplicate_key).unwrap_err(),
            LuaAppManifestErrorModel::DuplicateKey
        );
    }

    #[test]
    fn rejects_missing_required_field() {
        let missing_capabilities = r#"
            id = "daily_notes"
            name = "Daily Notes"
            category = "Tools"
            type = "activity"
            version = "0.1.0"
            entry = "main.lua"
        "#;
        assert_eq!(
            parse_lua_app_manifest(missing_capabilities).unwrap_err(),
            LuaAppManifestErrorModel::MissingField
        );
    }

    #[test]
    fn validates_app_id_version_and_entry_path() {
        assert!(is_valid_lua_app_id("daily_notes"));
        assert!(is_valid_lua_app_id("reader-tool"));
        assert!(!is_valid_lua_app_id("DailyMantra"));
        assert!(!is_valid_lua_app_id("_daily_notes"));

        assert!(is_valid_lua_app_version("0.1.0"));
        assert!(is_valid_lua_app_version("1.0.0-alpha+1"));
        assert!(!is_valid_lua_app_version(""));
        assert!(!is_valid_lua_app_version("0.1 beta"));

        assert!(is_safe_lua_entry_path("main.lua"));
        assert!(is_safe_lua_entry_path("src/main.lua"));
        assert!(!is_safe_lua_entry_path("/main.lua"));
        assert!(!is_safe_lua_entry_path("../main.lua"));
        assert!(!is_safe_lua_entry_path("main.txt"));
    }

    #[test]
    fn parser_ignores_blank_lines_and_comments() {
        let manifest = r#"
            # comment before manifest
            id = "calendar" # inline comment
            name = "Calendar"

            category = "Productivity"
            type = "activity"
            version = "0.1.0"
            entry = "main.lua"
            capabilities = ["display", "input", "storage", "time"]
        "#;
        let parsed = parse_lua_app_manifest(manifest).expect("manifest parses with comments");
        assert_eq!(parsed.id.as_str(), "calendar");
        assert_eq!(parsed.category, LuaAppCategoryModel::Productivity);
    }

    #[test]
    fn registry_tracks_discovered_apps_and_rejects_duplicates() {
        let daily = sample_manifest("daily_notes", LuaAppCategoryModel::Tools);
        let calendar = sample_manifest("calendar", LuaAppCategoryModel::Productivity);
        let duplicate_daily = sample_manifest("daily_notes", LuaAppCategoryModel::Tools);

        let mut registry = LuaAppRegistryModel::new();
        registry
            .add(
                LuaDiscoveredAppModel::new(
                    "/RUSTMIX/APPS/daily_notes",
                    "/RUSTMIX/APPS/daily_notes/app.toml",
                    daily,
                )
                .expect("discovered daily app is valid"),
            )
            .expect("daily app is registered");
        registry
            .add(
                LuaDiscoveredAppModel::new(
                    "/RUSTMIX/APPS/calendar",
                    "/RUSTMIX/APPS/calendar/app.toml",
                    calendar,
                )
                .expect("discovered calendar app is valid"),
            )
            .expect("calendar app is registered");

        assert_eq!(registry.len(), 2);
        assert_eq!(registry.count_for_category(LuaAppCategoryModel::Tools), 1);
        assert_eq!(
            registry
                .find_by_id("calendar")
                .expect("calendar exists")
                .manifest
                .name
                .as_str(),
            "Sample App"
        );

        let duplicate = LuaDiscoveredAppModel::new(
            "/RUSTMIX/APPS/daily_notes_copy",
            "/RUSTMIX/APPS/daily_notes_copy/app.toml",
            duplicate_daily,
        )
        .expect("duplicate discovered app record is structurally valid");
        assert_eq!(
            registry.add(duplicate).unwrap_err(),
            LuaAppManifestErrorModel::DuplicateAppId
        );
    }

    #[test]
    fn discovered_app_rejects_traversal_paths() {
        let manifest = sample_manifest("panchang", LuaAppCategoryModel::Tools);
        assert_eq!(
            LuaDiscoveredAppModel::new(
                "/RUSTMIX/APPS/../panchang",
                "/RUSTMIX/APPS/panchang/app.toml",
                manifest,
            )
            .unwrap_err(),
            LuaAppManifestErrorModel::InvalidPath
        );
    }
}
