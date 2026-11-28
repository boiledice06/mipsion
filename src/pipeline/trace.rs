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
        let Stall::RegisterDependency { pc, register } = stall;
        let _ = write!(text, " stall {pc:08X} waiting for ${register}");
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
