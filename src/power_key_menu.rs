//! Hardware-independent Power-key menu state.
//!
//! A physical Power short press opens a compact global display-maintenance
//! menu. A physical Power long press is handled by the AXP2101 runtime and
//! enters the existing sleep-image path without routing through this menu.

use crate::buttons::ButtonEvent;

pub const POWER_KEY_MENU_ACTION_COUNT: usize = 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PowerKeyMenuOutcome {
    None,
    ClearGhosting,
    Cancel,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PowerKeyMenuUiState {
    pub selected: usize,
}

impl PowerKeyMenuUiState {
    pub fn reset(&mut self) {
        self.selected = 0;
    }

    #[must_use]
    pub fn apply_button(&mut self, event: ButtonEvent) -> PowerKeyMenuOutcome {
        match event {
            ButtonEvent::Up => {
                self.selected = self
                    .selected
                    .checked_sub(1)
                    .unwrap_or(POWER_KEY_MENU_ACTION_COUNT - 1);
                PowerKeyMenuOutcome::None
            }
            ButtonEvent::Down => {
                self.selected = (self.selected + 1) % POWER_KEY_MENU_ACTION_COUNT;
                PowerKeyMenuOutcome::None
            }
            ButtonEvent::Select => match self.selected {
                0 => PowerKeyMenuOutcome::ClearGhosting,
                _ => PowerKeyMenuOutcome::Cancel,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{PowerKeyMenuOutcome, PowerKeyMenuUiState};
    use crate::buttons::ButtonEvent;

    #[test]
    fn defaults_to_clear_ghosting_and_cycles_cancel() {
        let mut menu = PowerKeyMenuUiState::default();
        assert_eq!(menu.selected, 0);
        assert_eq!(
            menu.apply_button(ButtonEvent::Select),
            PowerKeyMenuOutcome::ClearGhosting
        );
        assert_eq!(
            menu.apply_button(ButtonEvent::Down),
            PowerKeyMenuOutcome::None
        );
        assert_eq!(menu.selected, 1);
        assert_eq!(
            menu.apply_button(ButtonEvent::Select),
            PowerKeyMenuOutcome::Cancel
        );
        assert_eq!(
            menu.apply_button(ButtonEvent::Down),
            PowerKeyMenuOutcome::None
        );
        assert_eq!(menu.selected, 0);
    }
}
