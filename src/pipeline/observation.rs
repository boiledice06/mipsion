//! Owned views of modeled pipeline state, independent of terminal formatting.

use super::state::{Action, Effect, Executed, Fault, Instruction, Issued, Prepared};
use super::{CycleEvent, PipelineStatistics, Redirect};
use crate::cache::{CacheAccess, CacheLineLocation};
use crate::exception::Exception;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstructionSnapshot {
    /// Identity of this fetch attempt, including refetches of the same PC.
    pub sequence: u64,
    pub pc: u32,
    pub physical_address: Option<u32>,
    pub word: Result<u32, FaultSnapshot>,
    pub in_delay_slot: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FaultSnapshot {
    pub exception: Exception,
    pub bad_address: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecodedInputs {
    /// Nonzero GPR dependencies. An unused or zero-register operand has no dependency.
    pub source_registers: [Option<u8>; 2],
    pub register_values: [u32; 2],
    pub destination: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IssuedSnapshot {
    pub instruction: InstructionSnapshot,
    pub inputs: Option<DecodedInputs>,
    /// A fault or diagnostic already identified at decode.
    pub ready_effect: Option<StageEffect>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AluSnapshot {
    pub lhs: u32,
    pub rhs: u32,
    /// Absent when the checked ALU operation overflowed.
    pub result: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageAddress {
    Virtual(u32),
    Physical(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageEffect {
    /// An intermediate value for register zero is possible. WB discards it.
    Register {
        register: u8,
        value: u32,
    },
    Load {
        register: u8,
        virtual_address: u32,
        bytes: usize,
        signed: bool,
    },
    Store {
        address: StageAddress,
        bytes: usize,
        value: u32,
    },
    Control {
        redirect: Redirect,
        link: Option<(u8, u32)>,
    },
    Fault(FaultSnapshot),
    Unsupported {
        mnemonic: Option<&'static str>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecuteSnapshot {
    pub instruction: InstructionSnapshot,
    pub effect: StageEffect,
    /// Operands actually passed to the integer ALU helper.
    pub alu: Option<AluSnapshot>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreparedSnapshot {
    pub instruction: InstructionSnapshot,
    pub effect: StageEffect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingSnapshot<T> {
    pub entry: T,
    /// Zero can mean ready but blocked by an older data request.
    pub cycles_remaining: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cp0Snapshot {
    pub status: u32,
    pub cause: u32,
    pub epc: u32,
    pub bad_vaddr: u32,
    pub count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PipelineSnapshot {
    pub statistics: PipelineStatistics,
    pub architectural_pc: u32,
    pub architectural_next_pc: u32,
    pub cp0: Cp0Snapshot,
    pub fetch_pc: u32,
    pub fetch_enabled: bool,
    pub target_after_delay_slot: Option<u32>,
    pub stopped: bool,
    pub if_id: Option<InstructionSnapshot>,
    pub id_ex: Option<IssuedSnapshot>,
    pub ex_mem: Option<ExecuteSnapshot>,
    pub mem_wb: Option<PreparedSnapshot>,
    pub pending_fetch: Option<PendingSnapshot<InstructionSnapshot>>,
    pub pending_memory: Option<PendingSnapshot<PreparedSnapshot>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryRequest {
    pub instruction: InstructionSnapshot,
    pub kind: CacheAccess,
    pub virtual_address: u32,
    pub physical_address: u32,
    pub bytes: usize,
    /// The raw bytes as a little-endian value, before any load sign extension.
    pub read_value: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommittedWrite {
    Register {
        register: u8,
        before: u32,
        after: u32,
    },
    Memory {
        physical_address: u32,
        bytes: usize,
        before: u32,
        after: u32,
    },
}

/// A cycle result together with actual stage activity and committed effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CycleObservation {
    pub event: CycleEvent,
    pub snapshot: PipelineSnapshot,
    /// Present when EX consumed an ID/EX entry on this cycle.
    pub executed: Option<ExecuteSnapshot>,
    pub writeback_instruction: Option<InstructionSnapshot>,
    pub committed_write: Option<CommittedWrite>,
    pub memory_request: Option<MemoryRequest>,
    pub instruction_cache_line: Option<CacheLineLocation>,
    pub data_cache_line: Option<CacheLineLocation>,
    /// Location of the instruction-cache line immediately before invalidation.
    pub invalidated_line: Option<CacheLineLocation>,
}

#[derive(Default)]
pub(super) struct CycleDetails {
    pub executed: Option<ExecuteSnapshot>,
    pub writeback_instruction: Option<InstructionSnapshot>,
    pub committed_write: Option<CommittedWrite>,
    pub memory_request: Option<MemoryRequest>,
    pub invalidated_line: Option<CacheLineLocation>,
}

impl From<Instruction> for InstructionSnapshot {
    fn from(instruction: Instruction) -> Self {
        Self {
            sequence: instruction.sequence,
            pc: instruction.pc,
            physical_address: instruction.physical,
            word: instruction.word.map_err(FaultSnapshot::from),
            in_delay_slot: instruction.delay_slot,
        }
    }
}

impl From<Fault> for FaultSnapshot {
    fn from(fault: Fault) -> Self {
        Self {
            exception: fault.exception,
            bad_address: fault.bad_address,
        }
    }
}

impl From<Issued> for IssuedSnapshot {
    fn from(issued: Issued) -> Self {
        let (inputs, ready_effect) = match issued {
            Issued::Operation(decoded, register_values) => (
                Some(DecodedInputs {
                    source_registers: decoded.sources,
                    register_values,
                    destination: decoded.destination,
                }),
                None,
            ),
            Issued::Ready(_, action) => (None, Some(StageEffect::from(action))),
        };
        Self {
            instruction: issued.instruction().into(),
            inputs,
            ready_effect,
        }
    }
}

impl From<Executed> for ExecuteSnapshot {
    fn from(executed: Executed) -> Self {
        let effect = match executed.effect {
            Effect::Ready(action) => action.into(),
            Effect::Load {
                destination,
                address,
                width,
                signed,
            } => StageEffect::Load {
                register: destination,
                virtual_address: address,
                bytes: width.bytes(),
                signed,
            },
            Effect::Store(store) => StageEffect::Store {
                address: StageAddress::Virtual(store.address),
                bytes: store.width.bytes(),
                value: store.value,
            },
        };
        Self {
            instruction: executed.instruction.into(),
            effect,
            alu: executed.alu,
        }
    }
}

impl From<Prepared> for PreparedSnapshot {
    fn from(prepared: Prepared) -> Self {
        Self {
            instruction: prepared.instruction.into(),
            effect: prepared.action.into(),
        }
    }
}

impl From<Action> for StageEffect {
    fn from(action: Action) -> Self {
        match action {
            Action::Write(register, value) => Self::Register { register, value },
            Action::Store(store) => Self::Store {
                address: StageAddress::Physical(store.address),
                bytes: store.width.bytes(),
                value: store.value,
            },
            Action::Control { redirect, link } => Self::Control { redirect, link },
            Action::Fault(fault) => Self::Fault(fault.into()),
            Action::Unsupported(mnemonic) => Self::Unsupported { mnemonic },
        }
    }
}
