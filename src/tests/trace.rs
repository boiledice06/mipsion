use super::*;

#[test]
fn trace_last_instruction() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    memory.write_word(0, 0x2408_002A);

    assert_eq!(cpu.last_instruction(), None);

    cpu.step(&mut memory);

    assert_eq!(cpu.last_instruction(), Some(0x2408_002A),);

    cpu.step(&mut memory);

    assert_eq!(cpu.last_instruction(), None);
    assert_eq!(cpu.cp0().exception_code(), 6);
}

#[test]
fn trace_register_change() {
    use crate::trace::{CpuSnapshot, TraceEvent};

    let mut cpu = Cpu::new();
    let mut memory = Memory::new(64);

    memory.write_word(0, i(9, 0, 8, 42));

    let before = CpuSnapshot::capture(&cpu);

    cpu.step(&mut memory);

    let event = TraceEvent::capture(1, &before, &cpu);

    assert_eq!(event.step, 1);
    assert_eq!(event.pc, 0);
    assert_eq!(event.next_pc, 4);

    assert_eq!(event.instruction, Some(0x2408_002A),);

    assert_eq!(event.mnemonic, Some("addiu"));
    assert_eq!(event.register_changes.len(), 1);

    let change = event.register_changes[0];

    assert_eq!(change.index, 8);
    assert_eq!(change.before, 0);
    assert_eq!(change.after, 42);
    assert!(event.exception.is_none());
}

#[test]
fn trace_unchanged_registers() {
    use crate::trace::{CpuSnapshot, TraceEvent};

    let mut cpu = Cpu::new();
    let mut memory = Memory::new(64);

    memory.write_word(0, 0);

    let before = CpuSnapshot::capture(&cpu);

    cpu.step(&mut memory);

    let event = TraceEvent::capture(1, &before, &cpu);

    assert!(event.register_changes.is_empty());
    assert!(event.hi_change.is_none());
    assert!(event.lo_change.is_none());
}

#[test]
fn trace_hilo_changes() {
    use crate::trace::{CpuSnapshot, TraceEvent};

    let mut cpu = Cpu::new();

    let before = CpuSnapshot::capture(&cpu);

    cpu.write_hi(0x1234);
    cpu.write_lo(0x5678);

    let event = TraceEvent::capture(1, &before, &cpu);

    assert_eq!(event.hi_change.unwrap().before, 0);
    assert_eq!(event.hi_change.unwrap().after, 0x1234);

    assert_eq!(event.lo_change.unwrap().before, 0);
    assert_eq!(event.lo_change.unwrap().after, 0x5678);
}

#[test]
fn trace_syscall_exception() {
    use crate::trace::{CpuSnapshot, TraceEvent};

    let mut cpu = Cpu::new();
    let mut memory = Memory::new(64);

    memory.write_word(0, 0x0000_000C);

    let before = CpuSnapshot::capture(&cpu);

    cpu.step(&mut memory);

    let event = TraceEvent::capture(1, &before, &cpu);

    assert_eq!(event.mnemonic, Some("syscall"));
    assert_eq!(event.next_pc, 0x8000_0180);

    let exception = event.exception.unwrap();

    assert_eq!(exception.code, 8);
    assert_eq!(exception.epc, 0);
    assert!(!exception.in_delay_slot);
}

#[test]
fn trace_fetch_failure() {
    use crate::trace::{CpuSnapshot, TraceEvent};

    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    cpu.write_pc(4);

    let before = CpuSnapshot::capture(&cpu);

    cpu.step(&mut memory);

    let event = TraceEvent::capture(1, &before, &cpu);

    assert_eq!(event.instruction, None);
    assert_eq!(event.mnemonic, None);
    assert_eq!(event.exception.unwrap().code, 6);
}

#[test]
fn trace_delay_slot_exception() {
    use crate::runtime::{ExecutionConfig, run_with_trace};

    let mut cpu = Cpu::new();
    let mut memory = Memory::new(64);

    memory.write_word(0, i(4, 0, 0, 1));
    memory.write_word(4, 0x0000_000C);

    let mut events = Vec::new();

    run_with_trace(&mut cpu, &mut memory, ExecutionConfig::default(), |event| {
        events.push(event.clone())
    });

    assert_eq!(events.len(), 2);
    assert_eq!(events[1].pc, 4);

    let exception = events[1].exception.unwrap();

    assert_eq!(exception.code, 8);
    assert_eq!(exception.epc, 0);
    assert!(exception.in_delay_slot);
}

#[test]
fn trace_output_format() {
    use crate::trace::{CpuSnapshot, TraceEvent, format_trace};

    let mut cpu = Cpu::new();
    let mut memory = Memory::new(64);

    memory.write_word(0, i(9, 0, 8, 42));

    let before = CpuSnapshot::capture(&cpu);

    cpu.step(&mut memory);

    let event = TraceEvent::capture(1, &before, &cpu);
    let output = format_trace(&event);

    assert!(output.contains("[000001]"));
    assert!(output.contains("PC=00000000"));
    assert!(output.contains("2408002A"));
    assert!(output.contains("addiu"));
    assert!(output.contains("$t0: 00000000 -> 0000002A"));
    assert!(output.contains("PC -> 00000004"));
}
