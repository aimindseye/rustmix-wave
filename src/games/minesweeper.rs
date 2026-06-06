//! Native Minesweeper reference game for the SD Lua event bridge.
//!
//! The SD script declares one bounded beginner board through
//! `minesweeper.init(columns, rows, mines, seed)`. Rust owns placement,
//! first-reveal safety, flood reveal, flags, dirty metadata and redraws.

use crate::buttons::ButtonEvent;

use super::{
    canvas::{CanvasTextStyle, NativeGameCanvas},
    dirty_regions::DirtyRect,
};

pub const MINESWEEPER_COLUMNS: usize = 9;
pub const MINESWEEPER_ROWS: usize = 9;
pub const MINESWEEPER_CELL_COUNT: usize = MINESWEEPER_COLUMNS * MINESWEEPER_ROWS;
pub const MINESWEEPER_DEFAULT_MINES: usize = 10;
pub const MINESWEEPER_GRID_X: i32 = 51;
pub const MINESWEEPER_GRID_Y: i32 = 176;
pub const MINESWEEPER_CELL_SIZE: i32 = 42;
const MINESWEEPER_STATUS_RECT: DirtyRect = DirtyRect::new(16, 606, 448, 142);
const MINESWEEPER_BOARD_RECT: DirtyRect = DirtyRect::new(
    MINESWEEPER_GRID_X,
    MINESWEEPER_GRID_Y,
    MINESWEEPER_COLUMNS as i32 * MINESWEEPER_CELL_SIZE + 1,
    MINESWEEPER_ROWS as i32 * MINESWEEPER_CELL_SIZE + 1,
);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MinesweeperMode {
    Navigate,
    Action,
}

impl MinesweeperMode {
    #[must_use]
    pub const fn marker(self) -> &'static str {
        match self {
            Self::Navigate => "nav",
            Self::Action => "action",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MinesweeperMovementAxis {
    Horizontal,
    Vertical,
}

impl MinesweeperMovementAxis {
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MinesweeperAction {
    Reveal,
    Flag,
}

impl MinesweeperAction {
    #[must_use]
    pub const fn marker(self) -> &'static str {
        match self {
            Self::Reveal => "reveal",
            Self::Flag => "flag",
        }
    }

    #[must_use]
    pub const fn toggled(self) -> Self {
        match self {
            Self::Reveal => Self::Flag,
            Self::Flag => Self::Reveal,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MinesweeperOutcome {
    InProgress,
    Won,
    Lost,
}

impl MinesweeperOutcome {
    #[must_use]
    pub const fn marker(self) -> &'static str {
        match self {
            Self::InProgress => "in-progress",
            Self::Won => "won",
            Self::Lost => "lost",
        }
    }

    #[must_use]
    pub const fn completed(self) -> bool {
        !matches!(self, Self::InProgress)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MinesweeperEventResult {
    pub reason: &'static str,
    pub row: usize,
    pub column: usize,
    pub mode: MinesweeperMode,
    pub axis: MinesweeperMovementAxis,
    pub action: MinesweeperAction,
    pub flags: usize,
    pub safe_left: usize,
    pub outcome: MinesweeperOutcome,
    pub dirty_regions: Vec<DirtyRect>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MinesweeperGame {
    mines: [bool; MINESWEEPER_CELL_COUNT],
    revealed: [bool; MINESWEEPER_CELL_COUNT],
    flagged: [bool; MINESWEEPER_CELL_COUNT],
    adjacent: [u8; MINESWEEPER_CELL_COUNT],
    mine_count: usize,
    seed: u32,
    mines_placed: bool,
    cursor: usize,
    mode: MinesweeperMode,
    axis: MinesweeperMovementAxis,
    action: MinesweeperAction,
    status: String,
    outcome: MinesweeperOutcome,
}

impl MinesweeperGame {
    pub fn from_config(
        columns: usize,
        rows: usize,
        mine_count: usize,
        seed: u32,
    ) -> Result<Self, String> {
        if columns != MINESWEEPER_COLUMNS || rows != MINESWEEPER_ROWS {
            return Err(format!(
                "minesweeper v0.18.3 supports one bounded {}x{} board",
                MINESWEEPER_COLUMNS, MINESWEEPER_ROWS
            ));
        }
        if mine_count == 0 || mine_count > 20 {
            return Err("minesweeper mine count must be between 1 and 20".into());
        }
        Ok(Self {
            mines: [false; MINESWEEPER_CELL_COUNT],
            revealed: [false; MINESWEEPER_CELL_COUNT],
            flagged: [false; MINESWEEPER_CELL_COUNT],
            adjacent: [0; MINESWEEPER_CELL_COUNT],
            mine_count,
            seed: if seed == 0 { 0x6D2B_79F5 } else { seed },
            mines_placed: false,
            cursor: 0,
            mode: MinesweeperMode::Navigate,
            axis: MinesweeperMovementAxis::Horizontal,
            action: MinesweeperAction::Reveal,
            status: "AXIS H: UP/DOWN move  SELECT action".into(),
            outcome: MinesweeperOutcome::InProgress,
        })
    }

    #[must_use]
    pub const fn cursor_row(&self) -> usize {
        self.cursor / MINESWEEPER_COLUMNS
    }
    #[must_use]
    pub const fn cursor_column(&self) -> usize {
        self.cursor % MINESWEEPER_COLUMNS
    }
    #[must_use]
    pub const fn mode(&self) -> MinesweeperMode {
        self.mode
    }
    #[must_use]
    pub const fn movement_axis(&self) -> MinesweeperMovementAxis {
        self.axis
    }
    #[must_use]
    pub const fn action(&self) -> MinesweeperAction {
        self.action
    }
    #[must_use]
    pub const fn outcome(&self) -> MinesweeperOutcome {
        self.outcome
    }
    #[must_use]
    pub fn flags(&self) -> usize {
        self.flagged.iter().filter(|value| **value).count()
    }
    #[must_use]
    pub fn safe_left(&self) -> usize {
        if !self.mines_placed {
            return MINESWEEPER_CELL_COUNT - self.mine_count;
        }
        self.mines
            .iter()
            .zip(self.revealed.iter())
            .filter(|(mine, revealed)| !**mine && !**revealed)
            .count()
    }
    #[must_use]
    pub fn revealed(&self) -> &[bool; MINESWEEPER_CELL_COUNT] {
        &self.revealed
    }
    #[must_use]
    pub fn mines(&self) -> &[bool; MINESWEEPER_CELL_COUNT] {
        &self.mines
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
    ) -> Result<MinesweeperEventResult, String> {
        let old_cursor = self.cursor;
        let reason = if self.outcome.completed() {
            self.status = format!("Game {}  Hold BOOT back", self.outcome.marker());
            "game-finished"
        } else {
            match self.mode {
                MinesweeperMode::Navigate => self.apply_navigation_button(event),
                MinesweeperMode::Action => self.apply_action_button(event),
            }
        };
        self.render_commands(canvas)?;
        canvas.reset_dirty_regions();
        let dirty_regions = self.dirty_regions_for(reason, old_cursor);
        for rect in &dirty_regions {
            canvas.invalidate_rect(*rect);
        }
        canvas.request_refresh();
        Ok(self.result(reason, dirty_regions))
    }

    pub fn apply_boot_short_press_and_render(
        &mut self,
        canvas: &mut NativeGameCanvas,
    ) -> Result<MinesweeperEventResult, String> {
        let reason = match self.mode {
            MinesweeperMode::Navigate => {
                self.axis = self.axis.toggled();
                self.status = format!(
                    "AXIS {}: UP/DOWN move  SELECT action",
                    self.axis.short_marker()
                );
                "axis-toggle"
            }
            MinesweeperMode::Action => {
                self.mode = MinesweeperMode::Navigate;
                self.status = format!(
                    "Action canceled  AXIS {}: UP/DOWN move",
                    self.axis.short_marker()
                );
                "action-cancel"
            }
        };
        self.render_commands(canvas)?;
        canvas.reset_dirty_regions();
        let dirty_regions = if reason == "axis-toggle" {
            vec![MINESWEEPER_STATUS_RECT]
        } else {
            vec![cell_rect(self.cursor), MINESWEEPER_STATUS_RECT]
        };
        for rect in &dirty_regions {
            canvas.invalidate_rect(*rect);
        }
        canvas.request_refresh();
        Ok(self.result(reason, dirty_regions))
    }

    fn apply_navigation_button(&mut self, event: ButtonEvent) -> &'static str {
        match event {
            ButtonEvent::Up => {
                self.cursor = previous_cell(self.cursor, self.axis);
                self.status = format!(
                    "AXIS {}: UP/DOWN move  SELECT action",
                    self.axis.short_marker()
                );
                "cursor-move"
            }
            ButtonEvent::Down => {
                self.cursor = next_cell(self.cursor, self.axis);
                self.status = format!(
                    "AXIS {}: UP/DOWN move  SELECT action",
                    self.axis.short_marker()
                );
                "cursor-move"
            }
            ButtonEvent::Select => {
                self.mode = MinesweeperMode::Action;
                self.action = MinesweeperAction::Reveal;
                self.status = "ACTION REVEAL: UP/DOWN choose  SELECT apply".into();
                "action-enter"
            }
        }
    }

    fn apply_action_button(&mut self, event: ButtonEvent) -> &'static str {
        match event {
            ButtonEvent::Up | ButtonEvent::Down => {
                self.action = self.action.toggled();
                self.status = format!(
                    "ACTION {}: UP/DOWN choose  SELECT apply",
                    self.action.marker().to_ascii_uppercase()
                );
                "action-choice"
            }
            ButtonEvent::Select => self.apply_selected_action(),
        }
    }

    fn apply_selected_action(&mut self) -> &'static str {
        match self.action {
            MinesweeperAction::Flag => {
                if self.revealed[self.cursor] {
                    self.status = "Revealed cells cannot be flagged".into();
                    return "revealed-cell";
                }
                self.flagged[self.cursor] = !self.flagged[self.cursor];
                self.mode = MinesweeperMode::Navigate;
                self.status = if self.flagged[self.cursor] {
                    "Flag placed".into()
                } else {
                    "Flag removed".into()
                };
                "flag-toggle"
            }
            MinesweeperAction::Reveal => {
                if self.flagged[self.cursor] {
                    self.status = "Remove flag before reveal".into();
                    return "flagged-cell";
                }
                if !self.mines_placed {
                    self.place_mines_away_from(self.cursor);
                }
                self.mode = MinesweeperMode::Navigate;
                if self.mines[self.cursor] {
                    self.reveal_all_mines();
                    self.outcome = MinesweeperOutcome::Lost;
                    self.status = "Mine hit  Hold BOOT back".into();
                    return "mine-hit";
                }
                self.reveal_region(self.cursor);
                if self.safe_left() == 0 {
                    self.outcome = MinesweeperOutcome::Won;
                    self.status = "Board cleared  You win".into();
                    "win"
                } else {
                    self.status = "Cells revealed".into();
                    "reveal"
                }
            }
        }
    }

    fn dirty_regions_for(&self, reason: &'static str, old_cursor: usize) -> Vec<DirtyRect> {
        match reason {
            "reveal" | "mine-hit" | "win" => vec![MINESWEEPER_BOARD_RECT, MINESWEEPER_STATUS_RECT],
            "action-choice" | "game-finished" => vec![MINESWEEPER_STATUS_RECT],
            "action-enter" | "flag-toggle" | "flagged-cell" | "revealed-cell" => {
                vec![cell_rect(self.cursor), MINESWEEPER_STATUS_RECT]
            }
            _ => {
                let mut regions = vec![
                    cell_rect(old_cursor),
                    cell_rect(self.cursor),
                    MINESWEEPER_STATUS_RECT,
                ];
                regions.dedup();
                regions
            }
        }
    }

    fn result(
        &self,
        reason: &'static str,
        dirty_regions: Vec<DirtyRect>,
    ) -> MinesweeperEventResult {
        MinesweeperEventResult {
            reason,
            row: self.cursor_row(),
            column: self.cursor_column(),
            mode: self.mode,
            axis: self.axis,
            action: self.action,
            flags: self.flags(),
            safe_left: self.safe_left(),
            outcome: self.outcome,
            dirty_regions,
        }
    }

    fn place_mines_away_from(&mut self, safe_index: usize) {
        let mut random = self.seed ^ (safe_index as u32).wrapping_mul(0x9E37_79B9);
        let mut placed = 0;
        while placed < self.mine_count {
            random = xorshift32(random);
            let index = random as usize % MINESWEEPER_CELL_COUNT;
            if self.mines[index] || is_first_reveal_safe_neighbor(index, safe_index) {
                continue;
            }
            self.mines[index] = true;
            placed += 1;
        }
        for index in 0..MINESWEEPER_CELL_COUNT {
            self.adjacent[index] = neighbor_indices(index)
                .iter()
                .flatten()
                .filter(|neighbor| self.mines[**neighbor])
                .count() as u8;
        }
        self.mines_placed = true;
    }

    fn reveal_region(&mut self, start: usize) {
        let mut queue = [0_usize; MINESWEEPER_CELL_COUNT];
        let mut queued = [false; MINESWEEPER_CELL_COUNT];
        let mut head = 0;
        let mut tail = 1;
        queue[0] = start;
        queued[start] = true;
        while head < tail {
            let index = queue[head];
            head += 1;
            if self.revealed[index] || self.flagged[index] || self.mines[index] {
                continue;
            }
            self.revealed[index] = true;
            if self.adjacent[index] != 0 {
                continue;
            }
            for neighbor in neighbor_indices(index).iter().flatten().copied() {
                if !queued[neighbor] && !self.flagged[neighbor] && !self.mines[neighbor] {
                    queued[neighbor] = true;
                    queue[tail] = neighbor;
                    tail += 1;
                }
            }
        }
    }

    fn reveal_all_mines(&mut self) {
        for index in 0..MINESWEEPER_CELL_COUNT {
            if self.mines[index] {
                self.revealed[index] = true;
            }
        }
    }

    fn render_commands(&self, canvas: &mut NativeGameCanvas) -> Result<(), String> {
        canvas.clear_frame();
        canvas.text(24, 66, "Minesweeper".into(), CanvasTextStyle::Heading)?;
        canvas.text(
            24,
            104,
            "SD Lua declaration / native beginner board".into(),
            CanvasTextStyle::Detail,
        )?;
        canvas.grid(
            MINESWEEPER_GRID_X,
            MINESWEEPER_GRID_Y,
            MINESWEEPER_COLUMNS as u8,
            MINESWEEPER_ROWS as u8,
            MINESWEEPER_CELL_SIZE,
            MINESWEEPER_CELL_SIZE,
        )?;
        for index in 0..MINESWEEPER_CELL_COUNT {
            let value = if self.flagged[index] && !self.revealed[index] {
                Some("F".to_string())
            } else if self.revealed[index] && self.mines[index] {
                Some("*".to_string())
            } else if self.revealed[index] && self.adjacent[index] > 0 {
                Some(self.adjacent[index].to_string())
            } else {
                None
            };
            if let Some(value) = value {
                canvas.text(
                    MINESWEEPER_GRID_X
                        + (index % MINESWEEPER_COLUMNS) as i32 * MINESWEEPER_CELL_SIZE
                        + 14,
                    MINESWEEPER_GRID_Y
                        + (index / MINESWEEPER_COLUMNS) as i32 * MINESWEEPER_CELL_SIZE
                        + 30,
                    value,
                    CanvasTextStyle::Heading,
                )?;
            }
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
            640,
            format!(
                "R{} C{}  MODE {}  AXIS {}  FLAGS {}/{}",
                self.cursor_row() + 1,
                self.cursor_column() + 1,
                self.mode.marker().to_ascii_uppercase(),
                self.axis.short_marker(),
                self.flags(),
                self.mine_count,
            ),
            CanvasTextStyle::Body,
        )?;
        canvas.text(
            24,
            682,
            format!("SAFE LEFT {}  {}", self.safe_left(), self.status),
            CanvasTextStyle::Detail,
        )?;
        let footer = match self.mode {
            MinesweeperMode::Navigate => "BOOT short axis  SELECT action",
            MinesweeperMode::Action => "UP/DOWN action  BOOT short cancel",
        };
        canvas.text(24, 742, footer.to_string(), CanvasTextStyle::Detail)?;
        canvas.request_refresh();
        Ok(())
    }
}

fn xorshift32(mut value: u32) -> u32 {
    if value == 0 {
        value = 0x6D2B_79F5;
    }
    value ^= value << 13;
    value ^= value >> 17;
    value ^= value << 5;
    value
}

fn is_first_reveal_safe_neighbor(index: usize, safe: usize) -> bool {
    let row = index / MINESWEEPER_COLUMNS;
    let column = index % MINESWEEPER_COLUMNS;
    let safe_row = safe / MINESWEEPER_COLUMNS;
    let safe_column = safe % MINESWEEPER_COLUMNS;
    row.abs_diff(safe_row) <= 1 && column.abs_diff(safe_column) <= 1
}

fn neighbor_indices(index: usize) -> [Option<usize>; 8] {
    let row = index / MINESWEEPER_COLUMNS;
    let column = index % MINESWEEPER_COLUMNS;
    let mut neighbors = [None; 8];
    let mut output = 0;
    for row_delta in -1_i32..=1 {
        for column_delta in -1_i32..=1 {
            if row_delta == 0 && column_delta == 0 {
                continue;
            }
            let candidate_row = row as i32 + row_delta;
            let candidate_column = column as i32 + column_delta;
            if (0..MINESWEEPER_ROWS as i32).contains(&candidate_row)
                && (0..MINESWEEPER_COLUMNS as i32).contains(&candidate_column)
            {
                neighbors[output] =
                    Some(candidate_row as usize * MINESWEEPER_COLUMNS + candidate_column as usize);
                output += 1;
            }
        }
    }
    neighbors
}

fn previous_cell(index: usize, axis: MinesweeperMovementAxis) -> usize {
    let row = index / MINESWEEPER_COLUMNS;
    let column = index % MINESWEEPER_COLUMNS;
    match axis {
        MinesweeperMovementAxis::Horizontal => {
            row * MINESWEEPER_COLUMNS + column.checked_sub(1).unwrap_or(MINESWEEPER_COLUMNS - 1)
        }
        MinesweeperMovementAxis::Vertical => {
            row.checked_sub(1).unwrap_or(MINESWEEPER_ROWS - 1) * MINESWEEPER_COLUMNS + column
        }
    }
}

fn next_cell(index: usize, axis: MinesweeperMovementAxis) -> usize {
    let row = index / MINESWEEPER_COLUMNS;
    let column = index % MINESWEEPER_COLUMNS;
    match axis {
        MinesweeperMovementAxis::Horizontal => {
            row * MINESWEEPER_COLUMNS + (column + 1) % MINESWEEPER_COLUMNS
        }
        MinesweeperMovementAxis::Vertical => {
            (row + 1) % MINESWEEPER_ROWS * MINESWEEPER_COLUMNS + column
        }
    }
}

fn cell_rect(index: usize) -> DirtyRect {
    DirtyRect::new(
        MINESWEEPER_GRID_X + (index % MINESWEEPER_COLUMNS) as i32 * MINESWEEPER_CELL_SIZE,
        MINESWEEPER_GRID_Y + (index / MINESWEEPER_COLUMNS) as i32 * MINESWEEPER_CELL_SIZE,
        MINESWEEPER_CELL_SIZE + 1,
        MINESWEEPER_CELL_SIZE + 1,
    )
}

#[cfg(test)]
mod tests {
    use crate::{buttons::ButtonEvent, games::canvas::NativeGameCanvas};

    use super::{MinesweeperGame, MinesweeperMode, MinesweeperMovementAxis, MinesweeperOutcome};

    #[test]
    fn loads_beginner_board_and_renders_bounded_canvas() {
        let game = MinesweeperGame::from_config(9, 9, 10, 1803).unwrap();
        let mut canvas = NativeGameCanvas::default();
        game.render_initial(&mut canvas).unwrap();
        assert!(!canvas.commands().is_empty());
        assert!(canvas.commands().len() < 256);
        assert!(canvas.refresh_requested());
    }

    #[test]
    fn first_reveal_is_safe_and_places_ten_mines() {
        let mut game = MinesweeperGame::from_config(9, 9, 10, 1803).unwrap();
        let mut canvas = NativeGameCanvas::default();
        game.render_initial(&mut canvas).unwrap();
        game.apply_button_and_render(ButtonEvent::Select, &mut canvas)
            .unwrap();
        let event = game
            .apply_button_and_render(ButtonEvent::Select, &mut canvas)
            .unwrap();
        assert_eq!(event.reason, "reveal");
        assert!(!game.mines()[0]);
        assert!(game.revealed()[0]);
        assert_eq!(game.mines().iter().filter(|value| **value).count(), 10);
    }

    #[test]
    fn boot_short_toggles_axis_in_nav_and_cancels_action_mode() {
        let mut game = MinesweeperGame::from_config(9, 9, 10, 1803).unwrap();
        let mut canvas = NativeGameCanvas::default();
        let axis = game.apply_boot_short_press_and_render(&mut canvas).unwrap();
        assert_eq!(axis.reason, "axis-toggle");
        assert_eq!(axis.axis, MinesweeperMovementAxis::Vertical);
        game.apply_button_and_render(ButtonEvent::Select, &mut canvas)
            .unwrap();
        assert_eq!(game.mode(), MinesweeperMode::Action);
        let cancel = game.apply_boot_short_press_and_render(&mut canvas).unwrap();
        assert_eq!(cancel.reason, "action-cancel");
        assert_eq!(game.mode(), MinesweeperMode::Navigate);
    }

    #[test]
    fn can_place_and_remove_flag() {
        let mut game = MinesweeperGame::from_config(9, 9, 10, 1803).unwrap();
        let mut canvas = NativeGameCanvas::default();
        game.apply_button_and_render(ButtonEvent::Select, &mut canvas)
            .unwrap();
        game.apply_button_and_render(ButtonEvent::Down, &mut canvas)
            .unwrap();
        let flag = game
            .apply_button_and_render(ButtonEvent::Select, &mut canvas)
            .unwrap();
        assert_eq!(flag.reason, "flag-toggle");
        assert_eq!(flag.flags, 1);
        assert_eq!(game.outcome(), MinesweeperOutcome::InProgress);
    }
}
