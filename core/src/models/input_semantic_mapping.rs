use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputSemanticActionModel {
    Up,
    Down,
    Left,
    Right,
    Select,
    Back,
    Menu,
    Power,
    #[default]
    Unknown,
}

impl InputSemanticActionModel {
    pub const fn is_directional(self) -> bool {
        matches!(self, Self::Up | Self::Down | Self::Left | Self::Right)
    }

    pub const fn is_navigation(self) -> bool {
        self.is_directional() || matches!(self, Self::Select | Self::Back | Self::Menu)
    }

    pub const fn is_activation(self) -> bool {
        matches!(self, Self::Select)
    }

    pub const fn is_exit(self) -> bool {
        matches!(self, Self::Back)
    }

    pub const fn is_system(self) -> bool {
        matches!(self, Self::Power)
    }

    pub const fn action_kind(self) -> InputActionKindModel {
        if self.is_directional() {
            InputActionKindModel::Navigation
        } else if self.is_activation() {
            InputActionKindModel::Activate
        } else if self.is_exit() {
            InputActionKindModel::Exit
        } else if matches!(self, Self::Menu) {
            InputActionKindModel::Menu
        } else if self.is_system() {
            InputActionKindModel::System
        } else {
            InputActionKindModel::Unknown
        }
    }

    pub const fn repeat_policy(self) -> InputRepeatPolicyModel {
        match self {
            Self::Up | Self::Down | Self::Left | Self::Right => InputRepeatPolicyModel::Repeatable,
            Self::Select | Self::Back | Self::Menu | Self::Power => InputRepeatPolicyModel::OneShot,
            Self::Unknown => InputRepeatPolicyModel::Disabled,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputPhysicalButtonModel {
    Up,
    Down,
    Left,
    Right,
    Select,
    Back,
    Menu,
    Power,
    #[default]
    Unknown,
}

impl InputPhysicalButtonModel {
    pub const fn semantic_default(self) -> InputSemanticActionModel {
        match self {
            Self::Up => InputSemanticActionModel::Up,
            Self::Down => InputSemanticActionModel::Down,
            Self::Left => InputSemanticActionModel::Left,
            Self::Right => InputSemanticActionModel::Right,
            Self::Select => InputSemanticActionModel::Select,
            Self::Back => InputSemanticActionModel::Back,
            Self::Menu => InputSemanticActionModel::Menu,
            Self::Power => InputSemanticActionModel::Power,
            Self::Unknown => InputSemanticActionModel::Unknown,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputAppContextModel {
    Home,
    Files,
    Reader,
    Settings,
    DateTime,
    WifiTransfer,
    #[default]
    Unknown,
}

impl InputAppContextModel {
    pub const fn allows_safe_back(self) -> bool {
        matches!(
            self,
            Self::Home
                | Self::Files
                | Self::Reader
                | Self::Settings
                | Self::DateTime
                | Self::WifiTransfer
        )
    }

    pub const fn has_repeatable_navigation(self) -> bool {
        matches!(
            self,
            Self::Home | Self::Files | Self::Reader | Self::Settings
        )
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputActionKindModel {
    Navigation,
    Activate,
    Exit,
    Menu,
    System,
    #[default]
    Unknown,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputRepeatPolicyModel {
    Repeatable,
    OneShot,
    #[default]
    Disabled,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReaderPageNavigationModel {
    PreviousPage,
    NextPage,
    ExitToLibrary,
    OpenMenu,
    #[default]
    None,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputSemanticMappingModel {
    pub context: InputAppContextModel,
    pub physical: InputPhysicalButtonModel,
    pub action: InputSemanticActionModel,
    pub repeat: InputRepeatPolicyModel,
    pub safe_back: bool,
}

impl InputSemanticMappingModel {
    pub const fn new(context: InputAppContextModel, physical: InputPhysicalButtonModel) -> Self {
        let action = semantic_action_for_physical(context, physical);
        Self {
            context,
            physical,
            action,
            repeat: repeat_policy_for_context(context, action),
            safe_back: is_safe_back_action(context, action),
        }
    }
}

pub fn normalize_physical_button_name(name: &str) -> InputPhysicalButtonModel {
    let mut compact = [0u8; 24];
    let mut len = 0usize;

    for b in name.bytes() {
        if len >= compact.len() {
            break;
        }
        if b.is_ascii_alphanumeric() {
            compact[len] = b.to_ascii_lowercase();
            len += 1;
        }
    }

    match &compact[..len] {
        b"up" | b"u" | b"north" | b"top" | b"btnup" | b"buttonup" => InputPhysicalButtonModel::Up,
        b"down" | b"d" | b"south" | b"bottom" | b"btndown" | b"buttondown" => {
            InputPhysicalButtonModel::Down
        }
        b"left" | b"l" | b"west" | b"prev" | b"previous" | b"btnleft" => {
            InputPhysicalButtonModel::Left
        }
        b"right" | b"r" | b"east" | b"next" | b"btnright" => InputPhysicalButtonModel::Right,
        b"select" | b"sel" | b"ok" | b"enter" | b"center" | b"middle" | b"confirm" => {
            InputPhysicalButtonModel::Select
        }
        b"back" | b"return" | b"cancel" | b"escape" | b"esc" => InputPhysicalButtonModel::Back,
        b"menu" | b"settings" | b"options" => InputPhysicalButtonModel::Menu,
        b"power" | b"sleep" | b"wake" | b"pwr" => InputPhysicalButtonModel::Power,
        _ => InputPhysicalButtonModel::Unknown,
    }
}

pub const fn semantic_action_for_physical(
    context: InputAppContextModel,
    physical: InputPhysicalButtonModel,
) -> InputSemanticActionModel {
    match context {
        InputAppContextModel::Reader => reader_action_for_physical(physical),
        InputAppContextModel::DateTime => date_time_action_for_physical(physical),
        InputAppContextModel::WifiTransfer => wifi_transfer_action_for_physical(physical),
        InputAppContextModel::Home
        | InputAppContextModel::Files
        | InputAppContextModel::Settings
        | InputAppContextModel::Unknown => physical.semantic_default(),
    }
}

pub const fn reader_action_for_physical(
    physical: InputPhysicalButtonModel,
) -> InputSemanticActionModel {
    match physical {
        InputPhysicalButtonModel::Up | InputPhysicalButtonModel::Left => {
            InputSemanticActionModel::Left
        }
        InputPhysicalButtonModel::Down | InputPhysicalButtonModel::Right => {
            InputSemanticActionModel::Right
        }
        InputPhysicalButtonModel::Select => InputSemanticActionModel::Select,
        InputPhysicalButtonModel::Back => InputSemanticActionModel::Back,
        InputPhysicalButtonModel::Menu => InputSemanticActionModel::Menu,
        InputPhysicalButtonModel::Power => InputSemanticActionModel::Power,
        InputPhysicalButtonModel::Unknown => InputSemanticActionModel::Unknown,
    }
}

pub const fn date_time_action_for_physical(
    physical: InputPhysicalButtonModel,
) -> InputSemanticActionModel {
    match physical {
        InputPhysicalButtonModel::Back => InputSemanticActionModel::Back,
        InputPhysicalButtonModel::Power => InputSemanticActionModel::Power,
        InputPhysicalButtonModel::Unknown => InputSemanticActionModel::Unknown,
        _ => physical.semantic_default(),
    }
}

pub const fn wifi_transfer_action_for_physical(
    physical: InputPhysicalButtonModel,
) -> InputSemanticActionModel {
    match physical {
        InputPhysicalButtonModel::Back => InputSemanticActionModel::Back,
        InputPhysicalButtonModel::Power => InputSemanticActionModel::Power,
        _ => InputSemanticActionModel::Unknown,
    }
}

pub const fn repeat_policy_for_context(
    context: InputAppContextModel,
    action: InputSemanticActionModel,
) -> InputRepeatPolicyModel {
    match action {
        InputSemanticActionModel::Up
        | InputSemanticActionModel::Down
        | InputSemanticActionModel::Left
        | InputSemanticActionModel::Right => {
            if matches!(context, InputAppContextModel::WifiTransfer) {
                InputRepeatPolicyModel::Disabled
            } else {
                InputRepeatPolicyModel::Repeatable
            }
        }
        InputSemanticActionModel::Select
        | InputSemanticActionModel::Back
        | InputSemanticActionModel::Menu
        | InputSemanticActionModel::Power => InputRepeatPolicyModel::OneShot,
        InputSemanticActionModel::Unknown => InputRepeatPolicyModel::Disabled,
    }
}

pub const fn is_safe_back_action(
    context: InputAppContextModel,
    action: InputSemanticActionModel,
) -> bool {
    matches!(action, InputSemanticActionModel::Back) && context.allows_safe_back()
}

pub const fn reader_page_navigation_for_action(
    action: InputSemanticActionModel,
) -> ReaderPageNavigationModel {
    match action {
        InputSemanticActionModel::Left | InputSemanticActionModel::Up => {
            ReaderPageNavigationModel::PreviousPage
        }
        InputSemanticActionModel::Right
        | InputSemanticActionModel::Down
        | InputSemanticActionModel::Select => ReaderPageNavigationModel::NextPage,
        InputSemanticActionModel::Back => ReaderPageNavigationModel::ExitToLibrary,
        InputSemanticActionModel::Menu => ReaderPageNavigationModel::OpenMenu,
        InputSemanticActionModel::Power | InputSemanticActionModel::Unknown => {
            ReaderPageNavigationModel::None
        }
    }
}

pub const fn reader_page_navigation_for_physical(
    physical: InputPhysicalButtonModel,
) -> ReaderPageNavigationModel {
    reader_page_navigation_for_action(reader_action_for_physical(physical))
}

pub const fn context_name(context: InputAppContextModel) -> &'static str {
    match context {
        InputAppContextModel::Home => "Home",
        InputAppContextModel::Files => "Files/Library",
        InputAppContextModel::Reader => "Reader",
        InputAppContextModel::Settings => "Settings",
        InputAppContextModel::DateTime => "Date & Time",
        InputAppContextModel::WifiTransfer => "Wi-Fi Transfer",
        InputAppContextModel::Unknown => "Unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn physical_button_names_normalize_common_aliases() {
        assert_eq!(
            normalize_physical_button_name("OK"),
            InputPhysicalButtonModel::Select
        );
        assert_eq!(
            normalize_physical_button_name("btn-left"),
            InputPhysicalButtonModel::Left
        );
        assert_eq!(
            normalize_physical_button_name("Escape"),
            InputPhysicalButtonModel::Back
        );
        assert_eq!(
            normalize_physical_button_name("PWR"),
            InputPhysicalButtonModel::Power
        );
    }

    #[test]
    fn reader_maps_page_forward_and_backward() {
        assert_eq!(
            reader_page_navigation_for_physical(InputPhysicalButtonModel::Right),
            ReaderPageNavigationModel::NextPage
        );
        assert_eq!(
            reader_page_navigation_for_physical(InputPhysicalButtonModel::Down),
            ReaderPageNavigationModel::NextPage
        );
        assert_eq!(
            reader_page_navigation_for_physical(InputPhysicalButtonModel::Left),
            ReaderPageNavigationModel::PreviousPage
        );
        assert_eq!(
            reader_page_navigation_for_physical(InputPhysicalButtonModel::Up),
            ReaderPageNavigationModel::PreviousPage
        );
    }

    #[test]
    fn back_returns_from_reader_without_becoming_repeatable() {
        let mapping = InputSemanticMappingModel::new(
            InputAppContextModel::Reader,
            InputPhysicalButtonModel::Back,
        );
        assert_eq!(mapping.action, InputSemanticActionModel::Back);
        assert_eq!(mapping.repeat, InputRepeatPolicyModel::OneShot);
        assert!(mapping.safe_back);
        assert_eq!(
            reader_page_navigation_for_action(mapping.action),
            ReaderPageNavigationModel::ExitToLibrary
        );
    }

    #[test]
    fn settings_row_navigation_and_selection_are_classified() {
        let up = InputSemanticMappingModel::new(
            InputAppContextModel::Settings,
            InputPhysicalButtonModel::Up,
        );
        let select = InputSemanticMappingModel::new(
            InputAppContextModel::Settings,
            InputPhysicalButtonModel::Select,
        );
        let right = InputSemanticMappingModel::new(
            InputAppContextModel::Settings,
            InputPhysicalButtonModel::Right,
        );
        assert_eq!(up.action.action_kind(), InputActionKindModel::Navigation);
        assert_eq!(up.repeat, InputRepeatPolicyModel::Repeatable);
        assert_eq!(select.action.action_kind(), InputActionKindModel::Activate);
        assert_eq!(select.repeat, InputRepeatPolicyModel::OneShot);
        assert_eq!(right.action, InputSemanticActionModel::Right);
    }

    #[test]
    fn date_time_back_is_safe_cancel_path() {
        let back = InputSemanticMappingModel::new(
            InputAppContextModel::DateTime,
            InputPhysicalButtonModel::Back,
        );
        assert_eq!(back.action, InputSemanticActionModel::Back);
        assert!(back.safe_back);
        assert_eq!(back.repeat, InputRepeatPolicyModel::OneShot);
    }

    #[test]
    fn home_category_navigation_is_repeatable() {
        let down = InputSemanticMappingModel::new(
            InputAppContextModel::Home,
            InputPhysicalButtonModel::Down,
        );
        let select = InputSemanticMappingModel::new(
            InputAppContextModel::Home,
            InputPhysicalButtonModel::Select,
        );
        assert_eq!(down.action, InputSemanticActionModel::Down);
        assert_eq!(down.repeat, InputRepeatPolicyModel::Repeatable);
        assert_eq!(select.action, InputSemanticActionModel::Select);
    }

    #[test]
    fn wifi_transfer_accepts_only_exit_or_power_buttons() {
        let down = InputSemanticMappingModel::new(
            InputAppContextModel::WifiTransfer,
            InputPhysicalButtonModel::Down,
        );
        let back = InputSemanticMappingModel::new(
            InputAppContextModel::WifiTransfer,
            InputPhysicalButtonModel::Back,
        );
        assert_eq!(down.action, InputSemanticActionModel::Unknown);
        assert_eq!(down.repeat, InputRepeatPolicyModel::Disabled);
        assert_eq!(back.action, InputSemanticActionModel::Back);
        assert!(back.safe_back);
    }

    #[test]
    fn power_remains_special_non_navigation() {
        let mapping = InputSemanticMappingModel::new(
            InputAppContextModel::Reader,
            InputPhysicalButtonModel::Power,
        );
        assert_eq!(mapping.action, InputSemanticActionModel::Power);
        assert_eq!(mapping.action.action_kind(), InputActionKindModel::System);
        assert!(!mapping.action.is_navigation());
        assert_eq!(mapping.repeat, InputRepeatPolicyModel::OneShot);
        assert_eq!(
            reader_page_navigation_for_action(mapping.action),
            ReaderPageNavigationModel::None
        );
    }
}
