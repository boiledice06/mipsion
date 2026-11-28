use super::parser::{Operand, symbol};
use super::{Diagnostic, ErrorKind, SourceLocation};
use crate::runtime::inspection::GPR_NAMES;
use std::collections::BTreeMap;

pub(super) type Symbols = BTreeMap<String, u32>;
pub(super) type LocatedOperand<'a> = (SourceLocation, Operand<'a>);

pub(super) fn number(text: &str) -> Option<i64> {
    let (sign, digits) = match text.as_bytes().first() {
        Some(b'-') => (-1, &text[1..]),
        Some(b'+') => (1, &text[1..]),
        _ => (1, text),
    };
    let (digits, radix) = if let Some(value) = digits
        .strip_prefix("0x")
        .or_else(|| digits.strip_prefix("0X"))
    {
        (value, 16)
    } else if let Some(value) = digits
        .strip_prefix("0b")
        .or_else(|| digits.strip_prefix("0B"))
    {
        (value, 2)
    } else {
        (digits, 10)
    };
    if digits.starts_with(['+', '-']) {
        return None;
    }
    i64::from_str_radix(digits, radix).ok()?.checked_mul(sign)
}

pub(super) fn address_or_number(
    location: SourceLocation,
    text: &str,
    symbols: &Symbols,
) -> Result<i64, Diagnostic> {
    if let Some(value) = number(text) {
        return Ok(value);
    }
    if symbol(text) {
        return symbols
            .get(text)
            .map(|value| i64::from(*value))
            .ok_or_else(|| {
                location.error(
                    ErrorKind::UndefinedSymbol,
                    format!("undefined symbol {text}"),
                )
            });
    }
    Err(location.error(
        ErrorKind::InvalidOperand,
        format!("expected a number or label, got {text}"),
    ))
}

pub(super) fn range(
    location: SourceLocation,
    value: i64,
    min: i64,
    max: i64,
) -> Result<u32, Diagnostic> {
    if !(min..=max).contains(&value) {
        return Err(location.error(
            ErrorKind::OutOfRange,
            format!("{value} is outside {min}..={max}"),
        ));
    }
    Ok(value as u32)
}

pub(super) fn atom<'a>((location, operand): &LocatedOperand<'a>) -> Result<&'a str, Diagnostic> {
    match operand {
        Operand::Atom(text) => Ok(text),
        _ => Err(location.error(
            ErrorKind::InvalidOperand,
            "expected a register, number or label",
        )),
    }
}

pub(super) fn numeric(operand: &LocatedOperand<'_>, min: i64, max: i64) -> Result<u32, Diagnostic> {
    let text = atom(operand)?;
    let value = number(text).ok_or_else(|| {
        operand
            .0
            .error(ErrorKind::InvalidOperand, format!("invalid number {text}"))
    })?;
    range(operand.0, value, min, max)
}

pub(super) fn gpr_name(location: SourceLocation, text: &str) -> Result<u32, Diagnostic> {
    let name = text.to_ascii_lowercase();
    let index = name
        .strip_prefix('$')
        .and_then(|value| value.parse::<u32>().ok())
        .filter(|&value| value < 32);
    index
        .or_else(|| {
            GPR_NAMES
                .iter()
                .position(|&value| value == name)
                .map(|value| value as u32)
        })
        .or_else(|| (name == "$s8").then_some(30))
        .ok_or_else(|| {
            location.error(
                ErrorKind::InvalidOperand,
                format!("invalid general-purpose register {text}"),
            )
        })
}

pub(super) fn special_number(
    operand: &LocatedOperand<'_>,
    prefix: &str,
    max: i64,
) -> Result<u32, Diagnostic> {
    let text = atom(operand)?;
    let name = text.to_ascii_lowercase();
    let digits = if prefix == "$" || prefix == "$fcc" {
        name.strip_prefix(prefix).unwrap_or(&name)
    } else {
        name.strip_prefix(prefix).unwrap_or("")
    };
    let value = digits
        .parse::<i64>()
        .ok()
        .filter(|&value| value >= 0)
        .ok_or_else(|| {
            operand.0.error(
                ErrorKind::InvalidOperand,
                format!("invalid register or condition code {text}"),
            )
        })?;
    range(operand.0, value, 0, max)
}
