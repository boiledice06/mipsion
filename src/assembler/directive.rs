use super::operand::{Symbols, address_or_number, atom, numeric, range};
use super::parser::{Operand, Statement};
use super::{Diagnostic, ErrorKind};

pub(super) enum Directive {
    Values(u32),
    Strings(bool),
    Fill(u64),
}

impl Directive {
    pub(super) fn parse(statement: &Statement<'_>, address: u64) -> Result<Self, Diagnostic> {
        let directive = match statement.name.as_str() {
            ".byte" => Self::Values(1),
            ".half" => Self::Values(2),
            ".word" => Self::Values(4),
            ".ascii" => Self::Strings(false),
            ".asciiz" => Self::Strings(true),
            ".space" | ".align" => {
                let [operand] = statement.operands.as_slice() else {
                    return Err(statement
                        .location
                        .error(ErrorKind::InvalidOperand, "expected one directive operand"));
                };
                let count = if statement.name == ".space" {
                    u64::from(numeric(operand, 0, i64::from(u32::MAX))?)
                } else {
                    let alignment = 1u64 << numeric(operand, 0, 16)?;
                    address.next_multiple_of(alignment) - address
                };
                return Ok(Self::Fill(count));
            }
            _ => {
                return Err(statement.location.error(
                    ErrorKind::UnknownDirective,
                    format!("unknown directive {}", statement.name),
                ));
            }
        };
        if statement.operands.is_empty() {
            return Err(statement.location.error(
                ErrorKind::InvalidOperand,
                "expected at least one directive operand",
            ));
        }
        Ok(directive)
    }

    pub(super) fn size(&self, statement: &Statement<'_>) -> Result<u64, Diagnostic> {
        match *self {
            Self::Values(width) => Ok(u64::from(width) * statement.operands.len() as u64),
            Self::Strings(terminated) => {
                statement
                    .operands
                    .iter()
                    .try_fold(0, |size, (location, operand)| match operand {
                        Operand::Bytes(bytes) => {
                            Ok(size + bytes.len() as u64 + u64::from(terminated))
                        }
                        _ => {
                            Err(location
                                .error(ErrorKind::InvalidOperand, "expected a quoted string"))
                        }
                    })
            }
            Self::Fill(count) => Ok(count),
        }
    }

    pub(super) fn emit(
        &self,
        statement: &Statement<'_>,
        symbols: &Symbols,
        bytes: &mut Vec<u8>,
    ) -> Result<(), Diagnostic> {
        match *self {
            Self::Values(width) => {
                let bits = 8 * width;
                for operand in &statement.operands {
                    let value = address_or_number(operand.0, atom(operand)?, symbols)?;
                    let field = range(operand.0, value, -(1i64 << (bits - 1)), (1i64 << bits) - 1)?;
                    bytes.extend_from_slice(&field.to_le_bytes()[..width as usize]);
                }
            }
            Self::Strings(terminated) => {
                for (_, operand) in &statement.operands {
                    let Operand::Bytes(value) = operand else {
                        unreachable!("validated during layout")
                    };
                    bytes.extend_from_slice(value);
                    if terminated {
                        bytes.push(0);
                    }
                }
            }
            Self::Fill(count) => bytes.resize(bytes.len() + count as usize, 0),
        }
        Ok(())
    }
}
