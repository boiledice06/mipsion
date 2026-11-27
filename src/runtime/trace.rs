use std::fmt::Write;

use crate::cpu::Cpu;
use crate::decoder::decode;
use crate::inspection::GPR_NAMES;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CpuSnapshot {
    pub pc: u32,
    pub gpr: [u32; 32],
    pub hi: u32,
    pub lo: u32,
    pub exception_count: u64,
}

impl CpuSnapshot {
    pub fn capture(cpu: &Cpu) -> Self {
        Self {
            pc: cpu.read_pc(),
            gpr: std::array::from_fn(|index| cpu.read_reg(index)),
            hi: cpu.read_hi(),
            lo: cpu.read_lo(),
            exception_count: cpu.exception_count(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegisterChange {
    pub index: usize,
    pub before: u32,
    pub after: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValueChange {
    pub before: u32,
    pub after: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TraceException {
    pub code: u32,
    pub epc: u32,
    pub in_delay_slot: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceEvent {
    pub step: u64,
    pub pc: u32,
    pub next_pc: u32,
    pub instruction: Option<u32>,
    pub mnemonic: Option<&'static str>,
    pub register_changes: Vec<RegisterChange>,
    pub hi_change: Option<ValueChange>,
    pub lo_change: Option<ValueChange>,
    pub exception: Option<TraceException>,
}

fn value_change(before: u32, after: u32) -> Option<ValueChange> {
    (before != after).then_some(ValueChange { before, after })
}

impl TraceEvent {
    pub fn capture(step: u64, before: &CpuSnapshot, cpu: &Cpu) -> Self {
        let instruction = cpu.last_instruction();

        let mnemonic = instruction
            .and_then(decode)
            .map(|definition| definition.mnemonic);

        let register_changes = before
            .gpr
            .iter()
            .enumerate()
            .filter_map(|(index, &old)| {
                let new = cpu.read_reg(index);

                (old != new).then_some(RegisterChange {
                    index,
                    before: old,
                    after: new,
                })
            })
            .collect();

        let exception = if cpu.exception_count() != before.exception_count {
            Some(TraceException {
                code: cpu.cp0().exception_code(),
                epc: cpu.cp0().read_epc(),
                in_delay_slot: cpu.cp0().branch_delay(),
            })
        } else {
            None
        };

        Self {
            step,
            pc: before.pc,
            next_pc: cpu.read_pc(),
            instruction,
            mnemonic,
            register_changes,
            hi_change: value_change(before.hi, cpu.read_hi()),
            lo_change: value_change(before.lo, cpu.read_lo()),
            exception,
        }
    }
}

pub fn format_trace(event: &TraceEvent) -> String {
    let mut output = String::new();

    write!(output, "[{:06}] PC={:08X}  ", event.step, event.pc,).unwrap();

    match event.instruction {
        Some(word) => {
            writeln!(
                output,
                "{word:08X}  {}",
                event.mnemonic.unwrap_or("<unknown>"),
            )
            .unwrap();
        }

        None => {
            writeln!(output, "--------  <no fetch>").unwrap();
        }
    }

    for change in &event.register_changes {
        writeln!(
            output,
            "         {}: {:08X} -> {:08X}",
            GPR_NAMES[change.index], change.before, change.after,
        )
        .unwrap();
    }

    for (name, change) in [("HI", event.hi_change), ("LO", event.lo_change)] {
        if let Some(change) = change {
            writeln!(
                output,
                "         {name}: {:08X} -> {:08X}",
                change.before, change.after,
            )
            .unwrap();
        }
    }

    if let Some(exception) = event.exception {
        writeln!(
            output,
            "         Exception: {} EPC={:08X} BD={}",
            exception.code, exception.epc, exception.in_delay_slot,
        )
        .unwrap();
    }

    writeln!(output, "         PC -> {:08X}", event.next_pc,).unwrap();

    output
}
