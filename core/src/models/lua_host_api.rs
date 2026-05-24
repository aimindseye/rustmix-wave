use serde::{Deserialize, Serialize};

use super::lua_app_manifest::{LuaAppCapabilityModel, LuaAppManifestModel};

pub const LUA_HOST_API_NAMESPACE_MAX: usize = 16;
pub const LUA_HOST_API_FUNCTION_MAX: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LuaHostApiNamespaceModel {
    System,
    Display,
    Input,
    Storage,
    Time,
    Settings,
    Network,
}

impl LuaHostApiNamespaceModel {
    pub const fn manifest_value(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Display => "display",
            Self::Input => "input",
            Self::Storage => "storage",
            Self::Time => "time",
            Self::Settings => "settings",
            Self::Network => "network",
        }
    }

    pub const fn required_capability(self) -> Option<LuaAppCapabilityModel> {
        match self {
            Self::System => None,
            Self::Display => Some(LuaAppCapabilityModel::Display),
            Self::Input => Some(LuaAppCapabilityModel::Input),
            Self::Storage => Some(LuaAppCapabilityModel::Storage),
            Self::Time => Some(LuaAppCapabilityModel::Time),
            Self::Settings => Some(LuaAppCapabilityModel::Settings),
            Self::Network => Some(LuaAppCapabilityModel::Network),
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "system" => Some(Self::System),
            "display" => Some(Self::Display),
            "input" => Some(Self::Input),
            "storage" => Some(Self::Storage),
            "time" => Some(Self::Time),
            "settings" => Some(Self::Settings),
            "network" => Some(Self::Network),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LuaHostApiFunctionModel {
    SystemLog,
    SystemVersion,
    SystemExit,
    SystemBatteryPercent,
    DisplayClear,
    DisplayText,
    DisplayRect,
    DisplayRefresh,
    InputNext,
    InputUp,
    InputDown,
    InputSelect,
    InputBack,
    StorageReadAppFile,
    StorageReadAppState,
    StorageWriteAppState,
    TimeDate,
    TimeStatus,
    SettingsRead,
    SettingsWrite,
    NetworkStatus,
    NetworkFetchText,
}

impl LuaHostApiFunctionModel {
    pub const fn namespace(self) -> LuaHostApiNamespaceModel {
        match self {
            Self::SystemLog
            | Self::SystemVersion
            | Self::SystemExit
            | Self::SystemBatteryPercent => LuaHostApiNamespaceModel::System,
            Self::DisplayClear | Self::DisplayText | Self::DisplayRect | Self::DisplayRefresh => {
                LuaHostApiNamespaceModel::Display
            }
            Self::InputNext
            | Self::InputUp
            | Self::InputDown
            | Self::InputSelect
            | Self::InputBack => LuaHostApiNamespaceModel::Input,
            Self::StorageReadAppFile | Self::StorageReadAppState | Self::StorageWriteAppState => {
                LuaHostApiNamespaceModel::Storage
            }
            Self::TimeDate | Self::TimeStatus => LuaHostApiNamespaceModel::Time,
            Self::SettingsRead | Self::SettingsWrite => LuaHostApiNamespaceModel::Settings,
            Self::NetworkStatus | Self::NetworkFetchText => LuaHostApiNamespaceModel::Network,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::SystemLog => "log",
            Self::SystemVersion => "version",
            Self::SystemExit => "exit",
            Self::SystemBatteryPercent => "battery_percent",
            Self::DisplayClear => "clear",
            Self::DisplayText => "text",
            Self::DisplayRect => "rect",
            Self::DisplayRefresh => "refresh",
            Self::InputNext => "next",
            Self::InputUp => "up",
            Self::InputDown => "down",
            Self::InputSelect => "select",
            Self::InputBack => "back",
            Self::StorageReadAppFile => "read_app_file",
            Self::StorageReadAppState => "read_app_state",
            Self::StorageWriteAppState => "write_app_state",
            Self::TimeDate => "date",
            Self::TimeStatus => "time_status",
            Self::SettingsRead => "read",
            Self::SettingsWrite => "write",
            Self::NetworkStatus => "status",
            Self::NetworkFetchText => "fetch_text",
        }
    }

    pub const fn argument_count(self) -> u8 {
        match self {
            Self::SystemLog => 1,
            Self::SystemVersion | Self::SystemExit | Self::SystemBatteryPercent => 0,
            Self::DisplayClear | Self::DisplayRefresh => 0,
            Self::DisplayText => 3,
            Self::DisplayRect => 4,
            Self::InputNext
            | Self::InputUp
            | Self::InputDown
            | Self::InputSelect
            | Self::InputBack => 0,
            Self::StorageReadAppFile | Self::StorageReadAppState => 1,
            Self::StorageWriteAppState => 2,
            Self::TimeDate | Self::TimeStatus => 0,
            Self::SettingsRead => 1,
            Self::SettingsWrite => 2,
            Self::NetworkStatus => 0,
            Self::NetworkFetchText => 1,
        }
    }

    pub const fn required_capability(self) -> Option<LuaAppCapabilityModel> {
        self.namespace().required_capability()
    }

    pub fn parse(namespace: LuaHostApiNamespaceModel, function: &str) -> Option<Self> {
        match (namespace, function.trim()) {
            (LuaHostApiNamespaceModel::System, "log") => Some(Self::SystemLog),
            (LuaHostApiNamespaceModel::System, "version") => Some(Self::SystemVersion),
            (LuaHostApiNamespaceModel::System, "exit") => Some(Self::SystemExit),
            (LuaHostApiNamespaceModel::System, "battery_percent") => {
                Some(Self::SystemBatteryPercent)
            }
            (LuaHostApiNamespaceModel::Display, "clear") => Some(Self::DisplayClear),
            (LuaHostApiNamespaceModel::Display, "text") => Some(Self::DisplayText),
            (LuaHostApiNamespaceModel::Display, "rect") => Some(Self::DisplayRect),
            (LuaHostApiNamespaceModel::Display, "refresh") => Some(Self::DisplayRefresh),
            (LuaHostApiNamespaceModel::Input, "next") => Some(Self::InputNext),
            (LuaHostApiNamespaceModel::Input, "up") => Some(Self::InputUp),
            (LuaHostApiNamespaceModel::Input, "down") => Some(Self::InputDown),
            (LuaHostApiNamespaceModel::Input, "select") => Some(Self::InputSelect),
            (LuaHostApiNamespaceModel::Input, "back") => Some(Self::InputBack),
            (LuaHostApiNamespaceModel::Storage, "read_app_file") => Some(Self::StorageReadAppFile),
            (LuaHostApiNamespaceModel::Storage, "read_app_state") => {
                Some(Self::StorageReadAppState)
            }
            (LuaHostApiNamespaceModel::Storage, "write_app_state") => {
                Some(Self::StorageWriteAppState)
            }
            (LuaHostApiNamespaceModel::Time, "date") => Some(Self::TimeDate),
            (LuaHostApiNamespaceModel::Time, "time_status") => Some(Self::TimeStatus),
            (LuaHostApiNamespaceModel::Settings, "read") => Some(Self::SettingsRead),
            (LuaHostApiNamespaceModel::Settings, "write") => Some(Self::SettingsWrite),
            (LuaHostApiNamespaceModel::Network, "status") => Some(Self::NetworkStatus),
            (LuaHostApiNamespaceModel::Network, "fetch_text") => Some(Self::NetworkFetchText),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LuaHostApiRuntimeModel {
    ContractOnly,
    HostBindingsAvailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LuaHostApiErrorModel {
    CapabilityDenied,
    UnknownNamespace,
    UnknownFunction,
    InvalidArgumentCount,
    UnsupportedInCurrentRuntime,
}

impl LuaHostApiErrorModel {
    pub const fn diagnostic(self) -> &'static str {
        match self {
            Self::CapabilityDenied => "Lua app manifest does not declare the required capability",
            Self::UnknownNamespace => "Lua host API namespace is unknown",
            Self::UnknownFunction => "Lua host API function is unknown for the namespace",
            Self::InvalidArgumentCount => "Lua host API call uses the wrong argument count",
            Self::UnsupportedInCurrentRuntime => {
                "Lua host API function is known but unsupported in the current runtime"
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LuaHostApiCallDescriptorModel {
    pub namespace: LuaHostApiNamespaceModel,
    pub function: LuaHostApiFunctionModel,
    pub argument_count: u8,
    pub required_capability: Option<LuaAppCapabilityModel>,
}

impl LuaHostApiCallDescriptorModel {
    pub const fn is_system_baseline(self) -> bool {
        matches!(self.namespace, LuaHostApiNamespaceModel::System)
    }

    pub const fn is_supported_by_runtime(self, runtime: LuaHostApiRuntimeModel) -> bool {
        match runtime {
            LuaHostApiRuntimeModel::ContractOnly => false,
            LuaHostApiRuntimeModel::HostBindingsAvailable => true,
        }
    }
}

pub fn describe_lua_host_api_call(
    namespace: &str,
    function: &str,
    argument_count: u8,
) -> Result<LuaHostApiCallDescriptorModel, LuaHostApiErrorModel> {
    let namespace =
        LuaHostApiNamespaceModel::parse(namespace).ok_or(LuaHostApiErrorModel::UnknownNamespace)?;
    let function = LuaHostApiFunctionModel::parse(namespace, function)
        .ok_or(LuaHostApiErrorModel::UnknownFunction)?;

    if function.argument_count() != argument_count {
        return Err(LuaHostApiErrorModel::InvalidArgumentCount);
    }

    Ok(LuaHostApiCallDescriptorModel {
        namespace,
        function,
        argument_count,
        required_capability: function.required_capability(),
    })
}

pub fn validate_lua_host_api_permission(
    manifest: &LuaAppManifestModel,
    descriptor: LuaHostApiCallDescriptorModel,
) -> Result<LuaHostApiCallDescriptorModel, LuaHostApiErrorModel> {
    if let Some(required_capability) = descriptor.required_capability {
        if !manifest.has_capability(required_capability) {
            return Err(LuaHostApiErrorModel::CapabilityDenied);
        }
    }

    Ok(descriptor)
}

pub fn validate_lua_host_api_call(
    manifest: &LuaAppManifestModel,
    namespace: &str,
    function: &str,
    argument_count: u8,
) -> Result<LuaHostApiCallDescriptorModel, LuaHostApiErrorModel> {
    let descriptor = describe_lua_host_api_call(namespace, function, argument_count)?;
    validate_lua_host_api_permission(manifest, descriptor)
}

pub fn validate_lua_host_api_runtime_call(
    manifest: &LuaAppManifestModel,
    namespace: &str,
    function: &str,
    argument_count: u8,
    runtime: LuaHostApiRuntimeModel,
) -> Result<LuaHostApiCallDescriptorModel, LuaHostApiErrorModel> {
    let descriptor = validate_lua_host_api_call(manifest, namespace, function, argument_count)?;
    if !descriptor.is_supported_by_runtime(runtime) {
        return Err(LuaHostApiErrorModel::UnsupportedInCurrentRuntime);
    }

    Ok(descriptor)
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
            "Daily Notes",
            LuaAppCategoryModel::Tools,
            LuaAppTypeModel::Activity,
            "0.1.0",
            "main.lua",
            capabilities,
        )
        .expect("test app manifest is valid")
    }

    #[test]
    fn namespaces_map_to_manifest_capabilities() {
        assert_eq!(LuaHostApiNamespaceModel::System.required_capability(), None);
        assert_eq!(
            LuaHostApiNamespaceModel::Display.required_capability(),
            Some(LuaAppCapabilityModel::Display)
        );
        assert_eq!(
            LuaHostApiNamespaceModel::Input.required_capability(),
            Some(LuaAppCapabilityModel::Input)
        );
        assert_eq!(
            LuaHostApiNamespaceModel::Storage.required_capability(),
            Some(LuaAppCapabilityModel::Storage)
        );
        assert_eq!(
            LuaHostApiNamespaceModel::Time.required_capability(),
            Some(LuaAppCapabilityModel::Time)
        );
        assert_eq!(
            LuaHostApiNamespaceModel::Settings.required_capability(),
            Some(LuaAppCapabilityModel::Settings)
        );
        assert_eq!(
            LuaHostApiNamespaceModel::Network.required_capability(),
            Some(LuaAppCapabilityModel::Network)
        );
    }

    #[test]
    fn describes_known_api_calls_and_argument_counts() {
        let descriptor = describe_lua_host_api_call("display", "text", 3)
            .expect("display.text descriptor is known");
        assert_eq!(descriptor.namespace, LuaHostApiNamespaceModel::Display);
        assert_eq!(descriptor.function, LuaHostApiFunctionModel::DisplayText);
        assert_eq!(descriptor.argument_count, 3);
        assert_eq!(
            descriptor.required_capability,
            Some(LuaAppCapabilityModel::Display)
        );

        let system_descriptor = describe_lua_host_api_call("system", "version", 0)
            .expect("system.version descriptor is known");
        assert!(system_descriptor.is_system_baseline());
        assert_eq!(system_descriptor.required_capability, None);
    }

    #[test]
    fn permission_accepts_system_baseline_without_capability() {
        let manifest = app_with_capabilities(&[]);
        let descriptor = validate_lua_host_api_call(&manifest, "system", "log", 1)
            .expect("system.log is baseline allowed");
        assert_eq!(descriptor.function, LuaHostApiFunctionModel::SystemLog);
    }

    #[test]
    fn permission_accepts_declared_capability() {
        let manifest =
            app_with_capabilities(&[LuaAppCapabilityModel::Display, LuaAppCapabilityModel::Input]);
        let descriptor = validate_lua_host_api_call(&manifest, "input", "back", 0)
            .expect("input.back is allowed with input capability");
        assert_eq!(descriptor.function, LuaHostApiFunctionModel::InputBack);
    }

    #[test]
    fn permission_denies_missing_capability() {
        let manifest = app_with_capabilities(&[LuaAppCapabilityModel::Storage]);
        let error = validate_lua_host_api_call(&manifest, "display", "clear", 0)
            .expect_err("display.clear needs display capability");
        assert_eq!(error, LuaHostApiErrorModel::CapabilityDenied);
    }

    #[test]
    fn invalid_call_shapes_are_diagnostic_specific() {
        assert_eq!(
            describe_lua_host_api_call("graphics", "clear", 0),
            Err(LuaHostApiErrorModel::UnknownNamespace)
        );
        assert_eq!(
            describe_lua_host_api_call("display", "sleep", 0),
            Err(LuaHostApiErrorModel::UnknownFunction)
        );
        assert_eq!(
            describe_lua_host_api_call("display", "text", 2),
            Err(LuaHostApiErrorModel::InvalidArgumentCount)
        );
    }

    #[test]
    fn network_namespace_requires_network_capability() {
        let denied = app_with_capabilities(&[LuaAppCapabilityModel::Display]);
        assert_eq!(
            validate_lua_host_api_call(&denied, "network", "fetch_text", 1),
            Err(LuaHostApiErrorModel::CapabilityDenied)
        );

        let allowed = app_with_capabilities(&[LuaAppCapabilityModel::Network]);
        let descriptor = validate_lua_host_api_call(&allowed, "network", "fetch_text", 1)
            .expect("network.fetch_text is allowed with network capability");
        assert_eq!(
            descriptor.function,
            LuaHostApiFunctionModel::NetworkFetchText
        );
        assert_eq!(descriptor.argument_count, 1);
    }

    #[test]
    fn contract_only_runtime_reports_known_calls_as_unsupported() {
        let manifest = app_with_capabilities(&[LuaAppCapabilityModel::Display]);
        assert_eq!(
            validate_lua_host_api_runtime_call(
                &manifest,
                "display",
                "refresh",
                0,
                LuaHostApiRuntimeModel::ContractOnly,
            ),
            Err(LuaHostApiErrorModel::UnsupportedInCurrentRuntime)
        );
    }

    #[test]
    fn host_bindings_runtime_accepts_known_permitted_call() {
        let manifest = app_with_capabilities(&[
            LuaAppCapabilityModel::Storage,
            LuaAppCapabilityModel::Settings,
            LuaAppCapabilityModel::Time,
        ]);
        assert!(
            validate_lua_host_api_runtime_call(
                &manifest,
                "storage",
                "write_app_state",
                2,
                LuaHostApiRuntimeModel::HostBindingsAvailable,
            )
            .is_ok()
        );
        assert!(
            validate_lua_host_api_runtime_call(
                &manifest,
                "settings",
                "read",
                1,
                LuaHostApiRuntimeModel::HostBindingsAvailable,
            )
            .is_ok()
        );
        assert!(
            validate_lua_host_api_runtime_call(
                &manifest,
                "time",
                "date",
                0,
                LuaHostApiRuntimeModel::HostBindingsAvailable,
            )
            .is_ok()
        );
    }
}
