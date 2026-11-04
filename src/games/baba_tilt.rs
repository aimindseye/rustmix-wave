// Repo Path: src/games/baba_tilt.rs

use super::{
    canvas::{CanvasTextStyle, NativeGameCanvas},
    dirty_regions::DirtyRect,
};
use crate::{
    buttons::ButtonEvent,
    imu_events::{ImuDetectedEvent, ImuDetectedEventKind, MotionAxis},
};

pub const GRID_W: usize = 12;
pub const GRID_H: usize = 8;
pub const MAX_ENTITIES: usize = 64;
pub const MAX_UNDO: usize = 32;

pub const BABA_GRID_X: i32 = 24;
pub const BABA_GRID_Y: i32 = 200;
pub const BABA_CELL_SIZE: i32 = 36;

const BABA_HEADER_RECT: DirtyRect = DirtyRect::new(16, 48, 448, 96);
const BABA_STATUS_RECT: DirtyRect = DirtyRect::new(16, 606, 448, 142);
const BABA_BOARD_RECT: DirtyRect = DirtyRect::new(
    BABA_GRID_X,
    BABA_GRID_Y,
    GRID_W as i32 * BABA_CELL_SIZE + 1,
    GRID_H as i32 * BABA_CELL_SIZE + 1,
);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BabaDirection {
    Up,
    Down,
    Left,
    Right,
}

impl BabaDirection {
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
    pub const fn delta(self) -> (i8, i8) {
        match self {
            Self::Up => (0, -1),
            Self::Down => (0, 1),
            Self::Left => (-1, 0),
            Self::Right => (1, 0),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BabaOutcome {
    InProgress,
    Won,
    Lost,
}

impl BabaOutcome {
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
        matches!(self, Self::Won)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BabaEventResult {
    pub reason: &'static str,
    pub row: usize,
    pub column: usize,
    pub moves: u32,
    pub rules_count: usize,
    pub outcome: BabaOutcome,
    pub direction: &'static str,
    pub raw_axis: &'static str,
    pub dirty_regions: Vec<DirtyRect>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Noun {
    Baba = 0,
    Wall = 1,
    Rock = 2,
    Flag = 3,
    Water = 4,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenRole {
    Noun(Noun),
    VerbIs,
    PropYou,
    PropStop,
    PropPush,
    PropWin,
    PropSink,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntityKind {
    Object(Noun),
    Text(TokenRole),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
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

#[derive(Clone, Debug, PartialEq, Eq)]
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
    pub custom_levels: Vec<[String; GRID_H]>,
    pub status: String,
}

impl Default for BabaGame {
    fn default() -> Self {
        Self::new()
    }
}

impl BabaGame {
    #[must_use]
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
            custom_levels: Vec::new(),
            status: "Tilt to move  UP undo  SELECT reset".into(),
        };
        game.load_ascii_level(DEFAULT_LEVELS[0]);
        game
    }

    pub fn from_map(encoded: &str) -> Result<Self, String> {
        let mut game = Self::new();
        let trimmed = encoded.trim();
        if !trimmed.is_empty() && !trimmed.eq_ignore_ascii_case("default") {
            let mut parsed_levels = Vec::new();
            for part in trimmed.split('|') {
                let part = part.trim();
                if part.is_empty() {
                    continue;
                }
                let rows: Vec<&str> = part.split('/').collect();
                if rows.len() == GRID_H && rows.iter().all(|r| r.len() >= GRID_W) {
                    let level_arr: [String; GRID_H] = std::array::from_fn(|i| rows[i].to_string());
                    parsed_levels.push(level_arr);
                }
            }
            if !parsed_levels.is_empty() {
                game.custom_levels = parsed_levels;
                game.load_level_by_index(0);
            }
        }
        Ok(game)
    }

    #[must_use]
    pub fn total_levels(&self) -> usize {
        if self.custom_levels.is_empty() {
            DEFAULT_LEVELS.len()
        } else {
            self.custom_levels.len()
        }
    }

    pub fn load_level_by_index(&mut self, index: usize) {
        let total = self.total_levels();
        self.level_idx = index % total;
        if self.custom_levels.is_empty() {
            self.load_ascii_level(DEFAULT_LEVELS[self.level_idx]);
        } else {
            let refs: [&str; GRID_H] =
                std::array::from_fn(|i| self.custom_levels[self.level_idx][i].as_str());
            self.load_ascii_level(&refs);
        }
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

    fn kind_glyph(kind: EntityKind) -> (&'static str, bool) {
        use EntityKind::{Object, Text};
        use Noun::{Baba, Flag, Rock, Wall, Water};
        use TokenRole::{
            Noun as RoleNoun, PropPush, PropSink, PropStop, PropWin, PropYou, VerbIs,
        };
        match kind {
            Text(RoleNoun(Baba)) => ("B", true),
            Text(RoleNoun(Wall)) => ("W", true),
            Text(RoleNoun(Rock)) => ("R", true),
            Text(RoleNoun(Flag)) => ("F", true),
            Text(RoleNoun(Water)) => ("K", true),
            Text(VerbIs) => ("I", true),
            Text(PropYou) => ("Y", true),
            Text(PropStop) => ("S", true),
            Text(PropPush) => ("P", true),
            Text(PropWin) => ("X", true),
            Text(PropSink) => ("N", true),
            Object(Baba) => ("b", false),
            Object(Wall) => ("#", false),
            Object(Rock) => ("o", false),
            Object(Flag) => ("f", false),
            Object(Water) => ("~", false),
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
                    if !self.entities[j].active
                        || self.entities[j].x != tx
                        || self.entities[j].y != ty
                    {
                        continue;
                    }
                    if let EntityKind::Text(role) = self.entities[j].kind {
                        match role {
                            TokenRole::PropYou => self.rules.props[subj as usize] |= 1 << BIT_YOU,
                            TokenRole::PropStop => self.rules.props[subj as usize] |= 1 << BIT_STOP,
                            TokenRole::PropPush => self.rules.props[subj as usize] |= 1 << BIT_PUSH,
                            TokenRole::PropWin => self.rules.props[subj as usize] |= 1 << BIT_WIN,
                            TokenRole::PropSink => self.rules.props[subj as usize] |= 1 << BIT_SINK,
                            TokenRole::Noun(target) => {
                                self.rules.transform[subj as usize] = Some(target)
                            }
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
            let next = (self.level_idx + 1) % self.total_levels();
            self.load_level_by_index(next);
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

    #[must_use]
    pub fn outcome(&self) -> BabaOutcome {
        if self.won {
            BabaOutcome::Won
        } else if self.lost {
            BabaOutcome::Lost
        } else {
            BabaOutcome::InProgress
        }
    }

    #[must_use]
    pub fn active_rules_count(&self) -> usize {
        let prop_bits: usize = self
            .rules
            .props
            .iter()
            .map(|bits| bits.count_ones() as usize)
            .sum();
        let transforms = self
            .rules
            .transform
            .iter()
            .filter(|target| target.is_some())
            .count();
        prop_bits + transforms
    }

    #[must_use]
    pub fn primary_you_coords(&self) -> (usize, usize) {
        for e in &self.entities {
            if e.active && self.entity_has_prop(e, BIT_YOU) {
                return (e.y.max(0) as usize, e.x.max(0) as usize);
            }
        }
        (0, 0)
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
    ) -> Result<Option<BabaEventResult>, String> {
        let ImuDetectedEventKind::Tilt(axis) = event.kind else {
            return Ok(None);
        };
        let Some(direction) = BabaDirection::from_raw_axis(axis) else {
            return Ok(None);
        };
        let (dx, dy) = direction.delta();
        let was_won = self.won;
        let moved = self.step(dx, dy);

        let reason = if was_won {
            self.status = format!("Level {} started  Tilt to move", self.level_idx + 1);
            "level-advance"
        } else if !moved {
            self.status = format!("{} blocked  UP undo  SELECT reset", direction.label());
            "blocked"
        } else if self.won {
            self.status = format!(
                "YOU WIN in {} steps! Tilt/SELECT next",
                self.steps
            );
            "goal-reached"
        } else if self.lost {
            self.status = "NO 'YOU' LEFT! Press UP to undo".into();
            "no-you"
        } else {
            self.status = format!(
                "{} moved ({} rules active)",
                direction.label(),
                self.active_rules_count()
            );
            "tilt-move"
        };

        self.render_commands(canvas)?;
        canvas.reset_dirty_regions();
        let dirty_regions = if moved || was_won {
            vec![BABA_HEADER_RECT, BABA_BOARD_RECT, BABA_STATUS_RECT]
        } else {
            vec![BABA_STATUS_RECT]
        };
        for rect in &dirty_regions {
            canvas.invalidate_rect(*rect);
        }
        canvas.request_refresh();
        Ok(Some(self.build_result(
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
    ) -> Result<Option<BabaEventResult>, String> {
        let reason = match event {
            ButtonEvent::Up => {
                if self.undo() {
                    self.status = format!("Undo (steps: {})", self.steps);
                    "undo"
                } else {
                    self.status = "Nothing to undo".into();
                    "undo-empty"
                }
            }
            ButtonEvent::Down => {
                let next = (self.level_idx + 1) % self.total_levels();
                self.load_level_by_index(next);
                self.status = format!("Switched to Level {}", self.level_idx + 1);
                "level-next"
            }
            ButtonEvent::Select => {
                if self.won {
                    let next = (self.level_idx + 1) % self.total_levels();
                    self.load_level_by_index(next);
                    self.status = format!("Level {}  Tilt to move", self.level_idx + 1);
                    "level-advance"
                } else {
                    let current = self.level_idx;
                    self.load_level_by_index(current);
                    self.status = "Level reset  Tilt to move".into();
                    "reset"
                }
            }
        };

        self.render_commands(canvas)?;
        canvas.reset_dirty_regions();
        let dirty_regions = vec![BABA_HEADER_RECT, BABA_BOARD_RECT, BABA_STATUS_RECT];
        for rect in &dirty_regions {
            canvas.invalidate_rect(*rect);
        }
        canvas.request_refresh();
        Ok(Some(self.build_result(reason, "none", "none", dirty_regions)))
    }

    fn build_result(
        &self,
        reason: &'static str,
        direction: &'static str,
        raw_axis: &'static str,
        dirty_regions: Vec<DirtyRect>,
    ) -> BabaEventResult {
        let (row, column) = self.primary_you_coords();
        BabaEventResult {
            reason,
            row,
            column,
            moves: self.steps,
            rules_count: self.active_rules_count(),
            outcome: self.outcome(),
            direction,
            raw_axis,
            dirty_regions,
        }
    }

    fn render_commands(&self, canvas: &mut NativeGameCanvas) -> Result<(), String> {
        canvas.clear_frame();
        canvas.text(
            24,
            82,
            format!(
                "Baba Tilt   Lv {}/{}",
                self.level_idx + 1,
                self.total_levels()
            ),
            CanvasTextStyle::Heading,
        )?;
        canvas.text(
            24,
            126,
            "UPPER=Word [B I Y]  lower=Obj (b # o f ~)".into(),
            CanvasTextStyle::Detail,
        )?;

        canvas.grid(
            BABA_GRID_X,
            BABA_GRID_Y,
            GRID_W as u8,
            GRID_H as u8,
            BABA_CELL_SIZE,
            BABA_CELL_SIZE,
        )?;

        for e in &self.entities {
            if !e.active {
                continue;
            }
            let cx = BABA_GRID_X + (e.x as i32) * BABA_CELL_SIZE;
            let cy = BABA_GRID_Y + (e.y as i32) * BABA_CELL_SIZE;
            let (glyph, is_text_token) = Self::kind_glyph(e.kind);

            if is_text_token {
                canvas.rect(
                    cx + 3,
                    cy + 3,
                    BABA_CELL_SIZE - 6,
                    BABA_CELL_SIZE - 6,
                    false,
                )?;
                canvas.text(cx + 10, cy + 25, glyph.into(), CanvasTextStyle::Heading)?;
            } else if e.kind == EntityKind::Object(Noun::Wall) {
                canvas.rect(
                    cx + 5,
                    cy + 5,
                    BABA_CELL_SIZE - 10,
                    BABA_CELL_SIZE - 10,
                    true,
                )?;
            } else {
                canvas.text(cx + 10, cy + 25, glyph.into(), CanvasTextStyle::Body)?;
            }
        }

        canvas.text(
            24,
            634,
            format!(
                "STEPS {}   RULES {}   {}",
                self.steps,
                self.active_rules_count(),
                self.outcome().marker().to_ascii_uppercase()
            ),
            CanvasTextStyle::Heading,
        )?;
        canvas.text(24, 682, self.status.clone(), CanvasTextStyle::Body)?;
        canvas.text(
            24,
            734,
            "TILT move  UP undo  DN next lv  SELECT reset".into(),
            CanvasTextStyle::Body,
        )?;
        Ok(())
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
