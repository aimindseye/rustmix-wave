#![allow(dead_code)]

//! Active input semantic mapping adapter.
//!
//! This module is the first active input-semantic takeover step. It does not
//! sample ADC pins, debounce buttons, poll input events, or edit vendored X4
//! code. Instead it owns the semantic button/action mapping contract and returns
//! a behavior-equivalent imported `ButtonMapper` to the still-imported
//! `AppManager` API.

use crate::rustmix_x4::x4_kernel::board::action::{
    Action as ImportedAction, ActionEvent as ImportedActionEvent,
    ButtonMapper as ImportedButtonMapper,
};
use crate::rustmix_x4::x4_kernel::board::button::Button as ImportedButton;
use crate::rustmix_x4::x4_kernel::drivers::input::Event as ImportedInputEvent;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RustmixActiveInputSemanticMapper;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RustmixActiveInputSemanticReport {
    pub default_mapping_ok: bool,
    pub swapped_mapping_ok: bool,
    pub event_mapping_ok: bool,
    pub imported_mapper_factory_ok: bool,
    pub owns_adc_sampling: bool,
    pub owns_debounce_repeat: bool,
}

impl RustmixActiveInputSemanticReport {
    pub const fn takeover_ok(self) -> bool {
        self.default_mapping_ok
            && self.swapped_mapping_ok
            && self.event_mapping_ok
            && self.imported_mapper_factory_ok
            && !self.owns_adc_sampling
            && !self.owns_debounce_repeat
    }
}

impl RustmixActiveInputSemanticMapper {
    pub const IMPLEMENTATION_OWNER: &'static str = "Rustmix-owned active semantic mapper adapter";
    pub const ADC_AND_DEBOUNCE_OWNER: &'static str = "Rustmix-owned X4 runtime";
    pub const ACTIVE_TAKEOVER_SCOPE: &'static str = "semantic mapping only";

    /// Construct the imported mapper behind a Rustmix-owned factory.
    ///
    /// `AppManager` still accepts the imported concrete `ButtonMapper` type.
    /// This module makes construction and equivalence validation Rustmix-owned
    /// without changing ADC/debounce/input-task behavior.
    pub fn new_imported_button_mapper() -> ImportedButtonMapper {
        ImportedButtonMapper::new()
    }

    pub fn configure_imported_button_mapper(mapper: &mut ImportedButtonMapper, swap: bool) {
        mapper.set_swap(swap);
    }

    pub const fn map_button_default(button: ImportedButton) -> ImportedAction {
        match button {
            ImportedButton::VolDown => ImportedAction::Next,
            ImportedButton::VolUp => ImportedAction::Prev,
            ImportedButton::Right => ImportedAction::NextJump,
            ImportedButton::Left => ImportedAction::PrevJump,
            ImportedButton::Confirm => ImportedAction::Select,
            ImportedButton::Back => ImportedAction::Back,
            ImportedButton::Power => ImportedAction::Menu,
        }
    }

    pub const fn map_button_swapped(button: ImportedButton) -> ImportedAction {
        match button {
            ImportedButton::VolDown => ImportedAction::Next,
            ImportedButton::VolUp => ImportedAction::Prev,
            ImportedButton::Right => ImportedAction::Select,
            ImportedButton::Left => ImportedAction::Back,
            ImportedButton::Confirm => ImportedAction::NextJump,
            ImportedButton::Back => ImportedAction::PrevJump,
            ImportedButton::Power => ImportedAction::Menu,
        }
    }

    pub fn map_button(swap: bool, button: ImportedButton) -> ImportedAction {
        if swap {
            Self::map_button_swapped(button)
        } else {
            Self::map_button_default(button)
        }
    }

    pub fn map_event(swap: bool, event: ImportedInputEvent) -> ImportedActionEvent {
        match event {
            ImportedInputEvent::Press(button) => {
                ImportedActionEvent::Press(Self::map_button(swap, button))
            }
            ImportedInputEvent::Release(button) => {
                ImportedActionEvent::Release(Self::map_button(swap, button))
            }
            ImportedInputEvent::LongPress(button) => {
                ImportedActionEvent::LongPress(Self::map_button(swap, button))
            }
            ImportedInputEvent::Repeat(button) => {
                ImportedActionEvent::Repeat(Self::map_button(swap, button))
            }
        }
    }

    pub fn active_runtime_preflight_report() -> RustmixActiveInputSemanticReport {
        RustmixActiveInputSemanticReport {
            default_mapping_ok: Self::imported_default_mapping_matches_rustmix(),
            swapped_mapping_ok: Self::imported_swapped_mapping_matches_rustmix(),
            event_mapping_ok: Self::imported_event_mapping_matches_rustmix(),
            imported_mapper_factory_ok: !Self::new_imported_button_mapper().is_swapped(),
            owns_adc_sampling: false,
            owns_debounce_repeat: false,
        }
    }

    pub fn active_runtime_preflight() -> bool {
        Self::active_runtime_preflight_report().takeover_ok()
    }

    fn imported_default_mapping_matches_rustmix() -> bool {
        let mapper = ImportedButtonMapper::new();
        !mapper.is_swapped()
            && mapper.map_button(ImportedButton::VolDown)
                == Self::map_button_default(ImportedButton::VolDown)
            && mapper.map_button(ImportedButton::VolUp)
                == Self::map_button_default(ImportedButton::VolUp)
            && mapper.map_button(ImportedButton::Right)
                == Self::map_button_default(ImportedButton::Right)
            && mapper.map_button(ImportedButton::Left)
                == Self::map_button_default(ImportedButton::Left)
            && mapper.map_button(ImportedButton::Confirm)
                == Self::map_button_default(ImportedButton::Confirm)
            && mapper.map_button(ImportedButton::Back)
                == Self::map_button_default(ImportedButton::Back)
            && mapper.map_button(ImportedButton::Power)
                == Self::map_button_default(ImportedButton::Power)
    }

    fn imported_swapped_mapping_matches_rustmix() -> bool {
        let mut mapper = ImportedButtonMapper::new();
        Self::configure_imported_button_mapper(&mut mapper, true);
        mapper.is_swapped()
            && mapper.map_button(ImportedButton::VolDown)
                == Self::map_button_swapped(ImportedButton::VolDown)
            && mapper.map_button(ImportedButton::VolUp)
                == Self::map_button_swapped(ImportedButton::VolUp)
            && mapper.map_button(ImportedButton::Right)
                == Self::map_button_swapped(ImportedButton::Right)
            && mapper.map_button(ImportedButton::Left)
                == Self::map_button_swapped(ImportedButton::Left)
            && mapper.map_button(ImportedButton::Confirm)
                == Self::map_button_swapped(ImportedButton::Confirm)
            && mapper.map_button(ImportedButton::Back)
                == Self::map_button_swapped(ImportedButton::Back)
            && mapper.map_button(ImportedButton::Power)
                == Self::map_button_swapped(ImportedButton::Power)
    }

    fn imported_event_mapping_matches_rustmix() -> bool {
        let mapper = ImportedButtonMapper::new();
        mapper.map_event(ImportedInputEvent::Press(ImportedButton::Confirm))
            == Self::map_event(false, ImportedInputEvent::Press(ImportedButton::Confirm))
            && mapper.map_event(ImportedInputEvent::Repeat(ImportedButton::VolDown))
                == Self::map_event(false, ImportedInputEvent::Repeat(ImportedButton::VolDown))
            && mapper.map_event(ImportedInputEvent::LongPress(ImportedButton::Back))
                == Self::map_event(false, ImportedInputEvent::LongPress(ImportedButton::Back))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ImportedAction, ImportedButton, ImportedInputEvent, RustmixActiveInputSemanticMapper,
    };

    #[test]
    fn active_runtime_preflight_passes() {
        assert!(RustmixActiveInputSemanticMapper::active_runtime_preflight());
    }

    #[test]
    fn default_mapping_matches_expected_reader_actions() {
        assert_eq!(
            RustmixActiveInputSemanticMapper::map_button_default(ImportedButton::Confirm),
            ImportedAction::Select
        );
        assert_eq!(
            RustmixActiveInputSemanticMapper::map_button_default(ImportedButton::Back),
            ImportedAction::Back
        );
        assert_eq!(
            RustmixActiveInputSemanticMapper::map_button_default(ImportedButton::VolDown),
            ImportedAction::Next
        );
        assert_eq!(
            RustmixActiveInputSemanticMapper::map_button_default(ImportedButton::VolUp),
            ImportedAction::Prev
        );
    }

    #[test]
    fn event_mapping_preserves_event_kind() {
        let mapped = RustmixActiveInputSemanticMapper::map_event(
            false,
            ImportedInputEvent::Repeat(ImportedButton::VolDown),
        );
        assert_eq!(mapped.action(), ImportedAction::Next);
        assert!(mapped.is_repeat());
    }
}
