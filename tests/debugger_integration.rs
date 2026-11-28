use std::io::Cursor;

use mipsion::cpu::Cpu;
use mipsion::debugger::{DebuggerSession, MemoryWidth, WatchTarget, run_repl};
use mipsion::memory::Memory;

fn session() -> DebuggerSession {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4096);

    cpu.write_pc(0x100);
    cpu.write_reg(29, 0xFF0);

    // addiu $t0, $zero, 42
    memory.write_word(0x100, 0x2408_002A);

    // addiu $t1, $t0, 1
    memory.write_word(0x104, 0x2509_0001);

    // sw $t1, 512($zero)
    memory.write_word(0x108, 0xAC09_0200);

    // syscall
    memory.write_word(0x10C, 0x0000_000C);

    DebuggerSession::new(cpu, memory)
}

fn run_script(session: &mut DebuggerSession, script: &str, max_steps: u64) -> String {
    let input = Cursor::new(script.as_bytes());
    let mut output = Vec::new();

    run_repl(session, input, &mut output, max_steps).unwrap();

    String::from_utf8(output).unwrap()
}

#[test]
fn repl_help_and_quit() {
    let mut session = session();

    let output = run_script(&mut session, "help\nquit\n", 100);

    assert!(output.contains("Debugger commands:"));
    assert!(output.contains("Debugger terminated"));
    assert_eq!(session.steps(), 0);
}

#[test]
fn repl_step_execution() {
    let mut session = session();

    let output = run_script(&mut session, "step\nstep\nquit\n", 100);

    assert_eq!(session.steps(), 2);
    assert_eq!(session.pc(), 0x108);

    assert_eq!(session.cpu().read_reg(8), 42);
    assert_eq!(session.cpu().read_reg(9), 43);

    assert!(output.contains("[000001]"));
    assert!(output.contains("[000002]"));
}

#[test]
fn repl_continue_until_exception() {
    let mut session = session();

    let output = run_script(&mut session, "continue\nquit\n", 100);

    assert_eq!(session.steps(), 4);
    assert_eq!(session.cpu().exception_count(), 1);

    assert!(output.contains("Exception: 8"));
    assert!(output.contains("EPC=0x0000010C"));

    assert_eq!(session.memory().read_word(0x200), 43);
}

#[test]
fn repl_breakpoint_execution() {
    let mut session = session();

    let output = run_script(&mut session, "break 0x104\ncontinue\nquit\n", 100);

    assert_eq!(session.steps(), 1);
    assert_eq!(session.pc(), 0x104);

    assert_eq!(session.cpu().read_reg(9), 0);

    assert!(output.contains("Breakpoint hit: 0x00000104"));
}

#[test]
fn repl_continue_after_breakpoint() {
    let mut session = session();

    let output = run_script(&mut session, "break 0x104\nc\nc 1\nquit\n", 100);

    assert_eq!(session.steps(), 2);
    assert_eq!(session.pc(), 0x108);
    assert_eq!(session.cpu().read_reg(9), 43);

    assert!(output.contains("Step limit reached"));
}

#[test]
fn repl_register_watchpoint() {
    let mut session = session();

    let output = run_script(&mut session, "watch t0\ncontinue\nquit\n", 100);

    assert_eq!(session.steps(), 1);

    assert!(output.contains("Watchpoint hit: $t0"));
    assert!(output.contains("0x0 -> 0x2A"));
}

#[test]
fn repl_memory_watchpoint() {
    let mut session = session();

    let output = run_script(&mut session, "watchmem 0x200 4\ncontinue\nquit\n", 100);

    assert_eq!(session.steps(), 3);
    assert_eq!(session.pc(), 0x10C);

    assert_eq!(session.memory().read_word(0x200), 43);

    assert!(output.contains("Watchpoint hit: Memory[0x00000200, 4 bytes]"));

    assert!(output.contains("0x0 -> 0x2B"));
}

#[test]
fn repl_breakpoint_watchpoint_interaction() {
    let mut session = session();

    let output = run_script(&mut session, "break 0x104\nwatch t0\nc\nc\ns\nquit\n", 100);

    let watchpoint = output.find("Watchpoint hit: $t0").unwrap();

    let breakpoint = output.find("Breakpoint hit: 0x00000104").unwrap();

    assert!(watchpoint < breakpoint);

    assert_eq!(session.steps(), 2);
    assert_eq!(session.pc(), 0x108);

    assert_eq!(session.cpu().read_reg(8), 42);
    assert_eq!(session.cpu().read_reg(9), 43);
}

#[test]
fn repl_reset_restores_machine() {
    let mut session = session();

    let output = run_script(&mut session, "continue\nreset\nquit\n", 100);

    assert!(output.contains("Session reset"));

    assert_eq!(session.pc(), 0x100);
    assert_eq!(session.steps(), 0);

    assert_eq!(session.cpu().read_reg(8), 0);
    assert_eq!(session.cpu().read_reg(9), 0);

    assert_eq!(session.cpu().read_reg(29), 0xFF0);

    assert_eq!(session.memory().read_word(0x200), 0);

    assert_eq!(session.cpu().exception_count(), 0);
}

#[test]
fn repl_reset_preserves_debug_configuration() {
    let mut session = session();

    run_script(
        &mut session,
        "break 0x104\n\
         watch t0\n\
         watchmem 0x200 4\n\
         continue\n\
         reset\n\
         quit\n",
        100,
    );

    assert!(session.has_breakpoint(0x104));

    assert!(session.has_watchpoint(WatchTarget::Register(8)));

    assert!(session.has_watchpoint(WatchTarget::Memory {
        address: 0x200,
        width: MemoryWidth::Word,
    }));

    assert_eq!(session.steps(), 0);
    assert_eq!(session.watchpoints().len(), 2);

    assert!(session.last_watchpoint_hits().is_empty());

    for (_, value) in session.watchpoints().iter() {
        assert_eq!(value, 0);
    }
}

#[test]
fn repl_inspection_does_not_execute() {
    let mut session = session();

    let output = run_script(
        &mut session,
        "regs\ncp0\nfpu\npc\nd\nmem 0x100 16\nstatus\nq\n",
        100,
    );

    assert_eq!(session.steps(), 0);
    assert_eq!(session.pc(), 0x100);
    assert_eq!(session.cpu().cp0().read_count(), 0);

    for expected in [
        "General Purpose Registers",
        "System Control Registers",
        "Floating-Point Registers",
        "Memory Dump",
        "PC=0x00000100",
    ] {
        assert!(output.contains(expected));
    }
}

#[test]
fn repl_recovers_from_invalid_command() {
    let mut session = session();

    let output = run_script(&mut session, "invalid-command\nstep\nquit\n", 100);

    assert!(output.contains("Error:"));
    assert!(output.contains("Debugger terminated"));

    assert_eq!(session.steps(), 1);
    assert_eq!(session.cpu().read_reg(8), 42);
}

#[test]
fn repl_recovers_from_invalid_address() {
    let mut session = session();

    let output = run_script(
        &mut session,
        "break 0x101\n\
         mem 0xFFFF0000 4\n\
         watchmem 0xFFFF0000 4\n\
         step\n\
         quit\n",
        100,
    );

    assert!(output.contains("unaligned breakpoint"));
    assert!(output.contains("out of bounds"));

    assert!(session.breakpoints().is_empty());
    assert!(session.watchpoints().is_empty());

    assert_eq!(session.steps(), 1);
}

#[test]
fn repl_handles_eof() {
    let mut session = session();

    let output = run_script(&mut session, "step\n", 100);

    assert_eq!(session.steps(), 1);

    assert!(output.contains("MIPSion Interactive Debugger"));
}

#[test]
fn repl_default_execution_limit() {
    let mut session = session();

    let output = run_script(&mut session, "continue\nquit\n", 2);

    assert_eq!(session.steps(), 2);
    assert_eq!(session.pc(), 0x108);

    assert!(output.contains("Step limit reached"));
}

#[test]
fn repl_removes_debug_configuration() {
    let mut session = session();

    let output = run_script(
        &mut session,
        "break 0x104\n\
         delete 0x104\n\
         watch t0\n\
         unwatch t0\n\
         watchmem 0x200 4\n\
         unwatchmem 0x200 4\n\
         breakpoints\n\
         watchpoints\n\
         quit\n",
        100,
    );

    assert!(session.breakpoints().is_empty());
    assert!(session.watchpoints().is_empty());

    assert!(output.contains("No breakpoints"));
    assert!(output.contains("No watchpoints"));
}
