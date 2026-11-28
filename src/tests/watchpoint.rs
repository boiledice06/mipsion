use crate::cpu::Cpu;
use crate::debugger::{DebuggerSession, MemoryWidth, WatchTarget, WatchpointHit, Watchpoints};
use crate::memory::Memory;
use crate::runtime::StopReason;

fn session() -> DebuggerSession {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.write_pc(0x100);

    memory.write_word(0x100, 0x2408_002A);
    memory.write_word(0x104, 0x2509_0001);
    memory.write_word(0x108, 0xAC08_0200);
    memory.write_word(0x10C, 0x0000_000C);

    DebuggerSession::new(cpu, memory)
}

fn memory_target(address: u32, width: MemoryWidth) -> WatchTarget {
    WatchTarget::Memory { address, width }
}

#[test]
fn watchpoint_initially_empty() {
    let watches = Watchpoints::new();

    assert!(watches.is_empty());
    assert_eq!(watches.len(), 0);
}

#[test]
fn watchpoint_add_register() {
    let mut session = session();
    let target = WatchTarget::Register(8);

    assert_eq!(session.add_watchpoint(target), Ok(true));

    assert!(session.has_watchpoint(target));
}

#[test]
fn watchpoint_duplicate() {
    let mut session = session();
    let target = WatchTarget::Register(8);

    assert_eq!(session.add_watchpoint(target), Ok(true));

    assert_eq!(session.add_watchpoint(target), Ok(false));

    assert_eq!(session.watchpoints().len(), 1);
}

#[test]
fn watchpoint_remove() {
    let mut session = session();
    let target = WatchTarget::Register(8);

    session.add_watchpoint(target).unwrap();

    assert!(session.remove_watchpoint(target));
    assert!(!session.has_watchpoint(target));
    assert!(!session.remove_watchpoint(target));
}

#[test]
fn watchpoint_invalid_register() {
    let mut session = session();

    assert!(session.add_watchpoint(WatchTarget::Register(32)).is_err());

    assert!(session.watchpoints().is_empty());
}

#[test]
fn watchpoint_memory_out_of_bounds() {
    let mut session = session();

    assert!(
        session
            .add_watchpoint(memory_target(0x400, MemoryWidth::Word,))
            .is_err()
    );
}

#[test]
fn watchpoint_memory_partial_out_of_bounds() {
    let mut session = session();

    assert!(
        session
            .add_watchpoint(memory_target(0x3FF, MemoryWidth::Word,))
            .is_err()
    );
}

#[test]
fn watchpoint_register_change() {
    let mut session = session();
    let target = WatchTarget::Register(8);

    session.add_watchpoint(target).unwrap();

    let result = session.continue_execution(100);

    assert_eq!(result.reason, StopReason::Watchpoint);
    assert_eq!(result.steps, 1);
    assert_eq!(session.pc(), 0x104);

    assert_eq!(
        session.last_watchpoint_hits(),
        &[WatchpointHit {
            target,
            before: 0,
            after: 42,
        }]
    );
}

#[test]
fn watchpoint_unchanged_register() {
    let mut session = session();

    session.add_watchpoint(WatchTarget::Register(16)).unwrap();

    let result = session.continue_execution(2);

    assert_eq!(result.reason, StopReason::StepLimit);
    assert!(session.last_watchpoint_hits().is_empty());
}

#[test]
fn watchpoint_zero_register() {
    let mut session = session();

    session.add_watchpoint(WatchTarget::Register(0)).unwrap();

    let result = session.continue_execution(3);

    assert_eq!(result.reason, StopReason::StepLimit);
    assert!(session.last_watchpoint_hits().is_empty());
}

#[test]
fn watchpoint_memory_word_change() {
    let mut session = session();

    let target = memory_target(0x200, MemoryWidth::Word);

    session.add_watchpoint(target).unwrap();

    let result = session.continue_execution(100);

    assert_eq!(result.reason, StopReason::Watchpoint);
    assert_eq!(result.steps, 3);
    assert_eq!(session.pc(), 0x10C);

    assert_eq!(
        session.last_watchpoint_hits(),
        &[WatchpointHit {
            target,
            before: 0,
            after: 42,
        }]
    );
}

#[test]
fn watchpoint_memory_byte_change() {
    let mut session = session();

    let target = memory_target(0x200, MemoryWidth::Byte);

    session.add_watchpoint(target).unwrap();

    let result = session.continue_execution(100);

    assert_eq!(result.reason, StopReason::Watchpoint);
    assert_eq!(session.last_watchpoint_hits()[0].after, 42);
}

#[test]
fn watchpoint_memory_halfword_change() {
    let mut session = session();

    let target = memory_target(0x200, MemoryWidth::Halfword);

    session.add_watchpoint(target).unwrap();

    let result = session.continue_execution(100);

    assert_eq!(result.reason, StopReason::Watchpoint);
    assert_eq!(session.last_watchpoint_hits()[0].after, 42);
}

#[test]
fn watchpoint_memory_doubleword_change() {
    let mut session = session();

    let target = memory_target(0x200, MemoryWidth::Doubleword);

    session.add_watchpoint(target).unwrap();

    let result = session.continue_execution(100);

    assert_eq!(result.reason, StopReason::Watchpoint);
    assert_eq!(session.last_watchpoint_hits()[0].after, 42);
}

#[test]
fn watchpoint_multiple_hits() {
    let mut session = session();

    session
        .add_watchpoint(memory_target(0x200, MemoryWidth::Byte))
        .unwrap();

    session
        .add_watchpoint(memory_target(0x200, MemoryWidth::Word))
        .unwrap();

    let result = session.continue_execution(100);

    assert_eq!(result.reason, StopReason::Watchpoint);
    assert_eq!(session.last_watchpoint_hits().len(), 2);
}

#[test]
fn watchpoint_continue_after_hit() {
    let mut session = session();

    session.add_watchpoint(WatchTarget::Register(8)).unwrap();

    let first = session.continue_execution(100);

    assert_eq!(first.reason, StopReason::Watchpoint);

    let second = session.continue_execution(1);

    assert_eq!(second.reason, StopReason::StepLimit);
    assert_eq!(session.pc(), 0x108);
    assert!(session.last_watchpoint_hits().is_empty());
}

#[test]
fn watchpoint_step_reports_hit() {
    let mut session = session();

    session.add_watchpoint(WatchTarget::Register(8)).unwrap();

    session.step().unwrap();

    assert_eq!(session.steps(), 1);
    assert_eq!(session.last_watchpoint_hits().len(), 1);
}

#[test]
fn watchpoint_step_updates_baseline() {
    let mut session = session();

    session.add_watchpoint(WatchTarget::Register(8)).unwrap();

    session.step().unwrap();
    session.step().unwrap();

    assert!(session.last_watchpoint_hits().is_empty());
}

#[test]
fn watchpoint_reset_preserves_configuration() {
    let mut session = session();
    let target = WatchTarget::Register(8);

    session.add_watchpoint(target).unwrap();
    session.continue_execution(100);

    session.reset();

    assert!(session.has_watchpoint(target));
    assert!(session.last_watchpoint_hits().is_empty());

    let result = session.continue_execution(100);

    assert_eq!(result.reason, StopReason::Watchpoint);
    assert_eq!(session.last_watchpoint_hits()[0].before, 0);
    assert_eq!(session.last_watchpoint_hits()[0].after, 42);
}

#[test]
fn watchpoint_clear() {
    let mut session = session();

    session.add_watchpoint(WatchTarget::Register(8)).unwrap();

    session.clear_watchpoints();

    assert!(session.watchpoints().is_empty());

    let result = session.continue_execution(2);

    assert_eq!(result.reason, StopReason::StepLimit);
}

#[test]
fn watchpoint_breakpoint_interaction() {
    let mut session = session();

    session.add_breakpoint(0x100).unwrap();

    session.add_watchpoint(WatchTarget::Register(8)).unwrap();

    let first = session.continue_execution(100);

    assert_eq!(first.reason, StopReason::Breakpoint { address: 0x100 });

    assert!(session.last_watchpoint_hits().is_empty());

    let second = session.continue_execution(100);

    assert_eq!(second.reason, StopReason::Watchpoint);
    assert_eq!(session.pc(), 0x104);
}

#[test]
fn watchpoint_hi_change() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.write_reg(8, 42);

    // mthi $t0
    memory.write_word(0, 0x0100_0011);

    let mut session = DebuggerSession::new(cpu, memory);

    session.add_watchpoint(WatchTarget::Hi).unwrap();

    let result = session.continue_execution(1);

    assert_eq!(result.reason, StopReason::Watchpoint);

    assert_eq!(
        session.last_watchpoint_hits(),
        &[WatchpointHit {
            target: WatchTarget::Hi,
            before: 0,
            after: 42,
        }]
    );
}

#[test]
fn watchpoint_lo_change() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.write_reg(8, 42);

    // mtlo $t0
    memory.write_word(0, 0x0100_0013);

    let mut session = DebuggerSession::new(cpu, memory);

    session.add_watchpoint(WatchTarget::Lo).unwrap();

    let result = session.continue_execution(1);

    assert_eq!(result.reason, StopReason::Watchpoint);
    assert_eq!(session.cpu().read_lo(), 42);
}

#[test]
fn watchpoint_zero_step_budget() {
    let mut session = session();

    session.add_watchpoint(WatchTarget::Register(8)).unwrap();

    let result = session.continue_execution(0);

    assert_eq!(result.reason, StopReason::StepLimit);
    assert_eq!(session.steps(), 0);
    assert!(session.last_watchpoint_hits().is_empty());
}
