pub mod inspection;
pub mod trace;

use crate::cpu::Cpu;
use crate::memory::Memory;
use crate::trace::{CpuSnapshot, TraceEvent};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    Halted,
    StepLimit,

    Breakpoint {
        address: u32,
    },

    Watchpoint,

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
type TraceCallback<'a> = dyn FnMut(&TraceEvent) + 'a;

type StopCondition<'a> = dyn FnMut(u32) -> bool + 'a;

type PostStepCondition<'a> = dyn FnMut(&Cpu, &Memory) -> bool + 'a;

#[derive(Default)]
struct ExecutionHooks<'a> {
    trace: Option<&'a mut TraceCallback<'a>>,
    before_step: Option<&'a mut StopCondition<'a>>,
    after_step: Option<&'a mut PostStepCondition<'a>>,
}

pub fn run(cpu: &mut Cpu, memory: &mut Memory, config: ExecutionConfig) -> ExecutionResult {
    run_internal(cpu, memory, config, ExecutionHooks::default())
}

pub fn run_with_trace(
    cpu: &mut Cpu,
    memory: &mut Memory,
    config: ExecutionConfig,
    mut callback: impl FnMut(&TraceEvent),
) -> ExecutionResult {
    run_internal(
        cpu,
        memory,
        config,
        ExecutionHooks {
            trace: Some(&mut callback),
            ..ExecutionHooks::default()
        },
    )
}

pub fn run_with_stop_condition(
    cpu: &mut Cpu,
    memory: &mut Memory,
    config: ExecutionConfig,
    mut should_stop: impl FnMut(u32) -> bool,
) -> ExecutionResult {
    run_internal(
        cpu,
        memory,
        config,
        ExecutionHooks {
            before_step: Some(&mut should_stop),
            ..ExecutionHooks::default()
        },
    )
}

pub fn run_with_debug_hooks(
    cpu: &mut Cpu,
    memory: &mut Memory,
    config: ExecutionConfig,
    mut before_step: impl FnMut(u32) -> bool,
    mut after_step: impl FnMut(&Cpu, &Memory) -> bool,
) -> ExecutionResult {
    run_internal(
        cpu,
        memory,
        config,
        ExecutionHooks {
            before_step: Some(&mut before_step),
            after_step: Some(&mut after_step),
            ..ExecutionHooks::default()
        },
    )
}

fn run_internal(
    cpu: &mut Cpu,
    memory: &mut Memory,
    config: ExecutionConfig,
    mut hooks: ExecutionHooks<'_>,
) -> ExecutionResult {
    let mut steps = 0;

    let reason = loop {
        if cpu.is_halted() {
            break StopReason::Halted;
        }

        if steps >= config.max_steps {
            break StopReason::StepLimit;
        }

        let pc = cpu.read_pc();

        if let Some(condition) = hooks.before_step.as_mut()
            && condition(pc)
        {
            break StopReason::Breakpoint { address: pc };
        }

        let previous_exception_count = cpu.exception_count();

        let snapshot = hooks.trace.is_some().then(|| CpuSnapshot::capture(cpu));

        cpu.step(memory);

        steps += 1;

        if let (Some(before), Some(callback)) = (snapshot, hooks.trace.as_mut()) {
            let event = TraceEvent::capture(steps, &before, cpu);

            callback(&event);
        }

        let watchpoint_hit = hooks
            .after_step
            .as_mut()
            .is_some_and(|condition| condition(cpu, memory));

        if config.stop_on_exception && cpu.exception_count() != previous_exception_count {
            break StopReason::Exception {
                code: cpu.cp0().exception_code(),
                epc: cpu.cp0().read_epc(),
                in_delay_slot: cpu.cp0().branch_delay(),
            };
        }

        if watchpoint_hit {
            break StopReason::Watchpoint;
        }
    };

    ExecutionResult {
        steps,
        final_pc: cpu.read_pc(),
        reason,
    }
}
