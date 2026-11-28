//! Two-pass assembly for flat, little-endian MIPS32 Release 2 images.

mod directive;
mod encoder;
mod error;
mod lexer;
mod operand;
mod parser;
mod symbol;

pub use error::{Diagnostic, ErrorKind, SourceLocation};

use directive::Directive;
use operand::Symbols;
use symbol::{Section, assign_sections, define_symbols, next_address};

const MAX_IMAGE_SIZE: usize = 256 * 1024 * 1024;

/// A flat memory image with text first, followed by word-aligned data.
#[derive(Debug, PartialEq, Eq)]
pub struct Assembly {
    pub bytes: Vec<u8>,
    pub load_address: u32,
    pub text_size: usize,
    pub data_offset: usize,
    pub symbols: std::collections::BTreeMap<String, u32>,
}

/// Assemble with the specified word-aligned base address. The first source
/// error is returned with a one-based line and UTF-8 byte column.
pub fn assemble(source: &str, load_address: u32) -> Result<Assembly, Diagnostic> {
    let start = SourceLocation { line: 1, column: 1 };
    if load_address & 3 != 0 {
        return Err(start.error(ErrorKind::Misaligned, "load address must be word-aligned"));
    }
    let body = assign_sections(parser::parse(source)?)?;
    let mut symbols = Symbols::new();
    let base = u64::from(load_address);
    let mut address = base;
    let mut entries = Vec::new();
    let mut text_size = 0;
    let mut data_offset = 0;

    for section in [Section::Text, Section::Data] {
        if section == Section::Data {
            text_size = (address - base) as usize;
            if body.iter().any(|(kind, _)| *kind == Section::Data) {
                address = address.next_multiple_of(4);
            }
            data_offset = (address - base) as usize;
        }
        for (_, line) in body.iter().filter(|(kind, _)| *kind == section) {
            define_symbols(line, address, &mut symbols)?;
            let Some(statement) = &line.statement else {
                continue;
            };
            let directive = if statement.name.starts_with('.') {
                Some(Directive::parse(statement, address)?)
            } else {
                if section == Section::Data {
                    return Err(statement
                        .location
                        .error(ErrorKind::Layout, "instructions belong in .text"));
                }
                if address & 3 != 0 {
                    return Err(statement.location.error(
                        ErrorKind::Misaligned,
                        "instruction address must be word-aligned",
                    ));
                }
                None
            };
            let size = match &directive {
                Some(directive) => directive.size(statement)?,
                None => encoder::size(statement),
            };
            entries.push((address, statement, directive));
            address = next_address(address, size, load_address, statement)?;
        }
    }

    let length = (address - base) as usize;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(length)
        .map_err(|_| start.error(ErrorKind::Layout, "could not allocate assembly image"))?;
    for (address, statement, directive) in entries {
        bytes.resize((address - base) as usize, 0);
        match directive {
            Some(directive) => directive.emit(statement, &symbols, &mut bytes)?,
            None => encoder::emit(statement, address as u32, &symbols, &mut bytes)?,
        }
    }
    bytes.resize(length, 0);
    Ok(Assembly {
        bytes,
        load_address,
        text_size,
        data_offset,
        symbols,
    })
}

#[cfg(test)]
mod tests;
