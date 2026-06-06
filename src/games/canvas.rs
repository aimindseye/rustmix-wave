//! Bounded native draw-command canvas for removable-storage applications.

use super::dirty_regions::{DirtyRect, DirtyRegions, GAME_CANVAS_HEIGHT, GAME_CANVAS_WIDTH};

pub const MAX_GAME_DRAW_COMMANDS: usize = 256;
pub const MAX_GAME_TEXT_BYTES: usize = 160;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CanvasTextStyle {
    Body,
    Heading,
    Detail,
}

impl CanvasTextStyle {
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "body" => Some(Self::Body),
            "heading" => Some(Self::Heading),
            "detail" => Some(Self::Detail),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DrawCommand {
    Clear,
    Text {
        x: i32,
        y: i32,
        text: String,
        style: CanvasTextStyle,
    },
    Line {
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
    },
    Rect {
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        filled: bool,
    },
    Grid {
        x: i32,
        y: i32,
        columns: u8,
        rows: u8,
        cell_width: i32,
        cell_height: i32,
    },
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct NativeGameCanvas {
    commands: Vec<DrawCommand>,
    dirty: DirtyRegions,
    refresh_requested: bool,
}

impl NativeGameCanvas {
    #[must_use]
    pub fn commands(&self) -> &[DrawCommand] {
        &self.commands
    }

    #[must_use]
    pub const fn dirty(&self) -> &DirtyRegions {
        &self.dirty
    }

    #[must_use]
    pub const fn refresh_requested(&self) -> bool {
        self.refresh_requested
    }

    pub fn clear_frame(&mut self) {
        self.commands.clear();
        self.dirty.clear();
        self.refresh_requested = false;
    }

    /// Keep the rebuilt full command list but replace broad render-time dirty
    /// geometry with one bounded event-specific invalidation set.
    pub fn reset_dirty_regions(&mut self) {
        self.dirty.clear();
        self.refresh_requested = false;
    }

    pub fn clear(&mut self) -> Result<(), String> {
        self.push(DrawCommand::Clear)?;
        self.dirty.invalidate_full_canvas();
        Ok(())
    }

    pub fn text(
        &mut self,
        x: i32,
        y: i32,
        text: String,
        style: CanvasTextStyle,
    ) -> Result<(), String> {
        if text.len() > MAX_GAME_TEXT_BYTES {
            return Err(format!(
                "game text exceeds {MAX_GAME_TEXT_BYTES}-byte command limit"
            ));
        }
        self.validate_point(x, y)?;
        let width = (text.len() as i32).saturating_mul(14).max(8);
        self.push(DrawCommand::Text { x, y, text, style })?;
        self.dirty.invalidate(DirtyRect::new(
            x,
            (y - 28).max(0),
            width.min(GAME_CANVAS_WIDTH - x),
            36,
        ));
        Ok(())
    }

    pub fn line(&mut self, x1: i32, y1: i32, x2: i32, y2: i32) -> Result<(), String> {
        self.validate_point(x1, y1)?;
        self.validate_point(x2, y2)?;
        self.push(DrawCommand::Line { x1, y1, x2, y2 })?;
        let left = x1.min(x2);
        let top = y1.min(y2);
        self.dirty.invalidate(DirtyRect::new(
            left,
            top,
            (x1.max(x2) - left + 1).max(1),
            (y1.max(y2) - top + 1).max(1),
        ));
        Ok(())
    }

    pub fn rect(
        &mut self,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        filled: bool,
    ) -> Result<(), String> {
        self.validate_rect(DirtyRect::new(x, y, width, height))?;
        self.push(DrawCommand::Rect {
            x,
            y,
            width,
            height,
            filled,
        })?;
        self.dirty.invalidate(DirtyRect::new(x, y, width, height));
        Ok(())
    }

    pub fn grid(
        &mut self,
        x: i32,
        y: i32,
        columns: u8,
        rows: u8,
        cell_width: i32,
        cell_height: i32,
    ) -> Result<(), String> {
        if columns == 0 || rows == 0 || columns > 16 || rows > 16 {
            return Err("game grid dimensions must be between 1 and 16".into());
        }
        let width = i32::from(columns).saturating_mul(cell_width);
        let height = i32::from(rows).saturating_mul(cell_height);
        self.validate_rect(DirtyRect::new(x, y, width, height))?;
        self.push(DrawCommand::Grid {
            x,
            y,
            columns,
            rows,
            cell_width,
            cell_height,
        })?;
        self.dirty.invalidate(DirtyRect::new(x, y, width, height));
        Ok(())
    }

    pub fn invalidate_rect(&mut self, rect: DirtyRect) {
        self.dirty.invalidate(rect);
    }

    pub fn request_refresh(&mut self) {
        self.refresh_requested = true;
    }

    fn push(&mut self, command: DrawCommand) -> Result<(), String> {
        if self.commands.len() >= MAX_GAME_DRAW_COMMANDS {
            return Err(format!(
                "game frame exceeds {MAX_GAME_DRAW_COMMANDS}-command limit"
            ));
        }
        self.commands.push(command);
        Ok(())
    }

    fn validate_point(&self, x: i32, y: i32) -> Result<(), String> {
        if (0..GAME_CANVAS_WIDTH).contains(&x) && (0..GAME_CANVAS_HEIGHT).contains(&y) {
            Ok(())
        } else {
            Err(format!("game point ({x},{y}) is outside logical canvas"))
        }
    }

    fn validate_rect(&self, rect: DirtyRect) -> Result<(), String> {
        if rect.is_valid() {
            Ok(())
        } else {
            Err(format!(
                "game rectangle ({},{},{},{}) is outside logical canvas",
                rect.x, rect.y, rect.width, rect.height
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CanvasTextStyle, DrawCommand, NativeGameCanvas};

    #[test]
    fn canvas_bounds_commands_and_tracks_dirty_regions() {
        let mut canvas = NativeGameCanvas::default();
        canvas
            .text(24, 90, "Hello Grid".into(), CanvasTextStyle::Heading)
            .unwrap();
        canvas.grid(40, 180, 4, 4, 64, 64).unwrap();
        canvas.request_refresh();
        assert_eq!(canvas.commands().len(), 2);
        assert!(!canvas.dirty().is_empty());
        assert!(canvas.refresh_requested());
        assert!(matches!(canvas.commands()[1], DrawCommand::Grid { .. }));
    }

    #[test]
    fn canvas_rejects_out_of_bounds_commands() {
        let mut canvas = NativeGameCanvas::default();
        assert!(canvas.rect(470, 20, 20, 20, false).is_err());
    }
}
