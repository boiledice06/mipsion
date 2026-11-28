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
