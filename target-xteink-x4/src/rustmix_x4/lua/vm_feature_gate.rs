use rustmix_lua_vm::{LUA_VM_CRATE_MARKER, LuaVm};

pub const LUA_VM_FEATURE_GATE_MARKER: &str = "rustmix-lua-vm-feature-gate-ok";
pub const LUA_VM_SMOKE_SCRIPT: &str = "return 1 + 2";

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct LuaVmFeatureGateReport {
    pub marker: &'static str,
    pub crate_marker: &'static str,
    pub script: &'static str,
    pub result: i32,
    pub ok: bool,
}

pub fn run_lua_vm_feature_gate_smoke() -> LuaVmFeatureGateReport {
    let mut vm = LuaVm::new();
    match vm.eval_i32(LUA_VM_SMOKE_SCRIPT) {
        Ok(result) => LuaVmFeatureGateReport {
            marker: LUA_VM_FEATURE_GATE_MARKER,
            crate_marker: LUA_VM_CRATE_MARKER,
            script: LUA_VM_SMOKE_SCRIPT,
            result,
            ok: result == 3,
        },
        Err(_) => LuaVmFeatureGateReport {
            marker: LUA_VM_FEATURE_GATE_MARKER,
            crate_marker: LUA_VM_CRATE_MARKER,
            script: LUA_VM_SMOKE_SCRIPT,
            result: 0,
            ok: false,
        },
    }
}
