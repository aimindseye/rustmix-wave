use heapless::String;
use serde::{Deserialize, Serialize};

use super::lua_app_manifest::{LUA_APP_ID_MAX, is_valid_lua_app_id};

pub const LUA_APP_RUNTIME_REASON_MAX: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LuaAppLifecycleStateModel {
    Discovered,
    LaunchPending,
    Running,
    Suspended,
    Exiting,
    Crashed,
    Disabled,
}

impl LuaAppLifecycleStateModel {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Discovered => "Discovered",
            Self::LaunchPending => "LaunchPending",
            Self::Running => "Running",
            Self::Suspended => "Suspended",
            Self::Exiting => "Exiting",
            Self::Crashed => "Crashed",
            Self::Disabled => "Disabled",
        }
    }

    pub const fn can_accept_back_exit(self) -> bool {
        true
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LuaAppLifecycleEventModel {
    Discover,
    Launch,
    Suspend,
    Resume,
    Exit,
    Crash,
    Disable,
}

impl LuaAppLifecycleEventModel {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Discover => "discover",
            Self::Launch => "launch",
            Self::Suspend => "suspend",
            Self::Resume => "resume",
            Self::Exit => "exit",
            Self::Crash => "crash",
            Self::Disable => "disable",
        }
    }

    pub const fn is_safe_exit(self) -> bool {
        match self {
            Self::Exit => true,
            _ => false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LuaAppSafeReturnTargetModel {
    Dashboard,
    AppManager,
}

impl LuaAppSafeReturnTargetModel {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Dashboard => "dashboard",
            Self::AppManager => "app-manager",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LuaAppRuntimeErrorModel {
    InvalidAppId,
    InvalidTransition,
    ReasonRequired,
    ReasonTooLong,
}

impl LuaAppRuntimeErrorModel {
    pub const fn diagnostic(self) -> &'static str {
        match self {
            Self::InvalidAppId => "Lua app id is invalid for runtime state",
            Self::InvalidTransition => "Lua app lifecycle transition is not allowed",
            Self::ReasonRequired => "Lua app crash/exit diagnostic requires a reason",
            Self::ReasonTooLong => "Lua app crash/exit diagnostic reason is too long",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LuaAppRuntimeStateModel {
    pub app_id: String<LUA_APP_ID_MAX>,
    pub lifecycle_state: LuaAppLifecycleStateModel,
}

impl LuaAppRuntimeStateModel {
    pub fn new(
        app_id: &str,
        lifecycle_state: LuaAppLifecycleStateModel,
    ) -> Result<Self, LuaAppRuntimeErrorModel> {
        if !is_valid_lua_app_id(app_id) {
            return Err(LuaAppRuntimeErrorModel::InvalidAppId);
        }

        Ok(Self {
            app_id: copy_runtime_string(app_id)?,
            lifecycle_state,
        })
    }

    pub fn discovered(app_id: &str) -> Result<Self, LuaAppRuntimeErrorModel> {
        Self::new(app_id, LuaAppLifecycleStateModel::Discovered)
    }

    pub fn transition(
        &self,
        event: LuaAppLifecycleEventModel,
    ) -> Result<LuaAppLifecycleStateModel, LuaAppRuntimeErrorModel> {
        lua_app_lifecycle_transition(self.lifecycle_state, event)
    }

    pub fn apply_event(
        &mut self,
        event: LuaAppLifecycleEventModel,
    ) -> Result<LuaAppLifecycleStateModel, LuaAppRuntimeErrorModel> {
        let next_state = self.transition(event)?;
        self.lifecycle_state = next_state;
        Ok(next_state)
    }

    pub fn back_event(&self) -> LuaAppLifecycleEventModel {
        lua_app_back_event_policy(self.lifecycle_state)
    }

    pub fn safe_return_target(&self) -> LuaAppSafeReturnTargetModel {
        lua_app_safe_return_target_for_state(self.lifecycle_state)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LuaAppRuntimeDiagnosticModel {
    pub app_id: String<LUA_APP_ID_MAX>,
    pub reason: String<LUA_APP_RUNTIME_REASON_MAX>,
    pub last_lifecycle_state: LuaAppLifecycleStateModel,
    pub safe_return_target: LuaAppSafeReturnTargetModel,
}

impl LuaAppRuntimeDiagnosticModel {
    pub fn new(
        app_id: &str,
        reason: &str,
        last_lifecycle_state: LuaAppLifecycleStateModel,
        safe_return_target: LuaAppSafeReturnTargetModel,
    ) -> Result<Self, LuaAppRuntimeErrorModel> {
        if !is_valid_lua_app_id(app_id) {
            return Err(LuaAppRuntimeErrorModel::InvalidAppId);
        }
        let reason = reason.trim();
        if reason.is_empty() {
            return Err(LuaAppRuntimeErrorModel::ReasonRequired);
        }

        Ok(Self {
            app_id: copy_runtime_string(app_id)?,
            reason: copy_reason_string(reason)?,
            last_lifecycle_state,
            safe_return_target,
        })
    }

    pub fn returns_to_dashboard(&self) -> bool {
        self.safe_return_target == LuaAppSafeReturnTargetModel::Dashboard
    }
}

pub fn lua_app_lifecycle_transition(
    state: LuaAppLifecycleStateModel,
    event: LuaAppLifecycleEventModel,
) -> Result<LuaAppLifecycleStateModel, LuaAppRuntimeErrorModel> {
    use self::LuaAppLifecycleEventModel as Event;
    use self::LuaAppLifecycleStateModel as State;

    match (state, event) {
        (State::Discovered, Event::Discover) => Ok(State::Discovered),
        (State::Discovered, Event::Launch) => Ok(State::LaunchPending),
        (State::LaunchPending, Event::Launch) => Ok(State::Running),
        (State::Running, Event::Suspend) => Ok(State::Suspended),
        (State::Suspended, Event::Resume) => Ok(State::Running),
        (
            State::Discovered | State::LaunchPending | State::Running | State::Suspended,
            Event::Exit,
        ) => Ok(State::Exiting),
        (
            State::Discovered | State::LaunchPending | State::Running | State::Suspended,
            Event::Crash,
        ) => Ok(State::Crashed),
        (
            State::Discovered
            | State::LaunchPending
            | State::Running
            | State::Suspended
            | State::Exiting
            | State::Crashed
            | State::Disabled,
            Event::Disable,
        ) => Ok(State::Disabled),
        (State::Disabled, Event::Discover) => Ok(State::Disabled),
        (State::Disabled, Event::Exit) => Ok(State::Disabled),
        (State::Crashed, Event::Exit) => Ok(State::Exiting),
        (State::Exiting, Event::Exit) => Ok(State::Exiting),
        _ => Err(LuaAppRuntimeErrorModel::InvalidTransition),
    }
}

pub fn lua_app_back_event_policy(_state: LuaAppLifecycleStateModel) -> LuaAppLifecycleEventModel {
    LuaAppLifecycleEventModel::Exit
}

pub fn lua_app_safe_return_target_for_state(
    _state: LuaAppLifecycleStateModel,
) -> LuaAppSafeReturnTargetModel {
    LuaAppSafeReturnTargetModel::Dashboard
}

pub fn lua_app_crash_diagnostic(
    app_id: &str,
    reason: &str,
    last_lifecycle_state: LuaAppLifecycleStateModel,
) -> Result<LuaAppRuntimeDiagnosticModel, LuaAppRuntimeErrorModel> {
    LuaAppRuntimeDiagnosticModel::new(
        app_id,
        reason,
        last_lifecycle_state,
        lua_app_safe_return_target_for_state(last_lifecycle_state),
    )
}

pub fn lua_app_exit_diagnostic(
    app_id: &str,
    last_lifecycle_state: LuaAppLifecycleStateModel,
) -> Result<LuaAppRuntimeDiagnosticModel, LuaAppRuntimeErrorModel> {
    LuaAppRuntimeDiagnosticModel::new(
        app_id,
        "app requested exit",
        last_lifecycle_state,
        lua_app_safe_return_target_for_state(last_lifecycle_state),
    )
}

pub fn lua_app_back_exit_diagnostic(
    app_id: &str,
    last_lifecycle_state: LuaAppLifecycleStateModel,
) -> Result<LuaAppRuntimeDiagnosticModel, LuaAppRuntimeErrorModel> {
    LuaAppRuntimeDiagnosticModel::new(
        app_id,
        "back button exit",
        last_lifecycle_state,
        lua_app_safe_return_target_for_state(last_lifecycle_state),
    )
}

fn copy_runtime_string<const N: usize>(value: &str) -> Result<String<N>, LuaAppRuntimeErrorModel> {
    let mut out = String::new();
    out.push_str(value.trim())
        .map_err(|_| LuaAppRuntimeErrorModel::InvalidAppId)?;
    Ok(out)
}

fn copy_reason_string(
    value: &str,
) -> Result<String<LUA_APP_RUNTIME_REASON_MAX>, LuaAppRuntimeErrorModel> {
    let mut out = String::new();
    out.push_str(value.trim())
        .map_err(|_| LuaAppRuntimeErrorModel::ReasonTooLong)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovered_app_moves_to_running_through_launch_pending() {
        let mut state = LuaAppRuntimeStateModel::discovered("daily_mantra").unwrap();
        assert_eq!(state.lifecycle_state, LuaAppLifecycleStateModel::Discovered);

        assert_eq!(
            state
                .apply_event(LuaAppLifecycleEventModel::Launch)
                .unwrap(),
            LuaAppLifecycleStateModel::LaunchPending
        );
        assert_eq!(
            state
                .apply_event(LuaAppLifecycleEventModel::Launch)
                .unwrap(),
            LuaAppLifecycleStateModel::Running
        );
    }

    #[test]
    fn running_app_can_suspend_resume_and_exit() {
        let mut state =
            LuaAppRuntimeStateModel::new("calendar", LuaAppLifecycleStateModel::Running).unwrap();

        assert_eq!(
            state
                .apply_event(LuaAppLifecycleEventModel::Suspend)
                .unwrap(),
            LuaAppLifecycleStateModel::Suspended
        );
        assert_eq!(
            state
                .apply_event(LuaAppLifecycleEventModel::Resume)
                .unwrap(),
            LuaAppLifecycleStateModel::Running
        );
        assert_eq!(
            state.apply_event(LuaAppLifecycleEventModel::Exit).unwrap(),
            LuaAppLifecycleStateModel::Exiting
        );
    }

    #[test]
    fn invalid_lifecycle_transitions_are_rejected() {
        assert_eq!(
            lua_app_lifecycle_transition(
                LuaAppLifecycleStateModel::Discovered,
                LuaAppLifecycleEventModel::Resume,
            ),
            Err(LuaAppRuntimeErrorModel::InvalidTransition)
        );
        assert_eq!(
            lua_app_lifecycle_transition(
                LuaAppLifecycleStateModel::Running,
                LuaAppLifecycleEventModel::Launch,
            ),
            Err(LuaAppRuntimeErrorModel::InvalidTransition)
        );
        assert_eq!(
            lua_app_lifecycle_transition(
                LuaAppLifecycleStateModel::Disabled,
                LuaAppLifecycleEventModel::Launch,
            ),
            Err(LuaAppRuntimeErrorModel::InvalidTransition)
        );
    }

    #[test]
    fn crash_transition_records_crashed_state() {
        assert_eq!(
            lua_app_lifecycle_transition(
                LuaAppLifecycleStateModel::Suspended,
                LuaAppLifecycleEventModel::Crash,
            )
            .unwrap(),
            LuaAppLifecycleStateModel::Crashed
        );
    }

    #[test]
    fn disable_transition_is_safe_from_terminal_states() {
        assert_eq!(
            lua_app_lifecycle_transition(
                LuaAppLifecycleStateModel::Crashed,
                LuaAppLifecycleEventModel::Disable,
            )
            .unwrap(),
            LuaAppLifecycleStateModel::Disabled
        );
        assert_eq!(
            lua_app_lifecycle_transition(
                LuaAppLifecycleStateModel::Disabled,
                LuaAppLifecycleEventModel::Disable,
            )
            .unwrap(),
            LuaAppLifecycleStateModel::Disabled
        );
    }

    #[test]
    fn back_button_policy_always_maps_to_safe_exit() {
        let states = [
            LuaAppLifecycleStateModel::Discovered,
            LuaAppLifecycleStateModel::LaunchPending,
            LuaAppLifecycleStateModel::Running,
            LuaAppLifecycleStateModel::Suspended,
            LuaAppLifecycleStateModel::Exiting,
            LuaAppLifecycleStateModel::Crashed,
            LuaAppLifecycleStateModel::Disabled,
        ];

        for state in states {
            assert_eq!(
                lua_app_back_event_policy(state),
                LuaAppLifecycleEventModel::Exit
            );
            assert_eq!(
                lua_app_safe_return_target_for_state(state),
                LuaAppSafeReturnTargetModel::Dashboard
            );
            assert!(state.can_accept_back_exit());
        }
    }

    #[test]
    fn crash_and_exit_diagnostics_preserve_safe_return_target() {
        let crash = lua_app_crash_diagnostic(
            "panchang",
            "script panic",
            LuaAppLifecycleStateModel::Running,
        )
        .unwrap();
        assert_eq!(crash.app_id.as_str(), "panchang");
        assert_eq!(crash.reason.as_str(), "script panic");
        assert_eq!(
            crash.last_lifecycle_state,
            LuaAppLifecycleStateModel::Running
        );
        assert!(crash.returns_to_dashboard());

        let exit =
            lua_app_back_exit_diagnostic("calendar", LuaAppLifecycleStateModel::Suspended).unwrap();
        assert_eq!(exit.reason.as_str(), "back button exit");
        assert_eq!(
            exit.safe_return_target,
            LuaAppSafeReturnTargetModel::Dashboard
        );
    }

    #[test]
    fn diagnostics_reject_bad_app_id_and_missing_reason() {
        assert_eq!(
            LuaAppRuntimeStateModel::discovered("../bad"),
            Err(LuaAppRuntimeErrorModel::InvalidAppId)
        );
        assert_eq!(
            lua_app_crash_diagnostic("daily_mantra", "", LuaAppLifecycleStateModel::Running),
            Err(LuaAppRuntimeErrorModel::ReasonRequired)
        );
    }

    #[test]
    fn labels_are_stable_for_docs_and_diagnostics() {
        assert_eq!(
            LuaAppLifecycleStateModel::LaunchPending.label(),
            "LaunchPending"
        );
        assert_eq!(LuaAppLifecycleEventModel::Disable.label(), "disable");
        assert_eq!(LuaAppSafeReturnTargetModel::Dashboard.label(), "dashboard");
        assert_eq!(
            LuaAppRuntimeErrorModel::InvalidTransition.diagnostic(),
            "Lua app lifecycle transition is not allowed"
        );
    }
}
