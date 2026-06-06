//! Native Sudoku reference game for the SD Lua event bridge.
//!
//! The SD script declares one bounded puzzle through `sudoku.init(...)`.
//! Rust owns board state, conflict checks, dirty-cell invalidation and all
//! redraw commands. Scripts never receive panel, framebuffer or SPI access.

use crate::buttons::ButtonEvent;

use super::{
    canvas::{CanvasTextStyle, NativeGameCanvas},
    dirty_regions::DirtyRect,
};

pub const SUDOKU_CELL_COUNT: usize = 81;
pub const SUDOKU_GRID_X: i32 = 51;
pub const SUDOKU_GRID_Y: i32 = 176;
pub const SUDOKU_CELL_SIZE: i32 = 42;
const SUDOKU_STATUS_RECT: DirtyRect = DirtyRect::new(16, 612, 448, 96);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SudokuMode {
    Navigate,
    Edit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SudokuMovementAxis {
    Horizontal,
    Vertical,
}

impl SudokuMovementAxis {
    #[must_use]
    pub const fn marker(self) -> &'static str {
        match self {
            Self::Horizontal => "horizontal",
            Self::Vertical => "vertical",
        }
    }

    #[must_use]
    pub const fn short_marker(self) -> &'static str {
        match self {
            Self::Horizontal => "H",
            Self::Vertical => "V",
        }
    }

    #[must_use]
    pub const fn toggled(self) -> Self {
        match self {
            Self::Horizontal => Self::Vertical,
            Self::Vertical => Self::Horizontal,
        }
    }
}

impl SudokuMode {
    #[must_use]
    pub const fn marker(self) -> &'static str {
        match self {
            Self::Navigate => "nav",
            Self::Edit => "edit",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SudokuEventResult {
    pub reason: &'static str,
    pub row: usize,
    pub column: usize,
    pub mode: SudokuMode,
    pub axis: SudokuMovementAxis,
    pub candidate: u8,
    pub completed: bool,
    pub dirty_regions: Vec<DirtyRect>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SudokuGame {
    board: [u8; SUDOKU_CELL_COUNT],
    givens: [bool; SUDOKU_CELL_COUNT],
    cursor: usize,
    mode: SudokuMode,
    axis: SudokuMovementAxis,
    candidate: u8,
    status: String,
    completed: bool,
}

impl SudokuGame {
    pub fn from_puzzle(source: &str) -> Result<Self, String> {
        if source.len() != SUDOKU_CELL_COUNT {
            return Err(format!(
                "sudoku puzzle must contain {SUDOKU_CELL_COUNT} cells"
            ));
        }
        let mut board = [0_u8; SUDOKU_CELL_COUNT];
        let mut givens = [false; SUDOKU_CELL_COUNT];
        for (index, byte) in source.bytes().enumerate() {
            board[index] = match byte {
                b'0' | b'.' => 0,
                b'1'..=b'9' => byte - b'0',
                _ => return Err("sudoku puzzle accepts only digits, zero or dot".into()),
            };
            givens[index] = board[index] != 0;
        }
        validate_initial_board(&board)?;
        let completed = is_complete(&board);
        Ok(Self {
            board,
            givens,
            cursor: first_editable_cell(&givens).unwrap_or(0),
            mode: SudokuMode::Navigate,
            axis: SudokuMovementAxis::Horizontal,
            candidate: 1,
            status: if completed {
                "Puzzle complete".into()
            } else {
                "AXIS H: UP/DOWN move  SELECT edit".into()
            },
            completed,
        })
    }

    #[must_use]
    pub const fn mode(&self) -> SudokuMode {
        self.mode
    }

    #[must_use]
    pub const fn cursor_row(&self) -> usize {
        self.cursor / 9
    }

    #[must_use]
    pub const fn cursor_column(&self) -> usize {
        self.cursor % 9
    }

    #[must_use]
    pub const fn movement_axis(&self) -> SudokuMovementAxis {
        self.axis
    }

    #[must_use]
    pub const fn candidate(&self) -> u8 {
        self.candidate
    }

    #[must_use]
    pub const fn completed(&self) -> bool {
        self.completed
    }

    #[must_use]
    pub fn board(&self) -> &[u8; SUDOKU_CELL_COUNT] {
        &self.board
    }

    pub fn render_initial(&self, canvas: &mut NativeGameCanvas) -> Result<(), String> {
        self.render_commands(canvas)?;
        canvas.reset_dirty_regions();
        canvas.invalidate_rect(DirtyRect::new(0, 0, 480, 800));
        canvas.request_refresh();
        Ok(())
    }

    pub fn apply_button_and_render(
        &mut self,
        event: ButtonEvent,
        canvas: &mut NativeGameCanvas,
    ) -> Result<SudokuEventResult, String> {
        let old_cursor = self.cursor;
        let reason = match self.mode {
            SudokuMode::Navigate => self.apply_navigation_button(event),
            SudokuMode::Edit => self.apply_edit_button(event),
        };
        self.render_commands(canvas)?;
        canvas.reset_dirty_regions();
        let mut dirty_regions = vec![
            cell_rect(old_cursor),
            cell_rect(self.cursor),
            SUDOKU_STATUS_RECT,
        ];
        dirty_regions.dedup();
        for rect in &dirty_regions {
            canvas.invalidate_rect(*rect);
        }
        canvas.request_refresh();
        Ok(SudokuEventResult {
            reason,
            row: self.cursor_row(),
            column: self.cursor_column(),
            mode: self.mode,
            axis: self.axis,
            candidate: self.candidate,
            completed: self.completed,
            dirty_regions,
        })
    }

    pub fn apply_boot_short_press_and_render(
        &mut self,
        canvas: &mut NativeGameCanvas,
    ) -> Result<SudokuEventResult, String> {
        let reason = match self.mode {
            SudokuMode::Navigate => {
                self.axis = self.axis.toggled();
                self.status = format!(
                    "AXIS {}: UP/DOWN move  SELECT edit",
                    self.axis.short_marker()
                );
                "axis-toggle"
            }
            SudokuMode::Edit => {
                self.mode = SudokuMode::Navigate;
                self.candidate = self.board[self.cursor];
                self.status = format!(
                    "Edit canceled  AXIS {}: UP/DOWN move",
                    self.axis.short_marker()
                );
                "edit-cancel"
            }
        };
        self.render_commands(canvas)?;
        canvas.reset_dirty_regions();
        let dirty_regions = match reason {
            "axis-toggle" => vec![SUDOKU_STATUS_RECT],
            _ => vec![cell_rect(self.cursor), SUDOKU_STATUS_RECT],
        };
        for rect in &dirty_regions {
            canvas.invalidate_rect(*rect);
        }
        canvas.request_refresh();
        Ok(SudokuEventResult {
            reason,
            row: self.cursor_row(),
            column: self.cursor_column(),
            mode: self.mode,
            axis: self.axis,
            candidate: self.candidate,
            completed: self.completed,
            dirty_regions,
        })
    }

    fn apply_navigation_button(&mut self, event: ButtonEvent) -> &'static str {
        match event {
            ButtonEvent::Up => {
                self.cursor = previous_cell(self.cursor, self.axis);
                self.status = format!(
                    "AXIS {}: UP/DOWN move  SELECT edit",
                    self.axis.short_marker()
                );
                "cursor-move"
            }
            ButtonEvent::Down => {
                self.cursor = next_cell(self.cursor, self.axis);
                self.status = format!(
                    "AXIS {}: UP/DOWN move  SELECT edit",
                    self.axis.short_marker()
                );
                "cursor-move"
            }
            ButtonEvent::Select => {
                if self.givens[self.cursor] {
                    self.status = "Fixed clue: choose another cell".into();
                    "fixed-clue"
                } else {
                    self.mode = SudokuMode::Edit;
                    self.candidate = self.board[self.cursor];
                    self.status = "EDIT: UP/DOWN value  SELECT save".into();
                    "edit-enter"
                }
            }
        }
    }

    fn apply_edit_button(&mut self, event: ButtonEvent) -> &'static str {
        match event {
            ButtonEvent::Up => {
                self.candidate = if self.candidate == 0 {
                    9
                } else {
                    self.candidate - 1
                };
                self.status = candidate_status(self.candidate);
                "candidate-change"
            }
            ButtonEvent::Down => {
                self.candidate = if self.candidate >= 9 {
                    0
                } else {
                    self.candidate + 1
                };
                self.status = candidate_status(self.candidate);
                "candidate-change"
            }
            ButtonEvent::Select => {
                if self.candidate != 0 && conflicts(&self.board, self.cursor, self.candidate) {
                    self.status = "Conflict: value already used".into();
                    "conflict"
                } else {
                    self.board[self.cursor] = self.candidate;
                    self.mode = SudokuMode::Navigate;
                    self.completed = is_complete(&self.board);
                    self.status = if self.completed {
                        "Puzzle complete".into()
                    } else if self.candidate == 0 {
                        "Cell cleared".into()
                    } else {
                        "Value saved".into()
                    };
                    "commit"
                }
            }
        }
    }

    fn render_commands(&self, canvas: &mut NativeGameCanvas) -> Result<(), String> {
        canvas.clear_frame();
        canvas.text(24, 66, "Sudoku".into(), CanvasTextStyle::Heading)?;
        canvas.text(
            24,
            104,
            "SD Lua event bridge / native board".into(),
            CanvasTextStyle::Detail,
        )?;
        canvas.grid(
            SUDOKU_GRID_X,
            SUDOKU_GRID_Y,
            9,
            9,
            SUDOKU_CELL_SIZE,
            SUDOKU_CELL_SIZE,
        )?;
        for offset in [3, 6] {
            let x = SUDOKU_GRID_X + offset * SUDOKU_CELL_SIZE;
            let y = SUDOKU_GRID_Y + offset * SUDOKU_CELL_SIZE;
            canvas.line(
                x - 1,
                SUDOKU_GRID_Y,
                x - 1,
                SUDOKU_GRID_Y + 9 * SUDOKU_CELL_SIZE,
            )?;
            canvas.line(
                x + 1,
                SUDOKU_GRID_Y,
                x + 1,
                SUDOKU_GRID_Y + 9 * SUDOKU_CELL_SIZE,
            )?;
            canvas.line(
                SUDOKU_GRID_X,
                y - 1,
                SUDOKU_GRID_X + 9 * SUDOKU_CELL_SIZE,
                y - 1,
            )?;
            canvas.line(
                SUDOKU_GRID_X,
                y + 1,
                SUDOKU_GRID_X + 9 * SUDOKU_CELL_SIZE,
                y + 1,
            )?;
        }
        for (index, value) in self.board.iter().copied().enumerate() {
            if value == 0 {
                continue;
            }
            let row = index / 9;
            let column = index % 9;
            canvas.text(
                SUDOKU_GRID_X + column as i32 * SUDOKU_CELL_SIZE + 14,
                SUDOKU_GRID_Y + row as i32 * SUDOKU_CELL_SIZE + 30,
                value.to_string(),
                if self.givens[index] {
                    CanvasTextStyle::Heading
                } else {
                    CanvasTextStyle::Body
                },
            )?;
        }
        let cursor = cell_rect(self.cursor);
        canvas.rect(cursor.x, cursor.y, cursor.width, cursor.height, false)?;
        canvas.rect(
            cursor.x + 2,
            cursor.y + 2,
            cursor.width - 4,
            cursor.height - 4,
            false,
        )?;
        canvas.text(
            24,
            648,
            format!(
                "R{} C{}  MODE {}  AXIS {}  VALUE {}",
                self.cursor_row() + 1,
                self.cursor_column() + 1,
                self.mode.marker().to_ascii_uppercase(),
                self.axis.short_marker(),
                display_candidate(self.candidate)
            ),
            CanvasTextStyle::Body,
        )?;
        canvas.text(24, 690, self.status.clone(), CanvasTextStyle::Detail)?;
        let footer = match self.mode {
            SudokuMode::Navigate => "BOOT short axis  Hold BOOT back",
            SudokuMode::Edit => "BOOT short cancel  SELECT save",
        };
        canvas.text(24, 742, footer.to_string(), CanvasTextStyle::Detail)?;
        canvas.request_refresh();
        Ok(())
    }
}

fn validate_initial_board(board: &[u8; SUDOKU_CELL_COUNT]) -> Result<(), String> {
    for (index, value) in board.iter().copied().enumerate() {
        if value != 0 && conflicts(board, index, value) {
            return Err(format!(
                "sudoku puzzle contains a conflict at cell {}",
                index + 1
            ));
        }
    }
    Ok(())
}

fn conflicts(board: &[u8; SUDOKU_CELL_COUNT], index: usize, value: u8) -> bool {
    let row = index / 9;
    let column = index % 9;
    for peer in 0..9 {
        let row_index = row * 9 + peer;
        let column_index = peer * 9 + column;
        if row_index != index && board[row_index] == value {
            return true;
        }
        if column_index != index && board[column_index] == value {
            return true;
        }
    }
    let block_row = (row / 3) * 3;
    let block_column = (column / 3) * 3;
    for block_y in 0..3 {
        for block_x in 0..3 {
            let peer = (block_row + block_y) * 9 + block_column + block_x;
            if peer != index && board[peer] == value {
                return true;
            }
        }
    }
    false
}

fn is_complete(board: &[u8; SUDOKU_CELL_COUNT]) -> bool {
    board.iter().all(|value| *value != 0)
        && board
            .iter()
            .copied()
            .enumerate()
            .all(|(index, value)| !conflicts(board, index, value))
}

fn first_editable_cell(givens: &[bool; SUDOKU_CELL_COUNT]) -> Option<usize> {
    givens.iter().position(|given| !*given)
}

fn previous_cell(index: usize, axis: SudokuMovementAxis) -> usize {
    let row = index / 9;
    let column = index % 9;
    match axis {
        SudokuMovementAxis::Horizontal => row * 9 + column.checked_sub(1).unwrap_or(8),
        SudokuMovementAxis::Vertical => row.checked_sub(1).unwrap_or(8) * 9 + column,
    }
}

fn next_cell(index: usize, axis: SudokuMovementAxis) -> usize {
    let row = index / 9;
    let column = index % 9;
    match axis {
        SudokuMovementAxis::Horizontal => row * 9 + (column + 1) % 9,
        SudokuMovementAxis::Vertical => (row + 1) % 9 * 9 + column,
    }
}

fn cell_rect(index: usize) -> DirtyRect {
    DirtyRect::new(
        SUDOKU_GRID_X + (index % 9) as i32 * SUDOKU_CELL_SIZE,
        SUDOKU_GRID_Y + (index / 9) as i32 * SUDOKU_CELL_SIZE,
        SUDOKU_CELL_SIZE + 1,
        SUDOKU_CELL_SIZE + 1,
    )
}

fn candidate_status(candidate: u8) -> String {
    if candidate == 0 {
        "EDIT value CLEAR  SELECT save".into()
    } else {
        format!("EDIT value {candidate}  SELECT save")
    }
}

fn display_candidate(candidate: u8) -> String {
    if candidate == 0 {
        "CLEAR".into()
    } else {
        candidate.to_string()
    }
}

#[cfg(test)]
mod tests {
    use crate::{buttons::ButtonEvent, games::canvas::NativeGameCanvas};

    use super::{SudokuGame, SudokuMode, SudokuMovementAxis};

    const PUZZLE: &str =
        "530070000600195000098000060800060003400803001700020006060000280000419005000080079";

    #[test]
    fn loads_valid_puzzle_and_renders_bounded_native_canvas() {
        let game = SudokuGame::from_puzzle(PUZZLE).unwrap();
        let mut canvas = NativeGameCanvas::default();
        game.render_initial(&mut canvas).unwrap();
        assert!(!canvas.commands().is_empty());
        assert!(canvas.refresh_requested());
    }

    #[test]
    fn rotary_reference_flow_moves_edits_cycles_and_commits() {
        let mut game = SudokuGame::from_puzzle(PUZZLE).unwrap();
        let mut canvas = NativeGameCanvas::default();
        game.render_initial(&mut canvas).unwrap();
        let moved = game
            .apply_button_and_render(ButtonEvent::Down, &mut canvas)
            .unwrap();
        assert_eq!(moved.reason, "cursor-move");
        let entered = game
            .apply_button_and_render(ButtonEvent::Select, &mut canvas)
            .unwrap();
        if entered.reason == "fixed-clue" {
            game.apply_button_and_render(ButtonEvent::Down, &mut canvas)
                .unwrap();
            game.apply_button_and_render(ButtonEvent::Select, &mut canvas)
                .unwrap();
        }
        assert_eq!(game.mode(), SudokuMode::Edit);
        game.apply_button_and_render(ButtonEvent::Down, &mut canvas)
            .unwrap();
        let committed = game
            .apply_button_and_render(ButtonEvent::Select, &mut canvas)
            .unwrap();
        assert!(matches!(committed.reason, "commit" | "conflict"));
        assert!(canvas.dirty().regions().len() <= 4 || canvas.dirty().full_canvas_fallback());
    }

    #[test]
    fn boot_short_axis_toggle_switches_horizontal_and_vertical_navigation() {
        let mut game = SudokuGame::from_puzzle(PUZZLE).unwrap();
        let mut canvas = NativeGameCanvas::default();
        game.render_initial(&mut canvas).unwrap();
        assert_eq!(game.movement_axis(), SudokuMovementAxis::Horizontal);
        let horizontal = game
            .apply_button_and_render(ButtonEvent::Down, &mut canvas)
            .unwrap();
        assert_eq!((horizontal.row, horizontal.column), (0, 3));
        let toggled = game.apply_boot_short_press_and_render(&mut canvas).unwrap();
        assert_eq!(toggled.reason, "axis-toggle");
        assert_eq!(toggled.axis, SudokuMovementAxis::Vertical);
        assert_eq!(toggled.dirty_regions, vec![super::SUDOKU_STATUS_RECT]);
        let vertical = game
            .apply_button_and_render(ButtonEvent::Down, &mut canvas)
            .unwrap();
        assert_eq!((vertical.row, vertical.column), (1, 3));
    }

    #[test]
    fn boot_short_cancels_edit_and_preserves_navigation_axis() {
        let mut game = SudokuGame::from_puzzle(PUZZLE).unwrap();
        let mut canvas = NativeGameCanvas::default();
        game.render_initial(&mut canvas).unwrap();
        game.apply_boot_short_press_and_render(&mut canvas).unwrap();
        assert_eq!(game.movement_axis(), SudokuMovementAxis::Vertical);
        game.apply_button_and_render(ButtonEvent::Select, &mut canvas)
            .unwrap();
        assert_eq!(game.mode(), SudokuMode::Edit);
        game.apply_button_and_render(ButtonEvent::Down, &mut canvas)
            .unwrap();
        let canceled = game.apply_boot_short_press_and_render(&mut canvas).unwrap();
        assert_eq!(canceled.reason, "edit-cancel");
        assert_eq!(canceled.mode, SudokuMode::Navigate);
        assert_eq!(canceled.axis, SudokuMovementAxis::Vertical);
        assert_eq!(canceled.dirty_regions.len(), 2);
    }

    #[test]
    fn rejects_conflicting_initial_board() {
        let puzzle = format!("11{}", "0".repeat(79));
        assert!(SudokuGame::from_puzzle(&puzzle).is_err());
    }
}
