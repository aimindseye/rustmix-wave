// semantic actions decoupled from physical buttons
// apps match on Action, never on HwButton

use crate::rustmix_x4::x4_kernel::board::button::Button;
use crate::rustmix_x4::x4_kernel::drivers::input::Event;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Next,
    Prev,
    NextJump,
    PrevJump,
    Select,
    Back,
    Menu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionEvent {
    Press(Action),
    Release(Action),
    LongPress(Action),
    Repeat(Action),
}

impl ActionEvent {
    pub fn action(self) -> Action {
        match self {
            Self::Press(a) | Self::Release(a) | Self::LongPress(a) | Self::Repeat(a) => a,
        }
    }

    pub fn is_press(self) -> bool {
        matches!(self, Self::Press(_))
    }

    pub fn is_repeat(self) -> bool {
        matches!(self, Self::Repeat(_))
    }

    pub fn is_press_or_repeat(self) -> bool {
        matches!(self, Self::Press(_) | Self::Repeat(_))
    }
}

// portrait one-handed layout with optional button swap
//
// default layout (right-handed):
//   bottom row: Back  Confirm(=Select)  Left(=PrevJump)  Right(=NextJump)
//   side:       VolUp(=Prev)  VolDown(=Next)
//
// swapped layout (left-handed):
//   bottom row: Left(=PrevJump)  Right(=NextJump)  Back  Confirm(=Select)
//   this swaps the *physical roles* of Back<->Left and Confirm<->Right
//   so the spatial position of Back/OK moves to the right side of the
//   device where the left hand naturally rests.
//   volume buttons are NOT swapped (up=prev, down=next always).
#[derive(Default)]
pub struct ButtonMapper {
    swap_buttons: bool,
}

impl ButtonMapper {
    pub const fn new() -> Self {
        Self {
            swap_buttons: false,
        }
    }

    pub fn set_swap(&mut self, swap: bool) {
        self.swap_buttons = swap;
    }

    pub fn is_swapped(&self) -> bool {
        self.swap_buttons
    }

    pub fn map_button(&self, button: Button) -> Action {
        if self.swap_buttons {
            // swapped: Back<->Left, Confirm<->Right
            match button {
                Button::VolDown => Action::Next,
                Button::VolUp => Action::Prev,
                Button::Right => Action::Select,     // was NextJump
                Button::Left => Action::Back,        // was PrevJump
                Button::Confirm => Action::NextJump, // was Select
                Button::Back => Action::PrevJump,    // was Back
                Button::Power => Action::Menu,
            }
        } else {
            // default right-handed layout
            match button {
                Button::VolDown => Action::Next,
                Button::VolUp => Action::Prev,
                Button::Right => Action::NextJump,
                Button::Left => Action::PrevJump,
                Button::Confirm => Action::Select,
                Button::Back => Action::Back,
                Button::Power => Action::Menu,
            }
        }
    }

    pub fn map_event(&self, event: Event) -> ActionEvent {
        match event {
            Event::Press(b) => ActionEvent::Press(self.map_button(b)),
            Event::Release(b) => ActionEvent::Release(self.map_button(b)),
            Event::LongPress(b) => ActionEvent::LongPress(self.map_button(b)),
            Event::Repeat(b) => ActionEvent::Repeat(self.map_button(b)),
        }
    }
}
