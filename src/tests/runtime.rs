use super::*;

#[test]
fn runtime_executes_program() {
    use crate::runtime::{ExecutionConfig, StopReason, run};

    let mut cpu = Cpu::new();
    let mut memory = Memory::new(64);

    memory.write_word(0, i(9, 0, 8, 42));
    memory.write_word(4, i(9, 8, 9, 1));

    let result = run(
        &mut cpu,
        &mut memory,
        ExecutionConfig {
            max_steps: 2,
            stop_on_exception: true,
        },
    );

    assert_eq!(cpu.read_reg(8), 42);
    assert_eq!(cpu.read_reg(9), 43);

    assert_eq!(result.steps, 2);
    assert_eq!(result.final_pc, 8);
    assert_eq!(result.reason, StopReason::StepLimit);
}

#[test]
fn runtime_zero_step_limit() {
    use crate::runtime::{ExecutionConfig, StopReason, run};

    let mut cpu = Cpu::new();
    let mut memory = Memory::new(64);

    memory.write_word(0, i(9, 0, 8, 42));

    let result = run(
        &mut cpu,
        &mut memory,
        ExecutionConfig {
            max_steps: 0,
            stop_on_exception: true,
        },
    );

    assert_eq!(result.steps, 0);
    assert_eq!(result.final_pc, 0);
    assert_eq!(result.reason, StopReason::StepLimit);
    assert_eq!(cpu.read_reg(8), 0);
}

#[test]
fn runtime_infinite_loop_step_limit() {
    use crate::runtime::{ExecutionConfig, StopReason, run};

    let mut cpu = Cpu::new();
    let mut memory = Memory::new(64);

    // J 0
    memory.write_word(0, 0x0800_0000);

    // NOP (delay slot)
    memory.write_word(4, 0);

    let result = run(
        &mut cpu,
        &mut memory,
        ExecutionConfig {
            max_steps: 100,
            stop_on_exception: true,
        },
    );

    assert_eq!(result.steps, 100);
    assert_eq!(result.final_pc, 0);
    assert_eq!(result.reason, StopReason::StepLimit);
    assert_eq!(cpu.exception_count(), 0);
}

#[test]
fn runtime_nonzero_entry_point() {
    use crate::runtime::{ExecutionConfig, StopReason, run};

    let mut cpu = Cpu::new();
    let mut memory = Memory::new(64);

    memory.write_word(16, i(9, 0, 8, 100));
    cpu.write_pc(16);

    let result = run(
        &mut cpu,
        &mut memory,
        ExecutionConfig {
            max_steps: 1,
            stop_on_exception: true,
        },
    );

    assert_eq!(cpu.read_reg(8), 100);
    assert_eq!(result.steps, 1);
    assert_eq!(result.final_pc, 20);
    assert_eq!(result.reason, StopReason::StepLimit);
}

#[test]
fn runtime_stops_on_syscall() {
    use crate::runtime::{ExecutionConfig, StopReason, run};

    let mut cpu = Cpu::new();
    let mut memory = Memory::new(64);

    memory.write_word(0, 0x0000_000C);

    let result = run(&mut cpu, &mut memory, ExecutionConfig::default());

    assert_eq!(result.steps, 1);
    assert_eq!(result.final_pc, 0x8000_0180);

    assert_eq!(
        result.reason,
        StopReason::Exception {
            code: 8,
            epc: 0,
            in_delay_slot: false,
        }
    );
}

#[test]
fn runtime_stops_on_instruction_bus_error() {
    use crate::runtime::{ExecutionConfig, StopReason, run};

    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    memory.write_word(0, 0);

    let result = run(&mut cpu, &mut memory, ExecutionConfig::default());

    assert_eq!(result.steps, 2);

    assert_eq!(
        result.reason,
        StopReason::Exception {
            code: 6,
            epc: 4,
            in_delay_slot: false,
        }
    );
}

#[test]
fn runtime_exception_in_delay_slot() {
    use crate::runtime::{ExecutionConfig, StopReason, run};

    let mut cpu = Cpu::new();
    let mut memory = Memory::new(64);

    memory.write_word(0, i(4, 0, 0, 1));
    memory.write_word(4, 0x0000_000C);

    let result = run(&mut cpu, &mut memory, ExecutionConfig::default());

    assert_eq!(result.steps, 2);

    assert_eq!(
        result.reason,
        StopReason::Exception {
            code: 8,
            epc: 0,
            in_delay_slot: true,
        }
    );
}

#[test]
fn runtime_continues_after_exception() {
    use crate::runtime::{ExecutionConfig, StopReason, run};

    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    // SYSCALL
    memory.write_word(0, 0x0000_000C);

    // Exception handler
    memory.write_word(0x180, i(9, 0, 8, 42));

    let result = run(
        &mut cpu,
        &mut memory,
        ExecutionConfig {
            max_steps: 2,
            stop_on_exception: false,
        },
    );

    assert_eq!(result.steps, 2);
    assert_eq!(result.reason, StopReason::StepLimit);
    assert_eq!(cpu.read_reg(8), 42);
    assert_eq!(cpu.exception_count(), 1);
    assert_eq!(result.final_pc, 0x8000_0184);
}

#[test]
fn runtime_detects_nested_exception() {
    use crate::runtime::{ExecutionConfig, StopReason, run};

    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    memory.write_word(0, 0x0000_000C);
    memory.write_word(0x180, 0x0000_000D);

    cpu.step(&mut memory);

    assert!(cpu.cp0().exception_level());
    assert_eq!(cpu.exception_count(), 1);

    let result = run(&mut cpu, &mut memory, ExecutionConfig::default());

    assert_eq!(result.steps, 1);
    assert_eq!(cpu.exception_count(), 2);

    assert_eq!(
        result.reason,
        StopReason::Exception {
            code: 9,
            epc: 0,
            in_delay_slot: false,
        }
    );
}

#[test]
fn runtime_raw_binary_integration() {
    use crate::loader::load_raw;
    use crate::runtime::{ExecutionConfig, StopReason, run};

    let mut cpu = Cpu::new();
    let mut memory = Memory::new(64);

    let bytes = [
        0x2A, 0x00, 0x08, 0x24, 0x01, 0x00, 0x09, 0x25, 0x0C, 0x00, 0x00, 0x00,
    ];

    load_raw(&mut memory, 16, &bytes).unwrap();

    cpu.write_pc(16);

    let result = run(&mut cpu, &mut memory, ExecutionConfig::default());

    assert_eq!(cpu.read_reg(8), 42);
    assert_eq!(cpu.read_reg(9), 43);
    assert_eq!(result.steps, 3);

    assert_eq!(
        result.reason,
        StopReason::Exception {
            code: 8,
            epc: 24,
            in_delay_slot: false,
        }
    );
}

#[test]
fn runtime_trace_event_sequence() {
    use crate::runtime::{ExecutionConfig, run_with_trace};

    let mut cpu = Cpu::new();
    let mut memory = Memory::new(64);

    memory.write_word(0, i(9, 0, 8, 42));
    memory.write_word(4, i(9, 8, 9, 1));
    memory.write_word(8, 0x0000_000C);

    let mut events = Vec::new();

    let result = run_with_trace(&mut cpu, &mut memory, ExecutionConfig::default(), |event| {
        events.push(event.clone())
    });

    assert_eq!(result.steps, 3);
    assert_eq!(events.len(), 3);

    for (index, event) in events.iter().enumerate() {
        assert_eq!(event.step, index as u64 + 1);
        assert_eq!(event.pc, index as u32 * 4);
    }

    assert_eq!(cpu.read_reg(8), 42);
    assert_eq!(cpu.read_reg(9), 43);
    assert_eq!(events[2].exception.unwrap().code, 8);
}

#[test]
fn runtime_trace_does_not_change_execution() {
    use crate::runtime::{ExecutionConfig, run, run_with_trace};
    use crate::trace::CpuSnapshot;

    let mut normal_cpu = Cpu::new();
    let mut traced_cpu = Cpu::new();

    let mut normal_memory = Memory::new(128);
    let mut traced_memory = Memory::new(128);

    let words = [
        i(9, 0, 8, 42),
        i(9, 8, 9, 1),
        i(0x2B, 0, 9, 64),
        0x0000_000C,
    ];

    for (index, &word) in words.iter().enumerate() {
        normal_memory.write_word(index as u32 * 4, word);
        traced_memory.write_word(index as u32 * 4, word);
    }

    let config = ExecutionConfig::default();

    let normal_result = run(&mut normal_cpu, &mut normal_memory, config);

    let traced_result = run_with_trace(&mut traced_cpu, &mut traced_memory, config, |_| {});

    assert_eq!(normal_result, traced_result);

    assert_eq!(
        CpuSnapshot::capture(&normal_cpu),
        CpuSnapshot::capture(&traced_cpu),
    );

    assert_eq!(normal_cpu.cp0().read_cause(), traced_cpu.cp0().read_cause(),);

    for address in 0..128 {
        assert_eq!(
            normal_memory.read_byte(address),
            traced_memory.read_byte(address),
        );
    }
}
