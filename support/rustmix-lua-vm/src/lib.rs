#![cfg_attr(not(test), no_std)]

/// Marker emitted by the first real Lua VM feature-gate smoke path.
pub const LUA_VM_CRATE_MARKER: &str = "rustmix-lua-vm-crate-ok";

/// A tiny, no_std Lua VM smoke dependency for Rustmix's embedded feature gate.
///
/// This crate intentionally implements only the smallest executable Lua subset
/// needed to prove that target-xteink-x4 can depend on and run a VM-style Lua
/// execution boundary on ESP32-C3 before a larger upstream VM is selected.
///
/// Numeric scripts accepted by `eval_i32`:
///
/// - `return 1`
/// - `return 1 + 2`
/// - `return 7 - 4`
/// - `return 3 * 5`
/// - `return 8 / 2`
///
/// String scripts accepted by `eval_str`:
///
/// - `return "Monday"`
/// - `return 'Monday'`
#[derive(Debug, Default)]
pub struct LuaVm;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum LuaVmError {
    EmptyScript,
    ExpectedReturn,
    UnsupportedExpression,
    InvalidInteger,
    DivideByZero,
    TrailingInput,
    UnterminatedString,
    InvalidStringLiteral,
}

impl LuaVm {
    pub const fn new() -> Self {
        Self
    }

    pub fn eval_i32(&mut self, script: &str) -> Result<i32, LuaVmError> {
        let rest = return_body(script)?;
        let mut parts = rest.split_whitespace();
        let lhs = parse_i32(parts.next().ok_or(LuaVmError::UnsupportedExpression)?)?;

        match parts.next() {
            None => Ok(lhs),
            Some(op) => {
                let rhs = parse_i32(parts.next().ok_or(LuaVmError::UnsupportedExpression)?)?;
                if parts.next().is_some() {
                    return Err(LuaVmError::TrailingInput);
                }
                eval_binary(lhs, op, rhs)
            }
        }
    }

    pub fn eval_str<'a>(&mut self, script: &'a str) -> Result<&'a str, LuaVmError> {
        let rest = return_body(script)?;
        parse_lua_string_literal(rest)
    }
}

fn return_body(script: &str) -> Result<&str, LuaVmError> {
    let script = script.trim();
    if script.is_empty() {
        return Err(LuaVmError::EmptyScript);
    }

    let rest = script
        .strip_prefix("return")
        .ok_or(LuaVmError::ExpectedReturn)?
        .trim();
    if rest.is_empty() {
        return Err(LuaVmError::UnsupportedExpression);
    }
    Ok(rest)
}

fn parse_i32(value: &str) -> Result<i32, LuaVmError> {
    value.parse::<i32>().map_err(|_| LuaVmError::InvalidInteger)
}

fn eval_binary(lhs: i32, op: &str, rhs: i32) -> Result<i32, LuaVmError> {
    match op {
        "+" => Ok(lhs + rhs),
        "-" => Ok(lhs - rhs),
        "*" => Ok(lhs * rhs),
        "/" => {
            if rhs == 0 {
                Err(LuaVmError::DivideByZero)
            } else {
                Ok(lhs / rhs)
            }
        }
        _ => Err(LuaVmError::UnsupportedExpression),
    }
}

fn parse_lua_string_literal(value: &str) -> Result<&str, LuaVmError> {
    let value = value.trim();
    if value.len() < 2 {
        return Err(LuaVmError::InvalidStringLiteral);
    }
    let bytes = value.as_bytes();
    let quote = bytes[0];
    if quote != b'\'' && quote != b'\"' {
        return Err(LuaVmError::InvalidStringLiteral);
    }
    if bytes[bytes.len() - 1] != quote {
        return Err(LuaVmError::UnterminatedString);
    }
    let inner = &value[1..value.len() - 1];
    if inner.as_bytes().iter().any(|b| *b == b'\\' || *b == quote) {
        return Err(LuaVmError::InvalidStringLiteral);
    }
    Ok(inner)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_tiny_in_memory_lua_script() {
        let mut vm = LuaVm::new();
        assert_eq!(vm.eval_i32("return 1 + 2"), Ok(3));
    }

    #[test]
    fn supports_single_return_value_and_basic_operators() {
        let mut vm = LuaVm::new();
        assert_eq!(vm.eval_i32("return 7"), Ok(7));
        assert_eq!(vm.eval_i32("return 7 - 4"), Ok(3));
        assert_eq!(vm.eval_i32("return 3 * 5"), Ok(15));
        assert_eq!(vm.eval_i32("return 8 / 2"), Ok(4));
    }

    #[test]
    fn supports_tiny_string_return_for_weekday_selection() {
        let mut vm = LuaVm::new();
        assert_eq!(vm.eval_str("return \"Monday\""), Ok("Monday"));
        assert_eq!(vm.eval_str("return 'Tuesday'"), Ok("Tuesday"));
    }

    #[test]
    fn rejects_unsupported_scripts_without_panicking() {
        let mut vm = LuaVm::new();
        assert_eq!(vm.eval_i32("print('hi')"), Err(LuaVmError::ExpectedReturn));
        assert_eq!(
            vm.eval_i32("return 1 + 2 + 3"),
            Err(LuaVmError::TrailingInput)
        );
        assert_eq!(vm.eval_i32("return 1 / 0"), Err(LuaVmError::DivideByZero));
        assert_eq!(
            vm.eval_str("return Monday"),
            Err(LuaVmError::InvalidStringLiteral)
        );
    }
}
