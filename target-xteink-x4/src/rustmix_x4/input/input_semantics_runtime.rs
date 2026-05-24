#![allow(dead_code)]

pub struct RustmixInputSemanticsRuntimeBridge;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RustmixPhysicalButton {
    Right,
    Left,
    Confirm,
    Back,
    VolUp,
    VolDown,
    Power,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RustmixRuntimeInputAction {
    Next,
    Previous,
    NextJump,
    PreviousJump,
    Select,
    Back,
    Menu,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RustmixRuntimeInputEventKind {
    Press,
    Release,
    LongPress,
    Repeat,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RustmixRuntimeInputEvent {
    pub kind: RustmixRuntimeInputEventKind,
    pub action: RustmixRuntimeInputAction,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RustmixRuntimeButtonMapper {
    swap_buttons: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RustmixInputSemanticsRuntimeReport {
    pub default_layout_ok: bool,
    pub swapped_layout_ok: bool,
    pub event_mapping_ok: bool,
    pub physical_input_sampling_owned: bool,
    pub debounce_repeat_owned: bool,
}

impl RustmixInputSemanticsRuntimeReport {
    pub const fn preflight_ok(self) -> bool {
        self.default_layout_ok
            && self.swapped_layout_ok
            && self.event_mapping_ok
            && !self.physical_input_sampling_owned
            && !self.debounce_repeat_owned
    }
}

impl RustmixRuntimeButtonMapper {
    pub const fn new() -> Self {
        Self {
            swap_buttons: false,
        }
    }

    pub const fn swapped() -> Self {
        Self { swap_buttons: true }
    }

    pub const fn is_swapped(self) -> bool {
        self.swap_buttons
    }

    pub const fn map_button(self, button: RustmixPhysicalButton) -> RustmixRuntimeInputAction {
        if self.swap_buttons {
            match button {
                RustmixPhysicalButton::VolDown => RustmixRuntimeInputAction::Next,
                RustmixPhysicalButton::VolUp => RustmixRuntimeInputAction::Previous,
                RustmixPhysicalButton::Right => RustmixRuntimeInputAction::Select,
                RustmixPhysicalButton::Left => RustmixRuntimeInputAction::Back,
                RustmixPhysicalButton::Confirm => RustmixRuntimeInputAction::NextJump,
                RustmixPhysicalButton::Back => RustmixRuntimeInputAction::PreviousJump,
                RustmixPhysicalButton::Power => RustmixRuntimeInputAction::Menu,
            }
        } else {
            match button {
                RustmixPhysicalButton::VolDown => RustmixRuntimeInputAction::Next,
                RustmixPhysicalButton::VolUp => RustmixRuntimeInputAction::Previous,
                RustmixPhysicalButton::Right => RustmixRuntimeInputAction::NextJump,
                RustmixPhysicalButton::Left => RustmixRuntimeInputAction::PreviousJump,
                RustmixPhysicalButton::Confirm => RustmixRuntimeInputAction::Select,
                RustmixPhysicalButton::Back => RustmixRuntimeInputAction::Back,
                RustmixPhysicalButton::Power => RustmixRuntimeInputAction::Menu,
            }
        }
    }

    pub const fn map_event(
        self,
        kind: RustmixRuntimeInputEventKind,
        button: RustmixPhysicalButton,
    ) -> RustmixRuntimeInputEvent {
        RustmixRuntimeInputEvent {
            kind,
            action: self.map_button(button),
        }
    }
}

impl RustmixInputSemanticsRuntimeBridge {
    pub const IMPLEMENTATION_OWNER: &'static str = "Rustmix-owned input semantic runtime facade";
    pub const PHYSICAL_INPUT_SAMPLING_OWNER: &'static str = "Rustmix-owned X4 runtime";
    pub const DEBOUNCE_REPEAT_OWNER: &'static str = "Rustmix-owned X4 runtime";
    pub const PHYSICAL_INPUT_SAMPLING_OWNED_BY_BRIDGE: bool = false;
    pub const DEBOUNCE_REPEAT_OWNED_BY_BRIDGE: bool = false;

    pub fn active_runtime_preflight() -> bool {
        Self::preflight_report().preflight_ok()
    }

    pub fn preflight_report() -> RustmixInputSemanticsRuntimeReport {
        RustmixInputSemanticsRuntimeReport {
            default_layout_ok: Self::default_layout_ok(),
            swapped_layout_ok: Self::swapped_layout_ok(),
            event_mapping_ok: Self::event_mapping_ok(),
            physical_input_sampling_owned: Self::PHYSICAL_INPUT_SAMPLING_OWNED_BY_BRIDGE,
            debounce_repeat_owned: Self::DEBOUNCE_REPEAT_OWNED_BY_BRIDGE,
        }
    }

    fn default_layout_ok() -> bool {
        let mapper = RustmixRuntimeButtonMapper::new();
        !mapper.is_swapped()
            && mapper.map_button(RustmixPhysicalButton::VolDown) == RustmixRuntimeInputAction::Next
            && mapper.map_button(RustmixPhysicalButton::VolUp)
                == RustmixRuntimeInputAction::Previous
            && mapper.map_button(RustmixPhysicalButton::Right)
                == RustmixRuntimeInputAction::NextJump
            && mapper.map_button(RustmixPhysicalButton::Left)
                == RustmixRuntimeInputAction::PreviousJump
            && mapper.map_button(RustmixPhysicalButton::Confirm)
                == RustmixRuntimeInputAction::Select
            && mapper.map_button(RustmixPhysicalButton::Back) == RustmixRuntimeInputAction::Back
            && mapper.map_button(RustmixPhysicalButton::Power) == RustmixRuntimeInputAction::Menu
    }

    fn swapped_layout_ok() -> bool {
        let mapper = RustmixRuntimeButtonMapper::swapped();
        mapper.is_swapped()
            && mapper.map_button(RustmixPhysicalButton::VolDown) == RustmixRuntimeInputAction::Next
            && mapper.map_button(RustmixPhysicalButton::VolUp)
                == RustmixRuntimeInputAction::Previous
            && mapper.map_button(RustmixPhysicalButton::Right) == RustmixRuntimeInputAction::Select
            && mapper.map_button(RustmixPhysicalButton::Left) == RustmixRuntimeInputAction::Back
            && mapper.map_button(RustmixPhysicalButton::Confirm)
                == RustmixRuntimeInputAction::NextJump
            && mapper.map_button(RustmixPhysicalButton::Back)
                == RustmixRuntimeInputAction::PreviousJump
            && mapper.map_button(RustmixPhysicalButton::Power) == RustmixRuntimeInputAction::Menu
    }

    fn event_mapping_ok() -> bool {
        let mapper = RustmixRuntimeButtonMapper::new();
        mapper.map_event(
            RustmixRuntimeInputEventKind::Press,
            RustmixPhysicalButton::Confirm,
        ) == RustmixRuntimeInputEvent {
            kind: RustmixRuntimeInputEventKind::Press,
            action: RustmixRuntimeInputAction::Select,
        } && mapper.map_event(
            RustmixRuntimeInputEventKind::Repeat,
            RustmixPhysicalButton::VolDown,
        ) == RustmixRuntimeInputEvent {
            kind: RustmixRuntimeInputEventKind::Repeat,
            action: RustmixRuntimeInputAction::Next,
        } && mapper.map_event(
            RustmixRuntimeInputEventKind::LongPress,
            RustmixPhysicalButton::Back,
        ) == RustmixRuntimeInputEvent {
            kind: RustmixRuntimeInputEventKind::LongPress,
            action: RustmixRuntimeInputAction::Back,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        RustmixInputSemanticsRuntimeBridge, RustmixPhysicalButton, RustmixRuntimeButtonMapper,
        RustmixRuntimeInputAction,
    };

    #[test]
    fn runtime_input_semantics_probe_is_pure_and_valid() {
        assert!(RustmixInputSemanticsRuntimeBridge::active_runtime_preflight());
    }

    #[test]
    fn maps_default_and_swapped_layouts_like_active_runtime() {
        let default_mapper = RustmixRuntimeButtonMapper::new();
        assert_eq!(
            default_mapper.map_button(RustmixPhysicalButton::Confirm),
            RustmixRuntimeInputAction::Select
        );
        assert_eq!(
            default_mapper.map_button(RustmixPhysicalButton::Right),
            RustmixRuntimeInputAction::NextJump
        );

        let swapped = RustmixRuntimeButtonMapper::swapped();
        assert_eq!(
            swapped.map_button(RustmixPhysicalButton::Right),
            RustmixRuntimeInputAction::Select
        );
        assert_eq!(
            swapped.map_button(RustmixPhysicalButton::Confirm),
            RustmixRuntimeInputAction::NextJump
        );
    }
}
