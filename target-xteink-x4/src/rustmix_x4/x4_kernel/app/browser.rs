#![allow(dead_code)]

#[derive(Debug, Clone, Default)]
pub struct BrowserState {
    selected_index: usize,
    scroll: usize,
}

impl BrowserState {
    pub const fn new() -> Self {
        Self {
            selected_index: 0,
            scroll: 0,
        }
    }

    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    pub fn scroll(&self) -> usize {
        self.scroll
    }

    pub fn set_state(&mut self, scroll: usize, selected_index: usize) {
        self.scroll = scroll;
        self.selected_index = selected_index;
    }
}
