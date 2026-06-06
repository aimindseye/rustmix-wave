//! Native Motion 2048 sample for debounced portrait tilt events.
//!
//! The SD script declares one bounded seed. Rust owns the 4x4 board, tile
//! merging, deterministic tile spawning, rendering and dirty metadata. Only
//! debounced native tilt events cross the game boundary.

use crate::{
    buttons::ButtonEvent,
    imu_events::{ImuDetectedEvent, ImuDetectedEventKind, MotionAxis},
};

use super::{
    canvas::{CanvasTextStyle, NativeGameCanvas},
    dirty_regions::DirtyRect,
};

pub const MOTION_2048_COLUMNS: usize = 4;
pub const MOTION_2048_ROWS: usize = 4;
pub const MOTION_2048_CELL_COUNT: usize = MOTION_2048_COLUMNS * MOTION_2048_ROWS;
pub const MOTION_2048_GRID_X: i32 = 72;
pub const MOTION_2048_GRID_Y: i32 = 218;
pub const MOTION_2048_CELL_SIZE: i32 = 84;
const MOTION_2048_STATUS_RECT: DirtyRect = DirtyRect::new(16, 578, 448, 178);
const MOTION_2048_BOARD_RECT: DirtyRect = DirtyRect::new(
    MOTION_2048_GRID_X,
    MOTION_2048_GRID_Y,
    MOTION_2048_COLUMNS as i32 * MOTION_2048_CELL_SIZE + 1,
    MOTION_2048_ROWS as i32 * MOTION_2048_CELL_SIZE + 1,
);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Motion2048Direction {
    Up,
    Down,
    Left,
    Right,
}
impl Motion2048Direction {
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
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Motion2048Outcome {
    InProgress,
    Won,
    GameOver,
}
impl Motion2048Outcome {
    #[must_use]
    pub const fn marker(self) -> &'static str {
        match self {
            Self::InProgress => "in-progress",
            Self::Won => "won",
            Self::GameOver => "game-over",
        }
    }
    #[must_use]
    pub const fn completed(self) -> bool {
        !matches!(self, Self::InProgress)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Motion2048EventResult {
    pub reason: &'static str,
    pub row: usize,
    pub column: usize,
    pub score: u32,
    pub moves: u32,
    pub max_tile: u16,
    pub outcome: Motion2048Outcome,
    pub direction: &'static str,
    pub raw_axis: &'static str,
    pub dirty_regions: Vec<DirtyRect>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Motion2048Game {
    board: [u16; MOTION_2048_CELL_COUNT],
    rng_state: u32,
    score: u32,
    moves: u32,
    status: String,
    outcome: Motion2048Outcome,
}

impl Motion2048Game {
    pub fn from_seed(seed: u32) -> Result<Self, String> {
        if seed == 0 {
            return Err("motion2048 seed must be non-zero".into());
        }
        let mut game = Self {
            board: [0; MOTION_2048_CELL_COUNT],
            rng_state: seed,
            score: 0,
            moves: 0,
            status: "Tilt to slide tiles  SELECT reset".into(),
            outcome: Motion2048Outcome::InProgress,
        };
        game.spawn_tile();
        game.spawn_tile();
        Ok(game)
    }
    #[must_use]
    pub const fn score(&self) -> u32 {
        self.score
    }
    #[must_use]
    pub const fn moves(&self) -> u32 {
        self.moves
    }
    #[must_use]
    pub const fn outcome(&self) -> Motion2048Outcome {
        self.outcome
    }
    #[must_use]
    pub fn max_tile(&self) -> u16 {
        *self.board.iter().max().unwrap_or(&0)
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
    ) -> Result<Option<Motion2048EventResult>, String> {
        let ImuDetectedEventKind::Tilt(axis) = event.kind else {
            return Ok(None);
        };
        let Some(direction) = Motion2048Direction::from_raw_axis(axis) else {
            return Ok(None);
        };
        let reason = if self.outcome.completed() {
            self.status = "Game finished  SELECT reset".into();
            "game-finished"
        } else if self.slide(direction) {
            self.moves = self.moves.saturating_add(1);
            self.spawn_tile();
            self.update_outcome();
            self.status = match self.outcome {
                Motion2048Outcome::Won => {
                    format!("2048 reached in {} moves  SELECT reset", self.moves)
                }
                Motion2048Outcome::GameOver => "No moves left  SELECT reset".into(),
                Motion2048Outcome::InProgress => {
                    format!("{} slide  Keep combining", direction.label())
                }
            };
            "tilt-slide"
        } else {
            self.update_outcome();
            self.status = match self.outcome {
                Motion2048Outcome::GameOver => "No moves left  SELECT reset".into(),
                _ => format!("{} has no move  Tilt another way", direction.label()),
            };
            "no-change"
        };
        self.render_commands(canvas)?;
        canvas.reset_dirty_regions();
        let dirty_regions = if reason == "tilt-slide" {
            vec![MOTION_2048_BOARD_RECT, MOTION_2048_STATUS_RECT]
        } else {
            vec![MOTION_2048_STATUS_RECT]
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
    ) -> Result<Option<Motion2048EventResult>, String> {
        if event != ButtonEvent::Select {
            return Ok(None);
        }
        let seed = self.rng_state.max(1);
        *self = Self::from_seed(seed)?;
        self.status = "Board reset  Tilt to slide".into();
        self.render_commands(canvas)?;
        canvas.reset_dirty_regions();
        let dirty_regions = vec![MOTION_2048_BOARD_RECT, MOTION_2048_STATUS_RECT];
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
    ) -> Motion2048EventResult {
        Motion2048EventResult {
            reason,
            row: 0,
            column: 0,
            score: self.score,
            moves: self.moves,
            max_tile: self.max_tile(),
            outcome: self.outcome,
            direction,
            raw_axis,
            dirty_regions,
        }
    }

    fn slide(&mut self, direction: Motion2048Direction) -> bool {
        let before = self.board;
        let mut gained = 0u32;
        for line in 0..4 {
            let positions = line_positions(direction, line);
            let input = [
                self.board[positions[0]],
                self.board[positions[1]],
                self.board[positions[2]],
                self.board[positions[3]],
            ];
            let (output, line_score) = merge_line(input);
            gained = gained.saturating_add(line_score);
            for offset in 0..4 {
                self.board[positions[offset]] = output[offset];
            }
        }
        self.score = self.score.saturating_add(gained);
        self.board != before
    }
    fn spawn_tile(&mut self) {
        let mut empty = [0usize; MOTION_2048_CELL_COUNT];
        let mut count = 0usize;
        for (index, value) in self.board.iter().enumerate() {
            if *value == 0 {
                empty[count] = index;
                count += 1;
            }
        }
        if count == 0 {
            return;
        }
        let pick = (self.next_random() as usize) % count;
        let value = if self.next_random() % 10 == 0 { 4 } else { 2 };
        self.board[empty[pick]] = value;
    }
    fn next_random(&mut self) -> u32 {
        let mut x = self.rng_state;
        if x == 0 {
            x = 0xA341_316C;
        }
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng_state = x;
        x
    }
    fn update_outcome(&mut self) {
        self.outcome = if self.max_tile() >= 2048 {
            Motion2048Outcome::Won
        } else if has_available_move(&self.board) {
            Motion2048Outcome::InProgress
        } else {
            Motion2048Outcome::GameOver
        };
    }

    fn render_commands(&self, canvas: &mut NativeGameCanvas) -> Result<(), String> {
        canvas.clear_frame();
        canvas.text(24, 82, "Motion 2048".into(), CanvasTextStyle::Heading)?;
        canvas.text(
            24,
            126,
            "Native debounced tilt-swipe sample".into(),
            CanvasTextStyle::Detail,
        )?;
        canvas.grid(
            MOTION_2048_GRID_X,
            MOTION_2048_GRID_Y,
            4,
            4,
            MOTION_2048_CELL_SIZE,
            MOTION_2048_CELL_SIZE,
        )?;
        for index in 0..MOTION_2048_CELL_COUNT {
            let value = self.board[index];
            if value != 0 {
                let (x, y) = cell_origin(index);
                canvas.text(x + 18, y + 50, value.to_string(), CanvasTextStyle::Heading)?;
            }
        }
        canvas.text(
            24,
            610,
            format!(
                "SCORE {}  MOVES {}  MAX {}  {}",
                self.score,
                self.moves,
                self.max_tile(),
                self.outcome.marker().to_ascii_uppercase()
            ),
            CanvasTextStyle::Heading,
        )?;
        canvas.text(24, 664, self.status.clone(), CanvasTextStyle::Body)?;
        canvas.text(
            24,
            724,
            "TILT swipe  SELECT reset  Hold BOOT back".into(),
            CanvasTextStyle::Body,
        )?;
        Ok(())
    }
}

fn line_positions(direction: Motion2048Direction, line: usize) -> [usize; 4] {
    match direction {
        Motion2048Direction::Left => [line * 4, line * 4 + 1, line * 4 + 2, line * 4 + 3],
        Motion2048Direction::Right => [line * 4 + 3, line * 4 + 2, line * 4 + 1, line * 4],
        Motion2048Direction::Up => [line, line + 4, line + 8, line + 12],
        Motion2048Direction::Down => [line + 12, line + 8, line + 4, line],
    }
}
fn merge_line(input: [u16; 4]) -> ([u16; 4], u32) {
    let mut compact = [0u16; 4];
    let mut count = 0usize;
    for value in input {
        if value != 0 {
            compact[count] = value;
            count += 1;
        }
    }
    let mut out = [0u16; 4];
    let mut src = 0usize;
    let mut dst = 0usize;
    let mut score = 0u32;
    while src < count {
        if src + 1 < count && compact[src] == compact[src + 1] {
            let merged = compact[src].saturating_mul(2);
            out[dst] = merged;
            score = score.saturating_add(u32::from(merged));
            src += 2;
        } else {
            out[dst] = compact[src];
            src += 1;
        }
        dst += 1;
    }
    (out, score)
}
fn has_available_move(board: &[u16; 16]) -> bool {
    if board.iter().any(|value| *value == 0) {
        return true;
    }
    for row in 0..4 {
        for column in 0..4 {
            let index = row * 4 + column;
            if column + 1 < 4 && board[index] == board[index + 1] {
                return true;
            }
            if row + 1 < 4 && board[index] == board[index + 4] {
                return true;
            }
        }
    }
    false
}
fn cell_origin(index: usize) -> (i32, i32) {
    let row = index / 4;
    let column = index % 4;
    (
        MOTION_2048_GRID_X + column as i32 * MOTION_2048_CELL_SIZE,
        MOTION_2048_GRID_Y + row as i32 * MOTION_2048_CELL_SIZE,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn tilt(axis: MotionAxis) -> ImuDetectedEvent {
        ImuDetectedEvent {
            kind: ImuDetectedEventKind::Tilt(axis),
            at_ms: 1,
        }
    }
    #[test]
    fn merge_line_combines_each_pair_once() {
        assert_eq!(merge_line([2, 2, 2, 2]), ([4, 4, 0, 0], 8));
        assert_eq!(merge_line([4, 4, 8, 0]), ([8, 8, 0, 0], 8));
    }
    #[test]
    fn portrait_negative_y_slides_logical_right() {
        let mut game = Motion2048Game::from_seed(2048).unwrap();
        game.board = [2, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let mut canvas = NativeGameCanvas::default();
        let result = game
            .apply_motion_and_render(tilt(MotionAxis::NegativeY), &mut canvas)
            .unwrap()
            .unwrap();
        assert_eq!(result.reason, "tilt-slide");
        assert_eq!(result.direction, "right");
        assert_eq!(result.raw_axis, "-y");
        assert_eq!(game.board[3], 4);
        assert_eq!(result.dirty_regions.len(), 2);
    }
    #[test]
    fn no_change_uses_status_only_dirty() {
        let mut game = Motion2048Game::from_seed(2048).unwrap();
        game.board = [0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let mut canvas = NativeGameCanvas::default();
        let result = game
            .apply_motion_and_render(tilt(MotionAxis::NegativeY), &mut canvas)
            .unwrap()
            .unwrap();
        assert_eq!(result.reason, "no-change");
        assert_eq!(result.dirty_regions, vec![MOTION_2048_STATUS_RECT]);
    }
}
