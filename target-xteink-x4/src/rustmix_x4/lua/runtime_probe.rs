//! No-std runtime probe contract for the future Rustmix Lua app layer.
//!
//! This is a contract/probe seam only. It does not execute Lua yet. The built-in
//! script string is the future VM smoke script, and the native probe result keeps
//! the integration boundary testable without changing firmware behavior.

/// Compile-time marker used by validation and future boot diagnostics.
pub const LUA_RUNTIME_PROBE_MARKER: &str = "rustmix-lua-runtime-probe-ok";

/// Stable identifier for the built-in probe script.
pub const BUILTIN_PROBE_SCRIPT_ID: &str = "system-log-version-v1";

/// Built-in future smoke script for the first real Lua VM bridge.
pub const BUILTIN_PROBE_SCRIPT: &str = r#"system.log("lua-probe-start")
local version = system.version()
system.log("lua-probe-version:" .. version)
return version"#;

/// Minimal API values exposed to the future Lua `system` table during probing.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct LuaRuntimeProbeApi<'a> {
    firmware_version: &'a str,
}

impl<'a> LuaRuntimeProbeApi<'a> {
    /// Creates a probe API descriptor with the firmware version string that the
    /// future Lua `system.version()` call should return.
    pub const fn new(firmware_version: &'a str) -> Self {
        Self { firmware_version }
    }

    /// Returns the version value that will be exposed as `system.version()`.
    pub const fn firmware_version(&self) -> &'a str {
        self.firmware_version
    }
}

/// Sink used by the probe contract to model future `system.log(message)` calls.
pub trait LuaRuntimeProbeLogSink {
    /// Records a probe log message.
    fn log(&mut self, message: &str);
}

/// Log sink for callers that only need the probe report.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct NullLuaRuntimeProbeLogSink;

impl LuaRuntimeProbeLogSink for NullLuaRuntimeProbeLogSink {
    fn log(&mut self, _message: &str) {}
}

/// Status of the feature-gated probe contract.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum LuaRuntimeProbeStatus {
    /// Probe completed through the native contract seam.
    ContractReady,
    /// A future VM reported an execution failure.
    Failed,
}

impl LuaRuntimeProbeStatus {
    /// Returns true when the probe boundary is usable.
    pub const fn is_success(self) -> bool {
        matches!(self, Self::ContractReady)
    }
}

/// Compact no-std report returned by the probe seam.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct LuaRuntimeProbeReport<'a> {
    /// Stable marker for logs and validators.
    pub marker: &'static str,
    /// Built-in script identifier.
    pub script_id: &'static str,
    /// Firmware version observed through the probe API.
    pub version: &'a str,
    /// Probe status.
    pub status: LuaRuntimeProbeStatus,
}

impl<'a> LuaRuntimeProbeReport<'a> {
    /// Returns true when the probe finished successfully.
    pub const fn is_success(&self) -> bool {
        self.status.is_success()
    }
}

/// Runs the current native probe contract.
///
/// A later VM slice should replace the modeled calls with actual Lua execution
/// of `BUILTIN_PROBE_SCRIPT`, while preserving this function's non-panicking
/// report behavior.
pub fn run_builtin_lua_runtime_probe<'a, L>(
    api: LuaRuntimeProbeApi<'a>,
    log_sink: &mut L,
) -> LuaRuntimeProbeReport<'a>
where
    L: LuaRuntimeProbeLogSink,
{
    log_sink.log("lua-probe-start");
    log_sink.log("lua-probe-version");

    LuaRuntimeProbeReport {
        marker: LUA_RUNTIME_PROBE_MARKER,
        script_id: BUILTIN_PROBE_SCRIPT_ID,
        version: api.firmware_version(),
        status: LuaRuntimeProbeStatus::ContractReady,
    }
}

/// Returns a successful static report without requiring a log sink.
pub fn describe_lua_runtime_probe<'a>(firmware_version: &'a str) -> LuaRuntimeProbeReport<'a> {
    let api = LuaRuntimeProbeApi::new(firmware_version);
    let mut sink = NullLuaRuntimeProbeLogSink;
    run_builtin_lua_runtime_probe(api, &mut sink)
}
