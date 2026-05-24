// list selection and scrolling helper
//
// consolidates the repeated pattern of managing a selection cursor
// within a scrollable list. handles wrapping, scroll adjustment,
// and visibility calculations.
use crate::rustmix_x4::x4_apps::ui::{wrap_next, wrap_prev};

#[derive(Clone, Copy, Debug, Default)]
pub struct ListSelection {
    pub selected: usize,
    pub scroll: usize,
    pub count: usize,
    pub visible: usize,
}

impl ListSelection {
    pub const fn new(count: usize, visible: usize) -> Self {
        Self {
            selected: 0,
            scroll: 0,
            count,
            visible,
        }
    }

    pub fn reset(&mut self) {
        self.selected = 0;
        self.scroll = 0;
    }

    pub fn set_count(&mut self, count: usize) {
        self.count = count;
        if count == 0 {
            self.selected = 0;
            self.scroll = 0;
        } else {
            if self.selected >= count {
                self.selected = count - 1;
            }
            self.scroll_into_view();
        }
    }

    pub fn set_visible(&mut self, visible: usize) {
        self.visible = visible;
        self.scroll_into_view();
    }

    pub fn move_next(&mut self) -> bool {
        if self.count == 0 {
            return false;
        }
        let old = self.selected;
        self.selected = wrap_next(self.selected, self.count);
        if self.selected < old {
            // wrapped to start
            self.scroll = 0;
        } else {
            self.scroll_into_view();
        }
        self.selected != old
    }

    pub fn move_prev(&mut self) -> bool {
        if self.count == 0 {
            return false;
        }
        let old = self.selected;
        self.selected = wrap_prev(self.selected, self.count);
        if self.selected > old {
            // wrapped to end
            self.scroll = self.count.saturating_sub(self.visible);
        } else {
            self.scroll_into_view();
        }
        self.selected != old
    }

    pub fn move_down(&mut self) -> bool {
        if self.count == 0 || self.selected + 1 >= self.count {
            return false;
        }
        self.selected += 1;
        self.scroll_into_view();
        true
    }

    pub fn move_up(&mut self) -> bool {
        if self.count == 0 || self.selected == 0 {
            return false;
        }
        self.selected -= 1;
        self.scroll_into_view();
        true
    }

    pub fn page_down(&mut self) -> bool {
        if self.count == 0 {
            return false;
        }
        let old = self.selected;
        self.selected = (self.selected + self.visible).min(self.count - 1);
        self.scroll_into_view();
        self.selected != old
    }

    pub fn page_up(&mut self) -> bool {
        if self.count == 0 {
            return false;
        }
        let old = self.selected;
        self.selected = self.selected.saturating_sub(self.visible);
        self.scroll_into_view();
        self.selected != old
    }

    pub fn jump_to_start(&mut self) -> bool {
        if self.count == 0 || self.selected == 0 {
            return false;
        }
        self.selected = 0;
        self.scroll = 0;
        true
    }

    pub fn jump_to_end(&mut self) -> bool {
        if self.count == 0 || self.selected == self.count - 1 {
            return false;
        }
        self.selected = self.count - 1;
        self.scroll_into_view();
        true
    }

    pub fn select(&mut self, index: usize) -> bool {
        if self.count == 0 {
            return false;
        }
        let index = index.min(self.count - 1);
        if self.selected == index {
            return false;
        }
        self.selected = index;
        self.scroll_into_view();
        true
    }

    pub fn scroll_into_view(&mut self) {
        if self.count == 0 || self.visible == 0 {
            return;
        }

        // selected is above visible window
        if self.selected < self.scroll {
            self.scroll = self.selected;
        }

        // selected is below visible window
        if self.selected >= self.scroll + self.visible {
            self.scroll = self.selected + 1 - self.visible;
        }

        // clamp scroll to valid range
        let max_scroll = self.count.saturating_sub(self.visible);
        if self.scroll > max_scroll {
            self.scroll = max_scroll;
        }
    }

    pub fn visible_count(&self) -> usize {
        self.visible.min(self.count.saturating_sub(self.scroll))
    }

    pub fn is_visible(&self, index: usize) -> bool {
        index >= self.scroll && index < self.scroll + self.visible
    }

    pub fn to_visible_index(&self, index: usize) -> Option<usize> {
        if self.is_visible(index) {
            Some(index - self.scroll)
        } else {
            None
        }
    }
}
