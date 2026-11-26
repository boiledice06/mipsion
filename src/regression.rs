//! Regression tests for the current integer core. Include from main.rs with
//! #[cfg(test)] mod regression;
use crate::cpu::Cpu;
use crate::memory::Memory;
use crate::test_helpers::{encode_i as i, encode_r, run};

#[test]
fn register_alu() {
    for (funct, expected) in [
        (0x20, 22),
        (0x21, 22),
        (0x22, 2),
        (0x23, 2),
        (0x24, 8),
        (0x25, 14),
        (0x26, 6),
        (0x27, 0xfffffff1),
    ] {
        let cpu = run(&[encode_r(0, 8, 9, 10, 0, funct)], &[(8, 12), (9, 10)]);
        assert_eq!(cpu.read_reg(10), expected, "funct {funct:#x}");
    }
}
#[test]
fn negative_subtraction() {
    for funct in [0x22, 0x23] {
        assert_eq!(
            run(&[encode_r(0, 8, 9, 10, 0, funct)], &[(8, 10), (9, 12)]).read_reg(10),
            0xfffffffe
        );
    }
}
#[test]
fn immediate_add_sign_extension() {
    for op in [8, 9] {
        assert_eq!(
            run(&[i(op, 8, 10, 0xffff)], &[(8, 0)]).read_reg(10),
            0xffffffff
        );
    }
}
#[test]
fn comparison_signed_unsigned() {
    for (a, b, s, u) in [
        (0xffffffff, 1, 1, 0),
        (1, 0xffffffff, 0, 1),
        (10, 10, 0, 0),
        (10, 20, 1, 1),
    ] {
        let cpu = run(
            &[
                encode_r(0, 8, 9, 10, 0, 0x2a),
                encode_r(0, 8, 9, 11, 0, 0x2b),
            ],
            &[(8, a), (9, b)],
        );
        assert_eq!(cpu.read_reg(10), s);
        assert_eq!(cpu.read_reg(11), u);
    }
}
#[test]
fn shifts() {
    for (funct, expected) in [(0, 0x20), (2, 0x20000002), (3, 0xe0000002)] {
        assert_eq!(
            run(&[encode_r(0, 0, 8, 9, 2, funct)], &[(8, 0x80000008)]).read_reg(9),
            expected
        );
    }
    for funct in [0, 2, 3] {
        assert_eq!(
            run(&[encode_r(0, 0, 8, 9, 0, funct)], &[(8, 0x80000008)]).read_reg(9),
            0x80000008
        );
    }
}
#[test]
fn zero_register() {
    let cpu = run(&[i(9, 0, 0, 123)], &[(0, 42)]);
    assert_eq!(cpu.read_reg(0), 0);
}
#[test]
fn unsigned_wrap() {
    assert_eq!(
        run(&[encode_r(0, 8, 9, 10, 0, 0x21)], &[(8, u32::MAX), (9, 1)]).read_reg(10),
        0
    );
}

#[test]
fn decode_current_instructions() {
    use crate::decoder::decode;

    for (word, mnemonic) in [
        (0x0109_5020, "add"),
        (0x0109_5021, "addu"),
        (0x210a_ffff, "addi"),
        (0x250a_ffff, "addiu"),
        (0x0109_5022, "sub"),
        (0x0109_5023, "subu"),
        (0x0109_5024, "and"),
        (0x0109_5025, "or"),
        (0x0109_5026, "xor"),
        (0x0109_5027, "nor"),
        (0x0109_502a, "slt"),
        (0x0109_502b, "sltu"),
        (0x0008_4880, "sll"),
        (0x0008_4882, "srl"),
        (0x0008_4883, "sra"),
    ] {
        assert_eq!(decode(word).map(|inst| inst.mnemonic), Some(mnemonic));
    }
}

#[test]
fn decode_reserved_fields() {
    use crate::decoder::decode;

    for word in [0x0109_5060, 0x0028_4880, 0x0028_4883] {
        assert!(decode(word).is_none(), "{word:#010x}");
    }
    assert_ne!(decode(0x0028_4882).map(|inst| inst.mnemonic), Some("srl"));
}

#[test]
fn machine_word_encodings() {
    use crate::test_helpers::encode_j;

    assert_eq!(encode_r(0, 8, 9, 10, 0, 0x20), 0x0109_5020);
    assert_eq!(encode_r(0x1c, 8, 9, 10, 0, 2), 0x7109_5002);
    assert_eq!(i(0x09, 8, 10, 0xffff), 0x250a_ffff);
    assert_eq!(encode_j(0x02, 0x0010_0000), 0x0810_0000);
}

#[test]
fn word_boundaries() {
    let cpu = run(
        &[
            encode_r(0, 8, 9, 10, 0, 0x23),
            i(9, 8, 11, 0x8000),
            i(9, 8, 12, 0x7fff),
            encode_r(0, 0, 9, 13, 31, 0),
            encode_r(0, 0, 14, 15, 31, 2),
            encode_r(0, 0, 14, 16, 31, 3),
        ],
        &[(9, 1), (14, 0x8000_0000)],
    );
    assert_eq!(cpu.read_reg(10), u32::MAX);
    assert_eq!(cpu.read_reg(11), 0xffff_8000);
    assert_eq!(cpu.read_reg(12), 0x7fff);
    assert_eq!(cpu.read_reg(13), 0x8000_0000);
    assert_eq!(cpu.read_reg(15), 1);
    assert_eq!(cpu.read_reg(16), u32::MAX);
}

#[test]
fn immediate_logic() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.write_reg(8, 0xFFFFFFFF);
    cpu.write_reg(9, 0x12340000);

    memory.write_word(0, 0x310A8001); // andi
    memory.write_word(4, 0x352B8001); // ori
    memory.write_word(8, 0x390C8001); // xori
    memory.write_word(12, 0x3C0D8001); // lui

    for _ in 0..4 {
        cpu.step(&memory);
    }

    assert_eq!(cpu.read_reg(10), 0x00008001);
    assert_eq!(cpu.read_reg(11), 0x12348001);
    assert_eq!(cpu.read_reg(12), 0xFFFF7FFE);
    assert_eq!(cpu.read_reg(13), 0x80010000);
}

#[test]
fn immediate_comparison() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    memory.write_word(0, 0x2808FFFF); // slti
    memory.write_word(4, 0x2C09FFFF); // sltiu

    cpu.step(&memory);
    cpu.step(&memory);

    assert_eq!(cpu.read_reg(8), 0);
    assert_eq!(cpu.read_reg(9), 1);
}

#[test]
fn variable_shifts_and_rotate() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.write_reg(8, 0x80000007);
    cpu.write_reg(9, 35);

    memory.write_word(0, 0x01285004); // sllv
    memory.write_word(4, 0x01285806); // srlv
    memory.write_word(8, 0x01286007); // srav
    memory.write_word(12, 0x002868C2); // rotr
    memory.write_word(16, 0x01287046); // rotrv

    for _ in 0..5 {
        cpu.step(&memory);
    }

    assert_eq!(cpu.read_reg(10), 0x00000038);
    assert_eq!(cpu.read_reg(11), 0x10000000);
    assert_eq!(cpu.read_reg(12), 0xF0000000);
    assert_eq!(cpu.read_reg(13), 0xF0000000);
    assert_eq!(cpu.read_reg(14), 0xF0000000);
}

#[test]
fn bit_manipulation() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.write_reg(8, 0x00008080);
    cpu.write_reg(9, 0xFFF08080);
    cpu.write_reg(10, 0x12345678);

    memory.write_word(0, 0x710B5820); // clz
    memory.write_word(4, 0x712C6021); // clo
    memory.write_word(8, 0x7C086C20); // seb
    memory.write_word(12, 0x7C087620); // seh
    memory.write_word(16, 0x7C0A78A0); // wsbh

    for _ in 0..5 {
        cpu.step(&memory);
    }

    assert_eq!(cpu.read_reg(11), 16);
    assert_eq!(cpu.read_reg(12), 12);
    assert_eq!(cpu.read_reg(13), 0xFFFFFF80);
    assert_eq!(cpu.read_reg(14), 0xFFFF8080);
    assert_eq!(cpu.read_reg(15), 0x34127856);
}

#[test]
fn leading_bit_boundaries() {
    use crate::alu::compute;
    use crate::isa::Operation;

    assert_eq!(compute(0, 0, &Operation::Clz), 32);
    assert_eq!(compute(u32::MAX, 0, &Operation::Clo), 32);

    assert_eq!(compute(u32::MAX, 0, &Operation::Clz), 0);
    assert_eq!(compute(0, 0, &Operation::Clo), 0);
}

#[test]
fn bit_field_operations() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.write_reg(8, 0x12345678);
    cpu.write_reg(9, 0xFFFFFFFF);

    memory.write_word(0, 0x7D0A3A00); // ext
    memory.write_word(4, 0x7D097A04); // ins

    for _ in 0..2 {
        cpu.step(&memory);
    }

    assert_eq!(cpu.read_reg(10), 0x56);
    assert_eq!(cpu.read_reg(9), 0xFFFF78FF);
}

#[test]
fn bit_field_boundaries() {
    use crate::alu::{extract, insert};

    assert_eq!(extract(0x12345678, 0, 32), 0x12345678);
    assert_eq!(extract(0x80000000, 31, 1), 1);

    assert_eq!(insert(0x12345678, 0, 0, 32), 0x12345678);
    assert_eq!(insert(1, 0, 31, 1), 0x80000000);
}

#[test]
fn multiplication() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.write_reg(8, 0xFFFFFFFF);
    cpu.write_reg(9, 2);

    memory.write_word(0, 0x71095002); // mul
    memory.write_word(4, 0x01090018); // mult
    memory.write_word(8, 0x00005810); // mfhi
    memory.write_word(12, 0x00006012); // mflo
    memory.write_word(16, 0x01090019); // multu
    memory.write_word(20, 0x00006810); // mfhi
    memory.write_word(24, 0x00007012); // mflo

    for _ in 0..7 {
        cpu.step(&memory);
    }

    assert_eq!(cpu.read_reg(10), 0xFFFFFFFE);

    assert_eq!(cpu.read_reg(11), 0xFFFFFFFF);
    assert_eq!(cpu.read_reg(12), 0xFFFFFFFE);

    assert_eq!(cpu.read_reg(13), 1);
    assert_eq!(cpu.read_reg(14), 0xFFFFFFFE);
}

#[test]
fn division() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.write_reg(8, (-17i32) as u32);
    cpu.write_reg(9, 5);

    memory.write_word(0, 0x0109001A); // div
    memory.write_word(4, 0x00005010); // mfhi
    memory.write_word(8, 0x00005812); // mflo
    memory.write_word(12, 0x0109001B); // divu
    memory.write_word(16, 0x00006010); // mfhi
    memory.write_word(20, 0x00006812); // mflo

    for _ in 0..6 {
        cpu.step(&memory);
    }

    assert_eq!(cpu.read_reg(10), (-2i32) as u32);
    assert_eq!(cpu.read_reg(11), (-3i32) as u32);

    assert_eq!(cpu.read_reg(12), 4);
    assert_eq!(cpu.read_reg(13), 858993455);
}

#[test]
fn hilo_transfer() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.write_reg(8, 0x12345678);
    cpu.write_reg(9, 0xABCDEF01);

    memory.write_word(0, 0x01000011); // mthi
    memory.write_word(4, 0x01200013); // mtlo
    memory.write_word(8, 0x00005010); // mfhi
    memory.write_word(12, 0x00005812); // mflo

    for _ in 0..4 {
        cpu.step(&memory);
    }

    assert_eq!(cpu.read_reg(10), 0x12345678);
    assert_eq!(cpu.read_reg(11), 0xABCDEF01);
}

#[test]
fn multiply_accumulate() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.write_reg(8, 0xFFFFFFFF);
    cpu.write_reg(9, 2);

    cpu.write_hilo(0);

    memory.write_word(0, 0x71090000); // madd
    memory.write_word(4, 0x71090001); // maddu
    memory.write_word(8, 0x71090004); // msub
    memory.write_word(12, 0x71090005); // msubu

    cpu.step(&memory);
    assert_eq!(cpu.read_hilo(), 0xFFFFFFFFFFFFFFFE);

    cpu.step(&memory);
    assert_eq!(cpu.read_hilo(), 0x00000001FFFFFFFC);

    cpu.step(&memory);
    assert_eq!(cpu.read_hilo(), 0x00000001FFFFFFFE);

    cpu.step(&memory);
    assert_eq!(cpu.read_hilo(), 0);
}

#[test]
fn accumulate_wrap() {
    use crate::alu::accumulate;

    assert_eq!(accumulate(1, 2, u64::MAX, false, false), 1);

    assert_eq!(accumulate(1, 1, 0, false, true), u64::MAX);
}
