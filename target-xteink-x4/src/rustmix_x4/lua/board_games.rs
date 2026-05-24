//! Shared in-memory board game state for Lua board games.
//!
//! This first slice intentionally keeps Ludo and Snakes/Ladder compact and
//! deterministic. It proves board-game rendering, button input, and safe exit
//! before adding save/resume or richer rules.

use super::grid_games::GridMove;

pub const LUA_BOARD_GAMES_PLAYABLE_PACK_MARKER: &str = "rustmix-lua-board-games-playable-pack-ok";

const BOARD_W: u8 = 10;
const BOARD_H: u8 = 10;
const BOARD_CELLS: u8 = BOARD_W * BOARD_H;

fn board_cell_index(row: usize, col: usize) -> u8 {
    let row = row.min(9) as u8;
    let col = col.min(9) as u8;
    let base = (BOARD_H - 1 - row) * BOARD_W;
    if ((BOARD_H - 1 - row) & 1) == 0 {
        base + col
    } else {
        base + (BOARD_W - 1 - col)
    }
}

fn deterministic_die(rolls: u16) -> u8 {
    ((rolls % 6) as u8) + 1
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LudoState {
    tokens: [u8; 4],
    selected: u8,
    last_die: u8,
    rolls: u16,
}

impl LudoState {
    pub const fn new() -> Self {
        Self {
            tokens: [0, 0, 0, 0],
            selected: 0,
            last_die: 0,
            rolls: 0,
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    pub fn load_config_line(&mut self, data: &str) {
        // Optional format: positions=0,6,12,18
        for line in data.lines() {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix("positions=") {
                let mut parsed = [0u8; 4];
                let mut count = 0usize;
                for part in rest.split(',') {
                    if count >= 4 {
                        break;
                    }
                    if let Ok(value) = part.trim().parse::<u8>() {
                        parsed[count] = value.min(56);
                        count += 1;
                    }
                }
                if count == 4 {
                    self.tokens = parsed;
                    self.selected = 0;
                    self.last_die = 0;
                    self.rolls = 0;
                }
            }
        }
    }

    pub fn move_cursor(&mut self, direction: GridMove) {
        match direction {
            GridMove::Left | GridMove::Up => self.selected = self.selected.saturating_sub(1),
            GridMove::Right | GridMove::Down => self.selected = (self.selected + 1).min(3),
        }
    }

    pub fn select(&mut self) {
        let die = deterministic_die(self.rolls);
        self.rolls = self.rolls.saturating_add(1);
        self.last_die = die;
        let idx = self.selected as usize;
        self.tokens[idx] = self.tokens[idx].saturating_add(die).min(56);
        self.selected = (self.selected + 1) % 4;
    }

    pub const fn selected(&self) -> u8 {
        self.selected
    }
    pub const fn last_die(&self) -> u8 {
        self.last_die
    }
    pub const fn rolls(&self) -> u16 {
        self.rolls
    }
    pub const fn token_position(&self, idx: usize) -> u8 {
        self.tokens[idx]
    }

    pub fn finished(&self) -> bool {
        self.tokens.iter().all(|p| *p >= 56)
    }

    pub fn cell_char(&self, row: usize, col: usize) -> char {
        let cell = board_cell_index(row, col);
        for idx in 0..4usize {
            if self.tokens[idx] == cell.min(56) {
                return match idx {
                    0 => 'A',
                    1 => 'B',
                    2 => 'C',
                    _ => 'D',
                };
            }
        }
        if cell <= 56 { '.' } else { ' ' }
    }

    pub fn is_highlighted(&self, row: usize, col: usize) -> bool {
        let cell = board_cell_index(row, col).min(56);
        self.tokens[self.selected as usize] == cell
    }
}

impl Default for LudoState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SnakesState {
    position: u8,
    last_die: u8,
    rolls: u16,
    won: bool,
}

impl SnakesState {
    pub const fn new() -> Self {
        Self {
            position: 0,
            last_die: 0,
            rolls: 0,
            won: false,
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    pub fn load_board_line(&mut self, _data: &str) {
        // Reserved for future custom snakes/ladders. The first playable slice
        // keeps a deterministic built-in board so SD data remains optional.
        self.reset();
    }

    pub fn move_cursor(&mut self, _direction: GridMove) {
        // Snakes/Ladder is dice-driven. Arrow keys intentionally do not move
        // the token; Select rolls and redraws.
    }

    pub fn select(&mut self) {
        if self.won {
            return;
        }
        let die = deterministic_die(self.rolls);
        self.rolls = self.rolls.saturating_add(1);
        self.last_die = die;
        let next = self.position.saturating_add(die).min(99);
        self.position = Self::apply_jump(next);
        self.won = self.position >= 99;
    }

    const fn apply_jump(pos: u8) -> u8 {
        match pos {
            3 => 22,
            8 => 30,
            20 => 41,
            27 => 84,
            36 => 44,
            50 => 67,
            70 => 91,
            16 => 6,
            47 => 26,
            55 => 35,
            62 => 18,
            87 => 24,
            95 => 75,
            _ => pos,
        }
    }

    const fn is_ladder(pos: u8) -> bool {
        matches!(pos, 3 | 8 | 20 | 27 | 36 | 50 | 70)
    }

    const fn is_snake(pos: u8) -> bool {
        matches!(pos, 16 | 47 | 55 | 62 | 87 | 95)
    }

    pub const fn position(&self) -> u8 {
        self.position
    }
    pub const fn last_die(&self) -> u8 {
        self.last_die
    }
    pub const fn rolls(&self) -> u16 {
        self.rolls
    }
    pub const fn won(&self) -> bool {
        self.won
    }

    pub fn cell_char(&self, row: usize, col: usize) -> char {
        let cell = board_cell_index(row, col);
        if cell == self.position {
            'P'
        } else if cell >= 99 {
            'F'
        } else if Self::is_ladder(cell) {
            'L'
        } else if Self::is_snake(cell) {
            'S'
        } else {
            '.'
        }
    }

    pub fn is_highlighted(&self, row: usize, col: usize) -> bool {
        board_cell_index(row, col) == self.position
    }
}

impl Default for SnakesState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ludo_select_advances_selected_token() {
        let mut game = LudoState::new();
        game.select();
        assert_eq!(game.token_position(0), 1);
        assert_eq!(game.last_die(), 1);
        assert_eq!(game.selected(), 1);
    }

    #[test]
    fn ludo_arrows_cycle_selected_token() {
        let mut game = LudoState::new();
        game.move_cursor(GridMove::Right);
        assert_eq!(game.selected(), 1);
        game.move_cursor(GridMove::Left);
        assert_eq!(game.selected(), 0);
    }

    #[test]
    fn snakes_select_rolls_and_applies_ladder() {
        let mut game = SnakesState::new();
        game.select();
        assert_eq!(game.position(), 1);
        game.select();
        assert_eq!(game.position(), 22);
    }
}
