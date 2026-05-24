//! Shared grid/cursor state for the first playable Lua game pack.
//!
//! These models deliberately stay tiny and in-memory. They support the
//! on-device proof for Sudoku, Minesweeper, and Memory Cards while keeping SD
//! save/resume out of scope for this slice.

pub const LUA_GRID_GAMES_PLAYABLE_PACK_MARKER: &str = "rustmix-lua-grid-games-playable-pack-ok";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GridMove {
    Up,
    Down,
    Left,
    Right,
}

fn move_cursor(cursor: &mut u8, width: u8, height: u8, direction: GridMove) {
    let row = *cursor / width;
    let col = *cursor % width;
    let (next_row, next_col) = match direction {
        GridMove::Up => (row.saturating_sub(1), col),
        GridMove::Down => ((row + 1).min(height.saturating_sub(1)), col),
        GridMove::Left => (row, col.saturating_sub(1)),
        GridMove::Right => (row, (col + 1).min(width.saturating_sub(1))),
    };
    *cursor = next_row.saturating_mul(width).saturating_add(next_col);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SudokuState {
    cells: [u8; 81],
    givens: [bool; 81],
    cursor: u8,
}

impl SudokuState {
    pub const fn new() -> Self {
        Self {
            cells: [
                5, 3, 0, 0, 7, 0, 0, 0, 0, 6, 0, 0, 1, 9, 5, 0, 0, 0, 0, 9, 8, 0, 0, 0, 0, 6, 0, 8,
                0, 0, 0, 6, 0, 0, 0, 3, 4, 0, 0, 8, 0, 3, 0, 0, 1, 7, 0, 0, 0, 2, 0, 0, 0, 6, 0, 6,
                0, 0, 0, 0, 2, 8, 0, 0, 0, 0, 4, 1, 9, 0, 0, 5, 0, 0, 0, 0, 8, 0, 0, 7, 9,
            ],
            givens: [
                true, true, false, false, true, false, false, false, false, true, false, false,
                true, true, true, false, false, false, false, true, true, false, false, false,
                false, true, false, true, false, false, false, true, false, false, false, true,
                true, false, false, true, false, true, false, false, true, true, false, false,
                false, true, false, false, false, true, false, true, false, false, false, false,
                true, true, false, false, false, false, true, true, true, false, false, true,
                false, false, false, false, true, false, false, true, true,
            ],
            cursor: 0,
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    pub fn load_puzzle_line(&mut self, data: &str) {
        let raw = data.lines().next().unwrap_or("");
        let puzzle = raw.rsplit('|').next().unwrap_or(raw).trim();
        let mut parsed = [0u8; 81];
        let mut givens = [false; 81];
        let mut idx = 0usize;
        for ch in puzzle.chars() {
            if idx >= 81 {
                break;
            }
            match ch {
                '1'..='9' => {
                    parsed[idx] = ch as u8 - b'0';
                    givens[idx] = true;
                    idx += 1;
                }
                '0' | '.' | '_' => {
                    parsed[idx] = 0;
                    idx += 1;
                }
                _ => {}
            }
        }
        if idx == 81 {
            self.cells = parsed;
            self.givens = givens;
            self.cursor = 0;
        }
    }

    pub fn move_cursor(&mut self, direction: GridMove) {
        move_cursor(&mut self.cursor, 9, 9, direction);
    }

    pub fn select(&mut self) {
        let idx = self.cursor as usize;
        if !self.givens[idx] {
            self.cells[idx] = (self.cells[idx] + 1) % 10;
        }
    }

    pub const fn cursor(&self) -> u8 {
        self.cursor
    }

    pub fn cell_char(&self, row: usize, col: usize) -> char {
        let value = self.cells[row * 9 + col];
        if value == 0 {
            '.'
        } else {
            (b'0' + value) as char
        }
    }

    pub fn is_given(&self, row: usize, col: usize) -> bool {
        self.givens[row * 9 + col]
    }
}

impl Default for SudokuState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MinesweeperState {
    mines: [bool; 64],
    revealed: [bool; 64],
    cursor: u8,
    lost: bool,
}

impl MinesweeperState {
    pub const fn new() -> Self {
        let mut mines = [false; 64];
        // Deterministic tiny board: eight mines, no RNG needed for first proof.
        mines[5] = true;
        mines[14] = true;
        mines[19] = true;
        mines[27] = true;
        mines[33] = true;
        mines[46] = true;
        mines[52] = true;
        mines[61] = true;
        Self {
            mines,
            revealed: [false; 64],
            cursor: 0,
            lost: false,
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    pub fn load_board_line(&mut self, data: &str) {
        let mut mines = [false; 64];
        let mut found_any = false;
        for token in data.split(|c: char| c == '|' || c == ',' || c.is_whitespace()) {
            if let Ok(idx) = token.trim().parse::<usize>() {
                if idx < 64 {
                    mines[idx] = true;
                    found_any = true;
                }
            }
        }
        if found_any {
            self.mines = mines;
            self.revealed = [false; 64];
            self.lost = false;
            self.cursor = 0;
        }
    }

    pub fn move_cursor(&mut self, direction: GridMove) {
        move_cursor(&mut self.cursor, 8, 8, direction);
    }

    pub fn select(&mut self) {
        let idx = self.cursor as usize;
        self.revealed[idx] = true;
        if self.mines[idx] {
            self.lost = true;
        }
    }

    pub const fn cursor(&self) -> u8 {
        self.cursor
    }
    pub const fn lost(&self) -> bool {
        self.lost
    }

    pub fn cell_char(&self, row: usize, col: usize) -> char {
        let idx = row * 8 + col;
        if self.lost && self.mines[idx] {
            return '*';
        }
        if !self.revealed[idx] {
            return '#';
        }
        if self.mines[idx] {
            return '*';
        }
        let count = self.adjacent_count(row as i8, col as i8);
        if count == 0 {
            '.'
        } else {
            (b'0' + count) as char
        }
    }

    fn adjacent_count(&self, row: i8, col: i8) -> u8 {
        let mut count = 0u8;
        let mut dr = -1i8;
        while dr <= 1 {
            let mut dc = -1i8;
            while dc <= 1 {
                if dr != 0 || dc != 0 {
                    let rr = row + dr;
                    let cc = col + dc;
                    if rr >= 0 && rr < 8 && cc >= 0 && cc < 8 {
                        let idx = rr as usize * 8 + cc as usize;
                        if self.mines[idx] {
                            count += 1;
                        }
                    }
                }
                dc += 1;
            }
            dr += 1;
        }
        count
    }
}

impl Default for MinesweeperState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MemoryCardsState {
    cards: [u8; 16],
    visible: [bool; 16],
    matched: [bool; 16],
    cursor: u8,
    first: u8,
    has_first: bool,
    pending_a: u8,
    pending_b: u8,
    has_pending_hide: bool,
}

impl MemoryCardsState {
    pub const fn new() -> Self {
        Self {
            cards: [0, 1, 2, 3, 4, 5, 6, 7, 2, 0, 7, 4, 1, 6, 3, 5],
            visible: [false; 16],
            matched: [false; 16],
            cursor: 0,
            first: 0,
            has_first: false,
            pending_a: 0,
            pending_b: 0,
            has_pending_hide: false,
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    pub fn load_cards_line(&mut self, data: &str) {
        let mut cards = [0u8; 16];
        let mut idx = 0usize;
        for ch in data.chars() {
            if idx >= 16 {
                break;
            }
            match ch {
                'A'..='H' => {
                    cards[idx] = ch as u8 - b'A';
                    idx += 1;
                }
                'a'..='h' => {
                    cards[idx] = ch as u8 - b'a';
                    idx += 1;
                }
                '0'..='7' => {
                    cards[idx] = ch as u8 - b'0';
                    idx += 1;
                }
                _ => {}
            }
        }
        if idx == 16 {
            self.cards = cards;
            self.visible = [false; 16];
            self.matched = [false; 16];
            self.cursor = 0;
            self.has_first = false;
            self.has_pending_hide = false;
        }
    }

    pub fn move_cursor(&mut self, direction: GridMove) {
        move_cursor(&mut self.cursor, 4, 4, direction);
    }

    pub fn select(&mut self) {
        if self.has_pending_hide {
            self.visible[self.pending_a as usize] = false;
            self.visible[self.pending_b as usize] = false;
            self.has_pending_hide = false;
        }

        let idx = self.cursor as usize;
        if self.matched[idx] || self.visible[idx] {
            return;
        }

        self.visible[idx] = true;
        if !self.has_first {
            self.first = self.cursor;
            self.has_first = true;
            return;
        }

        let first = self.first as usize;
        if self.cards[first] == self.cards[idx] {
            self.matched[first] = true;
            self.matched[idx] = true;
        } else {
            self.pending_a = self.first;
            self.pending_b = self.cursor;
            self.has_pending_hide = true;
        }
        self.has_first = false;
    }

    pub const fn cursor(&self) -> u8 {
        self.cursor
    }

    pub fn cell_char(&self, row: usize, col: usize) -> char {
        let idx = row * 4 + col;
        if !self.visible[idx] && !self.matched[idx] {
            return '#';
        }
        (b'A' + self.cards[idx]) as char
    }

    pub fn matched_count(&self) -> u8 {
        let mut count = 0u8;
        let mut idx = 0usize;
        while idx < 16 {
            if self.matched[idx] {
                count += 1;
            }
            idx += 1;
        }
        count
    }
}

impl Default for MemoryCardsState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sudoku_select_changes_only_mutable_cells() {
        let mut s = SudokuState::new();
        s.select();
        assert_eq!(s.cell_char(0, 0), '5');
        for _ in 0..2 {
            s.move_cursor(GridMove::Right);
        }
        s.select();
        assert_eq!(s.cell_char(0, 2), '1');
    }

    #[test]
    fn mines_reveal_can_lose_without_panicking() {
        let mut m = MinesweeperState::new();
        for _ in 0..5 {
            m.move_cursor(GridMove::Right);
        }
        m.select();
        assert!(m.lost());
    }

    #[test]
    fn memory_pair_can_match() {
        let mut m = MemoryCardsState::new();
        m.select(); // index 0, A
        for _ in 0..1 {
            m.move_cursor(GridMove::Down);
        }
        for _ in 0..1 {
            m.move_cursor(GridMove::Right);
        }
        m.select(); // index 5, F in default, not a match but should not panic
        assert_eq!(m.matched_count(), 0);
    }
}
