//! App list state model for the Apps container.

use super::app_catalog::{SystemAppDescriptor, app_by_index, enabled_system_app_count};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AppListState {
    pub selected_index: usize,
}

impl AppListState {
    pub const fn new() -> Self {
        Self { selected_index: 0 }
    }

    pub const fn app_count(self) -> usize {
        enabled_system_app_count()
    }

    pub const fn selected_app(self) -> Option<SystemAppDescriptor> {
        app_by_index(self.selected_index)
    }

    pub fn move_next(&mut self) {
        let count = self.app_count();
        if count == 0 {
            self.selected_index = 0;
        } else {
            self.selected_index = (self.selected_index + 1) % count;
        }
    }

    pub fn move_previous(&mut self) {
        let count = self.app_count();
        if count == 0 {
            self.selected_index = 0;
        } else if self.selected_index == 0 {
            self.selected_index = count - 1;
        } else {
            self.selected_index -= 1;
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AppListRow {
    pub title: &'static str,
    pub summary: &'static str,
    pub selected: bool,
}

pub const fn app_list_row(index: usize, selected_index: usize) -> Option<AppListRow> {
    match app_by_index(index) {
        Some(app) => Some(AppListRow {
            title: app.name,
            summary: app.summary,
            selected: index == selected_index,
        }),
        None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{AppListState, app_list_row};
    use crate::rustmix_x4::apps::app_catalog::SystemAppId;

    #[test]
    fn app_list_wraps_forward_and_backward() {
        let mut state = AppListState::new();
        assert_eq!(state.selected_app().unwrap().id, SystemAppId::Reader);
        state.move_next();
        assert_eq!(state.selected_app().unwrap().id, SystemAppId::Reader);
        state.move_previous();
        assert_eq!(state.selected_app().unwrap().id, SystemAppId::Reader);
    }

    #[test]
    fn app_list_rows_expose_titles_and_selection() {
        let first = app_list_row(0, 0).unwrap();
        assert_eq!(first.title, "Reader");
        assert!(app_list_row(1, 0).is_none());
        assert!(first.selected);
    }
}
