use super::operand::{
    LocatedOperand, Symbols, address_or_number, atom, gpr_name, numeric, range, special_number,
};
use super::parser::{Operand, Statement};
use super::{Diagnostic, ErrorKind};
use crate::isa::mips32r2::INSTRUCTIONS;

include!(concat!(env!("OUT_DIR"), "/assembly_formats.rs"));

fn pattern(name: &str) -> u32 {
    INSTRUCTIONS
        .iter()
        .find(|definition| definition.mnemonic == name)
        .unwrap()
        .pattern
}

pub(super) fn size(statement: &Statement<'_>) -> u64 {
    match statement.name.as_str() {
        "li" | "la" => 8,
        _ => 4,
    }
}

pub(super) fn emit(
    statement: &Statement<'_>,
    pc: u32,
    symbols: &Symbols,
    bytes: &mut Vec<u8>,
) -> Result<(), Diagnostic> {
    let name = statement.name.as_str();
    let args = statement.operands.as_slice();
    let wrong_count = || {
        statement.location.error(
            ErrorKind::InvalidOperand,
            format!("invalid operands for {name}"),
        )
    };
    let register = |operand: &LocatedOperand<'_>| gpr_name(operand.0, atom(operand)?);
    let rt = |operand| register(operand).map(|reg| reg << 16);
    let rs = |operand| register(operand).map(|reg| reg << 21);
    let rd = |operand| register(operand).map(|reg| reg << 11);
    let value =
        |operand: &LocatedOperand<'_>| address_or_number(operand.0, atom(operand)?, symbols);
    let target = |operand: &LocatedOperand<'_>| {
        let result = range(operand.0, value(operand)?, 0, i64::from(u32::MAX))?;
        if result & 3 != 0 {
            return Err(operand.0.error(
                ErrorKind::Misaligned,
                "instruction target must be word-aligned",
            ));
        }
        Ok(result)
    };
    let branch = |operand: &LocatedOperand<'_>| {
        let delta = target(operand)?.wrapping_sub(pc.wrapping_add(4)) as i32;
        Ok(range(operand.0, i64::from(delta / 4), -32768, 32767)? & 0xffff)
    };
    let imm = |operand| numeric(operand, -32768, 32767).map(|bits| bits & 0xffff);
    let cc = |operand| special_number(operand, "$fcc", 7);
    let fpr = |operand: &LocatedOperand<'_>, double: bool| {
        let index = special_number(operand, "$f", 31)?;
        if double && index & 1 != 0 {
            return Err(operand.0.error(
                ErrorKind::InvalidOperand,
                "FR=0 doubleword operands require an even FPR",
            ));
        }
        Ok(index)
    };
    let memory = |(location, operand): &LocatedOperand<'_>| {
        let Operand::Memory(offset, base) = operand else {
            return Err(location.error(ErrorKind::InvalidOperand, "expected offset(base)"));
        };
        let offset = if let Some(offset) = offset {
            let value = address_or_number(*location, offset, symbols)?;
            range(*location, value, -32768, 32767)? & 0xffff
        } else {
            0
        };
        Ok(gpr_name(*location, base)? << 21 | offset)
    };

    let pseudo = match (name, args) {
        ("nop", []) => Some(0),
        ("move", [dest, source]) => Some(pattern("addu") | rd(dest)? | rs(source)?),
        ("b", [dest]) => Some(pattern("beq") | branch(dest)?),
        ("bal", [dest]) => Some(pattern("bgezal") | branch(dest)?),
        ("li" | "la", [dest, source]) => {
            let bits = if name == "li" {
                numeric(source, i64::from(i32::MIN), i64::from(u32::MAX))?
            } else {
                range(source.0, value(source)?, 0, i64::from(u32::MAX))?
            };
            let dest = register(dest)?;
            bytes.extend_from_slice(&(pattern("lui") | dest << 16 | bits >> 16).to_le_bytes());
            Some(pattern("ori") | dest << 21 | dest << 16 | (bits & 0xffff))
        }
        ("nop" | "move" | "b" | "bal" | "li" | "la", _) => return Err(wrong_count()),
        _ => None,
    };
    if let Some(word) = pseudo {
        bytes.extend_from_slice(&word.to_le_bytes());
        return Ok(());
    }

    let format = ASSEMBLY_FORMATS
        .iter()
        .find(|&&(mnemonic, _)| mnemonic == name)
        .map(|&(_, format)| format)
        .ok_or_else(|| {
            statement.location.error(
                ErrorKind::UnsupportedInstruction,
                format!("unsupported instruction {name}"),
            )
        })?;
    let definition = INSTRUCTIONS
        .iter()
        .find(|definition| definition.mnemonic == name)
        .unwrap();
    let double_source = name.ends_with(".d");
    let double_destination =
        name.starts_with("cvt.d.") || (double_source && !name.contains(".w.") && name != "cvt.s.d");
    let fields = match (format, args) {
        ("—", []) => 0,
        ("rd, rs, rt", [d, s, t]) => rd(d)? | rs(s)? | rt(t)?,
        ("rd, rt, rs", [d, t, s]) => rd(d)? | rt(t)? | rs(s)?,
        ("rd, rt, sa", [d, t, shift]) => rd(d)? | rt(t)? | (numeric(shift, 0, 31)? << 6),
        ("rd, rs", [d, s]) => rd(d)? | rt(d)? | rs(s)?,
        ("rd, rt", [d, t]) => rd(d)? | rt(t)?,
        ("rd", [d]) => rd(d)?,
        ("rs", [s]) => rs(s)?,
        ("rs, rt", [s, t]) => rs(s)? | rt(t)?,
        ("rs, rt", [s, t, code])
            if matches!(name, "teq" | "tne" | "tge" | "tgeu" | "tlt" | "tltu") =>
        {
            rs(s)? | rt(t)? | (numeric(code, 0, 1023)? << 6)
        }
        ("rt, rs, immediate", [t, s, i]) => {
            let immediate = if matches!(name, "andi" | "ori" | "xori") {
                numeric(i, 0, 65535)?
            } else {
                imm(i)?
            };
            rt(t)? | rs(s)? | immediate
        }
        ("rt, immediate", [t, i]) => rt(t)? | numeric(i, 0, 65535)?,
        ("rs, immediate", [s, i]) => rs(s)? | imm(i)?,
        ("rt, rs, pos, size", [t, s, position, size]) => {
            let position = numeric(position, 0, 31)?;
            let size = numeric(size, 1, i64::from(32 - position))?;
            let end = if name == "ins" {
                position + size - 1
            } else {
                size - 1
            };
            rt(t)? | rs(s)? | (position << 6) | (end << 11)
        }
        ("rt, offset(base)", [t, address]) => rt(t)? | memory(address)?,
        ("offset(base)", [address]) => memory(address)?,
        ("hint,offset(base)", [hint, address]) => (numeric(hint, 0, 31)? << 16) | memory(address)?,
        ("rs, rt, offset", [s, t, dest]) => rs(s)? | rt(t)? | branch(dest)?,
        ("rs, offset", [s, dest]) => rs(s)? | branch(dest)?,
        ("target", [dest]) => {
            let address = target(dest)?;
            if (address ^ pc.wrapping_add(4)) & 0xf000_0000 != 0 {
                return Err(dest.0.error(
                    ErrorKind::OutOfRange,
                    "jump target is outside the PC + 4 region",
                ));
            }
            (address >> 2) & 0x03ff_ffff
        }
        ("[rd,] rs", [s]) => (31 << 11) | rs(s)?,
        ("[rd,] rs", [d, s]) => rd(d)? | rs(s)?,
        ("[code]" | "[stype]" | "[rt]", []) => 0,
        ("[code]", [code]) => numeric(code, 0, 0x000f_ffff)? << 6,
        ("[stype]", [stype]) => numeric(stype, 0, 31)? << 6,
        ("[rt]", [t]) => rt(t)?,
        ("rt, rd" | "rt, rd[, sel]", [t, d]) => rt(t)? | (special_number(d, "$", 31)? << 11),
        ("rt, rd[, sel]", [t, d, sel]) => {
            rt(t)? | (special_number(d, "$", 31)? << 11) | numeric(sel, 0, 7)?
        }
        ("rt, fs", [t, s]) => {
            let s = if matches!(name, "cfc1" | "ctc1") {
                special_number(s, "$", 31)?
            } else {
                fpr(s, matches!(name, "mfhc1" | "mthc1"))?
            };
            rt(t)? | (s << 11)
        }
        ("ft, offset(base)", [t, address]) => {
            (fpr(t, matches!(name, "ldc1" | "sdc1"))? << 16) | memory(address)?
        }
        ("[cc,] offset", [dest]) => branch(dest)?,
        ("[cc,] offset", [condition, dest]) => (cc(condition)? << 18) | branch(dest)?,
        ("rd, rs, cc", [d, s, condition]) => rd(d)? | rs(s)? | (cc(condition)? << 18),
        ("fd, fs, cc", [d, s, condition]) => {
            (fpr(d, double_destination)? << 6)
                | (fpr(s, double_source)? << 11)
                | (cc(condition)? << 18)
        }
        ("fd, fs, rt", [d, s, t]) => {
            (fpr(d, double_destination)? << 6) | (fpr(s, double_source)? << 11) | rt(t)?
        }
        ("[cc,] fs, ft", [s, t]) => (fpr(s, double_source)? << 11) | (fpr(t, double_source)? << 16),
        ("[cc,] fs, ft", [condition, s, t]) => {
            (cc(condition)? << 8) | (fpr(s, double_source)? << 11) | (fpr(t, double_source)? << 16)
        }
        ("fd, fs, ft", [d, s, t]) => {
            (fpr(d, double_destination)? << 6)
                | (fpr(s, double_source)? << 11)
                | (fpr(t, double_source)? << 16)
        }
        ("fd, fs", [d, s]) => (fpr(d, double_destination)? << 6) | (fpr(s, double_source)? << 11),
        _ => return Err(wrong_count()),
    };
    if fields & definition.mask != 0 {
        return Err(statement.location.error(
            ErrorKind::InvalidOperand,
            "operand overlaps fixed instruction bits",
        ));
    }
    bytes.extend_from_slice(&(definition.pattern | fields).to_le_bytes());
    Ok(())
}
