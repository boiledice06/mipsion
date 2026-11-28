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
pub struct CycleEvent {
    pub cycle: u64,
    pub registers: PipelineRegisters,
    pub fetched: Option<u32>,
    pub writeback: Option<Writeback>,
    pub stall: Option<Stall>,
    pub redirect: Option<Redirect>,
    pub flushed: bool,
}
