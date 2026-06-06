//! Native Sokoban Tilt sample for debounced portrait tilt events.
//!
//! The SD script declares one bounded 9x9 level. Rust owns parsing, player and
//! crate state, push validation, goal detection, rendering and dirty metadata.
//! Only debounced native tilt events cross the game boundary.

use super::{
    canvas::{CanvasTextStyle, NativeGameCanvas},
    dirty_regions::DirtyRect,
};
use crate::{
    buttons::ButtonEvent,
    imu_events::{ImuDetectedEvent, ImuDetectedEventKind, MotionAxis},
};

pub const SOKOBAN_COLUMNS: usize = 9;
pub const SOKOBAN_ROWS: usize = 9;
pub const SOKOBAN_CELL_COUNT: usize = SOKOBAN_COLUMNS * SOKOBAN_ROWS;
pub const SOKOBAN_GRID_X: i32 = 51;
pub const SOKOBAN_GRID_Y: i32 = 176;
pub const SOKOBAN_CELL_SIZE: i32 = 42;
const SOKOBAN_STATUS_RECT: DirtyRect = DirtyRect::new(16, 606, 448, 142);
const SOKOBAN_BOARD_RECT: DirtyRect = DirtyRect::new(
    SOKOBAN_GRID_X,
    SOKOBAN_GRID_Y,
    SOKOBAN_COLUMNS as i32 * SOKOBAN_CELL_SIZE + 1,
    SOKOBAN_ROWS as i32 * SOKOBAN_CELL_SIZE + 1,
);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SokobanDirection {
    Up,
    Down,
    Left,
    Right,
}
impl SokobanDirection {
    #[must_use]
    pub const fn from_raw_axis(axis: MotionAxis) -> Option<Self> {
        match axis {
            MotionAxis::PositiveX => Some(Self::Down),
            MotionAxis::NegativeX => Some(Self::Up),
            MotionAxis::PositiveY => Some(Self::Left),
            MotionAxis::NegativeY => Some(Self::Right),
            MotionAxis::PositiveZ | MotionAxis::NegativeZ => None,
        }
    }
    #[must_use]
    pub const fn marker(self) -> &'static str {
        match self {
            Self::Up => "up",
            Self::Down => "down",
            Self::Left => "left",
            Self::Right => "right",
        }
    }
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Up => "UP",
            Self::Down => "DOWN",
            Self::Left => "LEFT",
            Self::Right => "RIGHT",
        }
    }
    #[must_use]
    pub const fn delta(self) -> (i32, i32) {
        match self {
            Self::Up => (-1, 0),
            Self::Down => (1, 0),
            Self::Left => (0, -1),
            Self::Right => (0, 1),
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SokobanOutcome {
    InProgress,
    Won,
}
impl SokobanOutcome {
    #[must_use]
    pub const fn marker(self) -> &'static str {
        match self {
            Self::InProgress => "in-progress",
            Self::Won => "won",
        }
    }
    #[must_use]
    pub const fn completed(self) -> bool {
        matches!(self, Self::Won)
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SokobanTiltEventResult {
    pub reason: &'static str,
    pub row: usize,
    pub column: usize,
    pub moves: u32,
    pub pushes: u32,
    pub crates_on_goal: usize,
    pub outcome: SokobanOutcome,
    pub direction: &'static str,
    pub raw_axis: &'static str,
    pub dirty_regions: Vec<DirtyRect>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SokobanTiltGame {
    walls: [bool; SOKOBAN_CELL_COUNT],
    goals: [bool; SOKOBAN_CELL_COUNT],
    initial_crates: [bool; SOKOBAN_CELL_COUNT],
    crates: [bool; SOKOBAN_CELL_COUNT],
    start: usize,
    player: usize,
    moves: u32,
    pushes: u32,
    status: String,
    outcome: SokobanOutcome,
}
impl SokobanTiltGame {
    pub fn from_map(encoded: &str) -> Result<Self, String> {
        let rows = encoded.split('/').collect::<Vec<_>>();
        if rows.len() != SOKOBAN_ROWS
            || rows
                .iter()
                .any(|row| row.chars().count() != SOKOBAN_COLUMNS)
        {
            return Err("sokoban v0.18.6 expects one 9x9 slash-separated map".into());
        }
        let mut walls = [false; SOKOBAN_CELL_COUNT];
        let mut goals = [false; SOKOBAN_CELL_COUNT];
        let mut crates = [false; SOKOBAN_CELL_COUNT];
        let mut start = None;
        for (row, text) in rows.iter().enumerate() {
            for (column, ch) in text.chars().enumerate() {
                let index = row * SOKOBAN_COLUMNS + column;
                match ch {
                    '#' => walls[index] = true,
                    '.' => {}
                    'S' if start.is_none() => start = Some(index),
                    'S' => return Err("sokoban map contains multiple starts".into()),
                    'C' => crates[index] = true,
                    'G' => goals[index] = true,
                    '*' => {
                        crates[index] = true;
                        goals[index] = true;
                    }
                    _ => return Err(format!("sokoban map contains unsupported cell {ch:?}")),
                }
            }
        }
        let start = start.ok_or_else(|| "sokoban map is missing S".to_string())?;
        let crate_count = crates.iter().filter(|v| **v).count();
        let goal_count = goals.iter().filter(|v| **v).count();
        if crate_count == 0 || crate_count != goal_count {
            return Err("sokoban requires the same non-zero number of crates and goals".into());
        }
        if walls[start] || crates[start] {
            return Err("sokoban start cell is blocked".into());
        }
        Ok(Self {
            walls,
            goals,
            initial_crates: crates,
            crates,
            start,
            player: start,
            moves: 0,
            pushes: 0,
            status: "Tilt to move  Push crates onto G".into(),
            outcome: SokobanOutcome::InProgress,
        })
    }
    #[must_use]
    pub const fn player_row(&self) -> usize {
        self.player / SOKOBAN_COLUMNS
    }
    #[must_use]
    pub const fn player_column(&self) -> usize {
        self.player % SOKOBAN_COLUMNS
    }
    #[must_use]
    pub const fn moves(&self) -> u32 {
        self.moves
    }
    #[must_use]
    pub const fn pushes(&self) -> u32 {
        self.pushes
    }
    #[must_use]
    pub const fn outcome(&self) -> SokobanOutcome {
        self.outcome
    }
    #[must_use]
    pub fn crates_on_goal(&self) -> usize {
        self.crates
            .iter()
            .zip(self.goals.iter())
            .filter(|(crate_cell, goal)| **crate_cell && **goal)
            .count()
    }
    pub fn render_initial(&self, canvas: &mut NativeGameCanvas) -> Result<(), String> {
        self.render_commands(canvas)?;
        canvas.reset_dirty_regions();
        canvas.invalidate_rect(DirtyRect::new(0, 0, 480, 800));
        canvas.request_refresh();
        Ok(())
    }
    pub fn apply_motion_and_render(
        &mut self,
        event: ImuDetectedEvent,
        canvas: &mut NativeGameCanvas,
    ) -> Result<Option<SokobanTiltEventResult>, String> {
        let ImuDetectedEventKind::Tilt(axis) = event.kind else {
            return Ok(None);
        };
        let Some(direction) = SokobanDirection::from_raw_axis(axis) else {
            return Ok(None);
        };
        let (row_delta, column_delta) = direction.delta();
        let old_player = self.player;
        let mut pushed = false;
        let reason = if self.outcome.completed() {
            self.status = "Puzzle solved  SELECT reset".into();
            "game-finished"
        } else if let Some(next) = shifted_cell(self.player, row_delta, column_delta) {
            if self.walls[next] {
                self.status = format!("{} blocked by wall", direction.label());
                "wall-blocked"
            } else if self.crates[next] {
                if let Some(crate_next) = shifted_cell(next, row_delta, column_delta) {
                    if !self.walls[crate_next] && !self.crates[crate_next] {
                        self.crates[next] = false;
                        self.crates[crate_next] = true;
                        self.player = next;
                        self.moves = self.moves.saturating_add(1);
                        self.pushes = self.pushes.saturating_add(1);
                        pushed = true;
                        if self.crates_on_goal() == self.goals.iter().filter(|v| **v).count() {
                            self.outcome = SokobanOutcome::Won;
                            self.status = format!("Solved in {} moves  SELECT reset", self.moves);
                            "goal-reached"
                        } else {
                            self.status = format!("{} push  Keep solving", direction.label());
                            "crate-push"
                        }
                    } else {
                        self.status = format!("{} crate blocked", direction.label());
                        "crate-blocked"
                    }
                } else {
                    self.status = format!("{} crate blocked", direction.label());
                    "crate-blocked"
                }
            } else {
                self.player = next;
                self.moves = self.moves.saturating_add(1);
                self.status = format!("{} moved  Push C onto G", direction.label());
                "tilt-move"
            }
        } else {
            self.status = format!("{} edge blocked", direction.label());
            "edge-blocked"
        };
        self.render_commands(canvas)?;
        canvas.reset_dirty_regions();
        let dirty_regions = if pushed || reason == "goal-reached" {
            vec![SOKOBAN_BOARD_RECT, SOKOBAN_STATUS_RECT]
        } else if old_player == self.player {
            vec![SOKOBAN_STATUS_RECT]
        } else {
            vec![
                cell_rect(old_player),
                cell_rect(self.player),
                SOKOBAN_STATUS_RECT,
            ]
        };
        for rect in &dirty_regions {
            canvas.invalidate_rect(*rect);
        }
        canvas.request_refresh();
        Ok(Some(self.result(
            reason,
            direction.marker(),
            axis.marker(),
            dirty_regions,
        )))
    }
    pub fn apply_button_and_render(
        &mut self,
        event: ButtonEvent,
        canvas: &mut NativeGameCanvas,
    ) -> Result<Option<SokobanTiltEventResult>, String> {
        if event != ButtonEvent::Select {
            return Ok(None);
        }
        self.crates = self.initial_crates;
        self.player = self.start;
        self.moves = 0;
        self.pushes = 0;
        self.outcome = SokobanOutcome::InProgress;
        self.status = "Puzzle reset  Tilt to move".into();
        self.render_commands(canvas)?;
        canvas.reset_dirty_regions();
        let dirty_regions = vec![SOKOBAN_BOARD_RECT, SOKOBAN_STATUS_RECT];
        for rect in &dirty_regions {
            canvas.invalidate_rect(*rect);
        }
        canvas.request_refresh();
        Ok(Some(self.result("reset", "none", "none", dirty_regions)))
    }
    fn result(
        &self,
        reason: &'static str,
        direction: &'static str,
        raw_axis: &'static str,
        dirty_regions: Vec<DirtyRect>,
    ) -> SokobanTiltEventResult {
        SokobanTiltEventResult {
            reason,
            row: self.player_row(),
            column: self.player_column(),
            moves: self.moves,
            pushes: self.pushes,
            crates_on_goal: self.crates_on_goal(),
            outcome: self.outcome,
            direction,
            raw_axis,
            dirty_regions,
        }
    }
    fn render_commands(&self, canvas: &mut NativeGameCanvas) -> Result<(), String> {
        canvas.clear_frame();
        canvas.text(24, 82, "Sokoban Tilt".into(), CanvasTextStyle::Heading)?;
        canvas.text(
            24,
            126,
            "Native tilt crate-pushing sample".into(),
            CanvasTextStyle::Detail,
        )?;
        canvas.grid(
            SOKOBAN_GRID_X,
            SOKOBAN_GRID_Y,
            SOKOBAN_COLUMNS as u8,
            SOKOBAN_ROWS as u8,
            SOKOBAN_CELL_SIZE,
            SOKOBAN_CELL_SIZE,
        )?;
        for index in 0..SOKOBAN_CELL_COUNT {
            let (x, y) = cell_origin(index);
            if self.walls[index] {
                canvas.rect(
                    x + 1,
                    y + 1,
                    SOKOBAN_CELL_SIZE - 2,
                    SOKOBAN_CELL_SIZE - 2,
                    true,
                )?;
            } else {
                if self.goals[index] {
                    canvas.text(x + 13, y + 30, "G".into(), CanvasTextStyle::Heading)?;
                }
                if self.crates[index] {
                    canvas.rect(
                        x + 7,
                        y + 7,
                        SOKOBAN_CELL_SIZE - 14,
                        SOKOBAN_CELL_SIZE - 14,
                        false,
                    )?;
                    canvas.text(x + 13, y + 30, "C".into(), CanvasTextStyle::Heading)?;
                }
            }
        }
        let (px, py) = cell_origin(self.player);
        canvas.rect(
            px + 13,
            py + 13,
            SOKOBAN_CELL_SIZE - 26,
            SOKOBAN_CELL_SIZE - 26,
            true,
        )?;
        canvas.text(
            24,
            634,
            format!(
                "R{} C{}  MOVES {}  PUSHES {}  {}",
                self.player_row() + 1,
                self.player_column() + 1,
                self.moves,
                self.pushes,
                self.outcome.marker().to_ascii_uppercase()
            ),
            CanvasTextStyle::Heading,
        )?;
        canvas.text(24, 682, self.status.clone(), CanvasTextStyle::Body)?;
        canvas.text(
            24,
            734,
            "TILT move  SELECT reset  Hold BOOT back".into(),
            CanvasTextStyle::Body,
        )?;
        Ok(())
    }
}
fn shifted_cell(index: usize, row_delta: i32, column_delta: i32) -> Option<usize> {
    let row = index / SOKOBAN_COLUMNS;
    let column = index % SOKOBAN_COLUMNS;
    let next_row = row as i32 + row_delta;
    let next_column = column as i32 + column_delta;
    if !(0..SOKOBAN_ROWS as i32).contains(&next_row)
        || !(0..SOKOBAN_COLUMNS as i32).contains(&next_column)
    {
        return None;
    }
    Some(next_row as usize * SOKOBAN_COLUMNS + next_column as usize)
}
fn cell_origin(index: usize) -> (i32, i32) {
    let row = index / SOKOBAN_COLUMNS;
    let column = index % SOKOBAN_COLUMNS;
    (
        SOKOBAN_GRID_X + column as i32 * SOKOBAN_CELL_SIZE,
        SOKOBAN_GRID_Y + row as i32 * SOKOBAN_CELL_SIZE,
    )
}
fn cell_rect(index: usize) -> DirtyRect {
    let (x, y) = cell_origin(index);
    DirtyRect::new(x, y, SOKOBAN_CELL_SIZE + 1, SOKOBAN_CELL_SIZE + 1)
}
#[cfg(test)]
mod tests {
    use super::*;
    const MAP: &str =
        "#########/#S......#/#.###...#/#.......#/#...C...#/#.......#/#...G...#/#.......#/#########";
    fn tilt(axis: MotionAxis) -> ImuDetectedEvent {
        ImuDetectedEvent {
            kind: ImuDetectedEventKind::Tilt(axis),
            at_ms: 1,
        }
    }
    #[test]
    fn portrait_negative_y_moves_logical_right() {
        let mut game = SokobanTiltGame::from_map(MAP).unwrap();
        let mut canvas = NativeGameCanvas::default();
        let result = game
            .apply_motion_and_render(tilt(MotionAxis::NegativeY), &mut canvas)
            .unwrap()
            .unwrap();
        assert_eq!(result.reason, "tilt-move");
        assert_eq!(result.direction, "right");
        assert_eq!(result.raw_axis, "-y");
        assert_eq!((result.row, result.column), (1, 2));
    }
    #[test]
    fn push_to_goal_can_win() {
        let mut game = SokobanTiltGame::from_map(MAP).unwrap();
        let mut canvas = NativeGameCanvas::default();
        game.player = 3 * SOKOBAN_COLUMNS + 4;
        let first = game
            .apply_motion_and_render(tilt(MotionAxis::PositiveX), &mut canvas)
            .unwrap()
            .unwrap();
        assert_eq!(first.reason, "crate-push");
        let second = game
            .apply_motion_and_render(tilt(MotionAxis::PositiveX), &mut canvas)
            .unwrap()
            .unwrap();
        assert_eq!(second.reason, "goal-reached");
        assert!(second.outcome.completed());
    }
    #[test]
    fn blocked_wall_uses_status_only_dirty() {
        let mut game = SokobanTiltGame::from_map(MAP).unwrap();
        let mut canvas = NativeGameCanvas::default();
        let result = game
            .apply_motion_and_render(tilt(MotionAxis::PositiveY), &mut canvas)
            .unwrap()
            .unwrap();
        assert_eq!(result.reason, "wall-blocked");
        assert_eq!(result.dirty_regions, vec![SOKOBAN_STATUS_RECT]);
    }
}
