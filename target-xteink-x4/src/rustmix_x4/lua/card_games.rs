//! Shared in-memory card table state for the Lua Card Games Playable Pack.
//!
//! FreeCell and Solitaire intentionally start with tiny, deterministic,
//! in-memory card-table behavior. Legal move enforcement and SD save/resume
//! are intentionally deferred until the UI/input model is accepted on-device.

use super::grid_games::GridMove;

pub const LUA_CARD_GAMES_PLAYABLE_PACK_MARKER: &str = "rustmix-lua-card-games-playable-pack-ok";

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

fn rank_char(value: u8) -> char {
    match value % 13 {
        0 => 'A',
        1 => '2',
        2 => '3',
        3 => '4',
        4 => '5',
        5 => '6',
        6 => '7',
        7 => '8',
        8 => '9',
        9 => 'T',
        10 => 'J',
        11 => 'Q',
        _ => 'K',
    }
}

fn parse_rank(ch: char) -> Option<u8> {
    match ch {
        'A' | 'a' => Some(1),
        '2' => Some(2),
        '3' => Some(3),
        '4' => Some(4),
        '5' => Some(5),
        '6' => Some(6),
        '7' => Some(7),
        '8' => Some(8),
        '9' => Some(9),
        'T' | 't' => Some(10),
        'J' | 'j' => Some(11),
        'Q' | 'q' => Some(12),
        'K' | 'k' => Some(13),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FreeCellState {
    cells: [u8; 56],
    cursor: u8,
    picked: u8,
    has_picked: bool,
    moves: u16,
}

impl FreeCellState {
    pub const fn new() -> Self {
        let mut cells = [0u8; 56];
        let mut idx = 0usize;
        while idx < 52 {
            cells[idx] = (idx % 13 + 1) as u8;
            idx += 1;
        }
        Self {
            cells,
            cursor: 0,
            picked: 0,
            has_picked: false,
            moves: 0,
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    pub fn load_cards_line(&mut self, data: &str) {
        let mut cells = [0u8; 56];
        let mut idx = 0usize;
        for ch in data.chars() {
            if idx >= 56 {
                break;
            }
            if ch == '.' || ch == '_' || ch == '-' {
                cells[idx] = 0;
                idx += 1;
            } else if let Some(rank) = parse_rank(ch) {
                cells[idx] = rank;
                idx += 1;
            }
        }
        if idx >= 16 {
            self.cells = cells;
            self.cursor = 0;
            self.has_picked = false;
            self.moves = 0;
        }
    }

    pub fn move_cursor(&mut self, direction: GridMove) {
        move_cursor(&mut self.cursor, 8, 7, direction);
    }

    pub fn select(&mut self) {
        let idx = self.cursor as usize;
        if !self.has_picked {
            if self.cells[idx] != 0 {
                self.picked = self.cursor;
                self.has_picked = true;
            }
            return;
        }
        let picked = self.picked as usize;
        if picked != idx {
            self.cells.swap(picked, idx);
            self.moves = self.moves.saturating_add(1);
        }
        self.has_picked = false;
    }

    pub const fn cursor(&self) -> u8 {
        self.cursor
    }
    pub const fn moves(&self) -> u16 {
        self.moves
    }
    pub const fn has_picked(&self) -> bool {
        self.has_picked
    }

    pub fn cell_char(&self, row: usize, col: usize) -> char {
        let value = self.cells[row * 8 + col];
        if value == 0 {
            '.'
        } else {
            rank_char(value.saturating_sub(1))
        }
    }

    pub fn is_highlighted(&self, row: usize, col: usize) -> bool {
        let idx = (row * 8 + col) as u8;
        idx == self.cursor || (self.has_picked && idx == self.picked)
    }
}

impl Default for FreeCellState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SolitaireState {
    cells: [u8; 49],
    cursor: u8,
    picked: u8,
    has_picked: bool,
    moves: u16,
}

impl SolitaireState {
    pub const fn new() -> Self {
        let mut cells = [0u8; 49];
        let mut pile = 0usize;
        let mut rank = 1u8;
        while pile < 7 {
            let mut row = 0usize;
            while row <= pile {
                cells[row * 7 + pile] = rank;
                rank = if rank >= 13 { 1 } else { rank + 1 };
                row += 1;
            }
            pile += 1;
        }
        Self {
            cells,
            cursor: 0,
            picked: 0,
            has_picked: false,
            moves: 0,
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    pub fn load_cards_line(&mut self, data: &str) {
        let mut cells = [0u8; 49];
        let mut idx = 0usize;
        for ch in data.chars() {
            if idx >= 49 {
                break;
            }
            if ch == '.' || ch == '_' || ch == '-' {
                cells[idx] = 0;
                idx += 1;
            } else if let Some(rank) = parse_rank(ch) {
                cells[idx] = rank;
                idx += 1;
            }
        }
        if idx >= 7 {
            self.cells = cells;
            self.cursor = 0;
            self.has_picked = false;
            self.moves = 0;
        }
    }

    pub fn move_cursor(&mut self, direction: GridMove) {
        move_cursor(&mut self.cursor, 7, 7, direction);
    }

    pub fn select(&mut self) {
        let idx = self.cursor as usize;
        if !self.has_picked {
            if self.cells[idx] != 0 {
                self.picked = self.cursor;
                self.has_picked = true;
            }
            return;
        }
        let picked = self.picked as usize;
        if picked != idx {
            self.cells.swap(picked, idx);
            self.moves = self.moves.saturating_add(1);
        }
        self.has_picked = false;
    }

    pub const fn cursor(&self) -> u8 {
        self.cursor
    }
    pub const fn moves(&self) -> u16 {
        self.moves
    }
    pub const fn has_picked(&self) -> bool {
        self.has_picked
    }

    pub fn cell_char(&self, row: usize, col: usize) -> char {
        let value = self.cells[row * 7 + col];
        if value == 0 {
            '.'
        } else {
            rank_char(value.saturating_sub(1))
        }
    }

    pub fn is_highlighted(&self, row: usize, col: usize) -> bool {
        let idx = (row * 7 + col) as u8;
        idx == self.cursor || (self.has_picked && idx == self.picked)
    }
}

impl Default for SolitaireState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn freecell_cursor_and_pick_drop_are_safe() {
        let mut game = FreeCellState::new();
        assert_eq!(game.cell_char(0, 0), 'A');
        game.select();
        assert!(game.has_picked());
        game.move_cursor(GridMove::Right);
        game.select();
        assert_eq!(game.moves(), 1);
    }

    #[test]
    fn solitaire_cursor_and_pick_drop_are_safe() {
        let mut game = SolitaireState::new();
        game.select();
        assert!(game.has_picked());
        game.move_cursor(GridMove::Right);
        game.select();
        assert_eq!(game.moves(), 1);
    }

    #[test]
    fn card_data_loader_accepts_rank_text() {
        let mut game = FreeCellState::new();
        game.load_cards_line("A23456789TJQK........");
        assert_eq!(game.cell_char(0, 0), 'A');
        assert_eq!(game.cell_char(0, 1), '2');
    }
}
