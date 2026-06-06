//! Native Tilt Maze sample for debounced QMI8658 motion events.
//!
//! The SD script declares one bounded 9x9 maze. Rust owns parsing, player
//! movement, collision checks, goal detection, rendering and dirty metadata.
//! Only debounced native tilt events cross the game boundary.

use crate::{
    buttons::ButtonEvent,
    imu_events::{ImuDetectedEvent, ImuDetectedEventKind, MotionAxis},
};

use super::{
    canvas::{CanvasTextStyle, NativeGameCanvas},
    dirty_regions::DirtyRect,
};

pub const TILT_MAZE_COLUMNS: usize = 9;
pub const TILT_MAZE_ROWS: usize = 9;
pub const TILT_MAZE_CELL_COUNT: usize = TILT_MAZE_COLUMNS * TILT_MAZE_ROWS;
pub const TILT_MAZE_GRID_X: i32 = 51;
pub const TILT_MAZE_GRID_Y: i32 = 176;
pub const TILT_MAZE_CELL_SIZE: i32 = 42;
const TILT_MAZE_STATUS_RECT: DirtyRect = DirtyRect::new(16, 606, 448, 142);
const TILT_MAZE_BOARD_RECT: DirtyRect = DirtyRect::new(
    TILT_MAZE_GRID_X,
    TILT_MAZE_GRID_Y,
    TILT_MAZE_COLUMNS as i32 * TILT_MAZE_CELL_SIZE + 1,
    TILT_MAZE_ROWS as i32 * TILT_MAZE_CELL_SIZE + 1,
);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TiltMazeDirection {
    Up,
    Down,
    Left,
    Right,
}
impl TiltMazeDirection {
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
pub enum TiltMazeOutcome {
    InProgress,
    Won,
}
impl TiltMazeOutcome {
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
pub struct TiltMazeEventResult {
    pub reason: &'static str,
    pub row: usize,
    pub column: usize,
    pub moves: u32,
    pub outcome: TiltMazeOutcome,
    pub direction: &'static str,
    pub raw_axis: &'static str,
    pub dirty_regions: Vec<DirtyRect>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TiltMazeGame {
    walls: [bool; TILT_MAZE_CELL_COUNT],
    start: usize,
    goal: usize,
    player: usize,
    moves: u32,
    status: String,
    outcome: TiltMazeOutcome,
}

impl TiltMazeGame {
    pub fn from_map(encoded: &str) -> Result<Self, String> {
        let rows = encoded.split('/').collect::<Vec<_>>();
        if rows.len() != TILT_MAZE_ROWS
            || rows
                .iter()
                .any(|row| row.chars().count() != TILT_MAZE_COLUMNS)
        {
            return Err("tiltmaze v0.18.5 expects one 9x9 slash-separated map".into());
        }
        let mut walls = [false; TILT_MAZE_CELL_COUNT];
        let mut start = None;
        let mut goal = None;
        for (row, text) in rows.iter().enumerate() {
            for (column, character) in text.chars().enumerate() {
                let index = row * TILT_MAZE_COLUMNS + column;
                match character {
                    '#' => walls[index] = true,
                    '.' => {}
                    'S' if start.is_none() => start = Some(index),
                    'G' if goal.is_none() => goal = Some(index),
                    'S' => return Err("tiltmaze map contains multiple starts".into()),
                    'G' => return Err("tiltmaze map contains multiple goals".into()),
                    _ => {
                        return Err(format!(
                            "tiltmaze map contains unsupported cell {character:?}"
                        ))
                    }
                }
            }
        }
        let start = start.ok_or_else(|| "tiltmaze map is missing S".to_string())?;
        let goal = goal.ok_or_else(|| "tiltmaze map is missing G".to_string())?;
        if !path_exists(&walls, start, goal) {
            return Err("tiltmaze goal is unreachable".into());
        }
        Ok(Self {
            walls,
            start,
            goal,
            player: start,
            moves: 0,
            status: "Tilt device to move  SELECT reset".into(),
            outcome: TiltMazeOutcome::InProgress,
        })
    }

    #[must_use]
    pub const fn player_row(&self) -> usize {
        self.player / TILT_MAZE_COLUMNS
    }
    #[must_use]
    pub const fn player_column(&self) -> usize {
        self.player % TILT_MAZE_COLUMNS
    }
    #[must_use]
    pub const fn moves(&self) -> u32 {
        self.moves
    }
    #[must_use]
    pub const fn outcome(&self) -> TiltMazeOutcome {
        self.outcome
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
    ) -> Result<Option<TiltMazeEventResult>, String> {
        let ImuDetectedEventKind::Tilt(axis) = event.kind else {
            return Ok(None);
        };
        let Some(direction) = TiltMazeDirection::from_raw_axis(axis) else {
            return Ok(None);
        };
        let (row_delta, column_delta) = direction.delta();
        let old = self.player;
        let reason = if self.outcome.completed() {
            self.status = "Goal reached  SELECT reset  Hold BOOT back".into();
            "game-finished"
        } else if let Some(next) = shifted_cell(self.player, row_delta, column_delta) {
            if self.walls[next] {
                self.status = format!("{} blocked  Tilt another way", direction.label());
                "wall-blocked"
            } else {
                self.player = next;
                self.moves = self.moves.saturating_add(1);
                if self.player == self.goal {
                    self.outcome = TiltMazeOutcome::Won;
                    self.status = format!("Goal reached in {} moves  SELECT reset", self.moves);
                    "goal-reached"
                } else {
                    self.status = format!("{} moved  Find G", direction.label());
                    "tilt-move"
                }
            }
        } else {
            self.status = format!("{} blocked  Tilt another way", direction.label());
            "edge-blocked"
        };
        self.render_commands(canvas)?;
        canvas.reset_dirty_regions();
        let dirty_regions = if reason == "goal-reached" {
            vec![TILT_MAZE_BOARD_RECT, TILT_MAZE_STATUS_RECT]
        } else if old == self.player {
            vec![TILT_MAZE_STATUS_RECT]
        } else {
            vec![
                cell_rect(old),
                cell_rect(self.player),
                TILT_MAZE_STATUS_RECT,
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
    ) -> Result<Option<TiltMazeEventResult>, String> {
        if event != ButtonEvent::Select {
            return Ok(None);
        }
        self.player = self.start;
        self.moves = 0;
        self.outcome = TiltMazeOutcome::InProgress;
        self.status = "Maze reset  Tilt device to move".into();
        self.render_commands(canvas)?;
        canvas.reset_dirty_regions();
        let dirty_regions = vec![TILT_MAZE_BOARD_RECT, TILT_MAZE_STATUS_RECT];
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
    ) -> TiltMazeEventResult {
        TiltMazeEventResult {
            reason,
            row: self.player_row(),
            column: self.player_column(),
            moves: self.moves,
            outcome: self.outcome,
            direction,
            raw_axis,
            dirty_regions,
        }
    }

    fn render_commands(&self, canvas: &mut NativeGameCanvas) -> Result<(), String> {
        canvas.clear_frame();
        canvas.text(24, 82, "Tilt Maze".into(), CanvasTextStyle::Heading)?;
        canvas.text(
            24,
            126,
            "Native debounced tilt-event sample".into(),
            CanvasTextStyle::Detail,
        )?;
        canvas.grid(
            TILT_MAZE_GRID_X,
            TILT_MAZE_GRID_Y,
            TILT_MAZE_COLUMNS as u8,
            TILT_MAZE_ROWS as u8,
            TILT_MAZE_CELL_SIZE,
            TILT_MAZE_CELL_SIZE,
        )?;
        for index in 0..TILT_MAZE_CELL_COUNT {
            if self.walls[index] {
                let (x, y) = cell_origin(index);
                canvas.rect(
                    x + 1,
                    y + 1,
                    TILT_MAZE_CELL_SIZE - 2,
                    TILT_MAZE_CELL_SIZE - 2,
                    true,
                )?;
            }
        }
        let (goal_x, goal_y) = cell_origin(self.goal);
        canvas.text(
            goal_x + 13,
            goal_y + 30,
            "G".into(),
            CanvasTextStyle::Heading,
        )?;
        let (player_x, player_y) = cell_origin(self.player);
        canvas.rect(
            player_x + 9,
            player_y + 9,
            TILT_MAZE_CELL_SIZE - 18,
            TILT_MAZE_CELL_SIZE - 18,
            true,
        )?;
        canvas.text(
            24,
            634,
            format!(
                "R{} C{}  MOVES {}  {}",
                self.player_row() + 1,
                self.player_column() + 1,
                self.moves,
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

// The QMI8658 axes follow the native landscape board. Product games render in
// the upright portrait logical UI, so raw planar axes must be rotated once at
// this boundary: +X -> down, -X -> up, +Y -> left, -Y -> right.
fn shifted_cell(index: usize, row_delta: i32, column_delta: i32) -> Option<usize> {
    let row = index as i32 / TILT_MAZE_COLUMNS as i32 + row_delta;
    let column = index as i32 % TILT_MAZE_COLUMNS as i32 + column_delta;
    if !(0..TILT_MAZE_ROWS as i32).contains(&row)
        || !(0..TILT_MAZE_COLUMNS as i32).contains(&column)
    {
        return None;
    }
    Some(row as usize * TILT_MAZE_COLUMNS + column as usize)
}
fn cell_origin(index: usize) -> (i32, i32) {
    (
        TILT_MAZE_GRID_X + (index % TILT_MAZE_COLUMNS) as i32 * TILT_MAZE_CELL_SIZE,
        TILT_MAZE_GRID_Y + (index / TILT_MAZE_COLUMNS) as i32 * TILT_MAZE_CELL_SIZE,
    )
}
fn cell_rect(index: usize) -> DirtyRect {
    let (x, y) = cell_origin(index);
    DirtyRect::new(x, y, TILT_MAZE_CELL_SIZE + 1, TILT_MAZE_CELL_SIZE + 1)
}
fn path_exists(walls: &[bool; TILT_MAZE_CELL_COUNT], start: usize, goal: usize) -> bool {
    let mut seen = [false; TILT_MAZE_CELL_COUNT];
    let mut queue = [0usize; TILT_MAZE_CELL_COUNT];
    let mut head = 0;
    let mut tail = 1;
    queue[0] = start;
    seen[start] = true;
    while head < tail {
        let current = queue[head];
        head += 1;
        if current == goal {
            return true;
        }
        for (dr, dc) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            if let Some(next) = shifted_cell(current, dr, dc) {
                if !walls[next] && !seen[next] {
                    seen[next] = true;
                    queue[tail] = next;
                    tail += 1;
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    const MAP: &str =
        "#########/#S..#...#/#.#.#.#.#/#.#...#.#/#.###.#.#/#...#.#.#/###.#...#/#.....#G#/#########";
    fn tilt(axis: MotionAxis) -> ImuDetectedEvent {
        ImuDetectedEvent {
            kind: ImuDetectedEventKind::Tilt(axis),
            at_ms: 1,
        }
    }
    #[test]
    fn portrait_raw_negative_y_moves_logical_right() {
        let mut game = TiltMazeGame::from_map(MAP).unwrap();
        let mut canvas = NativeGameCanvas::default();
        game.render_initial(&mut canvas).unwrap();
        let result = game
            .apply_motion_and_render(tilt(MotionAxis::NegativeY), &mut canvas)
            .unwrap()
            .unwrap();
        assert_eq!(result.reason, "tilt-move");
        assert_eq!(result.direction, "right");
        assert_eq!(result.raw_axis, "-y");
        assert_eq!((result.row, result.column), (1, 2));
        assert_eq!(result.dirty_regions.len(), 3);
    }
    #[test]
    fn portrait_raw_positive_x_moves_logical_down() {
        let mut game = TiltMazeGame::from_map(MAP).unwrap();
        let mut canvas = NativeGameCanvas::default();
        let result = game
            .apply_motion_and_render(tilt(MotionAxis::PositiveX), &mut canvas)
            .unwrap()
            .unwrap();
        assert_eq!(result.reason, "tilt-move");
        assert_eq!(result.direction, "down");
        assert_eq!((result.row, result.column), (2, 1));
    }
    #[test]
    fn portrait_raw_positive_y_is_blocked_logical_left() {
        let mut game = TiltMazeGame::from_map(MAP).unwrap();
        let mut canvas = NativeGameCanvas::default();
        let result = game
            .apply_motion_and_render(tilt(MotionAxis::PositiveY), &mut canvas)
            .unwrap()
            .unwrap();
        assert_eq!(result.reason, "wall-blocked");
        assert_eq!(result.direction, "left");
        assert_eq!(result.dirty_regions, vec![TILT_MAZE_STATUS_RECT]);
    }
    #[test]
    fn select_resets_position() {
        let mut game = TiltMazeGame::from_map(MAP).unwrap();
        let mut canvas = NativeGameCanvas::default();
        game.apply_motion_and_render(tilt(MotionAxis::NegativeY), &mut canvas)
            .unwrap();
        let result = game
            .apply_button_and_render(ButtonEvent::Select, &mut canvas)
            .unwrap()
            .unwrap();
        assert_eq!(result.reason, "reset");
        assert_eq!((result.row, result.column), (1, 1));
    }
}
