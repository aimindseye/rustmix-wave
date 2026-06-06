//! Bounded SD Lua event bridge.
//!
//! Static bootstrap drawing remains available. Rust owns mutable Sudoku,
//! Minesweeper, Tilt Maze, Motion 2048 and Sokoban Tilt state, redraw decisions
//! and hardware-facing input. SD declarations never receive raw QMI8658 or
//! panel access.

use crate::{
    buttons::ButtonEvent,
    games::{
        canvas::NativeGameCanvas,
        minesweeper::{MinesweeperEventResult, MinesweeperGame},
        motion_2048::{Motion2048EventResult, Motion2048Game},
        sokoban_tilt::{SokobanTiltEventResult, SokobanTiltGame},
        sudoku::{SudokuEventResult, SudokuGame},
        tilt_maze::{TiltMazeEventResult, TiltMazeGame},
    },
    imu_events::ImuDetectedEvent,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LuaGameEventResult {
    Sudoku(SudokuEventResult),
    Minesweeper(MinesweeperEventResult),
    TiltMaze(TiltMazeEventResult),
    Motion2048(Motion2048EventResult),
    SokobanTilt(SokobanTiltEventResult),
}
impl LuaGameEventResult {
    #[must_use]
    pub const fn bridge_marker(&self) -> &'static str {
        match self {
            Self::Sudoku(_) => "sudoku",
            Self::Minesweeper(_) => "minesweeper",
            Self::TiltMaze(_) => "tilt-maze",
            Self::Motion2048(_) => "motion-2048",
            Self::SokobanTilt(_) => "sokoban-tilt",
        }
    }
    #[must_use]
    pub const fn reason(&self) -> &'static str {
        match self {
            Self::Sudoku(r) => r.reason,
            Self::Minesweeper(r) => r.reason,
            Self::TiltMaze(r) => r.reason,
            Self::Motion2048(r) => r.reason,
            Self::SokobanTilt(r) => r.reason,
        }
    }
    #[must_use]
    pub const fn row(&self) -> usize {
        match self {
            Self::Sudoku(r) => r.row,
            Self::Minesweeper(r) => r.row,
            Self::TiltMaze(r) => r.row,
            Self::Motion2048(r) => r.row,
            Self::SokobanTilt(r) => r.row,
        }
    }
    #[must_use]
    pub const fn column(&self) -> usize {
        match self {
            Self::Sudoku(r) => r.column,
            Self::Minesweeper(r) => r.column,
            Self::TiltMaze(r) => r.column,
            Self::Motion2048(r) => r.column,
            Self::SokobanTilt(r) => r.column,
        }
    }
    #[must_use]
    pub const fn mode_marker(&self) -> &'static str {
        match self {
            Self::Sudoku(r) => r.mode.marker(),
            Self::Minesweeper(r) => r.mode.marker(),
            Self::TiltMaze(_) | Self::Motion2048(_) | Self::SokobanTilt(_) => "motion",
        }
    }
    #[must_use]
    pub const fn axis_marker(&self) -> &'static str {
        match self {
            Self::Sudoku(r) => r.axis.marker(),
            Self::Minesweeper(r) => r.axis.marker(),
            Self::TiltMaze(_) | Self::Motion2048(_) | Self::SokobanTilt(_) => "imu",
        }
    }
    #[must_use]
    pub fn detail_marker(&self) -> String {
        match self {
            Self::Sudoku(r) => format!("candidate={}", r.candidate),
            Self::Minesweeper(r) => format!(
                "action={} flags={} safe-left={} outcome={}",
                r.action.marker(),
                r.flags,
                r.safe_left,
                r.outcome.marker()
            ),
            Self::TiltMaze(r) => format!(
                "direction={} raw-axis={} moves={} outcome={}",
                r.direction,
                r.raw_axis,
                r.moves,
                r.outcome.marker()
            ),
            Self::Motion2048(r) => format!(
                "direction={} raw-axis={} score={} moves={} max-tile={} outcome={}",
                r.direction,
                r.raw_axis,
                r.score,
                r.moves,
                r.max_tile,
                r.outcome.marker()
            ),
            Self::SokobanTilt(r) => format!(
                "direction={} raw-axis={} moves={} pushes={} crates-on-goal={} outcome={}",
                r.direction,
                r.raw_axis,
                r.moves,
                r.pushes,
                r.crates_on_goal,
                r.outcome.marker()
            ),
        }
    }
    #[must_use]
    pub const fn completed(&self) -> bool {
        match self {
            Self::Sudoku(r) => r.completed,
            Self::Minesweeper(r) => r.outcome.completed(),
            Self::TiltMaze(r) => r.outcome.completed(),
            Self::Motion2048(r) => r.outcome.completed(),
            Self::SokobanTilt(r) => r.outcome.completed(),
        }
    }
    #[must_use]
    pub fn dirty_regions_len(&self) -> usize {
        match self {
            Self::Sudoku(r) => r.dirty_regions.len(),
            Self::Minesweeper(r) => r.dirty_regions.len(),
            Self::TiltMaze(r) => r.dirty_regions.len(),
            Self::Motion2048(r) => r.dirty_regions.len(),
            Self::SokobanTilt(r) => r.dirty_regions.len(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LuaEventBridge {
    Static,
    Sudoku(SudokuGame),
    Minesweeper(MinesweeperGame),
    TiltMaze(TiltMazeGame),
    Motion2048(Motion2048Game),
    SokobanTilt(SokobanTiltGame),
}
impl LuaEventBridge {
    pub fn load(source: &str, canvas: &mut NativeGameCanvas) -> Result<Self, String> {
        if let Some(map) = parse_sokoban_init(source)? {
            let game = SokobanTiltGame::from_map(&map)?;
            game.render_initial(canvas)?;
            Ok(Self::SokobanTilt(game))
        } else if let Some(seed) = parse_motion2048_init(source)? {
            let game = Motion2048Game::from_seed(seed)?;
            game.render_initial(canvas)?;
            Ok(Self::Motion2048(game))
        } else if let Some(map) = parse_tiltmaze_init(source)? {
            let game = TiltMazeGame::from_map(&map)?;
            game.render_initial(canvas)?;
            Ok(Self::TiltMaze(game))
        } else if let Some((columns, rows, mines, seed)) = parse_minesweeper_init(source)? {
            let game = MinesweeperGame::from_config(columns, rows, mines, seed)?;
            game.render_initial(canvas)?;
            Ok(Self::Minesweeper(game))
        } else if let Some(puzzle) = parse_sudoku_init(source)? {
            let game = SudokuGame::from_puzzle(&puzzle)?;
            game.render_initial(canvas)?;
            Ok(Self::Sudoku(game))
        } else {
            super::bootstrap::execute_bootstrap_script(source, canvas)?;
            Ok(Self::Static)
        }
    }
    #[must_use]
    pub const fn marker(&self) -> &'static str {
        match self {
            Self::Static => "static",
            Self::Sudoku(_) => "sudoku",
            Self::Minesweeper(_) => "minesweeper",
            Self::TiltMaze(_) => "tilt-maze",
            Self::Motion2048(_) => "motion-2048",
            Self::SokobanTilt(_) => "sokoban-tilt",
        }
    }
    #[must_use]
    pub const fn needs_imu_events(&self) -> bool {
        matches!(
            self,
            Self::TiltMaze(_) | Self::Motion2048(_) | Self::SokobanTilt(_)
        )
    }
    pub fn apply_button(
        &mut self,
        event: ButtonEvent,
        canvas: &mut NativeGameCanvas,
    ) -> Result<Option<LuaGameEventResult>, String> {
        match self {
            Self::Static => Ok(None),
            Self::Sudoku(g) => g
                .apply_button_and_render(event, canvas)
                .map(LuaGameEventResult::Sudoku)
                .map(Some),
            Self::Minesweeper(g) => g
                .apply_button_and_render(event, canvas)
                .map(LuaGameEventResult::Minesweeper)
                .map(Some),
            Self::TiltMaze(g) => g
                .apply_button_and_render(event, canvas)
                .map(|r| r.map(LuaGameEventResult::TiltMaze)),
            Self::Motion2048(g) => g
                .apply_button_and_render(event, canvas)
                .map(|r| r.map(LuaGameEventResult::Motion2048)),
            Self::SokobanTilt(g) => g
                .apply_button_and_render(event, canvas)
                .map(|r| r.map(LuaGameEventResult::SokobanTilt)),
        }
    }
    pub fn apply_boot_short_press(
        &mut self,
        canvas: &mut NativeGameCanvas,
    ) -> Result<Option<LuaGameEventResult>, String> {
        match self {
            Self::Static | Self::TiltMaze(_) | Self::Motion2048(_) | Self::SokobanTilt(_) => {
                Ok(None)
            }
            Self::Sudoku(g) => g
                .apply_boot_short_press_and_render(canvas)
                .map(LuaGameEventResult::Sudoku)
                .map(Some),
            Self::Minesweeper(g) => g
                .apply_boot_short_press_and_render(canvas)
                .map(LuaGameEventResult::Minesweeper)
                .map(Some),
        }
    }
    pub fn apply_motion_event(
        &mut self,
        event: ImuDetectedEvent,
        canvas: &mut NativeGameCanvas,
    ) -> Result<Option<LuaGameEventResult>, String> {
        match self {
            Self::TiltMaze(g) => g
                .apply_motion_and_render(event, canvas)
                .map(|r| r.map(LuaGameEventResult::TiltMaze)),
            Self::Motion2048(g) => g
                .apply_motion_and_render(event, canvas)
                .map(|r| r.map(LuaGameEventResult::Motion2048)),
            Self::SokobanTilt(g) => g
                .apply_motion_and_render(event, canvas)
                .map(|r| r.map(LuaGameEventResult::SokobanTilt)),
            Self::Static | Self::Sudoku(_) | Self::Minesweeper(_) => Ok(None),
        }
    }
}
fn parse_sokoban_init(source: &str) -> Result<Option<String>, String> {
    parse_single_quoted_init(source, "sokoban.init(", "sokoban")
}
fn parse_tiltmaze_init(source: &str) -> Result<Option<String>, String> {
    parse_single_quoted_init(source, "tiltmaze.init(", "tiltmaze")
}
fn parse_sudoku_init(source: &str) -> Result<Option<String>, String> {
    parse_single_quoted_init(source, "sudoku.init(", "sudoku")
}
fn parse_single_quoted_init(
    source: &str,
    prefix: &str,
    label: &str,
) -> Result<Option<String>, String> {
    let mut value = None;
    for (index, raw) in source.lines().enumerate() {
        let line_number = index + 1;
        let line = raw.split("--").next().unwrap_or_default().trim();
        if line.is_empty() {
            continue;
        }
        if !line.starts_with(prefix) || !line.ends_with(')') {
            if line.starts_with(&format!("{label}.")) {
                return Err(format!(
                    "MAIN.LUA line {line_number}: unsupported {label} call"
                ));
            }
            return Ok(None);
        }
        if value.is_some() {
            return Err(format!(
                "MAIN.LUA line {line_number}: duplicate {label}.init"
            ));
        }
        let argument = &line[prefix.len()..line.len() - 1];
        value = Some(
            parse_quoted(argument.trim(), label)
                .map_err(|error| format!("MAIN.LUA line {line_number}: {error}"))?,
        );
    }
    Ok(value)
}
fn parse_quoted(value: &str, label: &str) -> Result<String, String> {
    if value.len() < 2 || !value.starts_with('"') || !value.ends_with('"') {
        return Err(format!("{label}.init expects one quoted argument"));
    }
    let inner = &value[1..value.len() - 1];
    if inner.contains('"') {
        return Err(format!("escaped {label} strings are not supported"));
    }
    Ok(inner.to_string())
}
fn parse_motion2048_init(source: &str) -> Result<Option<u32>, String> {
    let mut seed = None;
    for (index, raw) in source.lines().enumerate() {
        let line_number = index + 1;
        let line = raw.split("--").next().unwrap_or_default().trim();
        if line.is_empty() {
            continue;
        }
        if !line.starts_with("motion2048.init(") || !line.ends_with(')') {
            if line.starts_with("motion2048.") {
                return Err(format!(
                    "MAIN.LUA line {line_number}: unsupported motion2048 call"
                ));
            }
            return Ok(None);
        }
        if seed.is_some() {
            return Err(format!(
                "MAIN.LUA line {line_number}: duplicate motion2048.init"
            ));
        }
        let argument = &line["motion2048.init(".len()..line.len() - 1];
        seed = Some(
            argument
                .trim()
                .parse::<u32>()
                .map_err(|_| format!("MAIN.LUA line {line_number}: invalid motion2048 seed"))?,
        );
    }
    Ok(seed)
}
fn parse_minesweeper_init(source: &str) -> Result<Option<(usize, usize, usize, u32)>, String> {
    let mut config = None;
    for (index, raw_line) in source.lines().enumerate() {
        let line_number = index + 1;
        let line = raw_line.split("--").next().unwrap_or_default().trim();
        if line.is_empty() {
            continue;
        }
        if !line.starts_with("minesweeper.init(") || !line.ends_with(')') {
            if line.starts_with("minesweeper.") {
                return Err(format!(
                    "MAIN.LUA line {line_number}: unsupported minesweeper call"
                ));
            }
            return Ok(None);
        }
        if config.is_some() {
            return Err(format!(
                "MAIN.LUA line {line_number}: duplicate minesweeper.init"
            ));
        }
        let arguments = &line["minesweeper.init(".len()..line.len() - 1];
        let values = arguments.split(',').map(str::trim).collect::<Vec<_>>();
        if values.len() != 4 {
            return Err(format!(
                "MAIN.LUA line {line_number}: minesweeper.init expects columns, rows, mines, seed"
            ));
        }
        let parse_usize = |value: &str, label: &str| {
            value
                .parse::<usize>()
                .map_err(|_| format!("MAIN.LUA line {line_number}: invalid minesweeper {label}"))
        };
        config = Some((
            parse_usize(values[0], "columns")?,
            parse_usize(values[1], "rows")?,
            parse_usize(values[2], "mine count")?,
            values[3]
                .parse::<u32>()
                .map_err(|_| format!("MAIN.LUA line {line_number}: invalid minesweeper seed"))?,
        ));
    }
    Ok(config)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        buttons::ButtonEvent,
        games::canvas::NativeGameCanvas,
        imu_events::{ImuDetectedEventKind, MotionAxis},
    };
    const PUZZLE: &str =
        "530070000600195000098000060800060003400803001700020006060000280000419005000080079";
    const MAP: &str =
        "#########/#S..#...#/#.#.#.#.#/#.#...#.#/#.###.#.#/#...#.#.#/###.#...#/#.....#G#/#########";
    const SOKOBAN: &str =
        "#########/#S......#/#.###...#/#.......#/#...C...#/#.......#/#...G...#/#.......#/#########";
    fn tilt() -> ImuDetectedEvent {
        ImuDetectedEvent {
            kind: ImuDetectedEventKind::Tilt(MotionAxis::PositiveX),
            at_ms: 1,
        }
    }
    #[test]
    fn loads_sudoku_init_and_routes_native_event() {
        let mut c = NativeGameCanvas::default();
        let mut b = LuaEventBridge::load(&format!("sudoku.init(\"{PUZZLE}\")"), &mut c).unwrap();
        assert_eq!(b.marker(), "sudoku");
        assert_eq!(
            b.apply_button(ButtonEvent::Down, &mut c)
                .unwrap()
                .unwrap()
                .reason(),
            "cursor-move"
        );
    }
    #[test]
    fn loads_minesweeper_init_and_routes_native_event() {
        let mut c = NativeGameCanvas::default();
        let mut b = LuaEventBridge::load("minesweeper.init(9, 9, 10, 1803)", &mut c).unwrap();
        assert_eq!(b.marker(), "minesweeper");
        assert_eq!(
            b.apply_button(ButtonEvent::Select, &mut c)
                .unwrap()
                .unwrap()
                .reason(),
            "action-enter"
        );
    }
    #[test]
    fn loads_tiltmaze_and_routes_native_motion() {
        let mut c = NativeGameCanvas::default();
        let mut b = LuaEventBridge::load(&format!("tiltmaze.init(\"{MAP}\")"), &mut c).unwrap();
        assert_eq!(b.marker(), "tilt-maze");
        assert!(b.needs_imu_events());
        assert_eq!(
            b.apply_motion_event(tilt(), &mut c)
                .unwrap()
                .unwrap()
                .reason(),
            "tilt-move"
        );
    }
    #[test]
    fn loads_motion2048_and_routes_native_motion() {
        let mut c = NativeGameCanvas::default();
        let mut b = LuaEventBridge::load("motion2048.init(2048)", &mut c).unwrap();
        assert_eq!(b.marker(), "motion-2048");
        assert!(b.needs_imu_events());
        assert!(b.apply_motion_event(tilt(), &mut c).unwrap().is_some());
    }
    #[test]
    fn loads_sokoban_and_routes_native_motion() {
        let mut c = NativeGameCanvas::default();
        let mut b = LuaEventBridge::load(&format!("sokoban.init(\"{SOKOBAN}\")"), &mut c).unwrap();
        assert_eq!(b.marker(), "sokoban-tilt");
        assert!(b.needs_imu_events());
        assert!(b.apply_motion_event(tilt(), &mut c).unwrap().is_some());
    }
    #[test]
    fn preserves_static_hello_grid_bootstrap() {
        let mut c = NativeGameCanvas::default();
        assert_eq!(
            LuaEventBridge::load("ui.grid(80, 220, 4, 4, 64, 64)", &mut c)
                .unwrap()
                .marker(),
            "static"
        );
    }
}
