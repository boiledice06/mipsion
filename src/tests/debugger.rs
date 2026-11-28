use crate::cpu::Cpu;
use crate::cpu::exception::Exception;
use crate::debugger::DebuggerSession;
use crate::memory::Memory;

fn session() -> DebuggerSession {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.write_pc(0x100);

    memory.write_word(0x100, 0x2408_002A);
    memory.write_word(0x104, 0x2509_0001);
    memory.write_word(0x108, 0x0000_000C);

    DebuggerSession::new(cpu, memory)
}

#[test]
fn debugger_initial_pc() {
    let session = session();

    assert_eq!(session.pc(), 0x100);
    assert_eq!(session.entry_point(), 0x100);
}

#[test]
fn debugger_initial_steps() {
    assert_eq!(session().steps(), 0);
}

#[test]
fn debugger_initial_halted_state() {
    assert!(!session().is_halted());
}

#[test]
fn debugger_current_instruction() {
    assert_eq!(session().current_instruction().ok(), Some(0x2408_002A));
}

#[test]
fn debugger_instruction_at_address() {
    assert_eq!(session().instruction_at(0x104).ok(), Some(0x2509_0001));
}

#[test]
fn debugger_current_disassembly() {
    assert_eq!(
        session().current_disassembly().ok().as_deref(),
        Some("addiu $t0, $zero, 42")
    );
}

#[test]
fn debugger_disassembly_at_address() {
    assert_eq!(
        session().disassemble_at(0x104).ok().as_deref(),
        Some("addiu $t1, $t0, 1")
    );
}

#[test]
fn debugger_formatted_disassembly() {
    assert_eq!(
        session().disassembly_line(0x100).ok().as_deref(),
        Some("00000100  2408002A  addiu $t0, $zero, 42")
    );
}

#[test]
fn debugger_peek_preserves_cpu_state() {
    let session = session();

    let pc = session.cpu().read_pc();
    let next_pc = session.cpu().read_next_pc();
    let count = session.cpu().exception_count();
    let cp0_count = session.cpu().cp0().read_count();

    assert!(session.current_instruction().is_ok());
    assert!(session.current_disassembly().is_ok());

    assert_eq!(session.cpu().read_pc(), pc);
    assert_eq!(session.cpu().read_next_pc(), next_pc);
    assert_eq!(session.cpu().exception_count(), count);
    assert_eq!(session.cpu().cp0().read_count(), cp0_count);
    assert_eq!(session.cpu().last_instruction(), None);
    assert_eq!(session.steps(), 0);
}

#[test]
fn debugger_rejects_unaligned_address() {
    let session = session();

    assert!(matches!(
        session.instruction_at(0x101),
        Err(Exception::AddressErrorLoad)
    ));

    assert_eq!(session.pc(), 0x100);
    assert_eq!(session.cpu().exception_count(), 0);
}

#[test]
fn debugger_rejects_out_of_bounds_address() {
    let session = session();

    assert!(matches!(
        session.instruction_at(0x400),
        Err(Exception::InstructionBusError)
    ));

    assert_eq!(session.pc(), 0x100);
    assert_eq!(session.cpu().exception_count(), 0);
}

#[test]
fn debugger_kernel_segment_alias() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.write_pc(0x8000_0100);

    memory.write_word(0x100, 0x2408_002A);

    let session = DebuggerSession::new(cpu, memory);

    assert_eq!(session.current_instruction().ok(), Some(0x2408_002A));

    assert_eq!(session.instruction_at(0xA000_0100).ok(), Some(0x2408_002A));

    assert_eq!(session.pc(), 0x8000_0100);
}

#[test]
fn debugger_register_inspection() {
    let session = session();

    assert!(session.registers().contains("$zero"));
    assert!(session.registers().contains("00000100"));
    assert!(session.cp0_registers().contains("Status"));
    assert!(session.fpu_registers().contains("FCSR"));
}

#[test]
fn debugger_memory_inspection() {
    let session = session();

    let output = session.memory_dump(0x100, 4).unwrap();

    assert!(output.contains("2A 00 08 24"));
    assert!(session.memory_dump(0x400, 4).is_err());
}

#[test]
fn debugger_memory_clone_is_independent() {
    let mut memory = Memory::new(16);

    memory.write_word(0, 0x1234_5678);

    let snapshot = memory.clone();

    memory.write_word(0, 0xDEAD_BEEF);

    assert_eq!(snapshot.read_word(0), 0x1234_5678);
    assert_eq!(memory.read_word(0), 0xDEAD_BEEF);
}

#[test]
fn debugger_step_executes_instruction() {
    let mut session = session();

    let event = session.step().unwrap();

    assert_eq!(session.cpu().read_reg(8), 42);
    assert_eq!(session.pc(), 0x104);
    assert_eq!(session.steps(), 1);

    assert_eq!(event.pc, 0x100);
    assert_eq!(event.next_pc, 0x104);
    assert_eq!(event.instruction, Some(0x2408_002A));
}

#[test]
fn debugger_step_reports_register_change() {
    let mut session = session();

    let event = session.step().unwrap();

    assert_eq!(event.register_changes.len(), 1);

    let change = &event.register_changes[0];

    assert_eq!(change.index, 8);
    assert_eq!(change.before, 0);
    assert_eq!(change.after, 42);
}

#[test]
fn debugger_multiple_steps() {
    let mut session = session();

    session.step().unwrap();
    let event = session.step().unwrap();

    assert_eq!(session.cpu().read_reg(8), 42);
    assert_eq!(session.cpu().read_reg(9), 43);
    assert_eq!(session.pc(), 0x108);
    assert_eq!(session.steps(), 2);
    assert_eq!(event.step, 2);
}

#[test]
fn debugger_step_reports_exception() {
    let mut session = session();

    session.step().unwrap();
    session.step().unwrap();

    let event = session.step().unwrap();

    let exception = event.exception.unwrap();

    assert_eq!(exception.code, 8);
    assert_eq!(exception.epc, 0x108);
    assert!(!exception.in_delay_slot);

    assert_eq!(session.steps(), 3);
    assert_eq!(session.cpu().exception_count(), 1);
}

#[test]
fn debugger_step_preserves_delay_slot() {
    let mut memory = Memory::new(1024);

    // beq $zero, $zero, 0x10
    memory.write_word(0, 0x1000_0003);

    // Delay slot
    memory.write_word(4, 0x2408_0001);

    // Branch target
    memory.write_word(0x10, 0x2409_0002);

    let mut session = DebuggerSession::new(Cpu::new(), memory);

    session.step().unwrap();

    assert_eq!(session.pc(), 4);
    assert_eq!(session.cpu().read_next_pc(), 0x10);

    session.step().unwrap();

    assert_eq!(session.cpu().read_reg(8), 1);
    assert_eq!(session.pc(), 0x10);

    session.step().unwrap();

    assert_eq!(session.cpu().read_reg(9), 2);
    assert_eq!(session.steps(), 3);
}

#[test]
fn debugger_continue_step_limit() {
    use crate::runtime::StopReason;

    let mut session = session();

    let result = session.continue_execution(2);

    assert_eq!(result.reason, StopReason::StepLimit);
    assert_eq!(result.steps, 2);
    assert_eq!(result.final_pc, 0x108);
    assert_eq!(session.steps(), 2);
}

#[test]
fn debugger_continue_stops_on_exception() {
    use crate::runtime::StopReason;

    let mut session = session();

    let result = session.continue_execution(100);

    assert_eq!(
        result.reason,
        StopReason::Exception {
            code: 8,
            epc: 0x108,
            in_delay_slot: false,
        }
    );

    assert_eq!(result.steps, 3);
    assert_eq!(session.steps(), 3);
}

#[test]
fn debugger_continue_accumulates_steps() {
    let mut session = session();

    session.step().unwrap();

    let result = session.continue_execution(1);

    assert_eq!(result.steps, 1);
    assert_eq!(session.steps(), 2);
    assert_eq!(session.pc(), 0x108);
}

#[test]
fn debugger_continue_zero_steps() {
    use crate::runtime::StopReason;

    let mut session = session();

    let result = session.continue_execution(0);

    assert_eq!(result.reason, StopReason::StepLimit);
    assert_eq!(result.steps, 0);
    assert_eq!(session.steps(), 0);
    assert_eq!(session.pc(), 0x100);
    assert_eq!(session.cpu().cp0().read_count(), 0);
}

#[test]
fn debugger_continue_bounds_infinite_loop() {
    use crate::runtime::StopReason;

    let mut memory = Memory::new(1024);

    // j 0
    memory.write_word(0, 0x0800_0000);

    // nop (delay slot)
    memory.write_word(4, 0);

    let mut session = DebuggerSession::new(Cpu::new(), memory);

    let result = session.continue_execution(1000);

    assert_eq!(result.reason, StopReason::StepLimit);
    assert_eq!(result.steps, 1000);
    assert_eq!(session.steps(), 1000);
    assert_eq!(session.pc(), 0);
}

#[test]
fn debugger_reset_restores_pc() {
    let mut session = session();

    session.continue_execution(2);

    session.reset();

    assert_eq!(session.pc(), 0x100);
    assert_eq!(session.cpu().read_next_pc(), 0x104);
}

#[test]
fn debugger_reset_restores_registers() {
    let mut session = session();

    session.continue_execution(2);

    session.reset();

    assert_eq!(session.cpu().read_reg(8), 0);
    assert_eq!(session.cpu().read_reg(9), 0);
}

#[test]
fn debugger_reset_restores_step_counter() {
    let mut session = session();

    session.continue_execution(2);

    session.reset();

    assert_eq!(session.steps(), 0);
}

#[test]
fn debugger_reset_restores_exception_state() {
    let mut session = session();

    session.continue_execution(100);

    assert_eq!(session.cpu().exception_count(), 1);

    session.reset();

    assert_eq!(session.cpu().exception_count(), 0);
    assert_eq!(session.cpu().cp0().read_count(), 0);
    assert!(!session.cpu().cp0().exception_level());
    assert_eq!(session.cpu().last_instruction(), None);
    assert_eq!(session.pc(), 0x100);
}

#[test]
fn debugger_reset_restores_memory() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.write_reg(8, 0xDEAD_BEEF);

    // sw $t0, 256($zero)
    memory.write_word(0, 0xAC08_0100);
    memory.write_word(0x100, 0x1234_5678);

    let mut session = DebuggerSession::new(cpu, memory);

    session.step().unwrap();

    assert_eq!(session.memory().read_word(0x100), 0xDEAD_BEEF);

    session.reset();

    assert_eq!(session.memory().read_word(0x100), 0x1234_5678);

    assert_eq!(session.cpu().read_reg(8), 0xDEAD_BEEF);
}

#[test]
fn debugger_reset_preserves_initial_cpu_configuration() {
    let mut cpu = Cpu::new();
    let memory = Memory::new(1024);

    cpu.write_reg(29, 0x300);
    cpu.write_hi(0x1234);
    cpu.write_lo(0x5678);

    cpu.cp1_mut().write_word(0, 0x3F80_0000);

    cpu.set_reservation(0x100);

    let mut session = DebuggerSession::new(cpu, memory);

    session.step().unwrap();

    session.reset();

    assert_eq!(session.cpu().read_reg(29), 0x300);
    assert_eq!(session.cpu().read_hi(), 0x1234);
    assert_eq!(session.cpu().read_lo(), 0x5678);

    assert_eq!(session.cpu().cp1().read_word(0), 0x3F80_0000);

    assert!(session.cpu().check_reservation(0x100));
}

#[test]
fn debugger_reset_restores_tlb_mode() {
    let mut cpu = Cpu::new_with_tlb();
    let memory = Memory::new(1024);

    cpu.write_pc(0x4000);

    let mut session = DebuggerSession::new(cpu, memory);

    assert_eq!(session.current_instruction(), Err(Exception::TlbRefillLoad));

    session.step().unwrap();
    session.reset();

    assert_eq!(session.pc(), 0x4000);

    assert_eq!(session.current_instruction(), Err(Exception::TlbRefillLoad));

    assert_eq!(session.cpu().exception_count(), 0);
}

#[test]
fn debugger_reset_execution_is_reproducible() {
    let mut session = session();

    let first = session.step().unwrap();

    session.reset();

    let second = session.step().unwrap();

    assert_eq!(first, second);
    assert_eq!(session.steps(), 1);
}
