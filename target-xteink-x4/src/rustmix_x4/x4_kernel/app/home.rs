#![allow(dead_code)]

use super::model::HomeMenuItem;

#[derive(Debug, Clone, Default)]
pub struct HomeState {
    selected_index: usize,
}

impl HomeState {
    pub const fn new() -> Self {
        Self { selected_index: 0 }
    }

    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    pub fn set_selected_index(&mut self, selected_index: usize) {
        self.selected_index = selected_index;
    }

    pub fn selected<'a>(&self, items: &'a [HomeMenuItem]) -> Option<HomeMenuItem> {
        items.get(self.selected_index).copied()
    }
}
