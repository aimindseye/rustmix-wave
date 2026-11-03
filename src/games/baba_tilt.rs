// Repo Path: src/games/baba_tilt.rs

pub const GRID_W: usize = 12;
pub const GRID_H: usize = 8;
pub const MAX_ENTITIES: usize = 64;
pub const MAX_UNDO: usize = 32;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Noun {
    Baba = 0,
    Wall = 1,
    Rock = 2,
    Flag = 3,
    Water = 4,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TokenRole {
    Noun(Noun),
    VerbIs,
    PropYou,
    PropStop,
    PropPush,
    PropWin,
    PropSink,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum EntityKind {
    Object(Noun),
    Text(TokenRole),
}

#[derive(Clone, Copy)]
pub struct Entity {
    pub active: bool,
    pub kind: EntityKind,
    pub x: i8,
    pub y: i8,
}

impl Entity {
    pub const fn empty() -> Self {
        Self {
            active: false,
            kind: EntityKind::Object(Noun::Baba),
            x: 0,
            y: 0,
        }
    }
}

#[derive(Clone, Copy, Default)]
pub struct RuleSet {
    pub props: [u8; 5],
    pub transform: [Option<Noun>; 5],
}

impl RuleSet {
    pub fn has_prop(&self, noun: Noun, bit: u8) -> bool {
        (self.props[noun as usize] & (1 << bit)) != 0
    }
}

pub const BIT_YOU: u8 = 0;
pub const BIT_STOP: u8 = 1;
pub const BIT_PUSH: u8 = 2;
pub const BIT_WIN: u8 = 3;
pub const BIT_SINK: u8 = 4;

pub struct BabaGame {
    pub entities: [Entity; MAX_ENTITIES],
    pub undo_ring: [[Entity; MAX_ENTITIES]; MAX_UNDO],
    pub undo_head: usize,
    pub undo_len: usize,
    pub rules: RuleSet,
    pub steps: u32,
    pub won: bool,
    pub lost: bool,
    pub level_idx: usize,
}

impl BabaGame {
    pub fn new() -> Self {
        let mut game = Self {
            entities: [Entity::empty(); MAX_ENTITIES],
            undo_ring: [[Entity::empty(); MAX_ENTITIES]; MAX_UNDO],
            undo_head: 0,
            undo_len: 0,
            rules: RuleSet::default(),
            steps: 0,
            won: false,
            lost: false,
            level_idx: 0,
        };
        game.load_ascii_level(DEFAULT_LEVELS[0]);
        game
    }

    pub fn load_ascii_level(&mut self, rows: &[&str; GRID_H]) {
        self.entities = [Entity::empty(); MAX_ENTITIES];
        self.undo_head = 0;
        self.undo_len = 0;
        self.steps = 0;
        self.won = false;
        self.lost = false;

        let mut idx = 0;
        for (y, row) in rows.iter().enumerate() {
            for (x, ch) in row.bytes().take(GRID_W).enumerate() {
                if idx >= MAX_ENTITIES {
                    break;
                }
                if let Some(kind) = Self::char_to_kind(ch) {
                    self.entities[idx] = Entity {
                        active: true,
                        kind,
                        x: x as i8,
                        y: y as i8,
                    };
                    idx += 1;
                }
            }
        }
        self.compile_rules();
        self.evaluate_status();
    }

    fn char_to_kind(ch: u8) -> Option<EntityKind> {
        use EntityKind::{Object, Text};
        use Noun::{Baba, Flag, Rock, Wall, Water};
        use TokenRole::{
            Noun as RoleNoun, PropPush, PropSink, PropStop, PropWin, PropYou, VerbIs,
        };
        match ch {
            b'B' => Some(Text(RoleNoun(Baba))),
            b'W' => Some(Text(RoleNoun(Wall))),
            b'R' => Some(Text(RoleNoun(Rock))),
            b'F' => Some(Text(RoleNoun(Flag))),
            b'K' => Some(Text(RoleNoun(Water))),
            b'I' => Some(Text(VerbIs)),
            b'Y' => Some(Text(PropYou)),
            b'S' => Some(Text(PropStop)),
            b'P' => Some(Text(PropPush)),
            b'X' => Some(Text(PropWin)),
            b'N' => Some(Text(PropSink)),
            b'b' => Some(Object(Baba)),
            b'w' => Some(Object(Wall)),
            b'r' => Some(Object(Rock)),
            b'f' => Some(Object(Flag)),
            b'k' => Some(Object(Water)),
            _ => None,
        }
    }

    pub fn compile_rules(&mut self) {
        self.rules = RuleSet::default();
        for i in 0..MAX_ENTITIES {
            if !self.entities[i].active {
                continue;
            }
            if self.entities[i].kind == EntityKind::Text(TokenRole::VerbIs) {
                let vx = self.entities[i].x;
                let vy = self.entities[i].y;
                self.check_sentence(vx - 1, vy, vx + 1, vy);
                self.check_sentence(vx, vy - 1, vx, vy + 1);
            }
        }
    }

    fn check_sentence(&mut self, sx: i8, sy: i8, tx: i8, ty: i8) {
        for i in 0..MAX_ENTITIES {
            if !self.entities[i].active || self.entities[i].x != sx || self.entities[i].y != sy {
                continue;
            }
            if let EntityKind::Text(TokenRole::Noun(subj)) = self.entities[i].kind {
                for j in 0..MAX_ENTITIES {
                    if !self.entities[j].active || self.entities[j].x != tx || self.entities[j].y != ty {
                        continue;
                    }
                    if let EntityKind::Text(role) = self.entities[j].kind {
                        match role {
                            TokenRole::PropYou => self.rules.props[subj as usize] |= 1 << BIT_YOU,
                            TokenRole::PropStop => self.rules.props[subj as usize] |= 1 << BIT_STOP,
                            TokenRole::PropPush => self.rules.props[subj as usize] |= 1 << BIT_PUSH,
                            TokenRole::PropWin => self.rules.props[subj as usize] |= 1 << BIT_WIN,
                            TokenRole::PropSink => self.rules.props[subj as usize] |= 1 << BIT_SINK,
                            TokenRole::Noun(target) => self.rules.transform[subj as usize] = Some(target),
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    pub fn entity_has_prop(&self, e: &Entity, bit: u8) -> bool {
        match e.kind {
            EntityKind::Text(_) => bit == BIT_PUSH,
            EntityKind::Object(noun) => self.rules.has_prop(noun, bit),
        }
    }

    fn can_push(&self, x: i8, y: i8, dx: i8, dy: i8, chain: &mut [bool; MAX_ENTITIES]) -> bool {
        let nx = x + dx;
        let ny = y + dy;
        if nx < 0 || nx >= GRID_W as i8 || ny < 0 || ny >= GRID_H as i8 {
            return false;
        }

        let mut needs_next = false;
        for (idx, e) in self.entities.iter().enumerate() {
            if e.active && e.x == nx && e.y == ny {
                let is_stop = self.entity_has_prop(e, BIT_STOP);
                let is_push = self.entity_has_prop(e, BIT_PUSH);
                if is_stop && !is_push {
                    return false;
                }
                if is_push {
                    chain[idx] = true;
                    needs_next = true;
                }
            }
        }

        if needs_next {
            return self.can_push(nx, ny, dx, dy, chain);
        }
        true
    }

    pub fn step(&mut self, dx: i8, dy: i8) -> bool {
        if self.won {
            self.level_idx = (self.level_idx + 1) % DEFAULT_LEVELS.len();
            self.load_ascii_level(DEFAULT_LEVELS[self.level_idx]);
            return true;
        }

        let snapshot = self.entities;
        let mut to_move = [false; MAX_ENTITIES];
        let mut any_moved = false;

        for i in 0..MAX_ENTITIES {
            let e = self.entities[i];
            if e.active && self.entity_has_prop(&e, BIT_YOU) {
                let mut chain = [false; MAX_ENTITIES];
                if self.can_push(e.x, e.y, dx, dy, &mut chain) {
                    to_move[i] = true;
                    for c in 0..MAX_ENTITIES {
                        if chain[c] {
                            to_move[c] = true;
                        }
                    }
                    any_moved = true;
                }
            }
        }

        if !any_moved {
            return false;
        }

        self.undo_ring[self.undo_head] = snapshot;
        self.undo_head = (self.undo_head + 1) % MAX_UNDO;
        if self.undo_len < MAX_UNDO {
            self.undo_len += 1;
        }

        for i in 0..MAX_ENTITIES {
            if to_move[i] {
                self.entities[i].x += dx;
                self.entities[i].y += dy;
            }
        }
        self.steps += 1;

        self.compile_rules();

        for e in self.entities.iter_mut() {
            if e.active {
                if let EntityKind::Object(noun) = e.kind {
                    if let Some(target) = self.rules.transform[noun as usize] {
                        e.kind = EntityKind::Object(target);
                    }
                }
            }
        }

        for y in 0..GRID_H as i8 {
            for x in 0..GRID_W as i8 {
                let mut count = 0;
                let mut has_sink = false;
                for e in self.entities.iter() {
                    if e.active && e.x == x && e.y == y {
                        count += 1;
                        if self.entity_has_prop(e, BIT_SINK) {
                            has_sink = true;
                        }
                    }
                }
                if has_sink && count > 1 {
                    for e in self.entities.iter_mut() {
                        if e.active && e.x == x && e.y == y {
                            e.active = false;
                        }
                    }
                }
            }
        }

        self.compile_rules();
        self.evaluate_status();
        true
    }

    pub fn undo(&mut self) -> bool {
        if self.undo_len == 0 {
            return false;
        }
        self.undo_head = (self.undo_head + MAX_UNDO - 1) % MAX_UNDO;
        self.entities = self.undo_ring[self.undo_head];
        self.undo_len -= 1;
        self.steps = self.steps.saturating_sub(1);
        self.compile_rules();
        self.evaluate_status();
        true
    }

    fn evaluate_status(&mut self) {
        self.won = false;
        let mut any_you = false;
        for a in self.entities.iter() {
            if a.active && self.entity_has_prop(a, BIT_YOU) {
                any_you = true;
                for b in self.entities.iter() {
                    if b.active && a.x == b.x && a.y == b.y && self.entity_has_prop(b, BIT_WIN) {
                        self.won = true;
                        return;
                    }
                }
            }
        }
        self.lost = !any_you;
    }
}

pub const DEFAULT_LEVELS: [&[&str; GRID_H]; 5] = [
    &[
        "BIY.....FI..",
        "..........X.",
        "..wwwwwwww..",
        "..w......w..",
        "..w.b..r.f..",
        "..w......w..",
        "..wwwwwwww..",
        "WIS.....RIP.",
    ],
    &[
        "BIY.....WIS.",
        "............",
        "..wwwwwwww..",
        "..w......w..",
        "..w.b..w.f..",
        "..w......w..",
        "..wwwwwwww..",
        "........FIX.",
    ],
    &[
        "BIY.....WIS.",
        "............",
        "wwwwwwwwwwww",
        "....b.......",
        "wwwwwwwwwwww",
        "............",
        "..r..R...f..",
        "....FIX.....",
    ],
    &[
        "BIY.....KIN.",
        "wwwwwwwwwwww",
        "w....k.....w",
        "w.b..k.r.f.w",
        "w....k.....w",
        "wwwwwwwwwwww",
        "RIP.....FIX.",
        "........WIS.",
    ],
    &[
        "BIY.....WIS.",
        "wwwwwwwwwwww",
        "w..........w",
        "w.b..r..R..w",
        "w....I.....w",
        "w....F.....w",
        "wwwwwwwwwwww",
        "RIP.....FIX.",
    ],
];
