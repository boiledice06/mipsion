use std::fmt::Write;

use crate::cpu::Cpu;
use crate::memory::Memory;

pub const GPR_NAMES: [&str; 32] = [
    "$zero", "$at", "$v0", "$v1", "$a0", "$a1", "$a2", "$a3", "$t0", "$t1", "$t2", "$t3", "$t4",
    "$t5", "$t6", "$t7", "$s0", "$s1", "$s2", "$s3", "$s4", "$s5", "$s6", "$s7", "$t8", "$t9",
    "$k0", "$k1", "$gp", "$sp", "$fp", "$ra",
];

pub fn dump_registers(cpu: &Cpu) -> String {
    let mut output = String::new();

    writeln!(output, "General Purpose Registers").unwrap();
    writeln!(output, "-------------------------").unwrap();

    for row in 0..8 {
        for col in 0..4 {
            let index = row * 4 + col;

            write!(
                output,
                "{:<5} {:08X}  ",
                GPR_NAMES[index],
                cpu.read_reg(index),
            )
            .unwrap();
        }

        writeln!(output).unwrap();
    }

    writeln!(output).unwrap();

    for (name, value) in [
        ("PC", cpu.read_pc()),
        ("Next PC", cpu.read_next_pc()),
        ("HI", cpu.read_hi()),
        ("LO", cpu.read_lo()),
    ] {
        writeln!(output, "{name:<8} {value:08X}").unwrap();
    }

    output
}

pub fn dump_cp0(cpu: &Cpu) -> String {
    let mut output = String::new();

    let cp0 = cpu.cp0();

    writeln!(output, "System Control Registers (CP0)").unwrap();
    writeln!(output, "------------------------------").unwrap();

    for (name, value) in [
        ("Status", cp0.read_status()),
        ("Cause", cp0.read_cause()),
        ("EPC", cp0.read_epc()),
        ("BadVAddr", cp0.read_bad_vaddr()),
        ("Count", cp0.read_count()),
        ("Compare", cp0.read_compare()),
        ("EBase", cp0.read_ebase()),
        ("ErrorEPC", cp0.read_error_epc()),
        ("Index", cp0.read_index()),
        ("Random", cp0.read_random()),
        ("EntryLo0", cp0.read_entry_lo0()),
        ("EntryLo1", cp0.read_entry_lo1()),
        ("EntryHi", cp0.read_entry_hi()),
        ("Context", cp0.read_context()),
        ("PageMask", cp0.read_page_mask()),
        ("Wired", cp0.read_wired()),
        ("Config", cp0.read_config()),
        ("Config1", cp0.read_config1()),
    ] {
        writeln!(output, "{name:<10} {value:08X}").unwrap();
    }

    writeln!(output).unwrap();
    writeln!(output, "ExcCode: {}", cp0.exception_code()).unwrap();
    writeln!(output, "EXL:     {}", cp0.exception_level()).unwrap();
    writeln!(output, "BD:      {}", cp0.branch_delay()).unwrap();

    output
}

pub fn dump_fpu(cpu: &Cpu) -> String {
    let mut output = String::new();

    let cp1 = cpu.cp1();

    writeln!(output, "Floating-Point Registers (FR=0)").unwrap();
    writeln!(output, "-------------------------------").unwrap();

    for row in 0..8 {
        for col in 0..4 {
            let index = row * 4 + col;

            write!(output, "$f{:<2} {:08X}  ", index, cp1.read_word(index),).unwrap();
        }

        writeln!(output).unwrap();
    }

    writeln!(output).unwrap();
    writeln!(output, "FCSR     {:08X}", cp1.read_fcsr()).unwrap();
    writeln!(output, "FIR      {:08X}", cp1.read_fir()).unwrap();

    output
}

pub fn dump_memory(memory: &Memory, address: u32, size: usize) -> Result<String, String> {
    if !memory.contains_range(address, size) {
        return Err("memory dump range is out of bounds".into());
    }

    let mut output = String::new();

    writeln!(output, "Memory Dump ({size} bytes)",).unwrap();

    writeln!(output, "----------------------").unwrap();

    for offset in (0..size).step_by(16) {
        let start = address + offset as u32;
        let count = (size - offset).min(16);

        write!(output, "{start:08X}  ").unwrap();

        for index in 0..16 {
            if index < count {
                let byte = memory.read_byte(start + index as u32);

                write!(output, "{byte:02X} ").unwrap();
            } else {
                write!(output, "   ").unwrap();
            }
        }

        write!(output, " |").unwrap();

        for index in 0..count {
            let byte = memory.read_byte(start + index as u32);

            let character = if byte.is_ascii_graphic() || byte == b' ' {
                char::from(byte)
            } else {
                '.'
            };

            write!(output, "{character}").unwrap();
        }

        writeln!(output, "|").unwrap();
    }

    Ok(output)
}
