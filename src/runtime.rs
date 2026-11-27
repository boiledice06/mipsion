use crate::cpu::Cpu;
use crate::memory::Memory;
use crate::trace::{CpuSnapshot, TraceEvent};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    Halted,
    StepLimit,
    Exception {
        code: u32,
        epc: u32,
        in_delay_slot: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecutionConfig {
    pub max_steps: u64,
    pub stop_on_exception: bool,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            max_steps: 1_000_000,
            stop_on_exception: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecutionResult {
    pub steps: u64,
    pub final_pc: u32,
    pub reason: StopReason,
}

pub fn run(cpu: &mut Cpu, memory: &mut Memory, config: ExecutionConfig) -> ExecutionResult {
    run_internal(cpu, memory, config, None)
}

pub fn run_with_trace(
    cpu: &mut Cpu,
    memory: &mut Memory,
    config: ExecutionConfig,
    mut callback: impl FnMut(&TraceEvent),
) -> ExecutionResult {
    run_internal(cpu, memory, config, Some(&mut callback))
}

fn run_internal(
    cpu: &mut Cpu,
    memory: &mut Memory,
    config: ExecutionConfig,
    mut callback: Option<&mut dyn FnMut(&TraceEvent)>,
) -> ExecutionResult {
    let mut steps = 0;

    let reason = loop {
        if cpu.is_halted() {
            break StopReason::Halted;
        }

        if steps >= config.max_steps {
            break StopReason::StepLimit;
        }

        let previous_exception_count = cpu.exception_count();

        let snapshot = callback.is_some().then(|| CpuSnapshot::capture(cpu));

        cpu.step(memory);

        steps += 1;

        if let (Some(before), Some(callback)) = (snapshot, callback.as_mut()) {
            let event = TraceEvent::capture(steps, &before, cpu);

            callback(&event);
        }

        if config.stop_on_exception && cpu.exception_count() != previous_exception_count {
            break StopReason::Exception {
                code: cpu.cp0().exception_code(),
                epc: cpu.cp0().read_epc(),
                in_delay_slot: cpu.cp0().branch_delay(),
            };
        }
    };

    ExecutionResult {
        steps,
        final_pc: cpu.read_pc(),
        reason,
    }
}
