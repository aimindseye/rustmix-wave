#![allow(dead_code)]

/// Rustmix-owned input boundary metadata for the Xteink X4 target.
///
/// The current implementation intentionally does not move physical ADC reads, debounce/repeat
/// handling, or button ladder calibration. The working implementation remains
/// in the imported X4/X4 runtime while Rustmix records the contract it will
/// own in later extraction steps.
pub struct RustmixInputBoundary;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RustmixButtonRole {
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
pub enum RustmixInputOwner {
    ImportedX4Runtime,
    RustmixBoundaryMetadata,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RustmixButtonRoleInfo {
    pub role: RustmixButtonRole,
    pub label: &'static str,
    pub imported_owner: RustmixInputOwner,
}

impl RustmixInputBoundary {
    pub const INPUT_BOUNDARY_MARKER: &'static str = "x4-input-boundary-ok";

    /// Current source of truth for physical input behavior.
    pub const IMPLEMENTATION_OWNER: &'static str = "Rustmix-owned X4 runtime";

    /// X4 ADC ladder / button GPIO metadata.
    pub const ROW1_ADC_GPIO: u8 = 1;
    pub const ROW2_ADC_GPIO: u8 = 2;
    pub const POWER_BUTTON_GPIO: u8 = 3;

    /// The current implementation records the boundary only. It does not move runtime behavior.
    pub const PHYSICAL_ADC_READS_MOVED_TO_BOUNDARY: bool = false;
    pub const BUTTON_LADDER_CALIBRATION_MOVED_TO_BOUNDARY: bool = false;
    pub const DEBOUNCE_REPEAT_HANDLING_MOVED_TO_BOUNDARY: bool = false;
    pub const BUTTON_EVENT_ROUTING_MOVED_TO_BOUNDARY: bool = false;

    /// Reader footer/action labels expected by Rustmix. The imported runtime
    /// continues to render and route actions in The current implementation
    pub const READER_FOOTER_ACTION_LABELS: [&'static str; 4] = ["Back", "Select", "Open", "Stay"];

    /// Role order used for documentation/checking. This is not the physical
    /// ADC threshold order; threshold ownership remains imported.
    pub const BUTTON_ROLES: [RustmixButtonRole; 7] = [
        RustmixButtonRole::Back,
        RustmixButtonRole::Select,
        RustmixButtonRole::Up,
        RustmixButtonRole::Down,
        RustmixButtonRole::Left,
        RustmixButtonRole::Right,
        RustmixButtonRole::Power,
    ];

    pub fn emit_boot_marker() {
        esp_println::println!("{}", Self::INPUT_BOUNDARY_MARKER);
    }

    pub const fn owns_physical_input_behavior() -> bool {
        false
    }

    pub const fn owner_for_runtime_behavior() -> RustmixInputOwner {
        RustmixInputOwner::ImportedX4Runtime
    }

    pub const fn is_adc_ladder_gpio(gpio: u8) -> bool {
        gpio == Self::ROW1_ADC_GPIO || gpio == Self::ROW2_ADC_GPIO
    }

    pub const fn is_power_button_gpio(gpio: u8) -> bool {
        gpio == Self::POWER_BUTTON_GPIO
    }

    pub const fn role_name(role: RustmixButtonRole) -> &'static str {
        match role {
            RustmixButtonRole::Back => "Back",
            RustmixButtonRole::Select => "Select",
            RustmixButtonRole::Up => "Up",
            RustmixButtonRole::Down => "Down",
            RustmixButtonRole::Left => "Left",
            RustmixButtonRole::Right => "Right",
            RustmixButtonRole::Power => "Power",
            RustmixButtonRole::Unknown => "Unknown",
        }
    }

    pub const fn role_info(role: RustmixButtonRole) -> RustmixButtonRoleInfo {
        RustmixButtonRoleInfo {
            role,
            label: Self::role_name(role),
            imported_owner: RustmixInputOwner::ImportedX4Runtime,
        }
    }

    pub const fn role_is_navigation(role: RustmixButtonRole) -> bool {
        matches!(
            role,
            RustmixButtonRole::Up
                | RustmixButtonRole::Down
                | RustmixButtonRole::Left
                | RustmixButtonRole::Right
        )
    }

    pub const fn role_is_reader_action(role: RustmixButtonRole) -> bool {
        matches!(
            role,
            RustmixButtonRole::Back
                | RustmixButtonRole::Select
                | RustmixButtonRole::Left
                | RustmixButtonRole::Right
        )
    }

    pub const fn role_is_system_action(role: RustmixButtonRole) -> bool {
        matches!(role, RustmixButtonRole::Power)
    }

    pub fn role_from_label(label: &str) -> RustmixButtonRole {
        if label.eq_ignore_ascii_case("back") {
            RustmixButtonRole::Back
        } else if label.eq_ignore_ascii_case("select") {
            RustmixButtonRole::Select
        } else if label.eq_ignore_ascii_case("up") {
            RustmixButtonRole::Up
        } else if label.eq_ignore_ascii_case("down") {
            RustmixButtonRole::Down
        } else if label.eq_ignore_ascii_case("left") {
            RustmixButtonRole::Left
        } else if label.eq_ignore_ascii_case("right") {
            RustmixButtonRole::Right
        } else if label.eq_ignore_ascii_case("power") {
            RustmixButtonRole::Power
        } else {
            RustmixButtonRole::Unknown
        }
    }
}
