#![allow(dead_code)]

pub struct RustmixInputSemantics;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RustmixSemanticButtonRole {
    Back,
    Select,
    Up,
    Down,
    Left,
    Right,
    Power,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RustmixReaderAction {
    BackToLibrary,
    OpenOrSelect,
    NextPage,
    PreviousPage,
    BookmarkOrMenu,
    Stay,
    Unsupported,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RustmixNavigationAction {
    Up,
    Down,
    Left,
    Right,
    None,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RustmixInputPinContract {
    pub row1_adc_gpio: u8,
    pub row2_adc_gpio: u8,
    pub power_gpio: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RustmixInputSemanticAdoptionReport {
    pub pins_ok: bool,
    pub roles_ok: bool,
    pub reader_actions_ok: bool,
    pub navigation_actions_ok: bool,
    pub physical_input_moved: bool,
}

impl RustmixInputSemanticAdoptionReport {
    pub const fn adoption_ok(self) -> bool {
        self.pins_ok
            && self.roles_ok
            && self.reader_actions_ok
            && self.navigation_actions_ok
            && !self.physical_input_moved
    }
}

impl RustmixInputSemantics {
    pub const IMPLEMENTATION_OWNER: &'static str = "Rustmix-owned pure input semantic helpers";
    pub const PHYSICAL_INPUT_OWNER: &'static str = "Rustmix-owned X4 runtime";
    pub const PHYSICAL_INPUT_MOVED_TO_BOUNDARY: bool = false;

    pub const ROW1_ADC_GPIO: u8 = 1;
    pub const ROW2_ADC_GPIO: u8 = 2;
    pub const POWER_BUTTON_GPIO: u8 = 3;

    pub const BUTTON_ROLES: [RustmixSemanticButtonRole; 7] = [
        RustmixSemanticButtonRole::Back,
        RustmixSemanticButtonRole::Select,
        RustmixSemanticButtonRole::Up,
        RustmixSemanticButtonRole::Down,
        RustmixSemanticButtonRole::Left,
        RustmixSemanticButtonRole::Right,
        RustmixSemanticButtonRole::Power,
    ];

    pub const fn pin_contract() -> RustmixInputPinContract {
        RustmixInputPinContract {
            row1_adc_gpio: Self::ROW1_ADC_GPIO,
            row2_adc_gpio: Self::ROW2_ADC_GPIO,
            power_gpio: Self::POWER_BUTTON_GPIO,
        }
    }

    pub const fn is_known_input_pin(gpio: u8) -> bool {
        gpio == Self::ROW1_ADC_GPIO
            || gpio == Self::ROW2_ADC_GPIO
            || gpio == Self::POWER_BUTTON_GPIO
    }

    pub const fn role_name(role: RustmixSemanticButtonRole) -> &'static str {
        match role {
            RustmixSemanticButtonRole::Back => "Back",
            RustmixSemanticButtonRole::Select => "Select",
            RustmixSemanticButtonRole::Up => "Up",
            RustmixSemanticButtonRole::Down => "Down",
            RustmixSemanticButtonRole::Left => "Left",
            RustmixSemanticButtonRole::Right => "Right",
            RustmixSemanticButtonRole::Power => "Power",
            RustmixSemanticButtonRole::Unknown => "Unknown",
        }
    }

    pub const fn is_navigation_role(role: RustmixSemanticButtonRole) -> bool {
        matches!(
            role,
            RustmixSemanticButtonRole::Up
                | RustmixSemanticButtonRole::Down
                | RustmixSemanticButtonRole::Left
                | RustmixSemanticButtonRole::Right
        )
    }

    pub const fn navigation_action_for_role(
        role: RustmixSemanticButtonRole,
    ) -> RustmixNavigationAction {
        match role {
            RustmixSemanticButtonRole::Up => RustmixNavigationAction::Up,
            RustmixSemanticButtonRole::Down => RustmixNavigationAction::Down,
            RustmixSemanticButtonRole::Left => RustmixNavigationAction::Left,
            RustmixSemanticButtonRole::Right => RustmixNavigationAction::Right,
            _ => RustmixNavigationAction::None,
        }
    }

    pub const fn reader_action_for_role(role: RustmixSemanticButtonRole) -> RustmixReaderAction {
        match role {
            RustmixSemanticButtonRole::Back | RustmixSemanticButtonRole::Left => {
                RustmixReaderAction::BackToLibrary
            }
            RustmixSemanticButtonRole::Select => RustmixReaderAction::OpenOrSelect,
            RustmixSemanticButtonRole::Right | RustmixSemanticButtonRole::Down => {
                RustmixReaderAction::NextPage
            }
            RustmixSemanticButtonRole::Up => RustmixReaderAction::PreviousPage,
            RustmixSemanticButtonRole::Power => RustmixReaderAction::BookmarkOrMenu,
            RustmixSemanticButtonRole::Unknown => RustmixReaderAction::Unsupported,
        }
    }

    pub fn input_semantics_adoption_report() -> RustmixInputSemanticAdoptionReport {
        let pins = Self::pin_contract();

        RustmixInputSemanticAdoptionReport {
            pins_ok: pins.row1_adc_gpio == 1
                && pins.row2_adc_gpio == 2
                && pins.power_gpio == 3
                && Self::is_known_input_pin(1)
                && Self::is_known_input_pin(2)
                && Self::is_known_input_pin(3)
                && !Self::is_known_input_pin(99),
            roles_ok: Self::BUTTON_ROLES.len() == 7
                && Self::role_name(RustmixSemanticButtonRole::Back) == "Back"
                && Self::role_name(RustmixSemanticButtonRole::Select) == "Select"
                && Self::role_name(RustmixSemanticButtonRole::Power) == "Power",
            reader_actions_ok: Self::reader_action_for_role(RustmixSemanticButtonRole::Back)
                == RustmixReaderAction::BackToLibrary
                && Self::reader_action_for_role(RustmixSemanticButtonRole::Select)
                    == RustmixReaderAction::OpenOrSelect
                && Self::reader_action_for_role(RustmixSemanticButtonRole::Right)
                    == RustmixReaderAction::NextPage
                && Self::reader_action_for_role(RustmixSemanticButtonRole::Up)
                    == RustmixReaderAction::PreviousPage
                && Self::reader_action_for_role(RustmixSemanticButtonRole::Power)
                    == RustmixReaderAction::BookmarkOrMenu,
            navigation_actions_ok: Self::navigation_action_for_role(RustmixSemanticButtonRole::Up)
                == RustmixNavigationAction::Up
                && Self::navigation_action_for_role(RustmixSemanticButtonRole::Down)
                    == RustmixNavigationAction::Down
                && Self::navigation_action_for_role(RustmixSemanticButtonRole::Left)
                    == RustmixNavigationAction::Left
                && Self::navigation_action_for_role(RustmixSemanticButtonRole::Right)
                    == RustmixNavigationAction::Right,
            physical_input_moved: Self::PHYSICAL_INPUT_MOVED_TO_BOUNDARY,
        }
    }

    pub fn active_runtime_adoption_probe() -> bool {
        Self::input_semantics_adoption_report().adoption_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::{RustmixInputSemantics, RustmixReaderAction, RustmixSemanticButtonRole};

    #[test]
    fn input_semantics_adoption_probe_is_pure_and_valid() {
        assert!(RustmixInputSemantics::active_runtime_adoption_probe());
    }

    #[test]
    fn maps_reader_semantic_actions() {
        assert_eq!(
            RustmixInputSemantics::reader_action_for_role(RustmixSemanticButtonRole::Back),
            RustmixReaderAction::BackToLibrary
        );
        assert_eq!(
            RustmixInputSemantics::reader_action_for_role(RustmixSemanticButtonRole::Select),
            RustmixReaderAction::OpenOrSelect
        );
    }
}
