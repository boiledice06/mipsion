use super::*;

#[test]
fn inspection_general_registers() {
    use crate::inspection::dump_registers;

    let mut cpu = Cpu::new();

    cpu.write_reg(8, 42);
    cpu.write_reg(9, 43);
    cpu.write_pc(0x100);

    cpu.write_hi(0x1234);
    cpu.write_lo(0x5678);

    let output = dump_registers(&cpu);

    assert!(output.contains("$t0   0000002A"));
    assert!(output.contains("$t1   0000002B"));
    assert!(output.contains("PC       00000100"));
    assert!(output.contains("Next PC  00000104"));
    assert!(output.contains("HI       00001234"));
    assert!(output.contains("LO       00005678"));
}

#[test]
fn inspection_cp0_exception_state() {
    use crate::inspection::dump_cp0;

    let mut cpu = Cpu::new();
    let mut memory = Memory::new(64);

    memory.write_word(0, 0x0000_000C);

    cpu.step(&mut memory);

    let output = dump_cp0(&cpu);

    assert!(output.contains("ExcCode: 8"));
    assert!(output.contains("EXL:     true"));
    assert!(output.contains("BD:      false"));
}

#[test]
fn inspection_fpu_registers() {
    use crate::inspection::dump_fpu;

    let mut cpu = Cpu::new();

    cpu.cp1_mut().write_word(0, 0x3F80_0000);
    cpu.cp1_mut().write_word(1, 0x4000_0000);

    let output = dump_fpu(&cpu);

    assert!(output.contains("$f0  3F800000"));
    assert!(output.contains("$f1  40000000"));
    assert!(output.contains("FCSR"));
    assert!(output.contains("FIR"));
}

#[test]
fn inspection_memory_little_endian() {
    use crate::inspection::dump_memory;

    let mut memory = Memory::new(64);

    memory.write_word(0, 0x1234_5678);

    let output = dump_memory(&memory, 0, 4).unwrap();

    assert!(output.contains("78 56 34 12"));
    assert!(output.contains("00000000"));
}

#[test]
fn inspection_memory_ascii() {
    use crate::inspection::dump_memory;

    let mut memory = Memory::new(64);

    for (index, byte) in b"MIPSion".iter().enumerate() {
        memory.write_byte(index as u32, *byte);
    }

    let output = dump_memory(&memory, 0, 7).unwrap();

    assert!(output.contains("|MIPSion|"));
}

#[test]
fn inspection_memory_multiple_rows() {
    use crate::inspection::dump_memory;

    let memory = Memory::new(64);

    let output = dump_memory(&memory, 0, 32).unwrap();

    assert!(output.contains("00000000"));
    assert!(output.contains("00000010"));
}

#[test]
fn inspection_memory_exact_boundary() {
    use crate::inspection::dump_memory;

    let mut memory = Memory::new(64);

    memory.write_byte(63, 0xAB);

    let output = dump_memory(&memory, 63, 1).unwrap();

    assert!(output.contains("0000003F"));
    assert!(output.contains("AB"));
}

#[test]
fn inspection_memory_invalid_range() {
    use crate::inspection::dump_memory;

    let memory = Memory::new(64);

    assert!(dump_memory(&memory, 60, 8).is_err());
    assert!(dump_memory(&memory, 0, 0).is_err());
    assert!(dump_memory(&memory, u32::MAX, 4).is_err());
}
