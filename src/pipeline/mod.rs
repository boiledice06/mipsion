//! In-order, single-issue pipeline with separate architectural retirement.

mod control;
mod hazard;
mod stages;
mod state;
mod timing;
mod trace;

pub use timing::{CacheModelConfig, PipelineOptions, PipelineStatistics};
pub use trace::{
    CacheEvent, CachePort, CycleEvent, Forwarded, ForwardingSource, PipelineRegisters, Redirect,
    Stall, Writeback, format_cycle, format_statistics,
};

use crate::{
    cache::{Cache, CacheAccess},
    cpu::Cpu,
    memory::Memory,
};
use stages::{decode_stage, execute, fetch, memory_stage, read_memory, write_store};
use state::{Action, Effect, Executed, Instruction, Issued, Prepared, Store};
use timing::{CacheTiming, Pending};

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
    after_delay_slot: Option<u32>,
    fetch_enabled: bool,
    stopped: bool,
    cycles: u64,
    retired: u64,
    options: PipelineOptions,
    caches: Option<CacheTiming>,
    pending_fetch: Option<Pending<Instruction>>,
    pending_memory: Option<Pending<Prepared>>,
}

impl Pipeline {
    pub fn new(cpu: Cpu, memory: Memory) -> Self {
        Self::with_options(cpu, memory, PipelineOptions::default())
    }

    pub fn with_options(cpu: Cpu, memory: Memory, options: PipelineOptions) -> Self {
        let fetch_pc = cpu.read_pc();
        let after_delay_slot = cpu
            .next_instruction_is_delay_slot()
            .then(|| cpu.read_next_pc());
        Self {
            cpu,
            memory,
            if_id: None,
            id_ex: None,
            ex_mem: None,
            mem_wb: None,
            fetch_pc,
            after_delay_slot,
            fetch_enabled: true,
            stopped: false,
            cycles: 0,
            retired: 0,
            options,
            caches: options.caches.map(CacheTiming::new),
            pending_fetch: None,
            pending_memory: None,
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

    pub fn statistics(&self) -> PipelineStatistics {
        PipelineStatistics {
            cycles: self.cycles,
            retired: self.retired,
            instruction_cache: self.instruction_cache().map(Cache::stats),
            data_cache: self.data_cache().map(Cache::stats),
        }
    }

    pub fn options(&self) -> PipelineOptions {
        self.options
    }

    pub fn instruction_cache(&self) -> Option<&Cache> {
        self.caches.as_ref().map(|caches| &caches.instruction)
    }

    pub fn data_cache(&self) -> Option<&Cache> {
        self.caches.as_ref().map(|caches| &caches.data)
    }

    /// Prevent new fetches while allowing outstanding accesses and latches to drain.
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
            && self.pending_fetch.is_none()
            && self.pending_memory.is_none()
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
            redirect: None,
            instruction_cache: None,
            data_cache: None,
            invalidated_instruction_line: None,
            forwarded: [None; 2],
            flushed: false,
        };
        if let Some(prepared) = self.mem_wb.take() {
            let outcome = self.commit(prepared, &mut event);
            event.writeback = Some(outcome);
            let overwrote_code = matches!(prepared.action, Action::Store(store) if self.overwrote_code(store.address));
            if !matches!(outcome, Writeback::Retired { .. }) || overwrote_code {
                self.flush();

                event.flushed = true;
                return Some(event);
            }
        }

        if let Some(pending) = &mut self.pending_fetch {
            pending.tick();
        }
        if let Some(pending) = &mut self.pending_memory {
            pending.tick();
        }
        let producers = [
            self.id_ex.and_then(Issued::destination),
            self.ex_mem.and_then(|entry| entry.effect.destination()),
        ];
        if !self.advance_memory(&mut event) {
            event.registers = self.registers();
            return Some(event);
        }
        self.ex_mem = self.id_ex.take().map(execute);
        if let Some(Executed {
            effect: Effect::Ready(Action::Control { redirect, .. }),
            ..
        }) = self.ex_mem
        {
            event.redirect = Some(redirect);
            event.flushed = self.redirect_fetch(redirect);
        }
        if let Some(instruction) = self.if_id.take() {
            self.id_ex = match decode_stage(instruction) {
                Ok(decoded) => {
                    match hazard::operands(
                        decoded,
                        &self.cpu,
                        producers,
                        self.ex_mem,
                        self.mem_wb,
                        self.options.forwarding,
                    ) {
                        Ok((values, forwarded)) => {
                            event.forwarded = forwarded;
                            Some(Issued::Operation(decoded, values))
                        }
                        Err(stall) => {
                            event.stall = Some(stall);
                            self.if_id = Some(instruction);
                            None
                        }
                    }
                }
                Err(action) => Some(Issued::Ready(instruction, action)),
            };
        }
        self.advance_fetch(&mut event);
        event.registers = self.registers();
        Some(event)
    }

    fn advance_memory(&mut self, event: &mut CycleEvent) -> bool {
        let Some(entry) = self.ex_mem else {
            return true;
        };
        let pc = entry.instruction.pc;
        if let Some(pending) = self.pending_memory {
            if pending.remaining == 0 {
                self.pending_memory = None;
                self.mem_wb = Some(pending.payload);
                return true;
            }
            event.stall = Some(Stall::DataMemory { pc });
            return false;
        }
        let prepared = memory_stage(entry, &self.cpu, &self.memory, |address, width, kind| {
            if let Some(caches) = &mut self.caches {
                let (value, cache_event) =
                    caches.access(&self.memory, CachePort::Data, kind, address, width.bytes());
                event.data_cache = Some(cache_event);
                value
            } else if kind == CacheAccess::Read {
                read_memory(&self.memory, address, width)
            } else {
                0
            }
        });
        let latency = event.data_cache.map_or(0, |access| access.latency);
        if latency == 0 {
            self.mem_wb = Some(prepared);
            true
        } else {
            self.pending_memory = Some(Pending {
                payload: prepared,
                remaining: latency,
            });
            event.stall = Some(Stall::DataMemory { pc });
            false
        }
    }

    fn advance_fetch(&mut self, event: &mut CycleEvent) {
        if self.if_id.is_some() {
            return;
        }
        if let Some(pending) = self.pending_fetch {
            if pending.remaining == 0 {
                self.pending_fetch = None;
                self.accept_fetch(pending.payload, event);
            } else {
                event.stall = Some(Stall::InstructionMemory {
                    pc: pending.payload.pc,
                });
            }
            return;
        }
        if !self.fetch_enabled {
            return;
        }

        let instruction = fetch(&self.cpu, &self.memory, self.fetch_pc, |address| {
            if let Some(caches) = &mut self.caches {
                let (value, cache_event) = caches.access(
                    &self.memory,
                    CachePort::Instruction,
                    CacheAccess::Read,
                    address,
                    4,
                );
                event.instruction_cache = Some(cache_event);
                value
            } else {
                self.memory.read_word(address)
            }
        });
        let latency = event.instruction_cache.map_or(0, |access| access.latency);
        if latency == 0 {
            self.accept_fetch(instruction, event);
        } else {
            self.pending_fetch = Some(Pending {
                payload: instruction,
                remaining: latency,
            });
            event.stall = Some(Stall::InstructionMemory { pc: instruction.pc });
        }
    }

    fn accept_fetch(&mut self, mut instruction: Instruction, event: &mut CycleEvent) {
        debug_assert_eq!(self.fetch_pc, instruction.pc);
        event.fetched = Some(instruction.pc);
        if let Some(target) = self.after_delay_slot.take() {
            instruction.delay_slot = true;
            self.fetch_pc = target;
        } else {
            self.fetch_pc = self.fetch_pc.wrapping_add(4);
        }
        self.if_id = Some(instruction);
    }

    fn redirect_fetch(&mut self, redirect: Redirect) -> bool {
        match redirect.delay_slot {
            Some(slot) => {
                if let Some(entry) = &mut self.if_id {
                    debug_assert_eq!(entry.pc, slot);
                    entry.delay_slot = true;
                    self.fetch_pc = redirect.target;
                } else {
                    debug_assert!(
                        self.pending_fetch
                            .is_none_or(|pending| pending.payload.pc == slot)
                    );
                    self.fetch_pc = slot;
                    self.after_delay_slot = Some(redirect.target);
                }
                false
            }
            None => {
                let occupied = self.if_id.take().is_some();
                let outstanding = self.pending_fetch.take().is_some();
                let annulled = occupied || outstanding;
                self.fetch_pc = redirect.target;
                annulled
            }
        }
    }

    fn flush(&mut self) {
        self.if_id = None;
        self.id_ex = None;
        self.ex_mem = None;
        self.pending_fetch = None;
        self.pending_memory = None;
        self.fetch_pc = self.cpu.read_pc();
        self.after_delay_slot = self
            .cpu
            .next_instruction_is_delay_slot()
            .then(|| self.cpu.read_next_pc());
    }

    fn overwrote_code(&self, address: u32) -> bool {
        let word = Some(address & !3);
        [
            self.if_id,
            self.id_ex.map(Issued::instruction),
            self.ex_mem.map(|entry| entry.instruction),
            self.pending_fetch.map(|pending| pending.payload),
        ]
        .into_iter()
        .flatten()
        .any(|entry| entry.physical == word)
    }

    fn commit(&mut self, entry: Prepared, event: &mut CycleEvent) -> Writeback {
        let pc = entry.instruction.pc;
        let word = entry.instruction.word.ok();
        debug_assert_eq!(pc, self.cpu.read_pc());
        if let Action::Unsupported(mnemonic) = entry.action
            && !self.cpu.interrupt_due_at_next_boundary()
        {
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
            Action::Store(store) => self.commit_store(store, event),
            Action::Control { redirect, link } => {
                if let Some((register, value)) = link {
                    self.cpu.write_reg(usize::from(register), value);
                }
                if redirect.delay_slot.is_some() {
                    self.cpu.write_next_pc(redirect.target);
                    self.cpu.mark_delay_slot();
                } else {
                    self.cpu.skip_delay_slot();
                }
            }
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

    fn commit_store(&mut self, store: Store, event: &mut CycleEvent) {
        if let Some(caches) = &mut self.caches {
            let bytes = store.value.to_le_bytes();
            caches.data.write_through(
                &mut self.memory,
                store.address,
                &bytes[..store.width.bytes()],
            );
            event.invalidated_instruction_line = caches.instruction.invalidate(store.address);
        } else {
            write_store(&mut self.memory, store);
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

#[cfg(test)]
mod equivalence_tests;

#[cfg(test)]
mod timing_tests;

#[cfg(test)]
mod audit_tests;
