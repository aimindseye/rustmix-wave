use heapless::{String, Vec};
use serde::{Deserialize, Serialize};

use super::lua_app_manifest::{
    LUA_APP_ID_MAX, LUA_APP_NAME_MAX, LUA_APP_REGISTRY_MAX, LuaAppCategoryModel,
    LuaAppRegistryModel, LuaAppTypeModel, is_valid_lua_app_id,
};
use super::lua_app_runtime::{LuaAppLifecycleStateModel, LuaAppRuntimeStateModel};

pub const LUA_APP_CATALOG_MAX: usize = LUA_APP_REGISTRY_MAX;
pub const LUA_APP_CATALOG_GROUP_MAX: usize = LUA_APP_REGISTRY_MAX;
pub const LUA_APP_CATALOG_DIAGNOSTICS_MAX: usize = LUA_APP_REGISTRY_MAX;
pub const LUA_APP_CATALOG_SORT_KEY_MAX: usize = 96;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LuaDashboardCategoryModel {
    Network,
    Productivity,
    Games,
    Reader,
    System,
    Tools,
}

impl LuaDashboardCategoryModel {
    pub const ORDER: [Self; 6] = [
        Self::Network,
        Self::Productivity,
        Self::Games,
        Self::Reader,
        Self::System,
        Self::Tools,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Network => "Network",
            Self::Productivity => "Productivity",
            Self::Games => "Games",
            Self::Reader => "Reader",
            Self::System => "System",
            Self::Tools => "Tools",
        }
    }

    pub const fn order_index(self) -> u8 {
        match self {
            Self::Network => 0,
            Self::Productivity => 1,
            Self::Games => 2,
            Self::Reader => 3,
            Self::System => 4,
            Self::Tools => 5,
        }
    }

    pub const fn from_lua_category(category: LuaAppCategoryModel) -> Self {
        match category {
            LuaAppCategoryModel::Network => Self::Network,
            LuaAppCategoryModel::Productivity => Self::Productivity,
            LuaAppCategoryModel::Games => Self::Games,
            LuaAppCategoryModel::Reader => Self::Reader,
            LuaAppCategoryModel::System => Self::System,
            LuaAppCategoryModel::Tools => Self::Tools,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LuaAppCatalogSourceModel {
    Native,
    LuaSdApp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LuaAppCatalogVisibilityModel {
    Visible,
    HiddenDisabled,
    HiddenCrashed,
}

impl LuaAppCatalogVisibilityModel {
    pub const fn is_visible(self) -> bool {
        matches!(self, Self::Visible)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LuaAppCatalogDiagnosticKindModel {
    HiddenDisabledApp,
    HiddenCrashedApp,
    DuplicateDisplayName,
    UnsupportedDashboardCategory,
    CatalogFull,
    DiagnosticFull,
    InvalidAppId,
    InvalidDisplayLabel,
    InvalidSortKey,
}

impl LuaAppCatalogDiagnosticKindModel {
    pub const fn diagnostic(self) -> &'static str {
        match self {
            Self::HiddenDisabledApp => "Lua app is hidden because it is disabled",
            Self::HiddenCrashedApp => "Lua app is hidden because it crashed",
            Self::DuplicateDisplayName => {
                "Lua app display name duplicates another visible app in the same category"
            }
            Self::UnsupportedDashboardCategory => {
                "Lua app category is not supported by the dashboard catalog"
            }
            Self::CatalogFull => "Lua app dashboard catalog is full",
            Self::DiagnosticFull => "Lua app dashboard catalog diagnostics are full",
            Self::InvalidAppId => "Lua app dashboard catalog entry has an invalid app id",
            Self::InvalidDisplayLabel => {
                "Lua app dashboard catalog entry has an invalid display label"
            }
            Self::InvalidSortKey => "Lua app dashboard catalog entry has an invalid sort key",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LuaAppCatalogDiagnosticModel {
    pub kind: LuaAppCatalogDiagnosticKindModel,
    pub app_id: String<LUA_APP_ID_MAX>,
    pub category: LuaDashboardCategoryModel,
}

impl LuaAppCatalogDiagnosticModel {
    pub fn new(
        kind: LuaAppCatalogDiagnosticKindModel,
        app_id: &str,
        category: LuaDashboardCategoryModel,
    ) -> Result<Self, LuaAppCatalogDiagnosticKindModel> {
        let mut copied_id: String<LUA_APP_ID_MAX> = String::new();
        copied_id
            .push_str(app_id)
            .map_err(|_| LuaAppCatalogDiagnosticKindModel::InvalidAppId)?;

        Ok(Self {
            kind,
            app_id: copied_id,
            category,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LuaAppCatalogEntryModel {
    pub app_id: String<LUA_APP_ID_MAX>,
    pub display_label: String<LUA_APP_NAME_MAX>,
    pub sort_key: String<LUA_APP_CATALOG_SORT_KEY_MAX>,
    pub category: LuaDashboardCategoryModel,
    pub source: LuaAppCatalogSourceModel,
    pub app_type: LuaAppTypeModel,
    pub visibility: LuaAppCatalogVisibilityModel,
}

impl LuaAppCatalogEntryModel {
    pub fn new_lua_app(
        app_id: &str,
        display_label: &str,
        category: LuaDashboardCategoryModel,
        app_type: LuaAppTypeModel,
        visibility: LuaAppCatalogVisibilityModel,
    ) -> Result<Self, LuaAppCatalogDiagnosticKindModel> {
        if !is_valid_lua_app_id(app_id) {
            return Err(LuaAppCatalogDiagnosticKindModel::InvalidAppId);
        }
        let display_label = display_label.trim();
        if display_label.is_empty() {
            return Err(LuaAppCatalogDiagnosticKindModel::InvalidDisplayLabel);
        }

        let app_id_string = copy_catalog_string::<LUA_APP_ID_MAX>(app_id)?;
        let label_string = copy_catalog_string::<LUA_APP_NAME_MAX>(display_label)?;
        let sort_key = lua_app_catalog_sort_key(category, display_label, app_id)?;

        Ok(Self {
            app_id: app_id_string,
            display_label: label_string,
            sort_key,
            category,
            source: LuaAppCatalogSourceModel::LuaSdApp,
            app_type,
            visibility,
        })
    }

    pub const fn is_visible(&self) -> bool {
        self.visibility.is_visible()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LuaAppCatalogCategoryGroupModel {
    pub category: LuaDashboardCategoryModel,
    pub entries: Vec<LuaAppCatalogEntryModel, LUA_APP_CATALOG_GROUP_MAX>,
}

impl LuaAppCatalogCategoryGroupModel {
    pub fn new(category: LuaDashboardCategoryModel) -> Self {
        Self {
            category,
            entries: Vec::new(),
        }
    }

    pub fn add(
        &mut self,
        entry: LuaAppCatalogEntryModel,
    ) -> Result<(), LuaAppCatalogDiagnosticKindModel> {
        self.entries
            .push(entry)
            .map_err(|_| LuaAppCatalogDiagnosticKindModel::CatalogFull)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LuaAppDashboardCatalogModel {
    pub entries: Vec<LuaAppCatalogEntryModel, LUA_APP_CATALOG_MAX>,
    pub diagnostics: Vec<LuaAppCatalogDiagnosticModel, LUA_APP_CATALOG_DIAGNOSTICS_MAX>,
}

impl LuaAppDashboardCatalogModel {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    pub fn from_registry(registry: &LuaAppRegistryModel) -> Self {
        Self::from_registry_with_runtime_states(registry, &[])
    }

    pub fn from_registry_with_runtime_states(
        registry: &LuaAppRegistryModel,
        runtime_states: &[LuaAppRuntimeStateModel],
    ) -> Self {
        let mut catalog = Self::new();

        for app in registry.apps.iter() {
            let category = LuaDashboardCategoryModel::from_lua_category(app.manifest.category);
            let visibility = lua_app_catalog_visibility_for_state(
                runtime_states
                    .iter()
                    .find(|state| state.app_id.as_str() == app.manifest.id.as_str())
                    .map(|state| state.lifecycle_state),
            );

            match LuaAppCatalogEntryModel::new_lua_app(
                app.manifest.id.as_str(),
                app.manifest.name.as_str(),
                category,
                app.manifest.app_type,
                visibility,
            ) {
                Ok(entry) => {
                    if !entry.is_visible() {
                        let kind = match entry.visibility {
                            LuaAppCatalogVisibilityModel::HiddenDisabled => {
                                LuaAppCatalogDiagnosticKindModel::HiddenDisabledApp
                            }
                            LuaAppCatalogVisibilityModel::HiddenCrashed => {
                                LuaAppCatalogDiagnosticKindModel::HiddenCrashedApp
                            }
                            LuaAppCatalogVisibilityModel::Visible => {
                                unreachable_visible_diagnostic()
                            }
                        };
                        let _ = catalog.add_diagnostic(kind, entry.app_id.as_str(), category);
                    }
                    if entry.is_visible()
                        && catalog.has_visible_display_label(category, entry.display_label.as_str())
                    {
                        let _ = catalog.add_diagnostic(
                            LuaAppCatalogDiagnosticKindModel::DuplicateDisplayName,
                            entry.app_id.as_str(),
                            category,
                        );
                    }
                    if catalog.entries.push(entry).is_err() {
                        let _ = catalog.add_diagnostic(
                            LuaAppCatalogDiagnosticKindModel::CatalogFull,
                            app.manifest.id.as_str(),
                            category,
                        );
                    }
                }
                Err(kind) => {
                    let _ = catalog.add_diagnostic(kind, app.manifest.id.as_str(), category);
                }
            }
        }

        catalog.sort_visible_entries();
        catalog
    }

    pub fn visible_entries_for_category(
        &self,
        category: LuaDashboardCategoryModel,
    ) -> Vec<LuaAppCatalogEntryModel, LUA_APP_CATALOG_GROUP_MAX> {
        let mut entries = Vec::new();
        for entry in self.entries.iter() {
            if entry.category == category && entry.is_visible() {
                let _ = entries.push(entry.clone());
            }
        }
        entries
    }

    pub fn grouped_by_category(
        &self,
    ) -> Vec<LuaAppCatalogCategoryGroupModel, { LuaDashboardCategoryModel::ORDER.len() }> {
        let mut groups = Vec::new();
        for category in LuaDashboardCategoryModel::ORDER {
            let mut group = LuaAppCatalogCategoryGroupModel::new(category);
            for entry in self.entries.iter() {
                if entry.category == category && entry.is_visible() {
                    let _ = group.add(entry.clone());
                }
            }
            let _ = groups.push(group);
        }
        groups
    }

    pub fn has_visible_display_label(
        &self,
        category: LuaDashboardCategoryModel,
        display_label: &str,
    ) -> bool {
        self.entries.iter().any(|entry| {
            entry.category == category
                && entry.is_visible()
                && labels_equal(entry.display_label.as_str(), display_label)
        })
    }

    fn add_diagnostic(
        &mut self,
        kind: LuaAppCatalogDiagnosticKindModel,
        app_id: &str,
        category: LuaDashboardCategoryModel,
    ) -> Result<(), LuaAppCatalogDiagnosticKindModel> {
        let diagnostic = LuaAppCatalogDiagnosticModel::new(kind, app_id, category)?;
        self.diagnostics
            .push(diagnostic)
            .map_err(|_| LuaAppCatalogDiagnosticKindModel::DiagnosticFull)
    }

    fn sort_visible_entries(&mut self) {
        let len = self.entries.len();
        let mut i = 1;
        while i < len {
            let mut j = i;
            while j > 0 && catalog_entry_less(&self.entries[j], &self.entries[j - 1]) {
                self.entries.swap(j, j - 1);
                j -= 1;
            }
            i += 1;
        }
    }
}

impl Default for LuaAppDashboardCatalogModel {
    fn default() -> Self {
        Self::new()
    }
}

pub const fn lua_app_catalog_visibility_for_state(
    state: Option<LuaAppLifecycleStateModel>,
) -> LuaAppCatalogVisibilityModel {
    match state {
        Some(LuaAppLifecycleStateModel::Disabled) => LuaAppCatalogVisibilityModel::HiddenDisabled,
        Some(LuaAppLifecycleStateModel::Crashed) => LuaAppCatalogVisibilityModel::HiddenCrashed,
        _ => LuaAppCatalogVisibilityModel::Visible,
    }
}

pub fn lua_app_catalog_sort_key(
    category: LuaDashboardCategoryModel,
    display_label: &str,
    app_id: &str,
) -> Result<String<LUA_APP_CATALOG_SORT_KEY_MAX>, LuaAppCatalogDiagnosticKindModel> {
    if display_label.trim().is_empty() {
        return Err(LuaAppCatalogDiagnosticKindModel::InvalidSortKey);
    }
    if !is_valid_lua_app_id(app_id) {
        return Err(LuaAppCatalogDiagnosticKindModel::InvalidAppId);
    }

    let mut sort_key: String<LUA_APP_CATALOG_SORT_KEY_MAX> = String::new();
    push_u8_decimal(&mut sort_key, category.order_index())?;
    sort_key
        .push('|')
        .map_err(|_| LuaAppCatalogDiagnosticKindModel::InvalidSortKey)?;
    push_lower_ascii(&mut sort_key, display_label.trim())?;
    sort_key
        .push('|')
        .map_err(|_| LuaAppCatalogDiagnosticKindModel::InvalidSortKey)?;
    sort_key
        .push_str(app_id)
        .map_err(|_| LuaAppCatalogDiagnosticKindModel::InvalidSortKey)?;
    Ok(sort_key)
}

pub const fn native_apps_are_authoritative() -> bool {
    true
}

fn unreachable_visible_diagnostic() -> LuaAppCatalogDiagnosticKindModel {
    LuaAppCatalogDiagnosticKindModel::UnsupportedDashboardCategory
}

fn catalog_entry_less(left: &LuaAppCatalogEntryModel, right: &LuaAppCatalogEntryModel) -> bool {
    left.sort_key.as_str() < right.sort_key.as_str()
}

fn labels_equal(left: &str, right: &str) -> bool {
    left.eq_ignore_ascii_case(right.trim())
}

fn push_u8_decimal(
    output: &mut String<LUA_APP_CATALOG_SORT_KEY_MAX>,
    value: u8,
) -> Result<(), LuaAppCatalogDiagnosticKindModel> {
    let tens = value / 10;
    let ones = value % 10;
    if tens > 0 {
        output
            .push(char::from(b'0' + tens))
            .map_err(|_| LuaAppCatalogDiagnosticKindModel::InvalidSortKey)?;
    }
    output
        .push(char::from(b'0' + ones))
        .map_err(|_| LuaAppCatalogDiagnosticKindModel::InvalidSortKey)
}

fn push_lower_ascii(
    output: &mut String<LUA_APP_CATALOG_SORT_KEY_MAX>,
    value: &str,
) -> Result<(), LuaAppCatalogDiagnosticKindModel> {
    for ch in value.chars() {
        let normalized = if ch.is_ascii_uppercase() {
            ch.to_ascii_lowercase()
        } else {
            ch
        };
        output
            .push(normalized)
            .map_err(|_| LuaAppCatalogDiagnosticKindModel::InvalidSortKey)?;
    }
    Ok(())
}

fn copy_catalog_string<const N: usize>(
    value: &str,
) -> Result<String<N>, LuaAppCatalogDiagnosticKindModel> {
    let mut output: String<N> = String::new();
    output.push_str(value).map_err(|_| {
        if N == LUA_APP_NAME_MAX {
            LuaAppCatalogDiagnosticKindModel::InvalidDisplayLabel
        } else {
            LuaAppCatalogDiagnosticKindModel::InvalidAppId
        }
    })?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::lua_app_manifest::{
        LuaAppCapabilityModel, LuaAppManifestModel, LuaDiscoveredAppModel,
    };

    fn manifest(id: &str, name: &str, category: LuaAppCategoryModel) -> LuaAppManifestModel {
        LuaAppManifestModel::new(
            id,
            name,
            category,
            LuaAppTypeModel::Activity,
            "0.1.0",
            "main.lua",
            &[LuaAppCapabilityModel::Display, LuaAppCapabilityModel::Input],
        )
        .unwrap()
    }

    fn discovered(id: &str, name: &str, category: LuaAppCategoryModel) -> LuaDiscoveredAppModel {
        let manifest = manifest(id, name, category);
        let app_dir = match id {
            "daily_mantra" => "/RUSTMIX/APPS/daily_mantra",
            "calendar" => "/RUSTMIX/APPS/calendar",
            "panchang" => "/RUSTMIX/APPS/panchang",
            "rss" => "/RUSTMIX/APPS/rss",
            "alpha" => "/RUSTMIX/APPS/alpha",
            "beta" => "/RUSTMIX/APPS/beta",
            _ => "/RUSTMIX/APPS/tool",
        };
        let manifest_path = match id {
            "daily_mantra" => "/RUSTMIX/APPS/daily_mantra/app.toml",
            "calendar" => "/RUSTMIX/APPS/calendar/app.toml",
            "panchang" => "/RUSTMIX/APPS/panchang/app.toml",
            "rss" => "/RUSTMIX/APPS/rss/app.toml",
            "alpha" => "/RUSTMIX/APPS/alpha/app.toml",
            "beta" => "/RUSTMIX/APPS/beta/app.toml",
            _ => "/RUSTMIX/APPS/tool/app.toml",
        };
        LuaDiscoveredAppModel::new(app_dir, manifest_path, manifest).unwrap()
    }

    #[test]
    fn category_order_matches_existing_dashboard_order() {
        let labels: Vec<&str, 6> = LuaDashboardCategoryModel::ORDER
            .iter()
            .map(|category| category.label())
            .collect();
        assert_eq!(
            labels.as_slice(),
            &[
                "Network",
                "Productivity",
                "Games",
                "Reader",
                "System",
                "Tools"
            ]
        );
    }

    #[test]
    fn registry_converts_to_visible_catalog_entries() {
        let mut registry = LuaAppRegistryModel::new();
        registry
            .add(discovered(
                "daily_mantra",
                "Daily Mantra",
                LuaAppCategoryModel::Tools,
            ))
            .unwrap();

        let catalog = LuaAppDashboardCatalogModel::from_registry(&registry);
        assert_eq!(catalog.entries.len(), 1);
        assert_eq!(catalog.entries[0].app_id.as_str(), "daily_mantra");
        assert_eq!(catalog.entries[0].display_label.as_str(), "Daily Mantra");
        assert_eq!(
            catalog.entries[0].category,
            LuaDashboardCategoryModel::Tools
        );
        assert_eq!(
            catalog.entries[0].source,
            LuaAppCatalogSourceModel::LuaSdApp
        );
        assert!(catalog.entries[0].is_visible());
        assert!(catalog.diagnostics.is_empty());
    }

    #[test]
    fn groups_visible_apps_by_dashboard_category() {
        let mut registry = LuaAppRegistryModel::new();
        registry
            .add(discovered(
                "calendar",
                "Calendar",
                LuaAppCategoryModel::Productivity,
            ))
            .unwrap();
        registry
            .add(discovered(
                "panchang",
                "Panchang",
                LuaAppCategoryModel::Tools,
            ))
            .unwrap();
        registry
            .add(discovered("rss", "RSS", LuaAppCategoryModel::Network))
            .unwrap();

        let catalog = LuaAppDashboardCatalogModel::from_registry(&registry);
        let groups = catalog.grouped_by_category();
        assert_eq!(groups.len(), 6);
        assert_eq!(groups[0].category, LuaDashboardCategoryModel::Network);
        assert_eq!(groups[0].entries[0].app_id.as_str(), "rss");
        assert_eq!(groups[1].category, LuaDashboardCategoryModel::Productivity);
        assert_eq!(groups[1].entries[0].app_id.as_str(), "calendar");
        assert_eq!(groups[5].category, LuaDashboardCategoryModel::Tools);
        assert_eq!(groups[5].entries[0].app_id.as_str(), "panchang");
    }

    #[test]
    fn sort_keys_order_by_category_label_and_app_id() {
        let mut registry = LuaAppRegistryModel::new();
        registry
            .add(discovered("beta", "Same", LuaAppCategoryModel::Tools))
            .unwrap();
        registry
            .add(discovered("alpha", "Alpha", LuaAppCategoryModel::Tools))
            .unwrap();

        let catalog = LuaAppDashboardCatalogModel::from_registry(&registry);
        let tools = catalog.visible_entries_for_category(LuaDashboardCategoryModel::Tools);
        assert_eq!(tools.len(), 2);
        assert_eq!(tools[0].app_id.as_str(), "alpha");
        assert_eq!(tools[1].app_id.as_str(), "beta");
        assert!(tools[0].sort_key.as_str() < tools[1].sort_key.as_str());
    }

    #[test]
    fn disabled_and_crashed_apps_are_hidden_with_diagnostics() {
        let mut registry = LuaAppRegistryModel::new();
        registry
            .add(discovered(
                "calendar",
                "Calendar",
                LuaAppCategoryModel::Productivity,
            ))
            .unwrap();
        registry
            .add(discovered(
                "panchang",
                "Panchang",
                LuaAppCategoryModel::Tools,
            ))
            .unwrap();

        let states = [
            LuaAppRuntimeStateModel::new("calendar", LuaAppLifecycleStateModel::Disabled).unwrap(),
            LuaAppRuntimeStateModel::new("panchang", LuaAppLifecycleStateModel::Crashed).unwrap(),
        ];
        let catalog =
            LuaAppDashboardCatalogModel::from_registry_with_runtime_states(&registry, &states);

        assert_eq!(
            catalog
                .visible_entries_for_category(LuaDashboardCategoryModel::Productivity)
                .len(),
            0
        );
        assert_eq!(
            catalog
                .visible_entries_for_category(LuaDashboardCategoryModel::Tools)
                .len(),
            0
        );
        assert_eq!(catalog.diagnostics.len(), 2);
        assert_eq!(
            catalog.diagnostics[0].kind,
            LuaAppCatalogDiagnosticKindModel::HiddenDisabledApp
        );
        assert_eq!(
            catalog.diagnostics[1].kind,
            LuaAppCatalogDiagnosticKindModel::HiddenCrashedApp
        );
    }

    #[test]
    fn duplicate_display_names_are_diagnostic_but_entries_remain_visible() {
        let mut registry = LuaAppRegistryModel::new();
        registry
            .add(discovered("alpha", "Tools App", LuaAppCategoryModel::Tools))
            .unwrap();
        registry
            .add(discovered("beta", "tools app", LuaAppCategoryModel::Tools))
            .unwrap();

        let catalog = LuaAppDashboardCatalogModel::from_registry(&registry);
        assert_eq!(
            catalog
                .visible_entries_for_category(LuaDashboardCategoryModel::Tools)
                .len(),
            2
        );
        assert_eq!(catalog.diagnostics.len(), 1);
        assert_eq!(
            catalog.diagnostics[0].kind,
            LuaAppCatalogDiagnosticKindModel::DuplicateDisplayName
        );
        assert_eq!(catalog.diagnostics[0].app_id.as_str(), "beta");
    }

    #[test]
    fn duplicate_display_names_in_different_categories_are_allowed() {
        let mut registry = LuaAppRegistryModel::new();
        registry
            .add(discovered("alpha", "Search", LuaAppCategoryModel::Network))
            .unwrap();
        registry
            .add(discovered("beta", "Search", LuaAppCategoryModel::Tools))
            .unwrap();

        let catalog = LuaAppDashboardCatalogModel::from_registry(&registry);
        assert_eq!(catalog.diagnostics.len(), 0);
        assert_eq!(catalog.entries.len(), 2);
    }

    #[test]
    fn native_apps_are_authoritative_and_lua_apps_are_optional() {
        assert!(native_apps_are_authoritative());
        let entry = LuaAppCatalogEntryModel::new_lua_app(
            "daily_mantra",
            "Daily Mantra",
            LuaDashboardCategoryModel::Tools,
            LuaAppTypeModel::Activity,
            LuaAppCatalogVisibilityModel::Visible,
        )
        .unwrap();
        assert_eq!(entry.source, LuaAppCatalogSourceModel::LuaSdApp);
    }

    #[test]
    fn diagnostic_labels_are_stable_for_dashboard_logs() {
        assert_eq!(
            LuaAppCatalogDiagnosticKindModel::HiddenDisabledApp.diagnostic(),
            "Lua app is hidden because it is disabled"
        );
        assert_eq!(
            LuaAppCatalogDiagnosticKindModel::HiddenCrashedApp.diagnostic(),
            "Lua app is hidden because it crashed"
        );
        assert_eq!(
            LuaAppCatalogDiagnosticKindModel::UnsupportedDashboardCategory.diagnostic(),
            "Lua app category is not supported by the dashboard catalog"
        );
    }
}
