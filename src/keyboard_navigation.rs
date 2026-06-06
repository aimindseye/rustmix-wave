//! Reusable H/V axis navigation for rotary keyboard and grid-style editors.
//!
//! BOOT short press toggles the active axis without moving the selected key.
//! Rotary Up / Down then move within the active row or column. BOOT long press
//! remains owned by the application router as hierarchical Back.

/// Rotary movement axis used by keyboard and grid-style text-entry screens.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum KeyboardNavigationAxis {
    /// Move left or right within the current row.
    #[default]
    Horizontal,
    /// Move up or down within the current column.
    Vertical,
}

impl KeyboardNavigationAxis {
    #[must_use]
    pub const fn toggled(self) -> Self {
        match self {
            Self::Horizontal => Self::Vertical,
            Self::Vertical => Self::Horizontal,
        }
    }

    #[must_use]
    pub const fn status_label(self) -> &'static str {
        match self {
            Self::Horizontal => "NAV H",
            Self::Vertical => "NAV V",
        }
    }
}

/// Compact reusable rotary navigator for a row-major keyboard grid.
///
/// The final row may contain fewer keys than `columns`; vertical movement skips
/// cells that do not exist. The selected key is preserved when the axis toggles.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KeyboardGridNavigation {
    total_keys: usize,
    columns: usize,
    selected: usize,
    axis: KeyboardNavigationAxis,
}

impl KeyboardGridNavigation {
    #[must_use]
    pub const fn new(total_keys: usize, columns: usize) -> Self {
        assert!(total_keys > 0, "keyboard must contain at least one key");
        assert!(columns > 0, "keyboard must contain at least one column");
        Self {
            total_keys,
            columns,
            selected: 0,
            axis: KeyboardNavigationAxis::Horizontal,
        }
    }

    #[must_use]
    pub const fn selected(self) -> usize {
        self.selected
    }

    #[must_use]
    pub const fn axis(self) -> KeyboardNavigationAxis {
        self.axis
    }

    #[must_use]
    pub const fn status_label(self) -> &'static str {
        self.axis.status_label()
    }

    pub fn toggle_axis(&mut self) {
        self.axis = self.axis.toggled();
    }

    pub fn move_previous(&mut self) {
        match self.axis {
            KeyboardNavigationAxis::Horizontal => self.move_horizontal(false),
            KeyboardNavigationAxis::Vertical => self.move_vertical(false),
        }
    }

    pub fn move_next(&mut self) {
        match self.axis {
            KeyboardNavigationAxis::Horizontal => self.move_horizontal(true),
            KeyboardNavigationAxis::Vertical => self.move_vertical(true),
        }
    }

    fn move_horizontal(&mut self, forward: bool) {
        let row_start = (self.selected / self.columns) * self.columns;
        let row_end = (row_start + self.columns).min(self.total_keys);
        self.selected = if forward {
            if self.selected + 1 < row_end {
                self.selected + 1
            } else {
                row_start
            }
        } else if self.selected > row_start {
            self.selected - 1
        } else {
            row_end - 1
        };
    }

    fn move_vertical(&mut self, forward: bool) {
        let rows = self.total_keys.div_ceil(self.columns);
        let column = self.selected % self.columns;
        let current_row = self.selected / self.columns;
        for step in 1..=rows {
            let row = if forward {
                (current_row + step) % rows
            } else {
                (current_row + rows - (step % rows)) % rows
            };
            let candidate = row * self.columns + column;
            if candidate < self.total_keys {
                self.selected = candidate;
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{KeyboardGridNavigation, KeyboardNavigationAxis};

    #[test]
    fn horizontal_navigation_wraps_inside_current_row() {
        let mut navigation = KeyboardGridNavigation::new(30, 6);
        navigation.move_previous();
        assert_eq!(navigation.selected(), 5);
        navigation.move_next();
        assert_eq!(navigation.selected(), 0);
    }

    #[test]
    fn vertical_navigation_preserves_column_and_wraps() {
        let mut navigation = KeyboardGridNavigation::new(30, 6);
        navigation.move_next();
        navigation.toggle_axis();
        assert_eq!(navigation.axis(), KeyboardNavigationAxis::Vertical);
        navigation.move_next();
        assert_eq!(navigation.selected(), 7);
        for _ in 0..4 {
            navigation.move_next();
        }
        assert_eq!(navigation.selected(), 1);
    }

    #[test]
    fn axis_toggle_preserves_selected_key() {
        let mut navigation = KeyboardGridNavigation::new(30, 6);
        navigation.move_next();
        navigation.move_next();
        let selected = navigation.selected();
        navigation.toggle_axis();
        assert_eq!(navigation.selected(), selected);
        assert_eq!(navigation.status_label(), "NAV V");
    }

    #[test]
    fn vertical_navigation_skips_missing_cells_in_partial_final_row() {
        let mut navigation = KeyboardGridNavigation::new(8, 6);
        navigation.move_next();
        navigation.move_next();
        navigation.toggle_axis();
        navigation.move_next();
        assert_eq!(navigation.selected(), 2);
    }
}
