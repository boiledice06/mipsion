use crate::cpu::Cpu;
use crate::debugger::{Breakpoints, DebuggerSession};
use crate::memory::Memory;
use crate::runtime::StopReason;

fn session() -> DebuggerSession {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.write_pc(0x100);

    memory.write_word(0x100, 0x2408_002A);
    memory.write_word(0x104, 0x2509_0001);
    memory.write_word(0x108, 0x0109_5020);
    memory.write_word(0x10C, 0x0000_000C);

    DebuggerSession::new(cpu, memory)
}

#[test]
fn breakpoint_initially_empty() {
    let breakpoints = Breakpoints::new();

    assert!(breakpoints.is_empty());
    assert_eq!(breakpoints.len(), 0);
}

#[test]
fn breakpoint_add() {
    let mut breakpoints = Breakpoints::new();

    assert_eq!(breakpoints.add(0x100), Ok(true));
    assert!(breakpoints.contains(0x100));
    assert_eq!(breakpoints.len(), 1);
}

#[test]
fn breakpoint_duplicate() {
    let mut breakpoints = Breakpoints::new();

    assert_eq!(breakpoints.add(0x100), Ok(true));
    assert_eq!(breakpoints.add(0x100), Ok(false));
    assert_eq!(breakpoints.len(), 1);
}

#[test]
fn breakpoint_remove() {
    let mut breakpoints = Breakpoints::new();

    breakpoints.add(0x100).unwrap();

    assert!(breakpoints.remove(0x100));
    assert!(!breakpoints.contains(0x100));
    assert!(!breakpoints.remove(0x100));
}

#[test]
fn breakpoint_rejects_unaligned_address() {
    let mut breakpoints = Breakpoints::new();

    assert!(breakpoints.add(0x101).is_err());
    assert!(breakpoints.is_empty());
}

#[test]
fn breakpoint_sorted_iteration() {
    let mut breakpoints = Breakpoints::new();

    breakpoints.add(0x300).unwrap();
    breakpoints.add(0x100).unwrap();
    breakpoints.add(0x200).unwrap();

    assert_eq!(
        breakpoints.iter().collect::<Vec<_>>(),
        vec![0x100, 0x200, 0x300]
    );
}

#[test]
fn breakpoint_clear() {
    let mut breakpoints = Breakpoints::new();

    breakpoints.add(0x100).unwrap();
    breakpoints.add(0x200).unwrap();

    breakpoints.clear();

    assert!(breakpoints.is_empty());
}

#[test]
fn breakpoint_session_management() {
    let mut session = session();

    session.add_breakpoint(0x100).unwrap();

    assert!(session.has_breakpoint(0x100));
    assert_eq!(session.breakpoints().len(), 1);

    assert!(session.remove_breakpoint(0x100));

    assert!(!session.has_breakpoint(0x100));
}

#[test]
fn breakpoint_stops_before_instruction() {
    let mut session = session();

    session.add_breakpoint(0x104).unwrap();

    let result = session.continue_execution(100);

    assert_eq!(result.reason, StopReason::Breakpoint { address: 0x104 });

    assert_eq!(result.steps, 1);
    assert_eq!(session.pc(), 0x104);
    assert_eq!(session.cpu().read_reg(8), 42);
    assert_eq!(session.cpu().read_reg(9), 0);
}

#[test]
fn breakpoint_at_entry_point() {
    let mut session = session();

    session.add_breakpoint(0x100).unwrap();

    let result = session.continue_execution(100);

    assert_eq!(result.reason, StopReason::Breakpoint { address: 0x100 });

    assert_eq!(result.steps, 0);
    assert_eq!(session.steps(), 0);
    assert_eq!(session.cpu().read_reg(8), 0);
    assert_eq!(session.cpu().cp0().read_count(), 0);
}

#[test]
fn breakpoint_continue_resumes_execution() {
    let mut session = session();

    session.add_breakpoint(0x100).unwrap();

    session.continue_execution(100);

    let result = session.continue_execution(100);

    assert_eq!(
        result.reason,
        StopReason::Exception {
            code: 8,
            epc: 0x10C,
            in_delay_slot: false,
        }
    );

    assert_eq!(result.steps, 4);
    assert_eq!(session.cpu().read_reg(10), 85);
}

#[test]
fn breakpoint_multiple_addresses() {
    let mut session = session();

    session.add_breakpoint(0x104).unwrap();
    session.add_breakpoint(0x108).unwrap();

    let first = session.continue_execution(100);
    let second = session.continue_execution(100);

    assert_eq!(first.reason, StopReason::Breakpoint { address: 0x104 });

    assert_eq!(second.reason, StopReason::Breakpoint { address: 0x108 });

    assert_eq!(session.steps(), 2);
}

#[test]
fn breakpoint_step_ignores_breakpoint() {
    let mut session = session();

    session.add_breakpoint(0x100).unwrap();

    session.step().unwrap();

    assert_eq!(session.pc(), 0x104);
    assert_eq!(session.cpu().read_reg(8), 42);
}

#[test]
fn breakpoint_remove_after_hit() {
    let mut session = session();

    session.add_breakpoint(0x100).unwrap();
    session.continue_execution(100);

    session.remove_breakpoint(0x100);

    let result = session.continue_execution(1);

    assert_eq!(result.reason, StopReason::StepLimit);
    assert_eq!(session.pc(), 0x104);
}

#[test]
fn breakpoint_reset_preserves_breakpoints() {
    let mut session = session();

    session.add_breakpoint(0x104).unwrap();
    session.continue_execution(100);

    session.reset();

    assert!(session.has_breakpoint(0x104));
    assert_eq!(session.pc(), 0x100);
    assert_eq!(session.steps(), 0);
}

#[test]
fn breakpoint_reset_restores_entry_hit() {
    let mut session = session();

    session.add_breakpoint(0x100).unwrap();
    session.continue_execution(100);

    session.reset();

    let result = session.continue_execution(100);

    assert_eq!(result.reason, StopReason::Breakpoint { address: 0x100 });

    assert_eq!(result.steps, 0);
}

#[test]
fn breakpoint_hits_again_after_loop() {
    let mut memory = Memory::new(1024);

    // j 0
    memory.write_word(0, 0x0800_0000);
    memory.write_word(4, 0);

    let mut session = DebuggerSession::new(Cpu::new(), memory);

    session.add_breakpoint(0).unwrap();

    session.continue_execution(100);

    let result = session.continue_execution(100);

    assert_eq!(result.reason, StopReason::Breakpoint { address: 0 });

    assert_eq!(result.steps, 2);
    assert_eq!(session.pc(), 0);
}

#[test]
fn breakpoint_in_delay_slot() {
    let mut memory = Memory::new(1024);

    // beq $zero, $zero, 0x10
    memory.write_word(0, 0x1000_0003);
    memory.write_word(4, 0x2408_0001);

    let mut session = DebuggerSession::new(Cpu::new(), memory);

    session.add_breakpoint(4).unwrap();

    let result = session.continue_execution(100);

    assert_eq!(result.reason, StopReason::Breakpoint { address: 4 });

    assert_eq!(session.cpu().read_reg(8), 0);
    assert_eq!(session.cpu().read_next_pc(), 0x10);

    session.step().unwrap();

    assert_eq!(session.cpu().read_reg(8), 1);
    assert_eq!(session.pc(), 0x10);
}

#[test]
fn breakpoint_zero_step_budget() {
    let mut session = session();

    session.add_breakpoint(0x100).unwrap();
    session.continue_execution(100);

    let result = session.continue_execution(0);

    assert_eq!(result.reason, StopReason::StepLimit);
    assert_eq!(session.steps(), 0);

    let result = session.continue_execution(1);

    assert_eq!(result.steps, 1);
    assert_eq!(session.pc(), 0x104);
}

#[test]
fn breakpoint_clear_session() {
    let mut session = session();

    session.add_breakpoint(0x100).unwrap();
    session.add_breakpoint(0x104).unwrap();

    session.clear_breakpoints();

    assert!(session.breakpoints().is_empty());

    let result = session.continue_execution(2);

    assert_eq!(result.reason, StopReason::StepLimit);
    assert_eq!(session.pc(), 0x108);
}
