use crate::cpu::Cpu;
use crate::cpu::exception::Exception;
use crate::isa::disassembler::{disassemble, format_disassembly};
use crate::memory::Memory;
use crate::runtime::inspection::{dump_cp0, dump_fpu, dump_memory, dump_registers};

use crate::runtime::trace::{CpuSnapshot, TraceEvent};
use crate::runtime::{ExecutionConfig, ExecutionResult, StopReason, run_with_debug_hooks};

use super::breakpoint::Breakpoints;
use super::watchpoint::{WatchTarget, WatchpointHit, Watchpoints};

pub struct DebuggerSession {
    cpu: Cpu,
    memory: Memory,

    initial_cpu: Cpu,
    initial_memory: Memory,
    entry_point: u32,

    steps: u64,

    breakpoints: Breakpoints,
    last_breakpoint: Option<u32>,

    watchpoints: Watchpoints,
    last_watchpoint_hits: Vec<WatchpointHit>,
}

impl DebuggerSession {
    pub fn new(cpu: Cpu, memory: Memory) -> Self {
        let entry_point = cpu.read_pc();

        let initial_cpu = cpu.clone();
        let initial_memory = memory.clone();

        Self {
            cpu,
            memory,

            initial_cpu,
            initial_memory,
            entry_point,

            steps: 0,

            breakpoints: Breakpoints::new(),
            last_breakpoint: None,
            watchpoints: Watchpoints::new(),
            last_watchpoint_hits: Vec::new(),
        }
    }

    pub fn cpu(&self) -> &Cpu {
        &self.cpu
    }

    pub fn memory(&self) -> &Memory {
        &self.memory
    }

    pub fn pc(&self) -> u32 {
        self.cpu.read_pc()
    }

    pub fn entry_point(&self) -> u32 {
        self.entry_point
    }

    pub fn steps(&self) -> u64 {
        self.steps
    }

    pub fn is_halted(&self) -> bool {
        self.cpu.is_halted()
    }

    pub fn instruction_at(&self, address: u32) -> Result<u32, Exception> {
        self.cpu.peek_instruction(&self.memory, address)
    }

    pub fn current_instruction(&self) -> Result<u32, Exception> {
        self.instruction_at(self.pc())
    }

    pub fn disassemble_at(&self, address: u32) -> Result<String, Exception> {
        let word = self.instruction_at(address)?;

        Ok(disassemble(word, address))
    }

    pub fn current_disassembly(&self) -> Result<String, Exception> {
        self.disassemble_at(self.pc())
    }

    pub fn disassembly_line(&self, address: u32) -> Result<String, Exception> {
        let word = self.instruction_at(address)?;

        Ok(format_disassembly(word, address))
    }

    pub fn registers(&self) -> String {
        dump_registers(&self.cpu)
    }

    pub fn cp0_registers(&self) -> String {
        dump_cp0(&self.cpu)
    }

    pub fn fpu_registers(&self) -> String {
        dump_fpu(&self.cpu)
    }

    pub fn memory_dump(&self, physical_address: u32, size: usize) -> Result<String, String> {
        dump_memory(&self.memory, physical_address, size)
    }

    pub fn step(&mut self) -> Option<TraceEvent> {
        self.last_watchpoint_hits.clear();

        if self.cpu.is_halted() {
            return None;
        }

        self.last_breakpoint = None;

        let before = CpuSnapshot::capture(&self.cpu);

        self.cpu.step(&mut self.memory);

        self.steps = self.steps.saturating_add(1);

        self.last_watchpoint_hits = self.watchpoints.detect_changes(&self.cpu, &self.memory);

        Some(TraceEvent::capture(self.steps, &before, &self.cpu))
    }

    pub fn continue_execution(&mut self, max_steps: u64) -> ExecutionResult {
        let config = ExecutionConfig {
            max_steps,
            stop_on_exception: true,
        };

        self.last_watchpoint_hits.clear();

        let mut skip_breakpoint = self.last_breakpoint == Some(self.pc());

        let breakpoints = &self.breakpoints;
        let watchpoints = &mut self.watchpoints;
        let hits = &mut self.last_watchpoint_hits;

        let result = run_with_debug_hooks(
            &mut self.cpu,
            &mut self.memory,
            config,
            |pc| {
                if skip_breakpoint {
                    skip_breakpoint = false;
                    return false;
                }

                breakpoints.contains(pc)
            },
            |cpu, memory| {
                if watchpoints.is_empty() {
                    return false;
                }

                *hits = watchpoints.detect_changes(cpu, memory);

                !hits.is_empty()
            },
        );

        self.steps = self.steps.saturating_add(result.steps);

        match result.reason {
            StopReason::Breakpoint { address } => {
                self.last_breakpoint = Some(address);
            }

            _ if result.steps > 0 => {
                self.last_breakpoint = None;
            }

            _ => {}
        }

        result
    }

    pub fn reset(&mut self) {
        self.cpu.clone_from(&self.initial_cpu);
        self.memory.clone_from(&self.initial_memory);

        self.steps = 0;
        self.last_breakpoint = None;

        self.last_watchpoint_hits.clear();

        self.watchpoints.refresh(&self.cpu, &self.memory);
    }

    pub fn add_breakpoint(&mut self, address: u32) -> Result<bool, String> {
        self.breakpoints.add(address)
    }

    pub fn remove_breakpoint(&mut self, address: u32) -> bool {
        if self.last_breakpoint == Some(address) {
            self.last_breakpoint = None;
        }

        self.breakpoints.remove(address)
    }

    pub fn has_breakpoint(&self, address: u32) -> bool {
        self.breakpoints.contains(address)
    }

    pub fn breakpoints(&self) -> &Breakpoints {
        &self.breakpoints
    }

    pub fn clear_breakpoints(&mut self) {
        self.breakpoints.clear();
        self.last_breakpoint = None;
    }

    pub fn add_watchpoint(&mut self, target: WatchTarget) -> Result<bool, String> {
        self.watchpoints.add(target, &self.cpu, &self.memory)
    }

    pub fn remove_watchpoint(&mut self, target: WatchTarget) -> bool {
        self.watchpoints.remove(target)
    }

    pub fn has_watchpoint(&self, target: WatchTarget) -> bool {
        self.watchpoints.contains(target)
    }

    pub fn watchpoints(&self) -> &Watchpoints {
        &self.watchpoints
    }

    pub fn clear_watchpoints(&mut self) {
        self.watchpoints.clear();
        self.last_watchpoint_hits.clear();
    }

    pub fn last_watchpoint_hits(&self) -> &[WatchpointHit] {
        &self.last_watchpoint_hits
    }
}
