use crate::cli::{parse_address, parse_number};
use crate::runtime::inspection::GPR_NAMES;

use super::watchpoint::{MemoryWidth, WatchTarget};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Help,
    Step,
    Continue(Option<u64>),
    Reset,
    Registers,
    Cp0,
    Fpu,
    Pc,
    Disassemble { address: Option<u32>, count: usize },
    Memory { address: u32, size: usize },
    Break(u32),
    Delete(u32),
    Breakpoints,
    ClearBreakpoints,
    Watch(WatchTarget),
    Unwatch(WatchTarget),
    Watchpoints,
    ClearWatchpoints,
    Status,
    Quit,
    Empty,
}

fn parse_register(value: &str) -> Result<WatchTarget, String> {
    let name = value.trim_start_matches('$').to_ascii_lowercase();

    match name.as_str() {
        "hi" => return Ok(WatchTarget::Hi),
        "lo" => return Ok(WatchTarget::Lo),
        "s8" => return Ok(WatchTarget::Register(30)),
        _ => {}
    }

    if let Ok(index) = name.parse::<usize>()
        && index < 32
    {
        return Ok(WatchTarget::Register(index));
    }

    GPR_NAMES
        .iter()
        .position(|register| register.trim_start_matches('$') == name)
        .map(WatchTarget::Register)
        .ok_or_else(|| format!("invalid register: {value}"))
}

fn parse_width(value: &str) -> Result<MemoryWidth, String> {
    match value {
        "1" | "byte" => Ok(MemoryWidth::Byte),
        "2" | "halfword" => Ok(MemoryWidth::Halfword),
        "4" | "word" => Ok(MemoryWidth::Word),
        "8" | "doubleword" => Ok(MemoryWidth::Doubleword),

        _ => Err(format!("invalid memory width: {value}")),
    }
}

fn parse_size(value: &str, max: usize) -> Result<usize, String> {
    let size =
        usize::try_from(parse_number(value)?).map_err(|_| "size out of range".to_string())?;

    if size == 0 || size > max {
        return Err(format!("size must be between 1 and {max}"));
    }

    Ok(size)
}

pub fn parse_command(line: &str) -> Result<Command, String> {
    let args: Vec<&str> = line.split_whitespace().collect();

    match args.as_slice() {
        [] => Ok(Command::Empty),

        ["help" | "h"] => Ok(Command::Help),

        ["step" | "s"] => Ok(Command::Step),

        ["continue" | "c"] => Ok(Command::Continue(None)),

        ["continue" | "c", count] => Ok(Command::Continue(Some(parse_number(count)?))),

        ["reset" | "r"] => Ok(Command::Reset),

        ["regs"] => Ok(Command::Registers),

        ["cp0"] => Ok(Command::Cp0),

        ["fpu"] => Ok(Command::Fpu),

        ["pc"] => Ok(Command::Pc),

        ["disasm" | "d"] => Ok(Command::Disassemble {
            address: None,
            count: 8,
        }),

        ["disasm" | "d", address] => Ok(Command::Disassemble {
            address: Some(parse_address(address)?),
            count: 8,
        }),

        ["disasm" | "d", address, count] => Ok(Command::Disassemble {
            address: Some(parse_address(address)?),
            count: parse_size(count, 256)?,
        }),

        ["mem" | "m", address, size] => Ok(Command::Memory {
            address: parse_address(address)?,
            size: parse_size(size, 65536)?,
        }),

        ["break" | "b", address] => Ok(Command::Break(parse_address(address)?)),

        ["delete", address] => Ok(Command::Delete(parse_address(address)?)),

        ["breakpoints" | "bl"] => Ok(Command::Breakpoints),

        ["clear-breakpoints"] => Ok(Command::ClearBreakpoints),

        ["watch" | "w", register] => Ok(Command::Watch(parse_register(register)?)),

        ["unwatch", register] => Ok(Command::Unwatch(parse_register(register)?)),

        ["watchmem" | "wm", address, width] => Ok(Command::Watch(WatchTarget::Memory {
            address: parse_address(address)?,
            width: parse_width(width)?,
        })),

        ["unwatchmem", address, width] => Ok(Command::Unwatch(WatchTarget::Memory {
            address: parse_address(address)?,
            width: parse_width(width)?,
        })),

        ["watchpoints" | "wl"] => Ok(Command::Watchpoints),

        ["clear-watchpoints"] => Ok(Command::ClearWatchpoints),

        ["status"] => Ok(Command::Status),

        ["quit" | "q" | "exit"] => Ok(Command::Quit),

        _ => Err(format!("unknown command or invalid arguments: {line}")),
    }
}
