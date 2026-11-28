use crate::cpu::Cpu;
use crate::cpu::exception::Exception;
use crate::isa::disassembler::{disassemble, format_disassembly};
use crate::memory::Memory;
use crate::runtime::inspection::{dump_cp0, dump_fpu, dump_memory, dump_registers};

use crate::runtime::trace::{CpuSnapshot, TraceEvent};
use crate::runtime::{ExecutionConfig, ExecutionResult, run};

pub struct DebuggerSession {
    cpu: Cpu,
    memory: Memory,

    initial_cpu: Cpu,
    initial_memory: Memory,
    entry_point: u32,

    steps: u64,
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
        if self.cpu.is_halted() {
            return None;
        }

        let before = CpuSnapshot::capture(&self.cpu);

        self.cpu.step(&mut self.memory);

        self.steps = self.steps.saturating_add(1);

        Some(TraceEvent::capture(self.steps, &before, &self.cpu))
    }

    pub fn continue_execution(&mut self, max_steps: u64) -> ExecutionResult {
        let config = ExecutionConfig {
            max_steps,
            stop_on_exception: true,
        };

        let result = run(&mut self.cpu, &mut self.memory, config);

        self.steps = self.steps.saturating_add(result.steps);

        result
    }

    pub fn reset(&mut self) {
        self.cpu.clone_from(&self.initial_cpu);
        self.memory.clone_from(&self.initial_memory);

        self.steps = 0;
    }
}
