use super::timing::PipelineStatistics;
use crate::cache::{AccessResult, CacheAccess};
use crate::exception::Exception;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PipelineRegisters {
    pub if_id: Option<u32>,
    pub id_ex: Option<u32>,
    pub ex_mem: Option<u32>,
    pub mem_wb: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stall {
    RegisterDependency { pc: u32, register: u8 },
    LoadUse { pc: u32, register: u8 },
    InstructionMemory { pc: u32 },
    DataMemory { pc: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Writeback {
    Retired {
        pc: u32,
        word: u32,
    },
    Exception {
        pc: u32,
        exception: Exception,
        epc: u32,
        in_delay_slot: bool,
    },
    Unsupported {
        pc: u32,
        word: u32,
        mnemonic: Option<&'static str>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Redirect {
    pub branch_pc: u32,
    pub delay_slot: Option<u32>,
    pub target: u32,
    pub taken: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CachePort {
    Instruction,
    Data,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CacheEvent {
    pub port: CachePort,
    pub kind: CacheAccess,
    pub address: u32,
    pub result: AccessResult,
    pub latency: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForwardingSource {
    ExMem,
    MemWb,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Forwarded {
    pub pc: u32,
    pub operand: u8,
    pub register: u8,
    pub value: u32,
    pub source: ForwardingSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CycleEvent {
    pub cycle: u64,
    pub registers: PipelineRegisters,
    pub fetched: Option<u32>,
    pub writeback: Option<Writeback>,
    pub stall: Option<Stall>,
    pub redirect: Option<Redirect>,
    pub instruction_cache: Option<CacheEvent>,
    pub data_cache: Option<CacheEvent>,
    pub invalidated_instruction_line: Option<u32>,
    pub forwarded: [Option<Forwarded>; 2],
    pub flushed: bool,
}

/// Format a cycle trace for the CLI and debugger.
pub fn format_cycle(event: &CycleEvent) -> String {
    use std::fmt::Write;

    fn address(pc: Option<u32>) -> String {
        pc.map_or_else(|| "-".into(), |pc| format!("{pc:08X}"))
    }
    let state = event.registers;
    let mut text = format!(
        "[cycle {}] IF={} IF/ID={} ID/EX={} EX/MEM={} MEM/WB={}",
        event.cycle,
        address(event.fetched),
        address(state.if_id),
        address(state.id_ex),
        address(state.ex_mem),
        address(state.mem_wb),
    );
    if let Some(stall) = event.stall {
        match stall {
            Stall::RegisterDependency { pc, register } => {
                let _ = write!(text, " stall {pc:08X} waiting for ${register}");
            }
            Stall::LoadUse { pc, register } => {
                let _ = write!(text, " load-use stall {pc:08X} waiting for ${register}");
            }
            Stall::InstructionMemory { pc } => {
                let _ = write!(text, " I-cache stall {pc:08X}");
            }
            Stall::DataMemory { pc } => {
                let _ = write!(text, " D-cache stall {pc:08X}");
            }
        }
    }

    for access in [event.instruction_cache, event.data_cache]
        .into_iter()
        .flatten()
    {
        let port = match access.port {
            CachePort::Instruction => "I-cache",
            CachePort::Data => "D-cache",
        };
        let kind = match access.kind {
            CacheAccess::Read => "read",
            CacheAccess::Write => "write",
        };
        let address = access.address;
        match access.result {
            AccessResult::Hit => {
                let _ = write!(text, " {port} {kind} hit {address:08X}");
            }
            AccessResult::Miss { evicted } => {
                let _ = write!(
                    text,
                    " {port} {kind} miss {address:08X} +{}",
                    access.latency
                );
                if let Some(base) = evicted {
                    let _ = write!(text, " evict {base:08X}");
                }
            }
        }
    }
    if let Some(base) = event.invalidated_instruction_line {
        let _ = write!(text, " I-cache invalidate {base:08X}");
    }
    for operand in event.forwarded.into_iter().flatten() {
        let source = match operand.source {
            ForwardingSource::ExMem => "EX/MEM",
            ForwardingSource::MemWb => "MEM/WB",
        };
        let _ = write!(
            text,
            " forward {source} ${}={:08X} to operand {}",
            operand.register, operand.value, operand.operand
        );
    }
    if let Some(redirect) = event.redirect {
        let _ = write!(
            text,
            " redirect {:08X} -> {:08X}",
            redirect.branch_pc, redirect.target
        );
    }
    if event.flushed {
        text.push_str(" flush");
    }
    match event.writeback {
        Some(Writeback::Retired { pc, word }) => {
            let _ = write!(text, " retire {pc:08X} {word:08X}");
        }
        Some(Writeback::Exception {
            pc,
            exception,
            epc,
            in_delay_slot,
        }) => {
            let _ = write!(
                text,
                " exception {exception:?} at {pc:08X} EPC={epc:08X} BD={in_delay_slot}"
            );
        }
        Some(Writeback::Unsupported { pc, word, mnemonic }) => {
            let _ = write!(
                text,
                " unsupported {} at {pc:08X} ({word:08X})",
                mnemonic.unwrap_or("unknown")
            );
        }
        None => {}
    }
    text
}

/// Format cumulative pipeline and cache counters for a terminal.
pub fn format_statistics(stats: PipelineStatistics) -> String {
    use std::fmt::Write;

    let rate =
        |value: Option<f64>| value.map_or_else(|| "n/a".into(), |value| format!("{value:.3}"));
    let mut text = format!(
        "Cycles:       {}\nRetired:      {}\nCPI:          {}\nIPC:          {}\n",
        stats.cycles,
        stats.retired,
        rate(stats.cpi()),
        rate(stats.ipc())
    );
    for (name, cache) in [
        ("I-cache", stats.instruction_cache),
        ("D-cache", stats.data_cache),
    ] {
        if let Some(cache) = cache {
            let _ = writeln!(
                text,
                "{name}: reads={} writes={} hits={} misses={} evictions={} invalidations={}",
                cache.reads,
                cache.writes,
                cache.hits,
                cache.misses,
                cache.evictions,
                cache.invalidations
            );
        }
    }
    text
}
