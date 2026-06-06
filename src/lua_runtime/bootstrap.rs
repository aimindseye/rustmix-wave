//! Deterministic bootstrap executor for the first SD Lua foundation slice.
//!
//! This intentionally supports only top-level `ui.*` calls. It proves SD
//! loading, bounds, canvas ownership and refresh policy without exposing an
//! unrestricted VM. v0.18.1 layers a bounded native Sudoku event bridge beside
//! this accepted static subset through the same Rust-owned canvas API.

use crate::games::{
    canvas::{CanvasTextStyle, NativeGameCanvas},
    dirty_regions::DirtyRect,
};

pub const LUA_SCRIPT_MAX_BYTES: usize = 64 * 1024;

pub fn execute_bootstrap_script(source: &str, canvas: &mut NativeGameCanvas) -> Result<(), String> {
    if source.len() > LUA_SCRIPT_MAX_BYTES {
        return Err(format!("script exceeds {LUA_SCRIPT_MAX_BYTES}-byte limit"));
    }
    canvas.clear_frame();
    for (index, raw_line) in source.lines().enumerate() {
        let line_number = index + 1;
        let line = raw_line.split("--").next().unwrap_or_default().trim();
        if line.is_empty() {
            continue;
        }
        execute_line(line, canvas)
            .map_err(|error| format!("MAIN.LUA line {line_number}: {error}"))?;
    }
    Ok(())
}

fn execute_line(line: &str, canvas: &mut NativeGameCanvas) -> Result<(), String> {
    if line == "ui.clear()" {
        return canvas.clear();
    }
    if line == "ui.request_refresh()" {
        canvas.request_refresh();
        return Ok(());
    }
    let (function, arguments) = parse_call(line)?;
    match function {
        "ui.text" => {
            let arguments = split_arguments(arguments)?;
            if arguments.len() != 4 {
                return Err("ui.text expects x, y, text, style".into());
            }
            canvas.text(
                parse_i32(&arguments[0])?,
                parse_i32(&arguments[1])?,
                parse_string(&arguments[2])?,
                CanvasTextStyle::parse(&parse_string(&arguments[3])?)
                    .ok_or_else(|| "ui.text style must be body, heading or detail".to_string())?,
            )
        }
        "ui.line" => {
            let arguments = split_arguments(arguments)?;
            if arguments.len() != 4 {
                return Err("ui.line expects x1, y1, x2, y2".into());
            }
            canvas.line(
                parse_i32(&arguments[0])?,
                parse_i32(&arguments[1])?,
                parse_i32(&arguments[2])?,
                parse_i32(&arguments[3])?,
            )
        }
        "ui.rect" => {
            let arguments = split_arguments(arguments)?;
            if arguments.len() != 5 {
                return Err("ui.rect expects x, y, width, height, filled".into());
            }
            canvas.rect(
                parse_i32(&arguments[0])?,
                parse_i32(&arguments[1])?,
                parse_i32(&arguments[2])?,
                parse_i32(&arguments[3])?,
                parse_bool(&arguments[4])?,
            )
        }
        "ui.grid" => {
            let arguments = split_arguments(arguments)?;
            if arguments.len() != 6 {
                return Err("ui.grid expects x, y, columns, rows, cell_width, cell_height".into());
            }
            canvas.grid(
                parse_i32(&arguments[0])?,
                parse_i32(&arguments[1])?,
                parse_u8(&arguments[2])?,
                parse_u8(&arguments[3])?,
                parse_i32(&arguments[4])?,
                parse_i32(&arguments[5])?,
            )
        }
        "ui.invalidate_rect" => {
            let arguments = split_arguments(arguments)?;
            if arguments.len() != 4 {
                return Err("ui.invalidate_rect expects x, y, width, height".into());
            }
            canvas.invalidate_rect(DirtyRect::new(
                parse_i32(&arguments[0])?,
                parse_i32(&arguments[1])?,
                parse_i32(&arguments[2])?,
                parse_i32(&arguments[3])?,
            ));
            Ok(())
        }
        _ => Err(format!("unsupported bootstrap call: {function}")),
    }
}

fn parse_call(line: &str) -> Result<(&str, &str), String> {
    let (function, arguments) = line
        .split_once('(')
        .ok_or_else(|| "expected ui function call".to_string())?;
    if !arguments.ends_with(')') {
        return Err("expected closing ')'".into());
    }
    Ok((function.trim(), &arguments[..arguments.len() - 1]))
}

fn split_arguments(source: &str) -> Result<Vec<String>, String> {
    let mut arguments = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    for character in source.chars() {
        match character {
            '"' => {
                quoted = !quoted;
                current.push(character);
            }
            ',' if !quoted => {
                arguments.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(character),
        }
    }
    if quoted {
        return Err("unterminated string argument".into());
    }
    if !current.trim().is_empty() {
        arguments.push(current.trim().to_string());
    }
    Ok(arguments)
}

fn parse_string(value: &str) -> Result<String, String> {
    if value.len() < 2 || !value.starts_with('"') || !value.ends_with('"') {
        return Err(format!("expected quoted string, found {value}"));
    }
    let value = &value[1..value.len() - 1];
    if value.contains('"') {
        return Err("escaped strings are deferred in bootstrap executor".into());
    }
    Ok(value.to_string())
}

fn parse_i32(value: &str) -> Result<i32, String> {
    value
        .trim()
        .parse::<i32>()
        .map_err(|_| format!("expected integer, found {value}"))
}

fn parse_u8(value: &str) -> Result<u8, String> {
    value
        .trim()
        .parse::<u8>()
        .map_err(|_| format!("expected byte integer, found {value}"))
}

fn parse_bool(value: &str) -> Result<bool, String> {
    match value.trim() {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!("expected true or false, found {value}")),
    }
}

#[cfg(test)]
mod tests {
    use super::execute_bootstrap_script;
    use crate::games::canvas::{DrawCommand, NativeGameCanvas};

    #[test]
    fn executes_bounded_static_hello_grid_script() {
        let mut canvas = NativeGameCanvas::default();
        execute_bootstrap_script(
            r#"
                ui.clear()
                ui.text(24, 120, "Hello Grid", "heading")
                ui.grid(80, 220, 4, 4, 64, 64)
                ui.request_refresh()
            "#,
            &mut canvas,
        )
        .unwrap();
        assert!(canvas.refresh_requested());
        assert!(canvas
            .commands()
            .iter()
            .any(|command| matches!(command, DrawCommand::Grid { .. })));
    }

    #[test]
    fn rejects_unbounded_or_unknown_calls() {
        let mut canvas = NativeGameCanvas::default();
        assert!(execute_bootstrap_script("while true do end", &mut canvas).is_err());
    }
}
