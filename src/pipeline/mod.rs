//! In-order, single-issue pipeline with separate architectural retirement.

mod stages;
mod state;
mod trace;

pub use trace::{CycleEvent, PipelineRegisters, Stall, Writeback};

use crate::{cpu::Cpu, memory::Memory};
use stages::{decode_stage, execute, fetch, memory_stage, write_store};
use state::{Action, Executed, Instruction, Issued, Prepared};

/// Architectural writes are visible at WB. Latches are absent when they
/// hold a bubble. The functional engine remains available through Cpu::step.
pub struct Pipeline {
    cpu: Cpu,
    memory: Memory,
    if_id: Option<Instruction>,
    id_ex: Option<Issued>,
    ex_mem: Option<Executed>,
    mem_wb: Option<Prepared>,
    fetch_pc: u32,
    fetch_enabled: bool,
    stopped: bool,
    cycles: u64,
    retired: u64,
}

impl Pipeline {
    pub fn new(cpu: Cpu, memory: Memory) -> Self {
        let fetch_pc = cpu.read_pc();
        Self {
            cpu,
            memory,
            if_id: None,
            id_ex: None,
            ex_mem: None,
            mem_wb: None,
            fetch_pc,
            fetch_enabled: true,
            stopped: false,
            cycles: 0,
            retired: 0,
        }
    }

    pub fn cpu(&self) -> &Cpu {
        &self.cpu
    }
    pub fn memory(&self) -> &Memory {
        &self.memory
    }
    pub fn cycles(&self) -> u64 {
        self.cycles
    }
    pub fn retired(&self) -> u64 {
        self.retired
    }

    /// Prevent new fetches while allowing occupied latches to drain.
    pub fn pause_fetch(&mut self) {
        self.fetch_enabled = false;
    }
    pub fn resume_fetch(&mut self) {
        self.fetch_enabled = true;
    }

    pub fn registers(&self) -> PipelineRegisters {
        PipelineRegisters {
            if_id: self.if_id.map(|entry| entry.pc),
            id_ex: self.id_ex.map(|entry| entry.instruction().pc),
            ex_mem: self.ex_mem.map(|entry| entry.instruction.pc),
            mem_wb: self.mem_wb.map(|entry| entry.instruction.pc),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.registers() == PipelineRegisters::default()
    }

    /// Advance one cycle and report post-cycle latches. An unsupported
    /// instruction stops the model at WB and subsequent calls return None.
    pub fn cycle(&mut self) -> Option<CycleEvent> {
        if self.stopped || self.cpu.is_halted() {
            return None;
        }
        self.cycles += 1;
        let mut event = CycleEvent {
            cycle: self.cycles,
            registers: PipelineRegisters::default(),
            fetched: None,
            writeback: None,
            stall: None,
            flushed: false,
        };
        if let Some(prepared) = self.mem_wb.take() {
            let outcome = self.commit(prepared);
            event.writeback = Some(outcome);
            if !matches!(outcome, Writeback::Retired { .. }) {
                self.if_id = None;
                self.id_ex = None;
                self.ex_mem = None;
                self.fetch_pc = self.cpu.read_pc();
                event.flushed = true;
                return Some(event);
            }
        }

        let producers = [
            self.id_ex.and_then(Issued::destination),
            self.ex_mem.and_then(|entry| entry.effect.destination()),
        ];
        self.mem_wb = self
            .ex_mem
            .take()
            .map(|entry| memory_stage(entry, &self.cpu, &self.memory));
        self.ex_mem = self.id_ex.take().map(execute);
        if let Some(instruction) = self.if_id.take() {
            self.id_ex = match decode_stage(instruction) {
                Ok(decoded) => {
                    if let Some(register) = decoded
                        .sources
                        .into_iter()
                        .flatten()
                        .find(|register| producers.contains(&Some(*register)))
                    {
                        event.stall = Some(Stall::RegisterDependency {
                            pc: instruction.pc,
                            register,
                        });
                        self.if_id = Some(instruction);
                        None
                    } else {
                        let values = decoded
                            .sources
                            .map(|reg| reg.map_or(0, |reg| self.cpu.read_reg(usize::from(reg))));
                        Some(Issued::Operation(decoded, values))
                    }
                }
                Err(action) => Some(Issued::Ready(instruction, action)),
            };
        }
        if self.if_id.is_none() && self.fetch_enabled {
            self.if_id = Some(fetch(&self.cpu, &self.memory, self.fetch_pc));
            event.fetched = Some(self.fetch_pc);
            self.fetch_pc = self.fetch_pc.wrapping_add(4);
        }
        event.registers = self.registers();
        Some(event)
    }

    fn commit(&mut self, entry: Prepared) -> Writeback {
        let pc = entry.instruction.pc;
        let word = entry.instruction.word.ok();
        debug_assert_eq!(pc, self.cpu.read_pc());
        if let Action::Unsupported(mnemonic) = entry.action {
            self.stopped = true;
            return Writeback::Unsupported {
                pc,
                word: word.unwrap(),
                mnemonic,
            };
        }
        let action = if self.cpu.begin_pipeline_step(word) {
            entry.action
        } else {
            return self.exception_event(pc, crate::exception::Exception::Interrupt);
        };
        match action {
            Action::Write(register, value) => self.cpu.write_reg(usize::from(register), value),
            Action::Store(store) => write_store(&mut self.memory, store),
            Action::Fault(fault) => {
                self.cpu.raise_exception(fault.exception, fault.bad_address);
                return self.exception_event(pc, fault.exception);
            }
            Action::Unsupported(_) => unreachable!("handled before architectural state changes"),
        }
        self.retired += 1;
        Writeback::Retired {
            pc,
            word: word.unwrap(),
        }
    }

    fn exception_event(&self, pc: u32, exception: crate::exception::Exception) -> Writeback {
        Writeback::Exception {
            pc,
            exception,
            epc: self.cpu.cp0().read_epc(),
            in_delay_slot: self.cpu.cp0().branch_delay(),
        }
    }
}

#[cfg(test)]
mod tests;
