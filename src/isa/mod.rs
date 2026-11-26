pub mod mips32r2;

pub struct InstructionDef {
    pub mnemonic: &'static str,
    pub mask: u32,
    pub pattern: u32,
    pub operation: Operation,
}

include!(concat!(env!("OUT_DIR"), "/operations.rs"));
