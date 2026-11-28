use super::operand::Symbols;
use super::parser::Line;
use super::{Diagnostic, ErrorKind, MAX_IMAGE_SIZE};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Section {
    Text,
    Data,
}

pub(super) fn assign_sections<'a>(
    lines: Vec<Line<'a>>,
) -> Result<Vec<(Section, Line<'a>)>, Diagnostic> {
    let mut section = Section::Text;
    let mut body = Vec::new();
    for line in lines {
        if let Some(statement) = &line.statement
            && let ".text" | ".data" = statement.name.as_str()
        {
            if !line.labels.is_empty() || !statement.operands.is_empty() {
                return Err(statement.location.error(
                    ErrorKind::InvalidOperand,
                    "section switches require a line of their own",
                ));
            }
            section = if statement.name == ".text" {
                Section::Text
            } else {
                Section::Data
            };
        } else {
            body.push((section, line));
        }
    }
    Ok(body)
}

pub(super) fn define_symbols(
    line: &Line<'_>,
    address: u64,
    symbols: &mut Symbols,
) -> Result<(), Diagnostic> {
    for &(location, name) in &line.labels {
        let address = u32::try_from(address).map_err(|_| {
            location.error(
                ErrorKind::Layout,
                "label is outside the 32-bit address space",
            )
        })?;
        if symbols.insert(name.to_string(), address).is_some() {
            return Err(location.error(
                ErrorKind::DuplicateSymbol,
                format!("duplicate label {name}"),
            ));
        }
    }
    Ok(())
}

pub(super) fn next_address(
    address: u64,
    size: u64,
    base: u32,
    statement: &super::parser::Statement<'_>,
) -> Result<u64, Diagnostic> {
    let end = address + size;
    if end > u64::from(u32::MAX) + 1 || end - u64::from(base) > MAX_IMAGE_SIZE as u64 {
        return Err(statement.location.error(
            ErrorKind::Layout,
            "image exceeds the 32-bit address space or 256 MiB size limit",
        ));
    }
    Ok(end)
}
