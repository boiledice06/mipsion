use crate::isa::InstructionDef;
use crate::isa::mips32r2::INSTRUCTIONS;

pub fn decode(word: u32) -> Option<&'static InstructionDef> {
    INSTRUCTIONS
        .iter()
        .find(|inst| (word & inst.mask) == inst.pattern)
}
