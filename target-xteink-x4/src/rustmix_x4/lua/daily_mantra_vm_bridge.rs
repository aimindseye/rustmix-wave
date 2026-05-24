//! Feature-gated Lua VM execution bridge for the Daily Mantra SD app.
//!
//! This module is compiled only with `--features lua-vm`. It keeps the
//! existing SD-file loader and safe declaration-subset parser as the fallback,
//! then proves the VM path by running one in-memory smoke script followed by
//! constrained expressions declared in `/RUSTMIX/APPS/MANTRA/MAIN.LUA`.
//!
//! The VM bridge does not overwrite the Daily Mantra text layout. The screen
//! continues to show title, day, and wrapped mantra; VM diagnostics live in the
//! footer.

use crate::rustmix_x4::lua::daily_mantra_script::{
    LuaDailyMantraScreen, build_daily_mantra_sd_runtime_for_day, normalize_weekday,
};
use rustmix_lua_vm::{LUA_VM_CRATE_MARKER, LuaVm};

pub const LUA_DAILY_MANTRA_VM_BRIDGE_MARKER: &str = "rustmix-lua-daily-mantra-vm-bridge-ok";
pub const LUA_DAILY_MANTRA_VM_SMOKE_SCRIPT: &str = "return 1 + 2";
pub const LUA_DAILY_MANTRA_VM_DAY_SMOKE_SCRIPT: &str = "return 'Monday'";
pub const LUA_DAILY_MANTRA_VM_DISABLED_DIAGNOSTIC: &str = "VM disabled";
pub const LUA_DAILY_MANTRA_VM_SCRIPT_LOADED_DIAGNOSTIC: &str = "VM script loaded";
pub const LUA_DAILY_MANTRA_VM_EXECUTION_FAILED_DIAGNOSTIC: &str = "VM execution failed";
pub const LUA_DAILY_MANTRA_FALLBACK_PARSER_USED_DIAGNOSTIC: &str = "fallback parser used";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LuaDailyMantraVmStatus {
    VmScriptLoaded,
    VmExecutionFailed,
    FallbackParserUsed,
}

impl LuaDailyMantraVmStatus {
    pub const fn label(self) -> &'static str {
        match self {
            Self::VmScriptLoaded => LUA_DAILY_MANTRA_VM_SCRIPT_LOADED_DIAGNOSTIC,
            Self::VmExecutionFailed => LUA_DAILY_MANTRA_VM_EXECUTION_FAILED_DIAGNOSTIC,
            Self::FallbackParserUsed => LUA_DAILY_MANTRA_FALLBACK_PARSER_USED_DIAGNOSTIC,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LuaDailyMantraVmReport {
    pub marker: &'static str,
    pub crate_marker: &'static str,
    pub status: LuaDailyMantraVmStatus,
    pub smoke_result: i32,
    pub sd_result: i32,
    pub selected_day: &'static str,
}

/// Builds the Daily Mantra screen using the VM path when possible.
///
/// Supported MAIN.LUA VM declarations:
///
/// ```lua
/// vm_expression = "return 108 + 0"
/// vm_day_expression = "return 'Monday'"
/// -- rustmix-vm-day: return 'Monday'
/// ```
pub fn build_daily_mantra_vm_sd_runtime(
    manifest: &str,
    script: &str,
    mantras: &str,
) -> LuaDailyMantraScreen {
    build_daily_mantra_vm_sd_runtime_for_day(manifest, script, mantras, None)
}

pub fn build_daily_mantra_vm_sd_runtime_for_day(
    manifest: &str,
    script: &str,
    mantras: &str,
    runtime_day: Option<&str>,
) -> LuaDailyMantraScreen {
    let mut vm = LuaVm::new();
    let runtime_selected_day = runtime_day.and_then(normalize_weekday);
    let smoke_result = match vm.eval_i32(LUA_DAILY_MANTRA_VM_SMOKE_SCRIPT) {
        Ok(value) if value == 3 => value,
        _ => {
            let mut screen = build_daily_mantra_sd_runtime_for_day(
                manifest,
                script,
                mantras,
                runtime_selected_day,
            );
            screen
                .footer
                .set("VM execution failed; fallback parser used");
            return screen;
        }
    };

    let selected_day = runtime_selected_day.or_else(|| {
        extract_daily_mantra_vm_day(script)
            .and_then(|expr| vm.eval_str(expr).ok())
            .and_then(normalize_weekday)
    });

    let mut screen = build_daily_mantra_sd_runtime_for_day(manifest, script, mantras, selected_day);
    if !screen.source.is_sd_loaded() {
        screen
            .footer
            .set(LUA_DAILY_MANTRA_FALLBACK_PARSER_USED_DIAGNOSTIC);
        return screen;
    }

    let Some(sd_expression) = extract_daily_mantra_vm_expression(script) else {
        screen.footer.set("VM disabled; fallback parser used");
        return screen;
    };

    match vm.eval_i32(sd_expression) {
        Ok(sd_result) => {
            screen.footer.set_i32_line("VM result: ", sd_result);
            let _report = LuaDailyMantraVmReport {
                marker: LUA_DAILY_MANTRA_VM_BRIDGE_MARKER,
                crate_marker: LUA_VM_CRATE_MARKER,
                status: LuaDailyMantraVmStatus::VmScriptLoaded,
                smoke_result,
                sd_result,
                selected_day: selected_day.unwrap_or("Today"),
            };
            screen
        }
        Err(_) => {
            screen
                .footer
                .set("VM execution failed; fallback parser used");
            screen
        }
    }
}

pub fn extract_daily_mantra_vm_expression(script: &str) -> Option<&str> {
    for line in script.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(expr) = trimmed.strip_prefix("-- rustmix-vm:") {
            let expr = expr.trim();
            if !expr.is_empty() {
                return Some(expr);
            }
        }
        if let Some(expr) = parse_vm_expression_assignment(trimmed, "vm_expression") {
            return Some(expr);
        }
    }
    None
}

pub fn extract_daily_mantra_vm_day(script: &str) -> Option<&str> {
    for line in script.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(expr) = trimmed.strip_prefix("-- rustmix-vm-day:") {
            let expr = expr.trim();
            if !expr.is_empty() {
                return Some(expr);
            }
        }
        if let Some(expr) = parse_vm_expression_assignment(trimmed, "vm_day_expression") {
            return Some(expr);
        }
    }
    None
}

fn parse_vm_expression_assignment<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let rest = line.strip_prefix(key)?.trim_start();
    let rest = rest.strip_prefix('=')?.trim_start();
    let value = strip_lua_trailing_comment(rest)
        .trim_end_matches(',')
        .trim();
    if value.len() < 2 || !value.starts_with('"') || !value.ends_with('"') {
        return None;
    }
    let inner = &value[1..value.len() - 1];
    if inner.trim().is_empty() {
        None
    } else {
        Some(inner.trim())
    }
}

fn strip_lua_trailing_comment(value: &str) -> &str {
    match value.find(" --") {
        Some(pos) => value[..pos].trim_end(),
        None => value,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str = r#"id = "daily_mantra"
name = "Daily Mantra"
category = "Tools"
type = "activity"
version = "0.1.0"
entry = "MAIN.LUA"
capabilities = ["display", "input", "storage", "time"]
"#;

    #[test]
    fn extracts_vm_expression_and_day_expression() {
        assert_eq!(
            extract_daily_mantra_vm_expression("vm_expression = \"return 108 + 0\""),
            Some("return 108 + 0")
        );
        assert_eq!(
            extract_daily_mantra_vm_expression("-- rustmix-vm: return 12 + 30"),
            Some("return 12 + 30")
        );
        assert_eq!(
            extract_daily_mantra_vm_day("vm_day_expression = \"return 'Monday'\""),
            Some("return 'Monday'")
        );
        assert_eq!(
            extract_daily_mantra_vm_day("-- rustmix-vm-day: return 'Tuesday'"),
            Some("return 'Tuesday'")
        );
    }

    #[test]
    fn vm_success_uses_vm_day_to_select_weekday_record() {
        let screen = build_daily_mantra_vm_sd_runtime(
            MANIFEST,
            "vm_expression = \"return 108 + 0\"\nvm_day_expression = \"return 'Monday'\"\n",
            "Om Namah Shivaya|old non-weekday first row.\nMonday|Om Namah Shivaya|A steady mind turns every page into practice.\nTuesday|Om Shanti|Peace.\n",
        );
        assert_eq!(screen.title(), "Daily Mantra");
        assert_eq!(screen.subtitle(), "Day: Monday");
        assert!(screen.line1().starts_with("Mantra: Om Namah Shivaya"));
        assert_eq!(screen.footer(), "VM result: 108");
    }

    #[test]
    fn runtime_day_overrides_vm_day_to_use_clock() {
        let screen = build_daily_mantra_vm_sd_runtime_for_day(
            MANIFEST,
            "vm_expression = \"return 108 + 0\"\nvm_day_expression = \"return 'Monday'\"\n",
            "Monday|Om Monday|Monday text.\nWednesday|Om Wednesday|Wednesday text.\n",
            Some("Wednesday"),
        );
        assert_eq!(screen.subtitle(), "Day: Wednesday");
        assert!(screen.line1().starts_with("Mantra: Om Wednesday"));
    }

    #[test]
    fn unsupported_vm_expression_falls_back_without_panicking() {
        let screen = build_daily_mantra_vm_sd_runtime(
            MANIFEST,
            "vm_expression = \"print('not supported yet')\"\nvm_day_expression = \"return 'Monday'\"\n",
            "Monday|Om Shanti|Peace.\n",
        );
        assert_eq!(screen.subtitle(), "Day: Monday");
        assert_eq!(screen.footer(), "VM execution failed; fallback parser used");
    }
}
