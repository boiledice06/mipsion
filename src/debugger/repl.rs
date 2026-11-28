use std::io::{self, BufRead, Write};

use crate::runtime::StopReason;
use crate::runtime::inspection::GPR_NAMES;
use crate::runtime::trace::format_trace;

use super::command::{Command, parse_command};
use super::{DebuggerSession, WatchTarget};

pub const DEBUGGER_HELP: &str = "\
Debugger commands:
    help, h                   Show debugger help
    step, s                   Execute one instruction
    continue, c [count]       Continue execution
    reset, r                  Restore initial state
    regs                      Show general-purpose registers
    cp0                       Show CP0 registers
    fpu                       Show floating-point registers
    pc                        Show current PC
    disasm, d [addr] [count]   Disassemble instructions
    mem, m <addr> <size>       Dump physical memory
    break, b <addr>            Add breakpoint
    delete <addr>              Remove breakpoint
    breakpoints, bl            List breakpoints
    clear-breakpoints          Remove all breakpoints
    watch, w <reg>             Watch register changes
    unwatch <reg>              Remove register watchpoint
    watchmem, wm <addr> <width>
                               Watch physical memory
    unwatchmem <addr> <width>  Remove memory watchpoint
    watchpoints, wl            List watchpoints
    clear-watchpoints          Remove all watchpoints
    status                    Show debugger status
    quit, q                   Exit debugger

Memory widths: 1, 2, 4, 8 bytes.
Numbers may be decimal or hexadecimal (0x prefix).
Breakpoints use virtual addresses.
Memory commands use physical addresses.
";

fn target_name(target: WatchTarget) -> String {
    match target {
        WatchTarget::Register(index) => GPR_NAMES
            .get(index)
            .map(|name| name.to_string())
            .unwrap_or_else(|| format!("${index}")),

        WatchTarget::Hi => "HI".into(),

        WatchTarget::Lo => "LO".into(),

        WatchTarget::Memory { address, width } => {
            format!("Memory[0x{address:08X}, {} bytes]", width.bytes(),)
        }
    }
}

fn print_watchpoint_hits(session: &DebuggerSession, output: &mut impl Write) -> io::Result<()> {
    for hit in session.last_watchpoint_hits() {
        writeln!(output, "Watchpoint hit: {}", target_name(hit.target),)?;

        writeln!(output, "  0x{:X} -> 0x{:X}", hit.before, hit.after,)?;
    }

    Ok(())
}

fn print_current_instruction(session: &DebuggerSession, output: &mut impl Write) -> io::Result<()> {
    match session.disassembly_line(session.pc()) {
        Ok(line) => writeln!(output, "{line}"),

        Err(error) => writeln!(output, "PC=0x{:08X} <unavailable: {error:?}>", session.pc(),),
    }
}

pub fn execute_command(
    session: &mut DebuggerSession,
    command: Command,
    output: &mut impl Write,
    default_max_steps: u64,
) -> io::Result<bool> {
    match command {
        Command::Empty => {}

        Command::Help => {
            write!(output, "{DEBUGGER_HELP}")?;
        }

        Command::Step => {
            match session.step() {
                Some(event) => {
                    write!(output, "{}", format_trace(&event),)?;

                    print_watchpoint_hits(session, output)?;
                }

                None => {
                    writeln!(output, "CPU halted")?;
                }
            }

            print_current_instruction(session, output)?;
        }

        Command::Continue(count) => {
            let result = session.continue_execution(count.unwrap_or(default_max_steps));

            match result.reason {
                StopReason::Halted => {
                    writeln!(output, "CPU halted")?;
                }

                StopReason::StepLimit => {
                    writeln!(output, "Step limit reached")?;
                }

                StopReason::Breakpoint { address } => {
                    writeln!(output, "Breakpoint hit: 0x{address:08X}",)?;
                }

                StopReason::Watchpoint => {}

                StopReason::Exception {
                    code,
                    epc,
                    in_delay_slot,
                } => {
                    writeln!(
                        output,
                        "Exception: {code} \
                         EPC=0x{epc:08X} \
                         BD={in_delay_slot}",
                    )?;
                }
            }

            print_watchpoint_hits(session, output)?;

            writeln!(output, "Steps: {}", result.steps)?;

            print_current_instruction(session, output)?;
        }

        Command::Reset => {
            session.reset();

            writeln!(output, "Session reset")?;

            print_current_instruction(session, output)?;
        }

        Command::Registers => {
            write!(output, "{}", session.registers())?;
        }

        Command::Cp0 => {
            write!(output, "{}", session.cp0_registers())?;
        }

        Command::Fpu => {
            write!(output, "{}", session.fpu_registers())?;
        }

        Command::Pc => {
            writeln!(output, "PC=0x{:08X}", session.pc())?;

            writeln!(output, "Next PC=0x{:08X}", session.cpu().read_next_pc(),)?;
        }

        Command::Disassemble { address, count } => {
            let start = address.unwrap_or(session.pc());

            for index in 0..count.min(256) {
                let Some(address) = start.checked_add((index as u32) * 4) else {
                    writeln!(output, "Address space boundary reached",)?;
                    break;
                };

                let current = if address == session.pc() { ">" } else { " " };

                let breakpoint = if session.has_breakpoint(address) {
                    "*"
                } else {
                    " "
                };

                match session.disassembly_line(address) {
                    Ok(line) => {
                        writeln!(output, "{current}{breakpoint} {line}",)?;
                    }

                    Err(error) => {
                        writeln!(output, "0x{address:08X}: {error:?}",)?;

                        break;
                    }
                }
            }
        }

        Command::Memory { address, size } => {
            if size == 0 || size > 65536 {
                writeln!(output, "Error: invalid dump size")?;
            } else {
                match session.memory_dump(address, size) {
                    Ok(dump) => write!(output, "{dump}")?,

                    Err(error) => {
                        writeln!(output, "Error: {error}")?;
                    }
                }
            }
        }

        Command::Break(address) => match session.add_breakpoint(address) {
            Ok(true) => writeln!(output, "Breakpoint added: 0x{address:08X}",)?,

            Ok(false) => writeln!(
                output,
                "Breakpoint already exists: \
                     0x{address:08X}",
            )?,

            Err(error) => {
                writeln!(output, "Error: {error}")?;
            }
        },

        Command::Delete(address) => {
            if session.remove_breakpoint(address) {
                writeln!(output, "Breakpoint removed: 0x{address:08X}",)?;
            } else {
                writeln!(output, "Breakpoint not found")?;
            }
        }

        Command::Breakpoints => {
            if session.breakpoints().is_empty() {
                writeln!(output, "No breakpoints")?;
            }

            for address in session.breakpoints().iter() {
                writeln!(output, "0x{address:08X}")?;
            }
        }

        Command::ClearBreakpoints => {
            session.clear_breakpoints();

            writeln!(output, "All breakpoints cleared")?;
        }

        Command::Watch(target) => match session.add_watchpoint(target) {
            Ok(true) => writeln!(output, "Watchpoint added: {}", target_name(target),)?,

            Ok(false) => writeln!(output, "Watchpoint already exists: {}", target_name(target),)?,

            Err(error) => {
                writeln!(output, "Error: {error}")?;
            }
        },

        Command::Unwatch(target) => {
            if session.remove_watchpoint(target) {
                writeln!(output, "Watchpoint removed: {}", target_name(target),)?;
            } else {
                writeln!(output, "Watchpoint not found")?;
            }
        }

        Command::Watchpoints => {
            if session.watchpoints().is_empty() {
                writeln!(output, "No watchpoints")?;
            }

            for (target, value) in session.watchpoints().iter() {
                writeln!(output, "{} = 0x{value:X}", target_name(target),)?;
            }
        }

        Command::ClearWatchpoints => {
            session.clear_watchpoints();

            writeln!(output, "All watchpoints cleared")?;
        }

        Command::Status => {
            writeln!(output, "Entry:       0x{:08X}", session.entry_point(),)?;

            writeln!(output, "PC:          0x{:08X}", session.pc(),)?;

            writeln!(output, "Steps:       {}", session.steps(),)?;

            writeln!(output, "Halted:      {}", session.is_halted(),)?;

            writeln!(output, "Breakpoints: {}", session.breakpoints().len(),)?;

            writeln!(output, "Watchpoints: {}", session.watchpoints().len(),)?;
        }

        Command::Quit => {
            writeln!(output, "Debugger terminated")?;

            return Ok(false);
        }
    }

    Ok(true)
}

pub fn run_repl(
    session: &mut DebuggerSession,
    mut input: impl BufRead,
    mut output: impl Write,
    default_max_steps: u64,
) -> io::Result<()> {
    writeln!(output)?;
    writeln!(output, "MIPSion Interactive Debugger")?;
    writeln!(output, "Type 'help' for available commands.")?;
    writeln!(output)?;

    print_current_instruction(session, &mut output)?;

    let mut line = String::new();

    loop {
        write!(output, "(mipsion) ")?;
        output.flush()?;

        line.clear();

        if input.read_line(&mut line)? == 0 {
            writeln!(output)?;
            break;
        }

        match parse_command(line.trim()) {
            Ok(command) => {
                if !execute_command(session, command, &mut output, default_max_steps)? {
                    break;
                }
            }

            Err(error) => {
                writeln!(output, "Error: {error}")?;
            }
        }
    }

    output.flush()
}
