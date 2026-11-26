use super::{InstructionDef, Operation};

pub static INSTRUCTIONS: &[InstructionDef] = &[
    InstructionDef {
        mnemonic: "add",
        mask: 0xFC0007FF,
        pattern: 0x00000020,
        operation: Operation::Add,
    },
    InstructionDef {
        mnemonic: "addu",
        mask: 0xFC0007FF,
        pattern: 0x00000021,
        operation: Operation::Addu,
    },
    InstructionDef {
        mnemonic: "sub",
        mask: 0xFC0007FF,
        pattern: 0x00000022,
        operation: Operation::Sub,
    },
    InstructionDef {
        mnemonic: "subu",
        mask: 0xFC0007FF,
        pattern: 0x00000023,
        operation: Operation::Subu,
    },
    InstructionDef {
        mnemonic: "and",
        mask: 0xFC0007FF,
        pattern: 0x00000024,
        operation: Operation::And,
    },
    InstructionDef {
        mnemonic: "or",
        mask: 0xFC0007FF,
        pattern: 0x00000025,
        operation: Operation::Or,
    },
    InstructionDef {
        mnemonic: "xor",
        mask: 0xFC0007FF,
        pattern: 0x00000026,
        operation: Operation::Xor,
    },
    InstructionDef {
        mnemonic: "nor",
        mask: 0xFC0007FF,
        pattern: 0x00000027,
        operation: Operation::Nor,
    },
    InstructionDef {
        mnemonic: "addi",
        mask: 0xFC000000,
        pattern: 0x20000000,
        operation: Operation::Addi,
    },
    InstructionDef {
        mnemonic: "addiu",
        mask: 0xFC000000,
        pattern: 0x24000000,
        operation: Operation::Addiu,
    },
];
