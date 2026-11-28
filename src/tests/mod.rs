mod cli;
mod control_flow;
mod cp0;
mod cp1;
mod decoder;
mod disassembler;
mod elf;
mod exception;
mod fpu;
pub mod helpers;
mod inspection;
mod integer;
mod loader;
mod memory;
mod mmu;
mod runtime;
mod tlb;
mod trace;

use crate::cpu::Cpu;
use crate::memory::Memory;
use crate::test_helpers::{encode_i as i, encode_r, run};

fn encode_cp0(rs: u32, rt: u32, rd: u32, sel: u32) -> u32 {
    assert!(sel < 8);
    encode_r(0x10, rs, rt, rd, 0, sel)
}

fn run_kernel_alias_program() -> Cpu {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.write_reg(8, 0x8000_0100);
    cpu.write_reg(9, 0xDEAD_BEEF);
    cpu.write_reg(11, 0xA000_0100);

    memory.write_word(0, i(0x2B, 8, 9, 0));
    memory.write_word(4, i(0x23, 11, 10, 0));

    cpu.step(&mut memory);
    cpu.step(&mut memory);

    assert_eq!(memory.read_word(0x100), 0xDEAD_BEEF);

    cpu
}

fn encode_fp_compare(fmt: u8, fs: u8, ft: u8, cc: u8, cond: u8) -> u32 {
    (0x11u32 << 26)
        | (u32::from(fmt) << 21)
        | (u32::from(ft) << 16)
        | (u32::from(fs) << 11)
        | (u32::from(cc) << 8)
        | 0x30
        | u32::from(cond)
}

fn encode_fp_branch(cc: u8, likely: bool, expected: bool, offset: i16) -> u32 {
    (0x11u32 << 26)
        | (0x08 << 21)
        | (u32::from(cc) << 18)
        | ((likely as u32) << 17)
        | ((expected as u32) << 16)
        | u32::from(offset as u16)
}

fn parse_cli(args: &[&str]) -> Result<crate::cli::CliAction, String> {
    crate::cli::parse_args(args.iter().map(|arg| arg.to_string()))
}

fn elf_fixture() -> Vec<u8> {
    let mut bytes = vec![0u8; 0x10C];

    bytes[..4].copy_from_slice(b"\x7FELF");

    bytes[4] = 1;
    bytes[5] = 1;
    bytes[6] = 1;

    let fields16: [(usize, u16); 5] = [(16, 2), (18, 8), (40, 52), (42, 32), (44, 1)];

    for (offset, value) in fields16 {
        bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }

    let fields32: [(usize, u32); 11] = [
        (20, 1),
        (24, 0x1000),
        (28, 52),
        // Program header
        (52, 1),
        (56, 0x100),
        (60, 0x1000),
        (64, 0x1000),
        (68, 12),
        (72, 32),
        (76, 5),
        (80, 4),
    ];

    for (offset, value) in fields32 {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    let words: [u32; 3] = [0x2408_002A, 0x2509_0001, 0x0000_000C];

    for (index, word) in words.iter().enumerate() {
        let offset = 0x100 + index * 4;

        bytes[offset..offset + 4].copy_from_slice(&word.to_le_bytes());
    }

    bytes
}
