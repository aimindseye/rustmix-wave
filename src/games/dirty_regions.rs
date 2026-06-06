//! Bounded logical dirty-region tracking for scriptable games.

/// Portrait product-canvas width in logical pixels.
pub const GAME_CANVAS_WIDTH: i32 = 480;
/// Portrait product-canvas height in logical pixels.
pub const GAME_CANVAS_HEIGHT: i32 = 800;
/// Maximum dirty regions retained for one script frame.
pub const MAX_DIRTY_REGIONS: usize = 4;

/// One half-open logical rectangle.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DirtyRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl DirtyRect {
    #[must_use]
    pub const fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    #[must_use]
    pub const fn right(self) -> i32 {
        self.x + self.width
    }

    #[must_use]
    pub const fn bottom(self) -> i32 {
        self.y + self.height
    }

    #[must_use]
    pub const fn is_valid(self) -> bool {
        self.width > 0
            && self.height > 0
            && self.x >= 0
            && self.y >= 0
            && self.right() <= GAME_CANVAS_WIDTH
            && self.bottom() <= GAME_CANVAS_HEIGHT
    }

    #[must_use]
    pub const fn overlaps_or_touches(self, other: Self) -> bool {
        self.x <= other.right()
            && self.right() >= other.x
            && self.y <= other.bottom()
            && self.bottom() >= other.y
    }

    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        let left = if self.x < other.x { self.x } else { other.x };
        let top = if self.y < other.y { self.y } else { other.y };
        let right = if self.right() > other.right() {
            self.right()
        } else {
            other.right()
        };
        let bottom = if self.bottom() > other.bottom() {
            self.bottom()
        } else {
            other.bottom()
        };
        Self::new(left, top, right - left, bottom - top)
    }

    /// Expand x coordinates to packed-framebuffer byte boundaries while
    /// retaining portrait logical dimensions. Invalid input remains invalid so
    /// refresh policy can select a safe global fallback.
    #[must_use]
    pub const fn align_x_to_byte_boundary(self) -> Self {
        if !self.is_valid() {
            return self;
        }
        let left = self.x & !7;
        let right = (self.right() + 7) & !7;
        let clipped_right = if right > GAME_CANVAS_WIDTH {
            GAME_CANVAS_WIDTH
        } else {
            right
        };
        Self::new(left, self.y, clipped_right - left, self.height)
    }
}

/// Fixed-capacity dirty-region accumulator. Overflow intentionally collapses
/// into a safe full-canvas fallback instead of allocating an unbounded list.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DirtyRegions {
    regions: Vec<DirtyRect>,
    full_canvas_fallback: bool,
}

impl DirtyRegions {
    #[must_use]
    pub fn regions(&self) -> &[DirtyRect] {
        &self.regions
    }

    #[must_use]
    pub const fn full_canvas_fallback(&self) -> bool {
        self.full_canvas_fallback
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.regions.is_empty() && !self.full_canvas_fallback
    }

    pub fn clear(&mut self) {
        self.regions.clear();
        self.full_canvas_fallback = false;
    }

    pub fn invalidate_full_canvas(&mut self) {
        self.regions.clear();
        self.full_canvas_fallback = true;
    }

    pub fn invalidate(&mut self, rect: DirtyRect) {
        let rect = rect.align_x_to_byte_boundary();
        if !rect.is_valid() {
            self.invalidate_full_canvas();
            return;
        }
        if self.full_canvas_fallback {
            return;
        }

        let mut merged = rect;
        let mut index = 0;
        while index < self.regions.len() {
            if self.regions[index].overlaps_or_touches(merged) {
                merged = self.regions.remove(index).union(merged);
            } else {
                index += 1;
            }
        }

        if self.regions.len() >= MAX_DIRTY_REGIONS {
            self.invalidate_full_canvas();
        } else {
            self.regions.push(merged);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{DirtyRect, DirtyRegions, GAME_CANVAS_HEIGHT, GAME_CANVAS_WIDTH};

    #[test]
    fn aligns_rectangles_to_packed_byte_boundaries() {
        assert_eq!(
            DirtyRect::new(5, 20, 10, 12).align_x_to_byte_boundary(),
            DirtyRect::new(0, 20, 16, 12)
        );
    }

    #[test]
    fn merges_touching_regions_and_bounds_capacity() {
        let mut dirty = DirtyRegions::default();
        dirty.invalidate(DirtyRect::new(8, 8, 8, 8));
        dirty.invalidate(DirtyRect::new(16, 8, 8, 8));
        assert_eq!(dirty.regions(), &[DirtyRect::new(8, 8, 16, 8)]);

        dirty.invalidate(DirtyRect::new(40, 40, 8, 8));
        dirty.invalidate(DirtyRect::new(80, 80, 8, 8));
        dirty.invalidate(DirtyRect::new(120, 120, 8, 8));
        dirty.invalidate(DirtyRect::new(160, 160, 8, 8));
        assert!(dirty.full_canvas_fallback());
    }

    #[test]
    fn invalid_geometry_requests_safe_full_canvas_fallback() {
        let mut dirty = DirtyRegions::default();
        dirty.invalidate(DirtyRect::new(GAME_CANVAS_WIDTH, 0, 8, 8));
        assert!(dirty.full_canvas_fallback());
        dirty.clear();
        dirty.invalidate(DirtyRect::new(0, GAME_CANVAS_HEIGHT, 8, 8));
        assert!(dirty.full_canvas_fallback());
    }
}
