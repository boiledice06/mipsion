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
        cpu.step(&mut memory);
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

    cpu.step(&mut memory);
    cpu.step(&mut memory);

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
        cpu.step(&mut memory);
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
        cpu.step(&mut memory);
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
        cpu.step(&mut memory);
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
        cpu.step(&mut memory);
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
        cpu.step(&mut memory);
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
        cpu.step(&mut memory);
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

    cpu.step(&mut memory);
    assert_eq!(cpu.read_hilo(), 0xFFFFFFFFFFFFFFFE);

    cpu.step(&mut memory);
    assert_eq!(cpu.read_hilo(), 0x00000001FFFFFFFC);

    cpu.step(&mut memory);
    assert_eq!(cpu.read_hilo(), 0x00000001FFFFFFFE);

    cpu.step(&mut memory);
    assert_eq!(cpu.read_hilo(), 0);
}

#[test]
fn accumulate_wrap() {
    use crate::alu::accumulate;

    assert_eq!(accumulate(1, 2, u64::MAX, false, false), 1);

    assert_eq!(accumulate(1, 1, 0, false, true), u64::MAX);
}

#[test]
fn load_store() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.write_reg(8, 256);
    cpu.write_reg(9, 0x12348081);

    memory.write_word(0, 0xAD090000); // sw
    memory.write_word(4, 0x8D0A0000); // lw
    memory.write_word(8, 0x810B0000); // lb
    memory.write_word(12, 0x910C0000); // lbu
    memory.write_word(16, 0x850D0000); // lh
    memory.write_word(20, 0x950E0000); // lhu
    memory.write_word(24, 0xA5090004); // sh
    memory.write_word(28, 0xA1090006); // sb

    for _ in 0..8 {
        cpu.step(&mut memory);
    }

    assert_eq!(cpu.read_reg(10), 0x12348081);
    assert_eq!(cpu.read_reg(11), 0xFFFFFF81);
    assert_eq!(cpu.read_reg(12), 0x00000081);
    assert_eq!(cpu.read_reg(13), 0xFFFF8081);
    assert_eq!(cpu.read_reg(14), 0x00008081);

    assert_eq!(memory.read_halfword(260), 0x8081);
    assert_eq!(memory.read_byte(262), 0x81);
}

#[test]
fn memory_negative_offset() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.write_reg(8, 256);
    cpu.write_reg(9, 0xDEADBEEF);

    memory.write_word(0, 0xAD09FFFC); // sw -4
    memory.write_word(4, 0x8D0AFFFC); // lw -4

    cpu.step(&mut memory);
    cpu.step(&mut memory);

    assert_eq!(memory.read_word(252), 0xDEADBEEF);
    assert_eq!(cpu.read_reg(10), 0xDEADBEEF);
}

#[test]
fn unaligned_load() {
    for offset in 0..4u32 {
        let mut cpu = Cpu::new();
        let mut memory = Memory::new(1024);

        let address = 256 + offset;

        cpu.write_reg(8, address);
        cpu.write_reg(9, 0xDEADBEEF);

        memory.write_byte(address, 0x11);
        memory.write_byte(address + 1, 0x22);
        memory.write_byte(address + 2, 0x33);
        memory.write_byte(address + 3, 0x44);

        memory.write_word(0, 0x89090003); // lwl
        memory.write_word(4, 0x99090000); // lwr

        cpu.step(&mut memory);
        cpu.step(&mut memory);

        assert_eq!(cpu.read_reg(9), 0x44332211, "offset: {offset}");
    }
}

#[test]
fn unaligned_store() {
    for offset in 0..4u32 {
        let mut cpu = Cpu::new();
        let mut memory = Memory::new(1024);

        let address = 256 + offset;

        cpu.write_reg(8, address);
        cpu.write_reg(9, 0x44332211);

        for i in 255..264 {
            memory.write_byte(i, 0xAA);
        }

        memory.write_word(0, 0xA9090003); // swl
        memory.write_word(4, 0xB9090000); // swr

        cpu.step(&mut memory);
        cpu.step(&mut memory);

        assert_eq!(memory.read_byte(address), 0x11);
        assert_eq!(memory.read_byte(address + 1), 0x22);
        assert_eq!(memory.read_byte(address + 2), 0x33);
        assert_eq!(memory.read_byte(address + 3), 0x44);

        assert_eq!(memory.read_byte(address - 1), 0xAA);
        assert_eq!(memory.read_byte(address + 4), 0xAA);
    }
}

#[test]
fn unaligned_partial_word() {
    let mut memory = Memory::new(1024);

    memory.write_word(256, 0x44332211);

    let old = 0xAABBCCDD;

    assert_eq!(memory.read_word_left(257, old), 0x2211CCDD);

    assert_eq!(memory.read_word_right(257, old), 0xAA443322);

    memory.write_word_left(257, 0xAABBCCDD);
    assert_eq!(memory.read_word(256), 0x4433AABB);

    memory.write_word(256, 0x44332211);

    memory.write_word_right(257, 0xAABBCCDD);
    assert_eq!(memory.read_word(256), 0xBBCCDD11);
}

#[test]
fn conditional_move() {
    for (funct, condition, expected) in [
        (0x0B, 1, 100),
        (0x0B, 0, 50),
        (0x0A, 1, 50),
        (0x0A, 0, 100),
        (0x0B, u32::MAX, 100),
        (0x0A, u32::MAX, 50),
    ] {
        let cpu = run(
            &[encode_r(0, 8, 9, 10, 0, funct)],
            &[(8, 100), (9, condition), (10, 50)],
        );

        assert_eq!(cpu.read_reg(10), expected);
    }
}

#[test]
fn conditional_branch() {
    for (opcode, rt, a, b, taken) in [
        (0x04, 9, 10, 10, true),
        (0x04, 9, 10, 20, false),
        (0x05, 9, 10, 20, true),
        (0x05, 9, 10, 10, false),
        (0x06, 0, 0, 0, true),
        (0x06, 0, 1, 0, false),
        (0x06, 0, u32::MAX, 0, true),
        (0x07, 0, 1, 0, true),
        (0x07, 0, 0, 0, false),
        (0x07, 0, u32::MAX, 0, false),
        (0x01, 0, u32::MAX, 0, true),
        (0x01, 0, 0, 0, false),
        (0x01, 1, 0, 0, true),
        (0x01, 1, 1, 0, true),
        (0x01, 1, u32::MAX, 0, false),
    ] {
        let mut cpu = Cpu::new();
        let mut memory = Memory::new(1024);

        cpu.write_reg(8, a);
        cpu.write_reg(9, b);

        memory.write_word(0, i(opcode, 8, rt, 3));
        memory.write_word(4, i(9, 0, 10, 100));
        memory.write_word(8, i(9, 0, 11, 200));
        memory.write_word(16, i(9, 0, 12, 300));

        cpu.step(&mut memory);

        assert_eq!(cpu.read_pc(), 4);
        assert_eq!(cpu.read_next_pc(), if taken { 16 } else { 8 });

        cpu.step(&mut memory);

        assert_eq!(cpu.read_reg(10), 100);
        assert_eq!(cpu.read_pc(), if taken { 16 } else { 8 });

        cpu.step(&mut memory);

        if taken {
            assert_eq!(cpu.read_reg(11), 0);
            assert_eq!(cpu.read_reg(12), 300);
        } else {
            assert_eq!(cpu.read_reg(11), 200);
            assert_eq!(cpu.read_reg(12), 0);
        }
    }
}

#[test]
fn branch_loop() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    memory.write_word(0, i(9, 0, 8, 5));
    memory.write_word(4, i(9, 8, 8, 0xFFFF));
    memory.write_word(8, i(5, 8, 0, 0xFFFE));
    memory.write_word(12, i(9, 9, 9, 1));

    for _ in 0..16 {
        cpu.step(&mut memory);
    }

    assert_eq!(cpu.read_reg(8), 0);
    assert_eq!(cpu.read_reg(9), 5);
    assert_eq!(cpu.read_pc(), 16);
}

#[test]
fn unconditional_jump() {
    use crate::test_helpers::encode_j;

    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    memory.write_word(0, encode_j(2, 4));
    memory.write_word(4, i(9, 0, 8, 100));
    memory.write_word(8, i(9, 0, 9, 200));
    memory.write_word(16, i(9, 0, 10, 300));

    cpu.step(&mut memory);
    assert_eq!(cpu.read_pc(), 4);
    assert_eq!(cpu.read_next_pc(), 16);

    cpu.step(&mut memory);
    assert_eq!(cpu.read_reg(8), 100);
    assert_eq!(cpu.read_pc(), 16);

    cpu.step(&mut memory);
    assert_eq!(cpu.read_reg(9), 0);
    assert_eq!(cpu.read_reg(10), 300);
}

#[test]
fn function_call() {
    use crate::test_helpers::encode_j;

    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    memory.write_word(0, encode_j(3, 8));
    memory.write_word(4, i(9, 0, 8, 10));
    memory.write_word(8, i(9, 0, 10, 30));

    memory.write_word(32, i(9, 8, 9, 10));
    memory.write_word(36, encode_r(0, 31, 0, 0, 0, 8));
    memory.write_word(40, 0x00000000);

    cpu.step(&mut memory);
    assert_eq!(cpu.read_reg(31), 8);

    cpu.step(&mut memory);
    assert_eq!(cpu.read_pc(), 32);

    cpu.step(&mut memory);
    assert_eq!(cpu.read_reg(9), 20);

    cpu.step(&mut memory);
    assert_eq!(cpu.read_pc(), 40);
    assert_eq!(cpu.read_next_pc(), 8);

    cpu.step(&mut memory);
    assert_eq!(cpu.read_pc(), 8);

    cpu.step(&mut memory);
    assert_eq!(cpu.read_reg(10), 30);
}

#[test]
fn jump_and_link_register() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.write_reg(8, 32);

    memory.write_word(0, encode_r(0, 8, 0, 9, 0, 9));

    memory.write_word(4, i(9, 0, 10, 100));
    memory.write_word(32, i(9, 0, 11, 200));

    cpu.step(&mut memory);

    assert_eq!(cpu.read_reg(9), 8);
    assert_eq!(cpu.read_pc(), 4);
    assert_eq!(cpu.read_next_pc(), 32);

    cpu.step(&mut memory);
    cpu.step(&mut memory);

    assert_eq!(cpu.read_reg(10), 100);
    assert_eq!(cpu.read_reg(11), 200);
}

#[test]
fn branch_likely() {
    for (opcode, rt, a, b, taken) in [
        (0x14, 9, 10, 10, true),
        (0x14, 9, 10, 20, false),
        (0x15, 9, 10, 20, true),
        (0x15, 9, 10, 10, false),
        (0x16, 0, 0, 0, true),
        (0x16, 0, 1, 0, false),
        (0x17, 0, 1, 0, true),
        (0x17, 0, 0, 0, false),
        (0x01, 2, u32::MAX, 0, true),
        (0x01, 2, 0, 0, false),
        (0x01, 3, 0, 0, true),
        (0x01, 3, u32::MAX, 0, false),
    ] {
        let mut cpu = Cpu::new();
        let mut memory = Memory::new(1024);

        cpu.write_reg(8, a);
        cpu.write_reg(9, b);

        memory.write_word(0, i(opcode, 8, rt, 3));
        memory.write_word(4, i(9, 0, 10, 100));
        memory.write_word(8, i(9, 0, 11, 200));
        memory.write_word(16, i(9, 0, 12, 300));

        cpu.step(&mut memory);

        if taken {
            assert_eq!(cpu.read_pc(), 4);
            assert_eq!(cpu.read_next_pc(), 16);

            cpu.step(&mut memory);

            assert_eq!(cpu.read_reg(10), 100);
            assert_eq!(cpu.read_pc(), 16);

            cpu.step(&mut memory);

            assert_eq!(cpu.read_reg(12), 300);
        } else {
            assert_eq!(cpu.read_pc(), 8);
            assert_eq!(cpu.read_next_pc(), 12);

            cpu.step(&mut memory);

            assert_eq!(cpu.read_reg(10), 0);
            assert_eq!(cpu.read_reg(11), 200);
        }
    }
}

#[test]
fn branch_and_link() {
    for (rt, a, taken, likely) in [
        (0x10, u32::MAX, true, false),
        (0x10, 0, false, false),
        (0x11, 0, true, false),
        (0x11, u32::MAX, false, false),
        (0x12, u32::MAX, true, true),
        (0x12, 0, false, true),
        (0x13, 0, true, true),
        (0x13, u32::MAX, false, true),
    ] {
        let mut cpu = Cpu::new();
        let mut memory = Memory::new(1024);

        cpu.write_reg(8, a);
        cpu.write_reg(31, 0xDEADBEEF);

        memory.write_word(0, i(1, 8, rt, 3));
        memory.write_word(4, i(9, 0, 10, 100));
        memory.write_word(8, i(9, 0, 11, 200));
        memory.write_word(16, i(9, 0, 12, 300));

        cpu.step(&mut memory);

        assert_eq!(cpu.read_reg(31), 8);

        if taken {
            assert_eq!(cpu.read_pc(), 4);
            assert_eq!(cpu.read_next_pc(), 16);

            cpu.step(&mut memory);

            assert_eq!(cpu.read_reg(10), 100);
            assert_eq!(cpu.read_pc(), 16);
        } else if likely {
            assert_eq!(cpu.read_pc(), 8);
            assert_eq!(cpu.read_next_pc(), 12);
            assert_eq!(cpu.read_reg(10), 0);
        } else {
            assert_eq!(cpu.read_pc(), 4);

            cpu.step(&mut memory);

            assert_eq!(cpu.read_reg(10), 100);
            assert_eq!(cpu.read_pc(), 8);
        }
    }
}

#[test]
fn exception_state() {
    use crate::exception::Exception;

    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    memory.write_word(0, 0x00000000);
    cpu.step(&mut memory);

    cpu.raise_exception(Exception::Overflow, None);

    assert_eq!(cpu.cp0().exception_code(), 12);
    assert_eq!(cpu.cp0().read_epc(), 0);
    assert!(cpu.cp0().exception_level());
    assert!(!cpu.cp0().branch_delay());
    assert!(!cpu.is_halted());
    assert_eq!(cpu.read_pc(), 0x8000_0180);
}

#[test]
fn delay_slot_exception() {
    use crate::exception::Exception;

    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    memory.write_word(0, i(4, 0, 0, 3));
    memory.write_word(4, 0x00000000);

    cpu.step(&mut memory);
    cpu.step(&mut memory);

    cpu.raise_exception(Exception::Overflow, None);

    assert_eq!(cpu.cp0().exception_code(), 12);
    assert_eq!(cpu.cp0().read_epc(), 0);
    assert!(cpu.cp0().branch_delay());
}

#[test]
fn nested_exception() {
    use crate::cp0::Cp0;
    use crate::exception::Exception;

    let mut cp0 = Cp0::new();

    cp0.enter_exception(Exception::Overflow, 0x104, true, None);

    cp0.enter_exception(Exception::AddressErrorLoad, 0x200, false, Some(0x123));

    assert_eq!(cp0.read_epc(), 0x100);
    assert!(cp0.branch_delay());
    assert_eq!(cp0.exception_code(), 4);
    assert_eq!(cp0.read_bad_vaddr(), 0x123);
}

#[test]
fn arithmetic_overflow() {
    for (word, a, b) in [
        (encode_r(0, 8, 9, 10, 0, 0x20), 0x7FFFFFFF, 1),
        (encode_r(0, 8, 9, 10, 0, 0x22), 0x80000000, 1),
        (i(8, 8, 10, 1), 0x7FFFFFFF, 0),
        (i(8, 8, 10, 0xFFFF), 0x80000000, 0),
    ] {
        let cpu = run(&[word], &[(8, a), (9, b), (10, 0xDEADBEEF)]);

        assert!(!cpu.is_halted());
        assert_eq!(cpu.read_pc(), 0x8000_0180);
        assert_eq!(cpu.cp0().exception_code(), 12);
        assert_eq!(cpu.cp0().read_epc(), 0);
        assert_eq!(cpu.read_reg(10), 0xDEADBEEF);
    }
}

#[test]
fn system_exception() {
    for (word, code) in [(0x0000000C, 8), (0x0000000D, 9)] {
        let cpu = run(&[word], &[]);

        assert!(!cpu.is_halted());
        assert_eq!(cpu.read_pc(), 0x8000_0180);
        assert_eq!(cpu.cp0().exception_code(), code);
        assert!(cpu.cp0().exception_level());
        assert_eq!(cpu.cp0().read_epc(), 0);
    }
}

#[test]
fn register_trap() {
    for (funct, a, b, expected) in [
        (0x34, 10, 10, true),
        (0x34, 10, 20, false),
        (0x36, 10, 20, true),
        (0x36, 10, 10, false),
        (0x30, 10, 10, true),
        (0x30, u32::MAX, 0, false),
        (0x31, u32::MAX, 0, true),
        (0x31, 0, 1, false),
        (0x32, u32::MAX, 0, true),
        (0x32, 10, 10, false),
        (0x33, 0, u32::MAX, true),
        (0x33, u32::MAX, 0, false),
    ] {
        let cpu = run(&[encode_r(0, 8, 9, 0, 0, funct)], &[(8, a), (9, b)]);

        assert!(!cpu.is_halted());

        if expected {
            assert_eq!(cpu.read_pc(), 0x8000_0180);
        } else {
            assert_eq!(cpu.read_pc(), 4);
        }
        assert_eq!(cpu.cp0().exception_code(), if expected { 13 } else { 0 });
    }
}

#[test]
fn immediate_trap() {
    for (rt, a, imm, expected) in [
        (0x0C, 10, 10, true),
        (0x0C, 10, 20, false),
        (0x0E, 10, 20, true),
        (0x0E, 10, 10, false),
        (0x08, 0, 0xFFFF, true),
        (0x08, u32::MAX, 0, false),
        (0x09, u32::MAX, 0xFFFF, true),
        (0x09, 0x00010000, 0xFFFF, false),
        (0x0A, u32::MAX, 0, true),
        (0x0A, 10, 10, false),
        (0x0B, 0x00010000, 0xFFFF, true),
        (0x0B, u32::MAX, 0xFFFF, false),
    ] {
        let cpu = run(&[i(1, 8, rt, imm)], &[(8, a)]);

        assert!(!cpu.is_halted());

        if expected {
            assert_eq!(cpu.read_pc(), 0x8000_0180);
        } else {
            assert_eq!(cpu.read_pc(), 4);
        }
        assert_eq!(cpu.cp0().exception_code(), if expected { 13 } else { 0 });
    }
}

#[test]
fn arithmetic_delay_slot_exception() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.write_reg(8, 0x7FFFFFFF);
    cpu.write_reg(9, 1);
    cpu.write_reg(10, 123);

    memory.write_word(0, i(4, 0, 0, 3));
    memory.write_word(4, encode_r(0, 8, 9, 10, 0, 0x20));

    cpu.step(&mut memory);
    cpu.step(&mut memory);

    assert!(!cpu.is_halted());
    assert_eq!(cpu.read_pc(), 0x8000_0180);
    assert!(cpu.cp0().branch_delay());
    assert_eq!(cpu.cp0().read_epc(), 0);
    assert_eq!(cpu.cp0().exception_code(), 12);
    assert_eq!(cpu.read_reg(10), 123);
}

#[test]
fn memory_alignment_exception() {
    for (opcode, address, code) in [
        (0x23, 257, 4), // lw
        (0x21, 257, 4), // lh
        (0x25, 257, 4), // lhu
        (0x2B, 258, 5), // sw
        (0x29, 257, 5), // sh
        (0x30, 257, 4), // ll
        (0x38, 257, 5), // sc
    ] {
        let mut cpu = Cpu::new();
        let mut memory = Memory::new(1024);

        cpu.write_reg(8, address);
        cpu.write_reg(9, 0xDEADBEEF);

        memory.write_word(0, i(opcode, 8, 9, 0));
        memory.write_word(256, 0x12345678);

        cpu.step(&mut memory);

        assert!(!cpu.is_halted());
        assert_eq!(cpu.read_pc(), 0x8000_0180);
        assert_eq!(cpu.cp0().exception_code(), code);
        assert_eq!(cpu.cp0().read_bad_vaddr(), address);
        assert_eq!(cpu.cp0().read_epc(), 0);

        assert_eq!(cpu.read_reg(9), 0xDEADBEEF);
        assert_eq!(memory.read_word(256), 0x12345678);
    }
}

#[test]
fn instruction_alignment_exception() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.write_pc(3);
    cpu.step(&mut memory);

    assert!(!cpu.is_halted());
    assert_eq!(cpu.read_pc(), 0x8000_0180);
    assert_eq!(cpu.cp0().exception_code(), 4);
    assert_eq!(cpu.cp0().read_bad_vaddr(), 3);
    assert_eq!(cpu.cp0().read_epc(), 3);
}

#[test]
fn atomic_increment() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.write_reg(8, 256);
    memory.write_word(256, 10);

    memory.write_word(0, i(0x30, 8, 9, 0));
    memory.write_word(4, i(0x09, 9, 9, 1));
    memory.write_word(8, i(0x38, 8, 9, 0));

    cpu.step(&mut memory);
    assert_eq!(cpu.read_reg(9), 10);
    assert!(cpu.check_reservation(256));

    cpu.step(&mut memory);
    assert_eq!(cpu.read_reg(9), 11);

    cpu.step(&mut memory);

    assert_eq!(memory.read_word(256), 11);
    assert_eq!(cpu.read_reg(9), 1);
    assert!(!cpu.check_reservation(256));
}

#[test]
fn atomic_store_failure() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.write_reg(8, 256);
    cpu.write_reg(10, 99);

    memory.write_word(256, 10);

    memory.write_word(0, i(0x30, 8, 9, 0));
    memory.write_word(4, i(0x2B, 8, 10, 0));
    memory.write_word(8, i(0x38, 8, 9, 0));

    for _ in 0..3 {
        cpu.step(&mut memory);
    }

    assert_eq!(memory.read_word(256), 99);
    assert_eq!(cpu.read_reg(9), 0);
    assert!(!cpu.check_reservation(256));
}

#[test]
fn store_conditional_without_ll() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.write_reg(8, 256);
    cpu.write_reg(9, 123);

    memory.write_word(256, 456);
    memory.write_word(0, i(0x38, 8, 9, 0));

    cpu.step(&mut memory);

    assert_eq!(memory.read_word(256), 456);
    assert_eq!(cpu.read_reg(9), 0);
}

#[test]
fn decode_synchronization() {
    use crate::decoder::decode;

    for (word, mnemonic) in [
        (0x00000040, "ssnop"),
        (0x000000C0, "ehb"),
        (0x00000140, "pause"),
        (0x0000000F, "sync"),
        (0x051F0000, "synci"),
        (0xCD000000, "pref"),
        (0x01000408, "jr.hb"),
        (0x01004C09, "jalr.hb"),
    ] {
        assert_eq!(
            decode(word).map(|inst| inst.mnemonic),
            Some(mnemonic),
            "word: {word:#010x}"
        );
    }
}

#[test]
fn synchronization_operations() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.write_reg(8, 256);
    cpu.write_reg(9, 0xDEADBEEF);

    memory.write_word(256, 0x12345678);

    let words = [
        0x00000040, // ssnop
        0x000000C0, // ehb
        0x00000140, // pause
        0x0000000F, // sync
        0x051F0000, // synci
        0xCD000000, // pref
    ];

    for (index, word) in words.into_iter().enumerate() {
        memory.write_word((index * 4) as u32, word);
    }

    for _ in 0..6 {
        cpu.step(&mut memory);
    }

    assert_eq!(cpu.read_pc(), 24);
    assert_eq!(cpu.read_reg(8), 256);
    assert_eq!(cpu.read_reg(9), 0xDEADBEEF);
    assert_eq!(memory.read_word(256), 0x12345678);
    assert!(!cpu.is_halted());
}

#[test]
fn hazard_barrier_jump() {
    for (word, link) in [(0x01000408, false), (0x01004C09, true)] {
        let mut cpu = Cpu::new();
        let mut memory = Memory::new(1024);

        cpu.write_reg(8, 32);
        cpu.write_reg(9, 0xDEADBEEF);

        memory.write_word(0, word);
        memory.write_word(4, i(9, 0, 10, 100));
        memory.write_word(32, i(9, 0, 11, 200));

        cpu.step(&mut memory);

        assert_eq!(cpu.read_pc(), 4);
        assert_eq!(cpu.read_next_pc(), 32);

        assert_eq!(cpu.read_reg(9), if link { 8 } else { 0xDEADBEEF });

        cpu.step(&mut memory);

        assert_eq!(cpu.read_reg(10), 100);
        assert_eq!(cpu.read_pc(), 32);

        cpu.step(&mut memory);

        assert_eq!(cpu.read_reg(11), 200);
        assert!(!cpu.is_halted());
    }
}

#[test]
fn hardware_register_read() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    let words = [
        encode_r(0x1F, 0, 8, 0, 0, 0x3B),
        encode_r(0x1F, 0, 9, 1, 0, 0x3B),
        encode_r(0x1F, 0, 10, 2, 0, 0x3B),
        encode_r(0x1F, 0, 11, 3, 0, 0x3B),
        encode_r(0x1F, 0, 12, 29, 0, 0x3B),
    ];

    for (index, word) in words.into_iter().enumerate() {
        memory.write_word((index * 4) as u32, word);
    }

    for _ in 0..5 {
        cpu.step(&mut memory);
    }

    assert_eq!(cpu.read_reg(8), 0);
    assert_eq!(cpu.read_reg(9), 0);
    assert_eq!(cpu.read_reg(10), 3);
    assert_eq!(cpu.read_reg(11), 1);
    assert_eq!(cpu.read_reg(12), 0);

    assert_eq!(cpu.cp0().read_count(), 5);
    assert!(!cpu.is_halted());
}

#[test]
fn invalid_hardware_register() {
    let cpu = run(&[encode_r(0x1F, 0, 8, 15, 0, 0x3B)], &[(8, 0xDEADBEEF)]);

    assert!(!cpu.is_halted());
    assert_eq!(cpu.read_pc(), 0x8000_0180);
    assert_eq!(cpu.cp0().exception_code(), 10);
    assert_eq!(cpu.cp0().read_epc(), 0);
    assert_eq!(cpu.read_reg(8), 0xDEADBEEF);
}

fn encode_cp0(rs: u32, rt: u32, rd: u32, sel: u32) -> u32 {
    assert!(sel < 8);
    encode_r(0x10, rs, rt, rd, 0, sel)
}

#[test]
fn decode_cp0_transfer() {
    use crate::decoder::decode;

    for (word, mnemonic) in [
        (0x40086000, "mfc0"),
        (0x40886000, "mtc0"),
        (0x40082002, "mfc0"),
        (0x40882002, "mtc0"),
    ] {
        assert_eq!(decode(word).map(|inst| inst.mnemonic), Some(mnemonic));
    }
}

#[test]
fn cp0_register_transfer() {
    let cpu = run(
        &[
            encode_cp0(4, 8, 12, 0),
            encode_cp0(0, 9, 12, 0),
            encode_cp0(4, 10, 14, 0),
            encode_cp0(0, 11, 14, 0),
        ],
        &[(8, 0x0000_FF01), (10, 0x8000_1000)],
    );

    assert_eq!(cpu.read_reg(9), 0x0000_FF01);
    assert_eq!(cpu.read_reg(11), 0x8000_1000);
    assert!(!cpu.is_halted());
}

#[test]
fn cp0_write_mask() {
    let cpu = run(
        &[
            encode_cp0(4, 8, 12, 0),
            encode_cp0(0, 9, 12, 0),
            encode_cp0(4, 8, 13, 0),
            encode_cp0(0, 10, 13, 0),
            encode_cp0(4, 8, 7, 0),
            encode_cp0(0, 11, 7, 0),
        ],
        &[(8, u32::MAX)],
    );

    assert_eq!(cpu.read_reg(9), 0x3040_FF1F);
    assert_eq!(cpu.read_reg(10), 0x0080_0300);
    assert_eq!(cpu.read_reg(11), 0x2000_000F);
    assert!(!cpu.is_halted());
}

#[test]
fn cp0_read_only_register() {
    let cpu = run(
        &[encode_cp0(4, 8, 8, 0), encode_cp0(0, 9, 8, 0)],
        &[(8, 0xDEADBEEF)],
    );

    assert_eq!(cpu.read_reg(9), 0);
    assert_eq!(cpu.cp0().read_bad_vaddr(), 0);
    assert!(!cpu.is_halted());
}

#[test]
fn cp0_user_local_transfer() {
    let cpu = run(
        &[
            encode_cp0(4, 8, 4, 2),
            encode_cp0(0, 9, 4, 2),
            encode_r(0x1F, 0, 10, 29, 0, 0x3B),
        ],
        &[(8, 0x1234_5678)],
    );

    assert_eq!(cpu.read_reg(9), 0x1234_5678);
    assert_eq!(cpu.read_reg(10), 0x1234_5678);
    assert!(!cpu.is_halted());
}

#[test]
fn cp0_count_transfer() {
    let cpu = run(
        &[encode_cp0(4, 8, 9, 0), encode_cp0(0, 9, 9, 0)],
        &[(8, 100)],
    );

    assert_eq!(cpu.read_reg(9), 101);
    assert_eq!(cpu.cp0().read_count(), 101);
    assert!(!cpu.is_halted());
}

#[test]
fn invalid_cp0_register() {
    for word in [encode_cp0(0, 8, 31, 7), encode_cp0(4, 8, 31, 7)] {
        let cpu = run(&[word], &[(8, 0xDEADBEEF)]);

        assert!(!cpu.is_halted());
        assert_eq!(cpu.read_pc(), 0x8000_0180);
        assert_eq!(cpu.cp0().exception_code(), 10);
        assert_eq!(cpu.cp0().read_epc(), 0);
        assert_eq!(cpu.read_reg(8), 0xDEADBEEF);
    }
}

#[test]
fn decode_interrupt_control() {
    use crate::decoder::decode;

    for (word, mnemonic) in [
        (0x41606000, "di"),
        (0x41606020, "ei"),
        (0x41686000, "di"),
        (0x41686020, "ei"),
    ] {
        assert_eq!(decode(word).map(|inst| inst.mnemonic), Some(mnemonic));
    }
}

#[test]
fn enable_interrupt() {
    let cpu = run(
        &[
            encode_cp0(4, 8, 12, 0),
            0x41696020, // ei $t1
        ],
        &[(8, 0x0000_FF00)],
    );

    assert_eq!(cpu.read_reg(9), 0x0000_FF00);
    assert_eq!(cpu.cp0().read_status(), 0x0000_FF01);
    assert!(cpu.cp0().interrupt_enabled());
    assert!(!cpu.is_halted());
}

#[test]
fn disable_interrupt() {
    let cpu = run(
        &[
            encode_cp0(4, 8, 12, 0),
            0x41696000, // di $t1
        ],
        &[(8, 0x0000_FF01)],
    );

    assert_eq!(cpu.read_reg(9), 0x0000_FF01);
    assert_eq!(cpu.cp0().read_status(), 0x0000_FF00);
    assert!(!cpu.cp0().interrupt_enabled());
    assert!(!cpu.is_halted());
}
#[test]
fn interrupt_control_zero_register() {
    for (word, status, expected) in [(0x41606020, 0xFF00, 0xFF01), (0x41606000, 0xFF01, 0xFF00)] {
        let cpu = run(&[encode_cp0(4, 8, 12, 0), word], &[(8, status)]);

        assert_eq!(cpu.read_reg(0), 0);
        assert_eq!(cpu.cp0().read_status(), expected);
        assert!(!cpu.is_halted());
    }
}

#[test]
fn repeated_interrupt_control() {
    let cpu = run(
        &[
            encode_cp0(4, 8, 12, 0),
            0x41696020, // ei $t1
            0x416A6020, // ei $t2
            0x416B6000, // di $t3
            0x416C6000, // di $t4
        ],
        &[(8, 0x0040_FF06)],
    );

    assert_eq!(cpu.read_reg(9), 0x0040_FF06);
    assert_eq!(cpu.read_reg(10), 0x0040_FF07);
    assert_eq!(cpu.read_reg(11), 0x0040_FF07);
    assert_eq!(cpu.read_reg(12), 0x0040_FF06);

    assert_eq!(cpu.cp0().read_status(), 0x0040_FF06);
    assert!(!cpu.is_halted());
}

#[test]
fn general_exception_vector() {
    let cpu = run(&[0x0000000C], &[]);

    assert_eq!(cpu.read_pc(), 0x8000_0180);
    assert_eq!(cpu.read_next_pc(), 0x8000_0184);

    assert_eq!(cpu.cp0().read_epc(), 0);
    assert_eq!(cpu.cp0().exception_code(), 8);
    assert!(cpu.cp0().exception_level());

    assert!(!cpu.is_halted());
}

#[test]
fn bootstrap_exception_vector() {
    let cpu = run(&[encode_cp0(4, 8, 12, 0), 0x0000000C], &[(8, 1 << 22)]);

    assert_eq!(cpu.read_pc(), 0xBFC0_0380);
    assert_eq!(cpu.read_next_pc(), 0xBFC0_0384);

    assert_eq!(cpu.cp0().read_epc(), 4);
    assert!(cpu.cp0().exception_level());
    assert!(!cpu.is_halted());
}

#[test]
fn relocated_exception_vector() {
    let cpu = run(
        &[encode_cp0(4, 8, 15, 1), encode_cp0(0, 9, 15, 1), 0x0000000C],
        &[(8, 0x8000_2000)],
    );

    assert_eq!(cpu.read_reg(9), 0x8000_2000);
    assert_eq!(cpu.cp0().read_ebase(), 0x8000_2000);

    assert_eq!(cpu.read_pc(), 0x8000_2180);
    assert_eq!(cpu.cp0().read_epc(), 8);
}

#[test]
fn ebase_write_mask() {
    let cpu = run(
        &[encode_cp0(4, 8, 15, 1), encode_cp0(0, 9, 15, 1)],
        &[(8, u32::MAX)],
    );

    assert_eq!(cpu.read_reg(9), 0xBFFF_F000);
    assert!(!cpu.is_halted());
}

#[test]
fn memory_segment_alias() {
    use crate::cp0::PrivilegeMode;
    use crate::mmu::{AccessType, Mmu};

    let mut memory = Memory::new(1024);

    let a = Mmu::translate(0x8000_0180, PrivilegeMode::Kernel, AccessType::Load).unwrap();

    let b = Mmu::translate(0xA000_0180, PrivilegeMode::Kernel, AccessType::Store).unwrap();

    assert_eq!(a, 0x180);
    assert_eq!(b, 0x180);

    memory.write_word(a, 0x1234_5678);
    assert_eq!(memory.read_word(b), 0x1234_5678);

    memory.write_word(b, 0xDEAD_BEEF);
    assert_eq!(memory.read_word(a), 0xDEAD_BEEF);
}

#[test]
fn exception_handler_execution() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    memory.write_word(0, 0x0000000C);

    // Exception Handler
    memory.write_word(0x180, encode_cp0(0, 8, 14, 0));

    memory.write_word(0x184, encode_cp0(0, 9, 13, 0));

    cpu.step(&mut memory);

    assert_eq!(cpu.read_pc(), 0x8000_0180);

    cpu.step(&mut memory);
    cpu.step(&mut memory);

    assert_eq!(cpu.read_reg(8), 0);
    assert_eq!(cpu.read_reg(9), 8 << 2);

    assert_eq!(cpu.read_pc(), 0x8000_0188);
    assert!(!cpu.is_halted());
}

#[test]
fn delay_slot_exception_vector() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    memory.write_word(0, i(4, 0, 0, 3));
    memory.write_word(4, 0x0000000C);

    cpu.step(&mut memory);
    cpu.step(&mut memory);

    assert_eq!(cpu.read_pc(), 0x8000_0180);
    assert_eq!(cpu.read_next_pc(), 0x8000_0184);

    assert!(cpu.cp0().branch_delay());
    assert_eq!(cpu.cp0().read_epc(), 0);

    cpu.step(&mut memory);

    assert_eq!(cpu.read_pc(), 0x8000_0184);
    assert!(!cpu.is_halted());
}

#[test]
fn nested_exception_preserves_epc() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    memory.write_word(4, 0x0000000C);
    memory.write_word(0x180, 0x0000000D);

    cpu.write_pc(4);

    cpu.step(&mut memory);

    assert_eq!(cpu.cp0().read_epc(), 4);
    assert_eq!(cpu.cp0().exception_code(), 8);

    cpu.step(&mut memory);

    assert_eq!(cpu.cp0().read_epc(), 4);
    assert_eq!(cpu.cp0().exception_code(), 9);

    assert_eq!(cpu.read_pc(), 0x8000_0180);
    assert!(cpu.cp0().exception_level());
    assert!(!cpu.is_halted());
}

#[test]
fn decode_eret() {
    use crate::decoder::decode;

    assert_eq!(decode(0x42000018).map(|inst| inst.mnemonic), Some("eret"));

    assert_ne!(decode(0x42010018).map(|inst| inst.mnemonic), Some("eret"));
}

#[test]
fn error_epc_transfer() {
    let cpu = run(
        &[encode_cp0(4, 8, 30, 0), encode_cp0(0, 9, 30, 0)],
        &[(8, 0x8000_1000)],
    );

    assert_eq!(cpu.read_reg(9), 0x8000_1000);
    assert_eq!(cpu.cp0().read_error_epc(), 0x8000_1000);
    assert!(!cpu.is_halted());
}

#[test]
fn exception_return() {
    let cpu = run(
        &[encode_cp0(4, 8, 14, 0), encode_cp0(4, 9, 12, 0), 0x42000018],
        &[(8, 0x100), (9, 0x0000_FF03)],
    );

    assert_eq!(cpu.read_pc(), 0x100);
    assert_eq!(cpu.read_next_pc(), 0x104);

    assert_eq!(cpu.cp0().read_status(), 0x0000_FF01);
    assert!(!cpu.cp0().exception_level());
    assert!(!cpu.is_halted());
}

#[test]
fn error_level_return() {
    let cpu = run(
        &[
            encode_cp0(4, 8, 14, 0),
            encode_cp0(4, 9, 30, 0),
            encode_cp0(4, 10, 12, 0),
            0x42000018,
        ],
        &[(8, 0x100), (9, 0x200), (10, 0x0000_0006)],
    );

    assert_eq!(cpu.read_pc(), 0x200);
    assert_eq!(cpu.read_next_pc(), 0x204);

    assert!(!cpu.cp0().error_level());
    assert!(cpu.cp0().exception_level());

    assert_eq!(cpu.cp0().read_status(), 2);
    assert!(!cpu.is_halted());
}

#[test]
fn eret_without_delay_slot() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.cp0_mut().write_reg(14, 0, 0x100);
    cpu.cp0_mut().write_reg(12, 0, 2);

    memory.write_word(0, 0x42000018);

    memory.write_word(4, i(9, 0, 8, 100));
    memory.write_word(0x100, i(9, 0, 9, 200));

    cpu.step(&mut memory);

    assert_eq!(cpu.read_pc(), 0x100);

    cpu.step(&mut memory);

    assert_eq!(cpu.read_reg(8), 0);
    assert_eq!(cpu.read_reg(9), 200);

    assert_eq!(cpu.read_pc(), 0x104);
}
#[test]
fn eret_clears_reservation() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.cp0_mut().write_reg(14, 0, 0x100);
    cpu.cp0_mut().write_reg(12, 0, 2);

    cpu.set_reservation(256);

    memory.write_word(0, 0x42000018);

    assert!(cpu.check_reservation(256));

    cpu.step(&mut memory);

    assert!(!cpu.check_reservation(256));
    assert_eq!(cpu.read_pc(), 0x100);
}

#[test]
fn complete_exception_cycle() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    // Main Program
    memory.write_word(0x000, i(9, 0, 8, 100));
    memory.write_word(0x004, 0x0000000C);
    memory.write_word(0x008, i(9, 0, 9, 200));

    // Exception Handler
    memory.write_word(0x180, encode_cp0(0, 26, 14, 0));

    memory.write_word(0x184, i(9, 26, 26, 4));

    memory.write_word(0x188, encode_cp0(4, 26, 14, 0));

    memory.write_word(0x18C, 0x42000018);

    // Main Program 실행
    cpu.step(&mut memory);

    assert_eq!(cpu.read_reg(8), 100);

    // SYSCALL
    cpu.step(&mut memory);

    assert_eq!(cpu.read_pc(), 0x8000_0180);
    assert_eq!(cpu.cp0().read_epc(), 4);
    assert!(cpu.cp0().exception_level());

    // Exception Handler 실행
    for _ in 0..4 {
        cpu.step(&mut memory);
    }

    assert_eq!(cpu.read_pc(), 8);
    assert_eq!(cpu.cp0().read_epc(), 8);
    assert!(!cpu.cp0().exception_level());

    // Main Program 복귀
    cpu.step(&mut memory);

    assert_eq!(cpu.read_reg(8), 100);
    assert_eq!(cpu.read_reg(9), 200);
    assert_eq!(cpu.read_pc(), 12);

    assert!(!cpu.is_halted());
}

#[test]
fn privilege_mode_selection() {
    use crate::cp0::{Cp0, PrivilegeMode};

    let mut cp0 = Cp0::new();

    for (status, expected) in [
        (0x00, PrivilegeMode::Kernel),
        (0x08, PrivilegeMode::Supervisor),
        (0x10, PrivilegeMode::User),
        (0x18, PrivilegeMode::Reserved),
    ] {
        cp0.write_reg(12, 0, status);

        assert_eq!(cp0.privilege_mode(), expected);
    }
}

#[test]
fn exception_privilege_override() {
    use crate::cp0::{Cp0, PrivilegeMode};

    let mut cp0 = Cp0::new();

    for status in [0x12, 0x14, 0x16, 0x0A, 0x0C] {
        cp0.write_reg(12, 0, status);

        assert_eq!(cp0.privilege_mode(), PrivilegeMode::Kernel);
    }
}

#[test]
fn user_cp0_access_denied() {
    for word in [
        encode_cp0(0, 8, 12, 0), // mfc0
        encode_cp0(4, 8, 12, 0), // mtc0
        0x41686020,              // ei
        0x41686000,              // di
        0x42000018,              // eret
    ] {
        let mut cpu = Cpu::new();
        let mut memory = Memory::new(1024);

        cpu.cp0_mut().write_reg(12, 0, 0x10);

        cpu.write_reg(8, 0xDEAD_BEEF);
        memory.write_word(0, word);

        cpu.step(&mut memory);

        assert_eq!(cpu.cp0().exception_code(), 11);
        assert_eq!(cpu.read_pc(), 0x8000_0180);
        assert_eq!(cpu.cp0().read_epc(), 0);
        assert_eq!(cpu.read_reg(8), 0xDEAD_BEEF);

        assert_eq!((cpu.cp0().read_cause() >> 28) & 3, 0);

        assert!(cpu.cp0().is_kernel_mode());
        assert!(!cpu.is_halted());
    }
}

#[test]
fn user_cp0_access_enabled() {
    use crate::cp0::PrivilegeMode;

    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.cp0_mut().write_reg(12, 0, 0x1000_0010);

    memory.write_word(0, encode_cp0(0, 8, 12, 0));

    cpu.step(&mut memory);

    assert_eq!(cpu.read_reg(8), 0x1000_0010);
    assert_eq!(cpu.read_pc(), 4);

    assert_eq!(cpu.cp0().privilege_mode(), PrivilegeMode::User);

    assert!(!cpu.cp0().exception_level());
}

#[test]
fn supervisor_cp0_access() {
    for (status, allowed) in [(0x0000_0008, false), (0x1000_0008, true)] {
        let mut cpu = Cpu::new();
        let mut memory = Memory::new(1024);

        cpu.cp0_mut().write_reg(12, 0, status);

        memory.write_word(0, encode_cp0(0, 8, 12, 0));

        cpu.step(&mut memory);

        if allowed {
            assert_eq!(cpu.read_reg(8), status);
            assert_eq!(cpu.read_pc(), 4);
        } else {
            assert_eq!(cpu.cp0().exception_code(), 11);
            assert_eq!(cpu.read_pc(), 0x8000_0180);
        }
    }
}

#[test]
fn user_rdhwr_access_denied() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.cp0_mut().write_reg(12, 0, 0x10);

    cpu.write_reg(8, 0xDEAD_BEEF);

    memory.write_word(0, encode_r(0x1F, 0, 8, 2, 0, 0x3B));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp0().exception_code(), 10);
    assert_eq!(cpu.read_reg(8), 0xDEAD_BEEF);
    assert_eq!(cpu.read_pc(), 0x8000_0180);
}

#[test]
fn user_rdhwr_access_enabled() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.cp0_mut().write_reg(12, 0, 0x10);
    cpu.cp0_mut().write_hwrena(1 << 2);

    memory.write_word(0, encode_r(0x1F, 0, 8, 2, 0, 0x3B));

    cpu.step(&mut memory);

    assert_eq!(cpu.read_reg(8), 1);
    assert_eq!(cpu.read_pc(), 4);
    assert!(!cpu.cp0().exception_level());
}

#[test]
fn user_kernel_user_transition() {
    use crate::cp0::PrivilegeMode;

    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    // User Mode
    cpu.cp0_mut().write_reg(12, 0, 0x10);

    // User Program
    memory.write_word(0, 0x0000000C);
    memory.write_word(4, i(9, 0, 8, 100));

    // Exception Handler
    memory.write_word(0x180, encode_cp0(0, 26, 14, 0));

    memory.write_word(0x184, i(9, 26, 26, 4));

    memory.write_word(0x188, encode_cp0(4, 26, 14, 0));

    memory.write_word(0x18C, 0x42000018);

    assert_eq!(cpu.cp0().privilege_mode(), PrivilegeMode::User);

    // SYSCALL
    cpu.step(&mut memory);

    assert_eq!(cpu.cp0().privilege_mode(), PrivilegeMode::Kernel);

    assert_eq!(cpu.read_pc(), 0x8000_0180);
    assert!(cpu.cp0().exception_level());

    // Exception Handler 실행
    for _ in 0..4 {
        cpu.step(&mut memory);
    }

    // ERET 이후 User Mode 복귀
    assert_eq!(cpu.cp0().privilege_mode(), PrivilegeMode::User);

    assert_eq!(cpu.read_pc(), 4);
    assert!(!cpu.cp0().exception_level());

    cpu.step(&mut memory);

    assert_eq!(cpu.read_reg(8), 100);
    assert_eq!(cpu.read_pc(), 8);
}
#[test]
fn compare_register_transfer() {
    let cpu = run(
        &[
            encode_cp0(4, 8, 11, 0),
            encode_cp0(0, 9, 11, 0),
            encode_cp0(0, 10, 12, 1),
        ],
        &[(8, 100)],
    );

    assert_eq!(cpu.read_reg(9), 100);
    assert_eq!(cpu.cp0().read_compare(), 100);

    assert_eq!(cpu.read_reg(10) >> 29, 7);

    assert!(!cpu.cp0().timer_interrupt_pending());
}

#[test]
fn timer_interrupt_pending() {
    use crate::cp0::Cp0;

    let mut cp0 = Cp0::new();

    cp0.write_compare(3);

    cp0.tick();
    cp0.tick();

    assert!(!cp0.timer_interrupt_pending());

    cp0.tick();

    assert_eq!(cp0.read_count(), 3);
    assert!(cp0.timer_interrupt_pending());
    assert_eq!(cp0.interrupt_pending(), 0x80);

    cp0.tick();

    assert_eq!(cp0.read_count(), 4);
    assert!(cp0.timer_interrupt_pending());
}

#[test]
fn compare_write_clears_interrupt() {
    use crate::cp0::Cp0;

    let mut cp0 = Cp0::new();

    cp0.write_compare(1);
    cp0.tick();

    assert!(cp0.timer_interrupt_pending());

    cp0.write_reg(13, 0, 0x100);
    cp0.write_compare(10);

    assert!(!cp0.timer_interrupt_pending());
    assert_eq!(cp0.interrupt_pending(), 1);
    assert_eq!(cp0.read_compare(), 10);
}

#[test]
fn interrupt_acceptance_conditions() {
    use crate::cp0::Cp0;

    let mut cp0 = Cp0::new();

    cp0.write_compare(1);
    cp0.tick();

    for (status, expected) in [
        (0x0000, false),
        (0x0001, false),
        (0x8000, false),
        (0x8001, true),
        (0x8003, false),
        (0x8005, false),
        (0x4001, false),
    ] {
        cp0.write_reg(12, 0, status);

        assert_eq!(cp0.interrupt_requested(), expected);

        assert!(cp0.timer_interrupt_pending());
    }
}

#[test]
fn timer_interrupt_delivery() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.cp0_mut().write_compare(2);
    cpu.cp0_mut().write_reg(12, 0, 0x8001);

    memory.write_word(0, i(9, 0, 8, 100));
    memory.write_word(4, i(9, 0, 9, 200));

    cpu.step(&mut memory);

    assert_eq!(cpu.read_reg(8), 100);
    assert_eq!(cpu.read_pc(), 4);

    cpu.step(&mut memory);

    assert_eq!(cpu.read_pc(), 0x8000_0180);
    assert_eq!(cpu.cp0().exception_code(), 0);
    assert_eq!(cpu.cp0().read_epc(), 4);

    assert_eq!(cpu.read_reg(9), 0);
    assert!(cpu.cp0().exception_level());
    assert!(cpu.cp0().timer_interrupt_pending());
    assert!(!cpu.is_halted());
}

#[test]
fn dedicated_interrupt_vector() {
    for (status, vector) in [(0x0000_8001, 0x8000_0200), (0x0040_8001, 0xBFC0_0400)] {
        let mut cpu = Cpu::new();
        let mut memory = Memory::new(1024);

        cpu.cp0_mut().write_compare(1);
        cpu.cp0_mut().write_reg(12, 0, status);
        cpu.cp0_mut().write_reg(13, 0, 1 << 23);

        cpu.step(&mut memory);

        assert_eq!(cpu.read_pc(), vector);
        assert_eq!(cpu.read_next_pc(), vector + 4);
        assert_eq!(cpu.cp0().read_epc(), 0);
        assert!(cpu.cp0().exception_level());
    }
}

#[test]
fn software_interrupt_delivery() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.cp0_mut().write_reg(12, 0, 0x0101);
    cpu.cp0_mut().write_reg(13, 0, 0x0100);

    memory.write_word(0, i(9, 0, 8, 100));

    cpu.step(&mut memory);

    assert_eq!(cpu.read_pc(), 0x8000_0180);
    assert_eq!(cpu.cp0().exception_code(), 0);
    assert_eq!(cpu.cp0().read_epc(), 0);

    assert_eq!(cpu.read_reg(8), 0);
    assert!(!cpu.cp0().timer_interrupt_pending());
}

#[test]
fn interrupt_before_delay_slot() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.cp0_mut().write_compare(2);
    cpu.cp0_mut().write_reg(12, 0, 0x8001);

    memory.write_word(0, i(4, 0, 0, 3));
    memory.write_word(4, i(9, 0, 8, 100));

    cpu.step(&mut memory);

    assert_eq!(cpu.read_pc(), 4);
    assert_eq!(cpu.read_next_pc(), 16);

    cpu.step(&mut memory);

    assert_eq!(cpu.read_pc(), 0x8000_0180);
    assert_eq!(cpu.cp0().read_epc(), 0);
    assert!(cpu.cp0().branch_delay());

    assert_eq!(cpu.read_reg(8), 0);
}

#[test]
fn timer_count_wraparound() {
    use crate::cp0::Cp0;

    let mut cp0 = Cp0::new();

    cp0.write_reg(9, 0, u32::MAX);
    cp0.write_compare(0);

    cp0.tick();

    assert_eq!(cp0.read_count(), 0);
    assert!(cp0.timer_interrupt_pending());
}

#[test]
fn complete_timer_interrupt_cycle() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.cp0_mut().write_compare(2);
    cpu.cp0_mut().write_reg(12, 0, 0x8001);

    // Main Program
    memory.write_word(0, i(9, 0, 8, 100));
    memory.write_word(4, i(9, 0, 9, 200));

    // Timer Interrupt Handler
    memory.write_word(0x180, i(9, 0, 26, 100));

    memory.write_word(0x184, encode_cp0(4, 26, 11, 0));

    memory.write_word(0x188, 0x42000018);

    // Main Program 실행
    cpu.step(&mut memory);

    assert_eq!(cpu.read_reg(8), 100);

    // Timer Interrupt 발생
    cpu.step(&mut memory);

    assert_eq!(cpu.read_pc(), 0x8000_0180);
    assert_eq!(cpu.cp0().read_epc(), 4);
    assert!(cpu.cp0().timer_interrupt_pending());

    // Interrupt Handler 실행
    for _ in 0..3 {
        cpu.step(&mut memory);
    }

    assert_eq!(cpu.read_pc(), 4);
    assert_eq!(cpu.cp0().read_compare(), 100);

    assert!(!cpu.cp0().timer_interrupt_pending());
    assert!(!cpu.cp0().exception_level());

    // 중단된 명령어 실행
    cpu.step(&mut memory);

    assert_eq!(cpu.read_reg(9), 200);
    assert_eq!(cpu.read_pc(), 8);
    assert!(!cpu.is_halted());
}

#[test]
fn mmu_direct_mapping() {
    use crate::cp0::PrivilegeMode::Kernel;
    use crate::mmu::{AccessType, Mmu};

    for (virtual_addr, physical_addr) in [
        (0x8000_0000, 0),
        (0xA000_0000, 0),
        (0x8000_1234, 0x1234),
        (0xA000_1234, 0x1234),
        (0x9FFF_FFFF, 0x1FFF_FFFF),
        (0xBFFF_FFFF, 0x1FFF_FFFF),
    ] {
        assert_eq!(
            Mmu::translate(virtual_addr, Kernel, AccessType::Load,),
            Ok(physical_addr)
        );
    }
}

#[test]
fn mmu_segment_permissions() {
    use crate::cp0::PrivilegeMode::*;
    use crate::mmu::{AccessType, Mmu};

    for (mode, address, allowed) in [
        (Kernel, 0x8000_0000, true),
        (Kernel, 0xE000_0000, true),
        (Supervisor, 0x0000_1000, true),
        (Supervisor, 0x8000_0000, false),
        (Supervisor, 0xA000_0000, false),
        (Supervisor, 0xC000_0000, true),
        (Supervisor, 0xE000_0000, false),
        (User, 0x7FFF_FFFF, true),
        (User, 0x8000_0000, false),
        (User, 0xA000_0000, false),
        (User, 0xC000_0000, false),
        (User, 0xE000_0000, false),
    ] {
        assert_eq!(
            Mmu::translate(address, mode, AccessType::Load,).is_ok(),
            allowed
        );
    }
}

#[test]
fn user_kernel_fetch_denied() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.cp0_mut().write_reg(12, 0, 0x10);
    cpu.write_pc(0x8000_0100);

    memory.write_word(0x100, i(9, 0, 8, 100));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp0().exception_code(), 4);
    assert_eq!(cpu.cp0().read_bad_vaddr(), 0x8000_0100);

    assert_eq!(cpu.cp0().read_epc(), 0x8000_0100);

    assert_eq!(cpu.read_pc(), 0x8000_0180);
    assert_eq!(cpu.read_reg(8), 0);
}

#[test]
fn user_kernel_load_denied() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.cp0_mut().write_reg(12, 0, 0x10);

    cpu.write_reg(8, 0x8000_0100);
    cpu.write_reg(9, 0xDEAD_BEEF);

    memory.write_word(0, i(0x23, 8, 9, 0));
    memory.write_word(0x100, 100);

    cpu.step(&mut memory);

    assert_eq!(cpu.cp0().exception_code(), 4);
    assert_eq!(cpu.cp0().read_bad_vaddr(), 0x8000_0100);

    assert_eq!(cpu.read_reg(9), 0xDEAD_BEEF);
    assert_eq!(cpu.read_pc(), 0x8000_0180);
}

#[test]
fn user_kernel_store_denied() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.cp0_mut().write_reg(12, 0, 0x10);

    cpu.write_reg(8, 0xA000_0100);
    cpu.write_reg(9, 0xDEAD_BEEF);

    memory.write_word(0, i(0x2B, 8, 9, 0));
    memory.write_word(0x100, 100);

    cpu.step(&mut memory);

    assert_eq!(cpu.cp0().exception_code(), 5);
    assert_eq!(cpu.cp0().read_bad_vaddr(), 0xA000_0100);

    assert_eq!(memory.read_word(0x100), 100);
    assert_eq!(cpu.read_pc(), 0x8000_0180);
}

#[test]
fn kernel_segment_load_store() {
    let cpu = run_kernel_alias_program();

    assert_eq!(cpu.read_reg(10), 0xDEAD_BEEF);
    assert!(!cpu.cp0().exception_level());
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

#[test]
fn atomic_segment_alias() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.write_reg(8, 0x8000_0100);
    cpu.write_reg(11, 0xA000_0100);

    memory.write_word(0x100, 10);

    memory.write_word(0, i(0x30, 8, 9, 0));
    memory.write_word(4, i(9, 9, 9, 1));
    memory.write_word(8, i(0x38, 11, 9, 0));

    cpu.step(&mut memory);

    assert!(cpu.check_reservation(0x100));
    assert_eq!(cpu.read_reg(9), 10);

    cpu.step(&mut memory);
    cpu.step(&mut memory);

    assert_eq!(memory.read_word(0x100), 11);
    assert_eq!(cpu.read_reg(9), 1);
    assert!(!cpu.check_reservation(0x100));
}

#[test]
fn mmu_delay_slot_exception() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.cp0_mut().write_reg(12, 0, 0x10);

    cpu.write_reg(8, 0x8000_0100);
    cpu.write_reg(9, 0xDEAD_BEEF);

    memory.write_word(0, i(4, 0, 0, 3));
    memory.write_word(4, i(0x23, 8, 9, 0));

    cpu.step(&mut memory);
    cpu.step(&mut memory);

    assert_eq!(cpu.cp0().exception_code(), 4);
    assert!(cpu.cp0().branch_delay());
    assert_eq!(cpu.cp0().read_epc(), 0);

    assert_eq!(cpu.cp0().read_bad_vaddr(), 0x8000_0100);

    assert_eq!(cpu.read_reg(9), 0xDEAD_BEEF);
    assert_eq!(cpu.read_pc(), 0x8000_0180);
}

#[test]
fn tlb_register_initialization() {
    use crate::cp0::{Cp0, TLB_SIZE};

    let cp0 = Cp0::new();

    assert_eq!(cp0.read_index(), 0);
    assert_eq!(cp0.read_random(), (TLB_SIZE - 1) as u32);

    assert_eq!(cp0.read_entry_lo0(), 0);
    assert_eq!(cp0.read_entry_lo1(), 0);
    assert_eq!(cp0.read_context(), 0);
    assert_eq!(cp0.read_page_mask(), 0);
    assert_eq!(cp0.read_wired(), 0);
    assert_eq!(cp0.read_entry_hi(), 0);
}

#[test]
fn tlb_register_transfer() {
    for (rd, value) in [
        (0, 15),
        (2, 0x0000_0047),
        (3, 0x0000_0087),
        (4, 0x8000_0000),
        (5, 0x0000_6000),
        (6, 4),
        (10, 0x0040_0001),
    ] {
        let cpu = run(
            &[encode_cp0(4, 8, rd, 0), encode_cp0(0, 9, rd, 0)],
            &[(8, value)],
        );

        assert_eq!(cpu.read_reg(9), value, "CP0 Register {rd}");

        assert!(!cpu.cp0().exception_level());
    }
}

#[test]
fn tlb_register_write_mask() {
    use crate::cp0::Cp0;

    let mut cp0 = Cp0::new();

    for (rd, expected) in [
        (0, 0x0000_001F),
        (2, 0x03FF_FFFF),
        (3, 0x03FF_FFFF),
        (4, 0xFF80_0000),
        (5, 0x1FFF_E000),
        (6, 0x0000_001F),
        (10, 0xFFFF_E0FF),
    ] {
        assert!(cp0.write_reg(rd, 0, u32::MAX));

        assert_eq!(cp0.read_reg(rd, 0), Some(expected));
    }
}

#[test]
fn tlb_random_read_only() {
    use crate::cp0::Cp0;

    let mut cp0 = Cp0::new();

    let old = cp0.read_random();

    assert!(cp0.write_reg(1, 0, 5));
    assert_eq!(cp0.read_random(), old);
}

#[test]
fn tlb_random_cycle() {
    use crate::cp0::Cp0;

    let mut cp0 = Cp0::new();

    for expected in (0..31u32).rev() {
        cp0.tick();

        assert_eq!(cp0.read_random(), expected);
    }

    assert_eq!(cp0.read_random(), 0);

    cp0.tick();

    assert_eq!(cp0.read_random(), 31);
}

#[test]
fn tlb_wired_boundary() {
    use crate::cp0::Cp0;

    let mut cp0 = Cp0::new();

    cp0.write_reg(6, 0, 28);

    assert_eq!(cp0.read_wired(), 28);
    assert_eq!(cp0.read_random(), 31);

    for expected in [30, 29, 28, 31, 30] {
        cp0.tick();

        assert_eq!(cp0.read_random(), expected);
    }

    cp0.write_reg(6, 0, 31);

    for _ in 0..10 {
        cp0.tick();

        assert_eq!(cp0.read_random(), 31);
    }
}

#[test]
fn tlb_asid_extraction() {
    use crate::cp0::Cp0;

    let mut cp0 = Cp0::new();

    cp0.write_reg(10, 0, 0x0040_00AB);

    assert_eq!(cp0.asid(), 0xAB);
    assert_eq!(cp0.read_entry_hi(), 0x0040_00AB);
}

#[test]
fn tlb_context_select_independence() {
    let cpu = run(
        &[
            encode_cp0(4, 8, 4, 0),
            encode_cp0(4, 9, 4, 2),
            encode_cp0(0, 10, 4, 0),
            encode_cp0(0, 11, 4, 2),
        ],
        &[(8, 0x8000_0000), (9, 0x1234_5678)],
    );

    assert_eq!(cpu.read_reg(10), 0x8000_0000);
    assert_eq!(cpu.read_reg(11), 0x1234_5678);

    assert!(!cpu.cp0().exception_level());
}

#[test]
fn decode_tlb_instructions() {
    use crate::decoder::decode;

    for (word, mnemonic) in [
        (0x42000001, "tlbr"),
        (0x42000002, "tlbwi"),
        (0x42000006, "tlbwr"),
        (0x42000008, "tlbp"),
    ] {
        assert_eq!(decode(word).map(|inst| inst.mnemonic), Some(mnemonic));
    }
}

#[test]
fn tlb_initialization() {
    use crate::cp0::TLB_SIZE;
    use crate::tlb::Tlb;

    let tlb = Tlb::new();

    for index in 0..TLB_SIZE {
        assert_eq!(tlb.read(index), None);
    }

    assert_eq!(tlb.probe(0, 0), None);
}

#[test]
fn tlb_indexed_write() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.cp0_mut().write_reg(0, 0, 5);
    cpu.cp0_mut().write_reg(10, 0, 0x0040_0001);
    cpu.cp0_mut().write_reg(2, 0, 0x0000_0046);
    cpu.cp0_mut().write_reg(3, 0, 0x0000_0086);
    cpu.cp0_mut().write_reg(5, 0, 0);

    memory.write_word(0, 0x42000002);

    cpu.step(&mut memory);

    let entry = cpu.cp0().tlb().read(5).unwrap();

    assert_eq!(entry.entry_hi, 0x0040_0001);
    assert_eq!(entry.entry_lo0, 0x0000_0046);
    assert_eq!(entry.entry_lo1, 0x0000_0086);
    assert_eq!(entry.page_mask, 0);

    assert_eq!(cpu.cp0().tlb().read(4), None);
    assert!(!cpu.cp0().exception_level());
}

#[test]
fn tlb_indexed_read() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.cp0_mut().write_reg(0, 0, 7);
    cpu.cp0_mut().write_reg(10, 0, 0x0080_0002);
    cpu.cp0_mut().write_reg(2, 0, 0x0000_00C6);
    cpu.cp0_mut().write_reg(3, 0, 0x0000_0106);
    cpu.cp0_mut().write_reg(5, 0, 0x6000);

    memory.write_word(0, 0x42000002);
    memory.write_word(4, 0x42000001);

    // TLBWI
    cpu.step(&mut memory);

    // CP0 Register 초기화
    cpu.cp0_mut().write_reg(10, 0, 0);
    cpu.cp0_mut().write_reg(2, 0, 0);
    cpu.cp0_mut().write_reg(3, 0, 0);
    cpu.cp0_mut().write_reg(5, 0, 0);

    // TLBR
    cpu.step(&mut memory);

    assert_eq!(cpu.cp0().read_entry_hi(), 0x0080_0002);
    assert_eq!(cpu.cp0().read_entry_lo0(), 0x0000_00C6);
    assert_eq!(cpu.cp0().read_entry_lo1(), 0x0000_0106);
    assert_eq!(cpu.cp0().read_page_mask(), 0x6000);
}

#[test]
fn tlb_probe_hit() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.cp0_mut().write_reg(0, 0, 9);
    cpu.cp0_mut().write_reg(10, 0, 0x0040_0003);
    cpu.cp0_mut().write_reg(2, 0, 0x46);
    cpu.cp0_mut().write_reg(3, 0, 0x86);

    memory.write_word(0, 0x42000002);
    memory.write_word(4, 0x42000008);

    cpu.step(&mut memory);

    cpu.cp0_mut().write_reg(0, 0, 0);

    cpu.step(&mut memory);

    assert_eq!(cpu.cp0().read_index(), 9);
    assert_eq!(cpu.cp0().read_index() >> 31, 0);
}

#[test]
fn tlb_probe_asid_miss() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.cp0_mut().write_reg(0, 0, 5);
    cpu.cp0_mut().write_reg(10, 0, 0x0040_0001);
    cpu.cp0_mut().write_reg(2, 0, 0x46);
    cpu.cp0_mut().write_reg(3, 0, 0x86);

    memory.write_word(0, 0x42000002);
    memory.write_word(4, 0x42000008);
    memory.write_word(8, 0x42000008);

    cpu.step(&mut memory);

    // VPN2는 동일하지만 ASID 불일치
    cpu.cp0_mut().write_reg(10, 0, 0x0040_0002);

    cpu.step(&mut memory);

    assert_ne!(cpu.cp0().read_index() & 0x8000_0000, 0);

    // ASID 복원
    cpu.cp0_mut().write_reg(10, 0, 0x0040_0001);

    cpu.step(&mut memory);

    assert_eq!(cpu.cp0().read_index(), 5);
}

#[test]
fn tlb_global_mapping() {
    use crate::tlb::TlbEntry;

    let mut cpu = Cpu::new();

    for (lo0, lo1, global) in [
        (0x47, 0x87, true),
        (0x47, 0x86, false),
        (0x46, 0x87, false),
        (0x46, 0x86, false),
    ] {
        cpu.cp0_mut().write_reg(0, 0, 0);
        cpu.cp0_mut().write_reg(10, 0, 0x0040_0001);
        cpu.cp0_mut().write_reg(2, 0, lo0);
        cpu.cp0_mut().write_reg(3, 0, lo1);

        cpu.cp0_mut().tlb_write_indexed();

        let entry: TlbEntry = cpu.cp0().tlb().read(0).unwrap();

        assert_eq!(entry.global(), global);
        assert_eq!(entry.matches(0x0040_0000, 2), global);

        // 동일 ASID는 항상 일치
        assert!(entry.matches(0x0040_0000, 1));
    }
}

#[test]
fn tlb_page_mask_probe() {
    use crate::tlb::TlbEntry;

    let entry = TlbEntry {
        entry_hi: 0x0040_0001,
        entry_lo0: 0x46,
        entry_lo1: 0x86,
        page_mask: 0x6000,
    };

    // 16 KiB Page 2개 = 32 KiB 범위
    for address in [0x0040_0000, 0x0040_2000, 0x0040_4000, 0x0040_7FFF] {
        assert!(entry.matches(address, 1));
    }

    assert!(!entry.matches(0x0040_8000, 1));
    assert!(!entry.matches(0x0040_0000, 2));
}

#[test]
fn tlb_random_write() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.cp0_mut().write_reg(6, 0, 28);

    cpu.cp0_mut().write_reg(10, 0, 0x0040_0001);
    cpu.cp0_mut().write_reg(2, 0, 0x46);
    cpu.cp0_mut().write_reg(3, 0, 0x86);

    memory.write_word(0, 0x42000006);

    assert_eq!(cpu.cp0().read_random(), 31);

    cpu.step(&mut memory);

    // step()에서 Random이 먼저 감소
    assert_eq!(cpu.cp0().read_random(), 30);

    let entry = cpu.cp0().tlb().read(30).unwrap();

    assert_eq!(entry.entry_hi, 0x0040_0001);

    for index in 0..28 {
        assert_eq!(cpu.cp0().tlb().read(index), None);
    }
}

#[test]
fn user_tlb_access_denied() {
    for word in [0x42000001, 0x42000002, 0x42000006, 0x42000008] {
        let mut cpu = Cpu::new();
        let mut memory = Memory::new(1024);

        cpu.cp0_mut().write_reg(12, 0, 0x10);
        cpu.cp0_mut().write_reg(0, 0, 5);

        memory.write_word(0, word);

        cpu.step(&mut memory);

        assert_eq!(cpu.cp0().exception_code(), 11);
        assert_eq!(cpu.read_pc(), 0x8000_0180);
        assert_eq!(cpu.cp0().read_epc(), 0);

        assert_eq!(cpu.cp0().tlb().read(5), None);
        assert_eq!(cpu.cp0().read_index(), 5);

        assert!(cpu.cp0().is_kernel_mode());
    }
}

#[test]
fn tlb_translate_even_odd() {
    use crate::tlb::TlbEntry;

    let entry = TlbEntry {
        entry_hi: 0x0040_0001,
        entry_lo0: (0x10 << 6) | 6,
        entry_lo1: (0x30 << 6) | 6,
        page_mask: 0,
    };

    assert_eq!(entry.translate(0x0040_0123, false), Ok(0x0001_0123));

    assert_eq!(entry.translate(0x0040_1456, false), Ok(0x0003_0456));
}

#[test]
fn tlb_translate_large_page() {
    use crate::tlb::TlbEntry;

    let entry = TlbEntry {
        entry_hi: 0x0040_0001,
        entry_lo0: (0x13 << 6) | 6,
        entry_lo1: (0x33 << 6) | 6,
        page_mask: 0x6000,
    };

    assert_eq!(entry.page_size(), Ok(16384));

    assert_eq!(entry.translate(0x0040_2345, false), Ok(0x0001_2345));

    assert_eq!(entry.translate(0x0040_4567, false), Ok(0x0003_0567));
}

#[test]
fn tlb_translate_miss() {
    use crate::tlb::{Tlb, TlbEntry, TlbFault};

    let mut tlb = Tlb::new();

    tlb.write(
        0,
        TlbEntry {
            entry_hi: 0x0040_0001,
            entry_lo0: 0x46,
            entry_lo1: 0x86,
            page_mask: 0,
        },
    );

    assert_eq!(tlb.translate(0x0040_0123, 1, false), Ok(0x1123));

    assert_eq!(tlb.translate(0x0040_0123, 2, false), Err(TlbFault::Miss));

    assert_eq!(tlb.translate(0x0080_0000, 1, false), Err(TlbFault::Miss));
}

#[test]
fn tlb_translate_invalid() {
    use crate::tlb::{TlbEntry, TlbFault};

    let entry = TlbEntry {
        entry_hi: 0x0040_0001,
        entry_lo0: 0x44,
        entry_lo1: 0x86,
        page_mask: 0,
    };

    assert_eq!(entry.translate(0x0040_0000, false), Err(TlbFault::Invalid));

    assert_eq!(entry.translate(0x0040_0000, true), Err(TlbFault::Invalid));

    assert_eq!(entry.translate(0x0040_1000, false), Ok(0x2000));
}

#[test]
fn tlb_translate_modified() {
    use crate::tlb::{TlbEntry, TlbFault};

    let entry = TlbEntry {
        entry_hi: 0x0040_0001,
        entry_lo0: 0x42,
        entry_lo1: 0x86,
        page_mask: 0,
    };

    // Read-only Page
    assert_eq!(entry.translate(0x0040_0000, false), Ok(0x1000));

    assert_eq!(entry.translate(0x0040_0000, true), Err(TlbFault::Modified));

    // Writable Page
    assert_eq!(entry.translate(0x0040_1000, true), Ok(0x2000));
}

#[test]
fn tlb_invalid_page_mask() {
    use crate::tlb::{TlbEntry, TlbFault};

    let entry = TlbEntry {
        entry_hi: 0,
        entry_lo0: 0x46,
        entry_lo1: 0x86,
        page_mask: 0x2000,
    };

    assert_eq!(entry.page_size(), Err(TlbFault::InvalidPageMask));

    assert_eq!(entry.translate(0, false), Err(TlbFault::InvalidPageMask));
}

#[test]
fn tlb_mapped_instruction_fetch() {
    let mut cpu = Cpu::new_with_tlb();
    let mut memory = Memory::new(8192);

    cpu.cp0_mut().write_reg(10, 0, 0x0040_0001);
    cpu.cp0_mut().write_reg(2, 0, 0x46);
    cpu.cp0_mut().write_reg(3, 0, 0);
    cpu.cp0_mut().tlb_write_indexed();

    cpu.write_pc(0x0040_0000);

    memory.write_word(0x1000, i(9, 0, 8, 100));

    cpu.step(&mut memory);

    assert_eq!(cpu.read_reg(8), 100);
    assert_eq!(cpu.read_pc(), 0x0040_0004);
    assert!(!cpu.cp0().exception_level());
}

#[test]
fn tlb_mapped_load_store() {
    let mut cpu = Cpu::new_with_tlb();
    let mut memory = Memory::new(8192);

    cpu.cp0_mut().write_reg(10, 0, 0x0040_0001);
    cpu.cp0_mut().write_reg(2, 0, 0x46);
    cpu.cp0_mut().tlb_write_indexed();

    cpu.write_pc(0x8000_0000);

    cpu.write_reg(8, 0x0040_0100);
    cpu.write_reg(9, 0xDEAD_BEEF);

    memory.write_word(0, i(0x2B, 8, 9, 0));
    memory.write_word(4, i(0x23, 8, 10, 0));

    cpu.step(&mut memory);
    cpu.step(&mut memory);

    assert_eq!(memory.read_word(0x1100), 0xDEAD_BEEF);

    assert_eq!(cpu.read_reg(10), 0xDEAD_BEEF);
    assert!(!cpu.cp0().exception_level());
}

#[test]
fn tlb_refill_exception() {
    let mut cpu = Cpu::new_with_tlb();
    let mut memory = Memory::new(1024);

    let address = 0x0040_1234;

    cpu.cp0_mut().write_reg(10, 0, 0xAB);
    cpu.cp0_mut().write_reg(4, 0, 0x8000_0000);

    cpu.write_pc(address);
    cpu.step(&mut memory);

    assert_eq!(cpu.cp0().exception_code(), 2);
    assert_eq!(cpu.cp0().read_bad_vaddr(), address);
    assert_eq!(cpu.cp0().read_epc(), address);

    assert_eq!(cpu.cp0().read_entry_hi(), 0x0040_00AB);

    assert_eq!(cpu.cp0().read_context(), 0x8000_2000);

    assert_eq!(cpu.read_pc(), 0x8000_0000);
    assert!(cpu.cp0().exception_level());
}

#[test]
fn nested_tlb_refill_exception() {
    let mut cpu = Cpu::new_with_tlb();
    let mut memory = Memory::new(1024);

    cpu.cp0_mut().write_reg(14, 0, 0x1234);
    cpu.cp0_mut().write_reg(12, 0, 2);

    cpu.write_pc(0x0040_0000);
    cpu.step(&mut memory);

    assert_eq!(cpu.cp0().exception_code(), 2);
    assert_eq!(cpu.cp0().read_epc(), 0x1234);
    assert_eq!(cpu.read_pc(), 0x8000_0180);
}

#[test]
fn tlb_invalid_exception() {
    let mut cpu = Cpu::new_with_tlb();
    let mut memory = Memory::new(1024);

    cpu.cp0_mut().write_reg(10, 0, 0x0040_0001);
    cpu.cp0_mut().write_reg(2, 0, 0x44);
    cpu.cp0_mut().tlb_write_indexed();

    cpu.write_pc(0x0040_0000);
    cpu.step(&mut memory);

    assert_eq!(cpu.cp0().exception_code(), 2);
    assert_eq!(cpu.read_pc(), 0x8000_0180);

    assert_eq!(cpu.cp0().read_bad_vaddr(), 0x0040_0000);
}

#[test]
fn tlb_modified_exception() {
    let mut cpu = Cpu::new_with_tlb();
    let mut memory = Memory::new(8192);

    cpu.cp0_mut().write_reg(10, 0, 0x0040_0001);
    cpu.cp0_mut().write_reg(2, 0, 0x42);
    cpu.cp0_mut().tlb_write_indexed();

    cpu.write_pc(0x8000_0000);

    cpu.write_reg(8, 0x0040_0000);
    cpu.write_reg(9, 0xDEAD_BEEF);

    memory.write_word(0, i(0x2B, 8, 9, 0));
    memory.write_word(0x1000, 123);

    cpu.step(&mut memory);

    assert_eq!(cpu.cp0().exception_code(), 1);
    assert_eq!(cpu.read_pc(), 0x8000_0180);
    assert_eq!(cpu.cp0().read_epc(), 0x8000_0000);

    assert_eq!(memory.read_word(0x1000), 123);
}

#[test]
fn tlb_store_refill_exception() {
    let mut cpu = Cpu::new_with_tlb();
    let mut memory = Memory::new(1024);

    cpu.write_pc(0x8000_0000);

    cpu.write_reg(8, 0x0040_0000);
    cpu.write_reg(9, 100);

    memory.write_word(0, i(0x2B, 8, 9, 0));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp0().exception_code(), 3);

    assert_eq!(cpu.cp0().read_bad_vaddr(), 0x0040_0000);

    assert_eq!(cpu.read_pc(), 0x8000_0000);
    assert_eq!(memory.read_word(0x100), 0);
    assert!(cpu.cp0().exception_level());
}

#[test]
fn complete_tlb_refill_cycle() {
    let mut cpu = Cpu::new_with_tlb();
    let mut memory = Memory::new(8192);

    // User Mode
    cpu.cp0_mut().write_reg(12, 0, 0x10);
    cpu.write_pc(0x0040_0000);

    // User Program
    memory.write_word(0x1000, i(9, 0, 8, 100));

    // TLB Refill Handler
    memory.write_word(0x00, i(9, 0, 26, 0x46));
    memory.write_word(0x04, encode_cp0(4, 26, 2, 0));
    memory.write_word(0x08, 0x42000002); // TLBWI
    memory.write_word(0x0C, 0x42000018); // ERET

    // TLB Miss
    cpu.step(&mut memory);

    assert_eq!(cpu.read_pc(), 0x8000_0000);
    assert!(cpu.cp0().is_kernel_mode());

    // Refill Handler 실행
    for _ in 0..4 {
        cpu.step(&mut memory);
    }

    assert_eq!(cpu.read_pc(), 0x0040_0000);
    assert!(!cpu.cp0().exception_level());
    assert!(!cpu.cp0().is_kernel_mode());

    // 실패했던 명령어 재실행
    cpu.step(&mut memory);

    assert_eq!(cpu.read_reg(8), 100);
    assert_eq!(cpu.read_pc(), 0x0040_0004);
}

#[test]
fn config_initialization() {
    use crate::cp0::Cp0;

    let cp0 = Cp0::new();

    assert_eq!(cp0.read_config(), 0x8000_0482);
    assert_eq!(cp0.read_config1(), 0x3E00_0000);
}

#[test]
fn config_architecture_fields() {
    use crate::cp0::Cp0;

    let cp0 = Cp0::new();
    let config = cp0.read_config();

    assert_eq!((config >> 31) & 1, 1);
    assert_eq!((config >> 15) & 1, 0);
    assert_eq!((config >> 13) & 3, 0);
    assert_eq!((config >> 10) & 7, 1);
    assert_eq!((config >> 7) & 7, 1);
    assert_eq!(config & 7, 2);
}

#[test]
fn config_tlb_size() {
    use crate::cp0::{Cp0, TLB_SIZE};

    let cp0 = Cp0::new();
    let config1 = cp0.read_config1();

    let mmu_size = (config1 >> 25) & 0x3F;

    assert_eq!(mmu_size as usize + 1, TLB_SIZE);

    // Config2 미지원
    assert_eq!(config1 >> 31, 0);

    // FPU 미지원
    assert_eq!(config1 & 1, 0);
}

#[test]
fn config_write_mask() {
    use crate::cp0::Cp0;

    let mut cp0 = Cp0::new();

    cp0.write_reg(16, 0, u32::MAX);

    assert_eq!(cp0.read_config(), 0x8000_0487);

    cp0.write_reg(16, 0, 0);

    assert_eq!(cp0.read_config(), 0x8000_0480);

    cp0.write_reg(16, 0, 2);

    assert_eq!(cp0.read_config(), 0x8000_0482);
}

#[test]
fn config1_read_only() {
    use crate::cp0::Cp0;

    let mut cp0 = Cp0::new();
    let old = cp0.read_config1();

    assert!(cp0.write_reg(16, 1, u32::MAX));
    assert_eq!(cp0.read_config1(), old);

    assert!(cp0.write_reg(16, 1, 0));
    assert_eq!(cp0.read_config1(), old);
}

#[test]
fn config_register_transfer() {
    let cpu = run(
        &[
            encode_cp0(0, 8, 16, 0),
            encode_cp0(0, 9, 16, 1),
            encode_cp0(4, 10, 16, 0),
            encode_cp0(0, 11, 16, 0),
            encode_cp0(4, 10, 16, 1),
            encode_cp0(0, 12, 16, 1),
        ],
        &[(10, 3)],
    );

    assert_eq!(cpu.read_reg(8), 0x8000_0482);
    assert_eq!(cpu.read_reg(9), 0x3E00_0000);
    assert_eq!(cpu.read_reg(11), 0x8000_0483);
    assert_eq!(cpu.read_reg(12), 0x3E00_0000);

    assert!(!cpu.cp0().exception_level());
}

#[test]
fn cp1_register_reset() {
    let cp1 = crate::cp1::Cp1::new();

    for index in 0..32 {
        assert_eq!(cp1.read_raw(index), 0);
    }
}

#[test]
fn cp1_raw_register_transfer() {
    let mut cp1 = crate::cp1::Cp1::new();

    let value = 0x1234_5678_9ABC_DEF0u64;

    cp1.write_raw(5, value);

    assert_eq!(cp1.read_raw(5), value);
}

#[test]
fn cp1_fpr_zero_is_writable() {
    let mut cp1 = crate::cp1::Cp1::new();

    cp1.write_raw(0, 0xDEAD_BEEF);

    assert_eq!(cp1.read_raw(0), 0xDEAD_BEEF);
}

#[test]
fn cp1_register_independence() {
    let mut cp1 = crate::cp1::Cp1::new();

    cp1.write_raw(30, 0xAAAA_AAAA_AAAA_AAAA);
    cp1.write_raw(31, 0xBBBB_BBBB_BBBB_BBBB);

    assert_eq!(cp1.read_raw(30), 0xAAAA_AAAA_AAAA_AAAA);
    assert_eq!(cp1.read_raw(31), 0xBBBB_BBBB_BBBB_BBBB);
    assert_eq!(cp1.read_raw(29), 0);
}

#[test]
fn cp1_control_register_reset() {
    let cp1 = crate::cp1::Cp1::new();

    assert_eq!(cp1.read_fir(), 0);
    assert_eq!(cp1.read_fcsr(), 0);
}

#[test]
fn cpu_cp1_integration() {
    let mut cpu = crate::cpu::Cpu::new();

    cpu.cp1_mut().write_raw(7, 0x1234_5678);

    assert_eq!(cpu.cp1().read_raw(7), 0x1234_5678);
    assert_eq!(cpu.read_reg(7), 0);
}

#[test]
fn cp1_disabled_on_reset() {
    let cpu = Cpu::new();

    assert!(!cpu.cp0().cp1_accessible());
}

#[test]
fn cp1_status_cu1_write() {
    let mut cpu = Cpu::new();

    assert!(cpu.cp0_mut().write_reg(12, 0, 1 << 29));
    assert!(cpu.cp0().cp1_accessible());

    assert!(cpu.cp0_mut().write_reg(12, 0, 0));
    assert!(!cpu.cp0().cp1_accessible());
}

#[test]
fn cp1_unusable_exception() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    // MFC1 $8, $f2
    memory.write_word(0, encode_r(0x11, 0, 8, 2, 0, 0));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp0().exception_code(), 11);
    assert_eq!(cpu.cp0().coprocessor_error(), 1);
    assert_eq!(cpu.cp0().read_epc(), 0);
    assert!(cpu.cp0().exception_level());
}

#[test]
fn cp1_unusable_prevents_register_write() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    cpu.write_reg(8, 0x1234_5678);
    cpu.cp1_mut().write_raw(2, 0xDEAD_BEEF);

    // MFC1 $8, $f2
    memory.write_word(0, encode_r(0x11, 0, 8, 2, 0, 0));

    cpu.step(&mut memory);

    assert_eq!(cpu.read_reg(8), 0x1234_5678);
    assert_eq!(cpu.cp1().read_raw(2), 0xDEAD_BEEF);
    assert_eq!(cpu.cp0().exception_code(), 11);
}

#[test]
fn cp1_memory_access_disabled() {
    for opcode in [0x31, 0x35, 0x39, 0x3D] {
        let mut cpu = Cpu::new();
        let mut memory = Memory::new(64);

        cpu.write_reg(8, 32);

        memory.write_word(0, i(opcode, 8, 2, 0));

        cpu.step(&mut memory);

        assert_eq!(cpu.cp0().exception_code(), 11);
        assert_eq!(cpu.cp0().coprocessor_error(), 1);
    }
}

#[test]
fn coprocessor_exception_number_update() {
    use crate::exception::Exception;

    let mut cpu = Cpu::new();

    cpu.raise_exception(Exception::CoprocessorUnusable(1), None);

    assert_eq!(cpu.cp0().coprocessor_error(), 1);

    cpu.raise_exception(Exception::CoprocessorUnusable(0), None);

    assert_eq!(cpu.cp0().coprocessor_error(), 0);
    assert_eq!(cpu.cp0().exception_code(), 11);
}

#[test]
fn cp1_word_read() {
    let mut cpu = Cpu::new();

    cpu.cp1_mut().write_raw(2, 0x1234_5678_9ABC_DEF0);

    assert_eq!(cpu.cp1().read_word(2), 0x9ABC_DEF0,);
}

#[test]
fn cp1_word_write() {
    let mut cpu = Cpu::new();

    cpu.cp1_mut().write_raw(2, 0x1234_5678_9ABC_DEF0);

    cpu.cp1_mut().write_word(2, 0x3F80_0000);

    assert_eq!(cpu.cp1().read_raw(2), 0x1234_5678_3F80_0000,);
}

#[test]
fn mtc1_register_transfer() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);
    cpu.write_reg(8, 0x3F80_0000);

    // MTC1 $8, $f2
    memory.write_word(0, encode_r(0x11, 4, 8, 2, 0, 0));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp1().read_word(2), 0x3F80_0000);
    assert_eq!(cpu.read_reg(8), 0x3F80_0000);
    assert!(!cpu.cp0().exception_level());
}

#[test]
fn mfc1_register_transfer() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);
    cpu.cp1_mut().write_word(2, 0xBF80_0000);

    // MFC1 $8, $f2
    memory.write_word(0, encode_r(0x11, 0, 8, 2, 0, 0));

    cpu.step(&mut memory);

    assert_eq!(cpu.read_reg(8), 0xBF80_0000);
    assert_eq!(cpu.cp1().read_word(2), 0xBF80_0000);
    assert!(!cpu.cp0().exception_level());
}

#[test]
fn cp1_transfer_round_trip() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(8);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    // NaN을 포함할 수 있는 비트열도 그대로 전달
    let value = 0x7FC0_1234;

    cpu.write_reg(8, value);

    // MTC1 $8, $f3
    memory.write_word(0, encode_r(0x11, 4, 8, 3, 0, 0));

    // MFC1 $9, $f3
    memory.write_word(4, encode_r(0x11, 0, 9, 3, 0, 0));

    cpu.step(&mut memory);
    cpu.step(&mut memory);

    assert_eq!(cpu.read_reg(9), value);
    assert_eq!(cpu.cp1().read_word(3), value);
    assert!(!cpu.cp0().exception_level());
}

#[test]
fn mfc1_preserves_gpr_zero() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);
    cpu.cp1_mut().write_word(2, 0xFFFF_FFFF);

    // MFC1 $zero, $f2
    memory.write_word(0, encode_r(0x11, 0, 0, 2, 0, 0));

    cpu.step(&mut memory);

    assert_eq!(cpu.read_reg(0), 0);
    assert!(!cpu.cp0().exception_level());
}

#[test]
fn mtc1_disabled_preserves_fpr() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    cpu.write_reg(8, 0xFFFF_FFFF);
    cpu.cp1_mut().write_word(2, 0x1234_5678);

    // MTC1 $8, $f2
    memory.write_word(0, encode_r(0x11, 4, 8, 2, 0, 0));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp1().read_word(2), 0x1234_5678,);

    assert_eq!(cpu.cp0().exception_code(), 11);
    assert_eq!(cpu.cp0().coprocessor_error(), 1);
}

#[test]
fn cp1_control_register_read() {
    let cpu = Cpu::new();

    for index in [0, 25, 26, 28, 31] {
        assert_eq!(cpu.cp1().read_control(index), Some(0),);
    }

    assert_eq!(cpu.cp1().read_control(1), None);
}

#[test]
fn cp1_fcsr_write_mask() {
    let mut cpu = Cpu::new();

    assert!(cpu.cp1_mut().write_control(31, u32::MAX));

    assert_eq!(cpu.cp1().read_fcsr(), 0xFF83_FFFF,);
}

#[test]
fn cp1_fccr_alias() {
    let mut cpu = Cpu::new();

    cpu.cp1_mut().write_control(31, 0x0100_0003);
    cpu.cp1_mut().write_control(25, 0xA5);

    assert_eq!(cpu.cp1().read_control(25), Some(0xA5),);

    assert_eq!(cpu.cp1().read_fcsr(), 0xA580_0003,);
}

#[test]
fn cp1_fexr_alias() {
    let mut cpu = Cpu::new();

    cpu.cp1_mut().write_control(31, 3);
    cpu.cp1_mut().write_control(26, 0x0001_0040);

    assert_eq!(cpu.cp1().read_control(26), Some(0x0001_0040),);

    assert_eq!(cpu.cp1().read_fcsr(), 0x0001_0043,);
}

#[test]
fn cp1_fenr_alias() {
    let mut cpu = Cpu::new();

    cpu.cp1_mut().write_control(31, 0x40);
    cpu.cp1_mut().write_control(28, 0x0000_0807);

    assert_eq!(cpu.cp1().read_control(28), Some(0x0000_0807),);

    assert_eq!(cpu.cp1().read_fcsr(), 0x0100_0843,);
}

#[test]
fn cp1_fir_is_read_only() {
    let mut cpu = Cpu::new();

    let old_fir = cpu.cp1().read_fir();

    assert!(!cpu.cp1_mut().write_control(0, u32::MAX));

    assert_eq!(cpu.cp1().read_fir(), old_fir);
}

#[test]
fn cp1_control_transfer_round_trip() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(8);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);
    cpu.write_reg(8, 3);

    // CTC1 $8, $31
    memory.write_word(0, encode_r(0x11, 6, 8, 31, 0, 0));

    // CFC1 $9, $31
    memory.write_word(4, encode_r(0x11, 2, 9, 31, 0, 0));

    cpu.step(&mut memory);
    cpu.step(&mut memory);

    assert_eq!(cpu.read_reg(9), 3);
    assert_eq!(cpu.cp1().read_fcsr(), 3);
    assert!(!cpu.cp0().exception_level());
}

#[test]
fn ctc1_enabled_fp_exception() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    // Invalid Operation Cause + Enable
    let value = (1 << 16) | (1 << 11);

    cpu.write_reg(8, value);

    memory.write_word(0, encode_r(0x11, 6, 8, 31, 0, 0));

    cpu.step(&mut memory);

    // FCSR must be written before the exception
    assert_eq!(cpu.cp1().read_fcsr(), value);
    assert_eq!(cpu.cp0().exception_code(), 15);
    assert_eq!(cpu.cp0().read_epc(), 0);
    assert!(cpu.cp0().exception_level());
}

#[test]
fn ctc1_unimplemented_operation_exception() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);
    cpu.write_reg(8, 1 << 17);

    memory.write_word(0, encode_r(0x11, 6, 8, 31, 0, 0));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp1().read_fcsr(), 1 << 17);
    assert_eq!(cpu.cp0().exception_code(), 15);
    assert!(cpu.cp0().exception_level());
}

#[test]
fn cp1_doubleword_read() {
    let mut cpu = Cpu::new();

    cpu.cp1_mut().write_word(2, 0x9ABC_DEF0);
    cpu.cp1_mut().write_word(3, 0x1234_5678);

    assert_eq!(cpu.cp1().read_doubleword(2), Some(0x1234_5678_9ABC_DEF0),);
}

#[test]
fn cp1_doubleword_write() {
    let mut cpu = Cpu::new();

    assert!(cpu.cp1_mut().write_doubleword(2, 0x1234_5678_9ABC_DEF0,));

    assert_eq!(cpu.cp1().read_word(2), 0x9ABC_DEF0);
    assert_eq!(cpu.cp1().read_word(3), 0x1234_5678);
}

#[test]
fn cp1_doubleword_last_register_pair() {
    let mut cpu = Cpu::new();

    let value = 0x4009_21FB_5444_2D18;

    assert!(cpu.cp1_mut().write_doubleword(30, value));

    assert_eq!(cpu.cp1().read_doubleword(30), Some(value),);

    assert_eq!(cpu.cp1().read_word(31), 0x4009_21FB);
}

#[test]
fn cp1_invalid_doubleword_register() {
    let mut cpu = Cpu::new();

    for index in [1, 3, 31, 32] {
        assert_eq!(cpu.cp1().read_doubleword(index), None,);

        assert!(!cpu.cp1_mut().write_doubleword(index, u64::MAX));

        assert_eq!(cpu.cp1().read_high_word(index), None,);

        assert!(!cpu.cp1_mut().write_high_word(index, u32::MAX));
    }

    for index in 0..32 {
        assert_eq!(cpu.cp1().read_raw(index), 0);
    }
}

#[test]
fn mfhc1_register_transfer() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    cpu.cp1_mut().write_doubleword(2, 0x1234_5678_9ABC_DEF0);

    // MFHC1 $8, $f2
    memory.write_word(0, encode_r(0x11, 3, 8, 2, 0, 0));

    cpu.step(&mut memory);

    assert_eq!(cpu.read_reg(8), 0x1234_5678);
    assert!(!cpu.cp0().exception_level());
}

#[test]
fn mthc1_preserves_low_word() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);
    cpu.write_reg(8, 0x3FF0_0000);

    cpu.cp1_mut().write_word(2, 0x1234_5678);

    // MTHC1 $8, $f2
    memory.write_word(0, encode_r(0x11, 7, 8, 2, 0, 0));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp1().read_doubleword(2), Some(0x3FF0_0000_1234_5678),);

    assert!(!cpu.cp0().exception_level());
}

#[test]
fn cp1_doubleword_transfer_round_trip() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(16);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    cpu.write_reg(8, 0x5444_2D18);
    cpu.write_reg(9, 0x4009_21FB);

    // MTC1 $8, $f2
    memory.write_word(0, encode_r(0x11, 4, 8, 2, 0, 0));

    // MTHC1 $9, $f2
    memory.write_word(4, encode_r(0x11, 7, 9, 2, 0, 0));

    // MFC1 $10, $f2
    memory.write_word(8, encode_r(0x11, 0, 10, 2, 0, 0));

    // MFHC1 $11, $f2
    memory.write_word(12, encode_r(0x11, 3, 11, 2, 0, 0));

    for _ in 0..4 {
        cpu.step(&mut memory);
    }

    assert_eq!(cpu.read_reg(10), 0x5444_2D18);
    assert_eq!(cpu.read_reg(11), 0x4009_21FB);

    assert_eq!(cpu.cp1().read_doubleword(2), Some(0x4009_21FB_5444_2D18),);

    assert!(!cpu.cp0().exception_level());
}

#[test]
fn mfhc1_odd_register_rejected() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);
    cpu.write_reg(8, 0x1234_5678);

    // MFHC1 $8, $f3
    memory.write_word(0, encode_r(0x11, 3, 8, 3, 0, 0));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp0().exception_code(), 10);
    assert_eq!(cpu.read_reg(8), 0x1234_5678);
}

#[test]
fn mthc1_odd_register_rejected() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);
    cpu.write_reg(8, u32::MAX);

    // MTHC1 $8, $f31
    memory.write_word(0, encode_r(0x11, 7, 8, 31, 0, 0));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp0().exception_code(), 10);

    for index in 0..32 {
        assert_eq!(cpu.cp1().read_raw(index), 0);
    }
}

#[test]
fn cp1_high_word_access_disabled() {
    for rs in [3, 7] {
        let mut cpu = Cpu::new();
        let mut memory = Memory::new(4);

        cpu.write_reg(8, 0x1234_5678);

        cpu.cp1_mut().write_doubleword(2, 0xAAAA_BBBB_CCCC_DDDD);

        memory.write_word(0, encode_r(0x11, rs, 8, 2, 0, 0));

        cpu.step(&mut memory);

        assert_eq!(cpu.cp0().exception_code(), 11);
        assert_eq!(cpu.cp0().coprocessor_error(), 1);

        assert_eq!(cpu.read_reg(8), 0x1234_5678);

        assert_eq!(cpu.cp1().read_doubleword(2), Some(0xAAAA_BBBB_CCCC_DDDD),);
    }
}

#[test]
fn lwc1_word_load() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(64);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);
    cpu.write_reg(8, 32);

    memory.write_word(32, 0x3F80_0000);

    // LWC1 $f2, 0($8)
    memory.write_word(0, i(0x31, 8, 2, 0));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp1().read_word(2), 0x3F80_0000);
    assert_eq!(cpu.read_reg(8), 32);
    assert!(!cpu.cp0().exception_level());
}

#[test]
fn swc1_word_store() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(64);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);
    cpu.write_reg(8, 32);

    cpu.cp1_mut().write_word(2, 0xBF80_0000);

    // SWC1 $f2, 0($8)
    memory.write_word(0, i(0x39, 8, 2, 0));

    cpu.step(&mut memory);

    assert_eq!(memory.read_word(32), 0xBF80_0000);
    assert_eq!(cpu.cp1().read_word(2), 0xBF80_0000);
    assert!(!cpu.cp0().exception_level());
}

#[test]
fn cp1_memory_round_trip() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(64);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);
    cpu.write_reg(8, 32);

    let value = 0x7FC0_1234;

    cpu.cp1_mut().write_word(2, value);

    // SWC1 $f2, 0($8)
    memory.write_word(0, i(0x39, 8, 2, 0));

    // LWC1 $f3, 0($8)
    memory.write_word(4, i(0x31, 8, 3, 0));

    cpu.step(&mut memory);
    cpu.step(&mut memory);

    assert_eq!(memory.read_word(32), value);
    assert_eq!(cpu.cp1().read_word(3), value);
    assert!(!cpu.cp0().exception_level());
}

#[test]
fn lwc1_negative_offset() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(64);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);
    cpu.write_reg(8, 36);

    memory.write_word(32, 0x1234_5678);

    // LWC1 $f0, -4($8)
    memory.write_word(0, i(0x31, 8, 0, (-4i16) as u16));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp1().read_word(0), 0x1234_5678);
    assert!(!cpu.cp0().exception_level());
}

#[test]
fn lwc1_alignment_exception() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(64);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);
    cpu.write_reg(8, 33);

    cpu.cp1_mut().write_word(2, 0x1234_5678);

    memory.write_word(0, i(0x31, 8, 2, 0));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp0().exception_code(), 4);
    assert_eq!(cpu.cp0().read_bad_vaddr(), 33);
    assert_eq!(cpu.cp1().read_word(2), 0x1234_5678);
}

#[test]
fn swc1_alignment_exception() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(64);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);
    cpu.write_reg(8, 33);

    cpu.cp1_mut().write_word(2, u32::MAX);

    memory.write_word(0, i(0x39, 8, 2, 0));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp0().exception_code(), 5);
    assert_eq!(cpu.cp0().read_bad_vaddr(), 33);

    assert_eq!(memory.read_word(32), 0);
    assert_eq!(memory.read_word(36), 0);
}

#[test]
fn cp1_memory_unusable_priority() {
    for opcode in [0x31, 0x39] {
        let mut cpu = Cpu::new();
        let mut memory = Memory::new(64);

        // CU1 = 0, 동시에 misaligned address
        cpu.write_reg(8, 33);

        cpu.cp1_mut().write_word(2, 0x1234_5678);

        memory.write_word(0, i(opcode, 8, 2, 0));

        cpu.step(&mut memory);

        assert_eq!(cpu.cp0().exception_code(), 11);
        assert_eq!(cpu.cp0().coprocessor_error(), 1);

        assert_eq!(cpu.cp1().read_word(2), 0x1234_5678);
        assert_eq!(memory.read_word(32), 0);
        assert_eq!(memory.read_word(36), 0);
    }
}

#[test]
fn lwc1_tlb_refill_exception() {
    let mut cpu = Cpu::new_with_tlb();
    let mut memory = Memory::new(64);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    // kseg0: instruction fetch는 TLB 우회
    cpu.write_pc(0x8000_0000);

    // 매핑되지 않은 가상 주소
    cpu.write_reg(8, 0x0040_0000);

    cpu.cp1_mut().write_word(2, 0x1234_5678);

    memory.write_word(0, i(0x31, 8, 2, 0));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp0().exception_code(), 2);

    assert_eq!(cpu.cp0().read_bad_vaddr(), 0x0040_0000,);

    assert_eq!(cpu.cp0().read_epc(), 0x8000_0000,);

    assert_eq!(cpu.cp1().read_word(2), 0x1234_5678);
}

#[test]
fn swc1_tlb_refill_exception() {
    let mut cpu = Cpu::new_with_tlb();
    let mut memory = Memory::new(64);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    cpu.write_pc(0x8000_0000);
    cpu.write_reg(8, 0x0040_0000);

    cpu.cp1_mut().write_word(2, u32::MAX);

    memory.write_word(0, i(0x39, 8, 2, 0));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp0().exception_code(), 3);

    assert_eq!(cpu.cp0().read_bad_vaddr(), 0x0040_0000,);

    assert_eq!(cpu.cp0().read_epc(), 0x8000_0000,);

    assert_eq!(memory.read_word(32), 0);
    assert_eq!(cpu.cp1().read_word(2), u32::MAX);
}

#[test]
fn swc1_clears_reservation() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(64);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    cpu.write_reg(8, 32);
    cpu.set_reservation(32);

    cpu.cp1_mut().write_word(2, 0xDEAD_BEEF);

    memory.write_word(0, i(0x39, 8, 2, 0));

    assert!(cpu.check_reservation(32));

    cpu.step(&mut memory);

    assert_eq!(memory.read_word(32), 0xDEAD_BEEF);
    assert!(!cpu.check_reservation(32));
    assert!(!cpu.cp0().exception_level());
}

#[test]
fn memory_doubleword_read() {
    let mut memory = Memory::new(64);

    memory.write_word(32, 0x5444_2D18);
    memory.write_word(36, 0x4009_21FB);

    assert_eq!(memory.read_doubleword(32), 0x4009_21FB_5444_2D18,);
}

#[test]
fn memory_doubleword_write() {
    let mut memory = Memory::new(64);

    memory.write_doubleword(32, 0x1234_5678_9ABC_DEF0);

    assert_eq!(memory.read_word(32), 0x9ABC_DEF0);
    assert_eq!(memory.read_word(36), 0x1234_5678);

    let expected = [0xF0, 0xDE, 0xBC, 0x9A, 0x78, 0x56, 0x34, 0x12];

    for (index, value) in expected.into_iter().enumerate() {
        assert_eq!(memory.read_byte(32 + index as u32), value,);
    }
}

#[test]
fn ldc1_doubleword_load() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(64);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);
    cpu.write_reg(8, 32);

    memory.write_doubleword(32, 0x4009_21FB_5444_2D18);

    // LDC1 $f2, 0($8)
    memory.write_word(0, i(0x35, 8, 2, 0));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp1().read_word(2), 0x5444_2D18);
    assert_eq!(cpu.cp1().read_word(3), 0x4009_21FB);
    assert!(!cpu.cp0().exception_level());
}

#[test]
fn sdc1_doubleword_store() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(64);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);
    cpu.write_reg(8, 32);

    cpu.cp1_mut().write_doubleword(2, 0x1234_5678_9ABC_DEF0);

    // SDC1 $f2, 0($8)
    memory.write_word(0, i(0x3D, 8, 2, 0));

    cpu.step(&mut memory);

    assert_eq!(memory.read_doubleword(32), 0x1234_5678_9ABC_DEF0,);

    assert!(!cpu.cp0().exception_level());
}

#[test]
fn cp1_doubleword_memory_round_trip() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(64);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);
    cpu.write_reg(8, 40);

    let value = 0x7FF8_1234_5678_9ABC;

    cpu.cp1_mut().write_doubleword(2, value);

    // SDC1 $f2, -8($8)
    memory.write_word(0, i(0x3D, 8, 2, (-8i16) as u16));

    // LDC1 $f30, -8($8)
    memory.write_word(4, i(0x35, 8, 30, (-8i16) as u16));

    cpu.step(&mut memory);
    cpu.step(&mut memory);

    assert_eq!(memory.read_doubleword(32), value);

    assert_eq!(cpu.cp1().read_doubleword(30), Some(value),);

    assert!(!cpu.cp0().exception_level());
}

#[test]
fn ldc1_alignment_exception() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(64);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    // Word aligned, but not doubleword aligned
    cpu.write_reg(8, 36);

    let value = 0x1234_5678_9ABC_DEF0;

    cpu.cp1_mut().write_doubleword(2, value);

    memory.write_word(0, i(0x35, 8, 2, 0));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp0().exception_code(), 4);
    assert_eq!(cpu.cp0().read_bad_vaddr(), 36);

    assert_eq!(cpu.cp1().read_doubleword(2), Some(value),);
}

#[test]
fn sdc1_alignment_exception() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(64);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);
    cpu.write_reg(8, 36);

    cpu.cp1_mut().write_doubleword(2, u64::MAX);

    memory.write_word(0, i(0x3D, 8, 2, 0));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp0().exception_code(), 5);
    assert_eq!(cpu.cp0().read_bad_vaddr(), 36);

    assert_eq!(memory.read_doubleword(32), 0);
    assert_eq!(memory.read_doubleword(40), 0);
}

#[test]
fn cp1_doubleword_memory_odd_register_rejected() {
    for opcode in [0x35, 0x3D] {
        let mut cpu = Cpu::new();
        let mut memory = Memory::new(64);

        cpu.cp0_mut().write_reg(12, 0, 1 << 29);
        cpu.write_reg(8, 32);

        memory.write_doubleword(32, 0x1234_5678_9ABC_DEF0);

        // Invalid doubleword register in FR=0
        memory.write_word(0, i(opcode, 8, 31, 0));

        cpu.step(&mut memory);

        assert_eq!(cpu.cp0().exception_code(), 10);

        assert_eq!(cpu.cp1().read_raw(31), 0);

        assert_eq!(memory.read_doubleword(32), 0x1234_5678_9ABC_DEF0,);
    }
}

#[test]
fn cp1_doubleword_memory_tlb_refill() {
    for (opcode, exception_code) in [(0x35, 2), (0x3D, 3)] {
        let mut cpu = Cpu::new_with_tlb();
        let mut memory = Memory::new(64);

        cpu.cp0_mut().write_reg(12, 0, 1 << 29);
        cpu.write_pc(0x8000_0000);
        cpu.write_reg(8, 0x0040_0000);

        let value = 0xAAAA_BBBB_CCCC_DDDD;

        cpu.cp1_mut().write_doubleword(2, value);

        memory.write_word(0, i(opcode, 8, 2, 0));

        cpu.step(&mut memory);

        assert_eq!(cpu.cp0().exception_code(), exception_code,);

        assert_eq!(cpu.cp0().read_bad_vaddr(), 0x0040_0000,);

        assert_eq!(cpu.cp0().read_epc(), 0x8000_0000,);

        assert_eq!(cpu.cp1().read_doubleword(2), Some(value),);

        assert_eq!(memory.read_doubleword(32), 0);
    }
}

#[test]
fn sdc1_clears_reservation() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(64);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);
    cpu.write_reg(8, 32);

    cpu.set_reservation(32);

    cpu.cp1_mut().write_doubleword(2, 0x1234_5678_9ABC_DEF0);

    memory.write_word(0, i(0x3D, 8, 2, 0));

    assert!(cpu.check_reservation(32));

    cpu.step(&mut memory);

    assert!(!cpu.check_reservation(32));

    assert_eq!(memory.read_doubleword(32), 0x1234_5678_9ABC_DEF0,);

    assert!(!cpu.cp0().exception_level());
}

#[test]
fn fpu_single_add() {
    use crate::fpu::compute_single;
    use crate::isa::Operation;

    let result = compute_single(&Operation::AddS, 1.5f32.to_bits(), 2.25f32.to_bits());

    assert_eq!(result, 3.75f32.to_bits());
}

#[test]
fn fpu_single_sub() {
    use crate::fpu::compute_single;
    use crate::isa::Operation;

    let result = compute_single(&Operation::SubS, 1.5f32.to_bits(), 2.25f32.to_bits());

    assert_eq!(result, (-0.75f32).to_bits());
}

#[test]
fn adds_execution() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    cpu.cp1_mut().write_word(2, 1.5f32.to_bits());
    cpu.cp1_mut().write_word(4, 2.25f32.to_bits());

    // ADD.S $f6, $f2, $f4
    memory.write_word(0, encode_r(0x11, 0x10, 4, 2, 6, 0));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp1().read_word(6), 3.75f32.to_bits(),);

    assert!(!cpu.cp0().exception_level());
}

#[test]
fn subs_execution() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    cpu.cp1_mut().write_word(2, 1.5f32.to_bits());
    cpu.cp1_mut().write_word(4, 2.25f32.to_bits());

    // SUB.S $f6, $f2, $f4
    memory.write_word(0, encode_r(0x11, 0x10, 4, 2, 6, 1));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp1().read_word(6), (-0.75f32).to_bits(),);

    assert!(!cpu.cp0().exception_level());
}

#[test]
fn adds_negative_operands() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    cpu.cp1_mut().write_word(2, (-3.5f32).to_bits());

    cpu.cp1_mut().write_word(4, (-2.25f32).to_bits());

    memory.write_word(0, encode_r(0x11, 0x10, 4, 2, 6, 0));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp1().read_word(6), (-5.75f32).to_bits(),);

    assert!(!cpu.cp0().exception_level());
}

#[test]
fn adds_overlapping_registers() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    cpu.cp1_mut().write_word(2, 1.5f32.to_bits());
    cpu.cp1_mut().write_word(4, 2.25f32.to_bits());

    // ADD.S $f2, $f2, $f4
    memory.write_word(0, encode_r(0x11, 0x10, 4, 2, 2, 0));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp1().read_word(2), 3.75f32.to_bits(),);

    assert_eq!(cpu.cp1().read_word(4), 2.25f32.to_bits(),);

    assert!(!cpu.cp0().exception_level());
}

#[test]
fn cp1_single_arithmetic_disabled() {
    for funct in [0, 1] {
        let mut cpu = Cpu::new();
        let mut memory = Memory::new(4);

        cpu.cp1_mut().write_word(2, 1.5f32.to_bits());
        cpu.cp1_mut().write_word(4, 2.25f32.to_bits());
        cpu.cp1_mut().write_word(6, 0xDEAD_BEEF);

        memory.write_word(0, encode_r(0x11, 0x10, 4, 2, 6, funct));

        cpu.step(&mut memory);

        assert_eq!(cpu.cp0().exception_code(), 11);
        assert_eq!(cpu.cp0().coprocessor_error(), 1);
        assert_eq!(cpu.cp1().read_word(6), 0xDEAD_BEEF);
    }
}

#[test]
fn cp1_single_arithmetic_integration() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(64);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    cpu.write_reg(8, 1.5f32.to_bits());
    cpu.write_reg(9, 2.25f32.to_bits());
    cpu.write_reg(11, 48);

    let program = [
        // MTC1 $8, $f2
        encode_r(0x11, 4, 8, 2, 0, 0),
        // MTC1 $9, $f4
        encode_r(0x11, 4, 9, 4, 0, 0),
        // ADD.S $f6, $f2, $f4
        encode_r(0x11, 0x10, 4, 2, 6, 0),
        // MFC1 $10, $f6
        encode_r(0x11, 0, 10, 6, 0, 0),
        // SWC1 $f6, 0($11)
        i(0x39, 11, 6, 0),
        // LWC1 $f8, 0($11)
        i(0x31, 11, 8, 0),
    ];

    for (index, word) in program.into_iter().enumerate() {
        memory.write_word((index * 4) as u32, word);
    }

    for _ in 0..6 {
        cpu.step(&mut memory);
    }

    let expected = 3.75f32.to_bits();

    assert_eq!(cpu.read_reg(10), expected);
    assert_eq!(cpu.cp1().read_word(6), expected);
    assert_eq!(cpu.cp1().read_word(8), expected);
    assert_eq!(memory.read_word(48), expected);
    assert!(!cpu.cp0().exception_level());
}

#[test]
fn fpu_single_mul() {
    use crate::fpu::compute_single;
    use crate::isa::Operation;

    let result = compute_single(&Operation::MulS, 1.5f32.to_bits(), 4.0f32.to_bits());

    assert_eq!(result, 6.0f32.to_bits());
}

#[test]
fn fpu_single_div() {
    use crate::fpu::compute_single;
    use crate::isa::Operation;

    let result = compute_single(&Operation::DivS, 7.5f32.to_bits(), 2.5f32.to_bits());

    assert_eq!(result, 3.0f32.to_bits());
}

#[test]
fn muls_execution() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    cpu.cp1_mut().write_word(2, 1.5f32.to_bits());
    cpu.cp1_mut().write_word(4, 4.0f32.to_bits());

    // MUL.S $f6, $f2, $f4
    memory.write_word(0, encode_r(0x11, 0x10, 4, 2, 6, 2));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp1().read_word(6), 6.0f32.to_bits(),);

    assert!(!cpu.cp0().exception_level());
}

#[test]
fn divs_execution() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    cpu.cp1_mut().write_word(2, 7.5f32.to_bits());
    cpu.cp1_mut().write_word(4, 2.5f32.to_bits());

    // DIV.S $f6, $f2, $f4
    memory.write_word(0, encode_r(0x11, 0x10, 4, 2, 6, 3));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp1().read_word(6), 3.0f32.to_bits(),);

    assert!(!cpu.cp0().exception_level());
}

#[test]
fn muls_negative_operand() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    cpu.cp1_mut().write_word(2, (-2.5f32).to_bits());
    cpu.cp1_mut().write_word(4, 4.0f32.to_bits());

    memory.write_word(0, encode_r(0x11, 0x10, 4, 2, 6, 2));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp1().read_word(6), (-10.0f32).to_bits(),);

    assert!(!cpu.cp0().exception_level());
}

#[test]
fn divs_overlapping_registers() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    cpu.cp1_mut().write_word(2, 12.0f32.to_bits());
    cpu.cp1_mut().write_word(4, 4.0f32.to_bits());

    // DIV.S $f4, $f2, $f4
    memory.write_word(0, encode_r(0x11, 0x10, 4, 2, 4, 3));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp1().read_word(4), 3.0f32.to_bits(),);

    assert_eq!(cpu.cp1().read_word(2), 12.0f32.to_bits(),);

    assert!(!cpu.cp0().exception_level());
}

#[test]
fn cp1_mul_div_disabled() {
    for funct in [2, 3] {
        let mut cpu = Cpu::new();
        let mut memory = Memory::new(4);

        cpu.cp1_mut().write_word(2, 6.0f32.to_bits());
        cpu.cp1_mut().write_word(4, 2.0f32.to_bits());
        cpu.cp1_mut().write_word(6, 0xDEAD_BEEF);

        memory.write_word(0, encode_r(0x11, 0x10, 4, 2, 6, funct));

        cpu.step(&mut memory);

        assert_eq!(cpu.cp0().exception_code(), 11);
        assert_eq!(cpu.cp0().coprocessor_error(), 1);
        assert_eq!(cpu.cp1().read_word(6), 0xDEAD_BEEF);
    }
}

#[test]
fn cp1_single_arithmetic_chain() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(16);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    cpu.cp1_mut().write_word(2, 8.0f32.to_bits());
    cpu.cp1_mut().write_word(4, 4.0f32.to_bits());
    cpu.cp1_mut().write_word(6, 2.0f32.to_bits());

    let program = [
        // ADD.S $f8, $f2, $f4
        encode_r(0x11, 0x10, 4, 2, 8, 0),
        // MUL.S $f8, $f8, $f6
        encode_r(0x11, 0x10, 6, 8, 8, 2),
        // SUB.S $f8, $f8, $f4
        encode_r(0x11, 0x10, 4, 8, 8, 1),
        // DIV.S $f8, $f8, $f6
        encode_r(0x11, 0x10, 6, 8, 8, 3),
    ];

    for (index, word) in program.into_iter().enumerate() {
        memory.write_word((index * 4) as u32, word);
    }

    for _ in 0..4 {
        cpu.step(&mut memory);
    }

    assert_eq!(cpu.cp1().read_word(8), 10.0f32.to_bits(),);

    assert!(!cpu.cp0().exception_level());
}

#[test]
fn fpu_single_abs() {
    use crate::fpu::compute_single_unary;
    use crate::isa::Operation;

    assert_eq!(
        compute_single_unary(&Operation::AbsS, (-2.5f32).to_bits(),),
        2.5f32.to_bits(),
    );
}

#[test]
fn fpu_single_neg() {
    use crate::fpu::compute_single_unary;
    use crate::isa::Operation;

    for value in [2.5f32, -2.5, 0.0, -0.0] {
        assert_eq!(
            compute_single_unary(&Operation::NegS, value.to_bits(),),
            (-value).to_bits(),
        );
    }
}

#[test]
fn fpu_single_mov_preserves_bits() {
    use crate::fpu::compute_single_unary;
    use crate::isa::Operation;

    for value in [0x0000_0000, 0x8000_0000, 0x7FC0_1234, 0xFFFF_FFFF] {
        assert_eq!(compute_single_unary(&Operation::MovS, value,), value,);
    }
}

#[test]
fn fpu_single_sqrt() {
    use crate::fpu::compute_single_unary;
    use crate::isa::Operation;

    assert_eq!(
        compute_single_unary(&Operation::SqrtS, 6.25f32.to_bits(),),
        2.5f32.to_bits(),
    );
}

#[test]
fn abss_execution() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    cpu.cp1_mut().write_word(2, (-3.5f32).to_bits());

    // ABS.S $f4, $f2
    memory.write_word(0, encode_r(0x11, 0x10, 0, 2, 4, 5));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp1().read_word(4), 3.5f32.to_bits(),);

    assert!(!cpu.cp0().exception_level());
}

#[test]
fn negs_execution() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    cpu.cp1_mut().write_word(2, 3.5f32.to_bits());

    // NEG.S $f4, $f2
    memory.write_word(0, encode_r(0x11, 0x10, 0, 2, 4, 7));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp1().read_word(4), (-3.5f32).to_bits(),);

    assert!(!cpu.cp0().exception_level());
}

#[test]
fn movs_execution() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    let value = 0x7FC0_1234;

    cpu.cp1_mut().write_word(2, value);

    // MOV.S $f4, $f2
    memory.write_word(0, encode_r(0x11, 0x10, 0, 2, 4, 6));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp1().read_word(4), value);
    assert_eq!(cpu.cp1().read_word(2), value);
    assert!(!cpu.cp0().exception_level());
}

#[test]
fn sqrts_execution() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    cpu.cp1_mut().write_word(2, 144.0f32.to_bits());

    // SQRT.S $f4, $f2
    memory.write_word(0, encode_r(0x11, 0x10, 0, 2, 4, 4));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp1().read_word(4), 12.0f32.to_bits(),);

    assert!(!cpu.cp0().exception_level());
}

#[test]
fn cp1_single_unary_disabled() {
    for funct in [4, 5, 6, 7] {
        let mut cpu = Cpu::new();
        let mut memory = Memory::new(4);

        cpu.cp1_mut().write_word(2, 4.0f32.to_bits());

        cpu.cp1_mut().write_word(4, 0xDEAD_BEEF);

        memory.write_word(0, encode_r(0x11, 0x10, 0, 2, 4, funct));

        cpu.step(&mut memory);

        assert_eq!(cpu.cp0().exception_code(), 11);
        assert_eq!(cpu.cp0().coprocessor_error(), 1);
        assert_eq!(cpu.cp1().read_word(4), 0xDEAD_BEEF);
    }
}

#[test]
fn cp1_single_unary_chain() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(16);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    cpu.cp1_mut().write_word(2, (-81.0f32).to_bits());

    let program = [
        // ABS.S $f4, $f2
        encode_r(0x11, 0x10, 0, 2, 4, 5),
        // SQRT.S $f4, $f4
        encode_r(0x11, 0x10, 0, 4, 4, 4),
        // NEG.S $f4, $f4
        encode_r(0x11, 0x10, 0, 4, 4, 7),
        // MOV.S $f6, $f4
        encode_r(0x11, 0x10, 0, 4, 6, 6),
    ];

    for (index, word) in program.into_iter().enumerate() {
        memory.write_word((index * 4) as u32, word);
    }

    for _ in 0..4 {
        cpu.step(&mut memory);
    }

    assert_eq!(cpu.cp1().read_word(6), (-9.0f32).to_bits(),);

    assert_eq!(cpu.cp1().read_word(2), (-81.0f32).to_bits(),);

    assert!(!cpu.cp0().exception_level());
}

#[test]
fn fpu_double_add() {
    use crate::fpu::compute_double;
    use crate::isa::Operation;

    assert_eq!(
        compute_double(&Operation::AddD, 1.5f64.to_bits(), 2.25f64.to_bits(),),
        3.75f64.to_bits(),
    );
}

#[test]
fn fpu_double_sub() {
    use crate::fpu::compute_double;
    use crate::isa::Operation;

    assert_eq!(
        compute_double(&Operation::SubD, 1.5f64.to_bits(), 2.25f64.to_bits(),),
        (-0.75f64).to_bits(),
    );
}

#[test]
fn addd_execution() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    cpu.cp1_mut().write_doubleword(2, 1.5f64.to_bits());

    cpu.cp1_mut().write_doubleword(4, 2.25f64.to_bits());

    // ADD.D $f6, $f2, $f4
    memory.write_word(0, encode_r(0x11, 0x11, 4, 2, 6, 0));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp1().read_doubleword(6), Some(3.75f64.to_bits()),);

    assert!(!cpu.cp0().exception_level());
}

#[test]
fn subd_execution() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    cpu.cp1_mut().write_doubleword(2, 1.5f64.to_bits());

    cpu.cp1_mut().write_doubleword(4, 2.25f64.to_bits());

    // SUB.D $f6, $f2, $f4
    memory.write_word(0, encode_r(0x11, 0x11, 4, 2, 6, 1));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp1().read_doubleword(6), Some((-0.75f64).to_bits()),);

    assert!(!cpu.cp0().exception_level());
}

#[test]
fn addd_precision() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    cpu.cp1_mut().write_doubleword(2, 16_777_216.0f64.to_bits());

    cpu.cp1_mut().write_doubleword(4, 1.0f64.to_bits());

    memory.write_word(0, encode_r(0x11, 0x11, 4, 2, 6, 0));

    cpu.step(&mut memory);

    assert_eq!(
        cpu.cp1().read_doubleword(6),
        Some(16_777_217.0f64.to_bits()),
    );

    assert!(!cpu.cp0().exception_level());
}

#[test]
fn addd_overlapping_registers() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    cpu.cp1_mut().write_doubleword(2, 1.5f64.to_bits());

    cpu.cp1_mut().write_doubleword(4, 2.25f64.to_bits());

    // ADD.D $f2, $f2, $f4
    memory.write_word(0, encode_r(0x11, 0x11, 4, 2, 2, 0));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp1().read_doubleword(2), Some(3.75f64.to_bits()),);

    assert_eq!(cpu.cp1().read_doubleword(4), Some(2.25f64.to_bits()),);

    assert!(!cpu.cp0().exception_level());
}

#[test]
fn cp1_double_arithmetic_invalid_source() {
    for (fs, ft) in [(3, 4), (2, 5)] {
        for funct in [0, 1] {
            let mut cpu = Cpu::new();
            let mut memory = Memory::new(4);

            cpu.cp0_mut().write_reg(12, 0, 1 << 29);

            cpu.cp1_mut().write_doubleword(6, 0x1234_5678_9ABC_DEF0);

            memory.write_word(0, encode_r(0x11, 0x11, ft, fs, 6, funct));

            cpu.step(&mut memory);

            assert_eq!(cpu.cp0().exception_code(), 10);

            assert_eq!(cpu.cp1().read_doubleword(6), Some(0x1234_5678_9ABC_DEF0),);
        }
    }
}

#[test]
fn cp1_double_arithmetic_invalid_destination() {
    for funct in [0, 1] {
        let mut cpu = Cpu::new();
        let mut memory = Memory::new(4);

        cpu.cp0_mut().write_reg(12, 0, 1 << 29);

        cpu.cp1_mut().write_doubleword(2, 1.5f64.to_bits());

        cpu.cp1_mut().write_doubleword(4, 2.25f64.to_bits());

        cpu.cp1_mut().write_word(31, 0xDEAD_BEEF);

        memory.write_word(0, encode_r(0x11, 0x11, 4, 2, 31, funct));

        cpu.step(&mut memory);

        assert_eq!(cpu.cp0().exception_code(), 10);
        assert_eq!(cpu.cp1().read_word(31), 0xDEAD_BEEF);
    }
}

#[test]
fn cp1_double_arithmetic_disabled() {
    for funct in [0, 1] {
        let mut cpu = Cpu::new();
        let mut memory = Memory::new(4);

        cpu.cp1_mut().write_doubleword(6, 0x1234_5678_9ABC_DEF0);

        memory.write_word(0, encode_r(0x11, 0x11, 4, 2, 6, funct));

        cpu.step(&mut memory);

        assert_eq!(cpu.cp0().exception_code(), 11);
        assert_eq!(cpu.cp0().coprocessor_error(), 1);

        assert_eq!(cpu.cp1().read_doubleword(6), Some(0x1234_5678_9ABC_DEF0),);
    }
}

#[test]
fn cp1_double_arithmetic_integration() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(128);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);
    cpu.write_reg(8, 64);

    memory.write_doubleword(64, 1.5f64.to_bits());

    memory.write_doubleword(72, 2.25f64.to_bits());

    let program = [
        // LDC1 $f2, 0($8)
        i(0x35, 8, 2, 0),
        // LDC1 $f4, 8($8)
        i(0x35, 8, 4, 8),
        // ADD.D $f6, $f2, $f4
        encode_r(0x11, 0x11, 4, 2, 6, 0),
        // SDC1 $f6, 16($8)
        i(0x3D, 8, 6, 16),
    ];

    for (index, word) in program.into_iter().enumerate() {
        memory.write_word((index * 4) as u32, word);
    }

    for _ in 0..4 {
        cpu.step(&mut memory);
    }

    assert_eq!(memory.read_doubleword(80), 3.75f64.to_bits(),);

    assert_eq!(cpu.cp1().read_doubleword(6), Some(3.75f64.to_bits()),);

    assert!(!cpu.cp0().exception_level());
}

#[test]
fn fpu_double_mul() {
    use crate::fpu::compute_double;
    use crate::isa::Operation;

    assert_eq!(
        compute_double(&Operation::MulD, 1.5f64.to_bits(), 4.0f64.to_bits(),),
        6.0f64.to_bits(),
    );
}

#[test]
fn fpu_double_div() {
    use crate::fpu::compute_double;
    use crate::isa::Operation;

    assert_eq!(
        compute_double(&Operation::DivD, 7.5f64.to_bits(), 2.5f64.to_bits(),),
        3.0f64.to_bits(),
    );
}

#[test]
fn muld_execution() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    cpu.cp1_mut().write_doubleword(2, 1.5f64.to_bits());

    cpu.cp1_mut().write_doubleword(4, 4.0f64.to_bits());

    // MUL.D $f6, $f2, $f4
    memory.write_word(0, encode_r(0x11, 0x11, 4, 2, 6, 2));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp1().read_doubleword(6), Some(6.0f64.to_bits()),);

    assert!(!cpu.cp0().exception_level());
}

#[test]
fn divd_execution() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    cpu.cp1_mut().write_doubleword(2, 7.5f64.to_bits());

    cpu.cp1_mut().write_doubleword(4, 2.5f64.to_bits());

    // DIV.D $f6, $f2, $f4
    memory.write_word(0, encode_r(0x11, 0x11, 4, 2, 6, 3));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp1().read_doubleword(6), Some(3.0f64.to_bits()),);

    assert!(!cpu.cp0().exception_level());
}

#[test]
fn muld_extended_exponent_range() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    cpu.cp1_mut()
        .write_doubleword(2, (2.0f64.powi(100)).to_bits());

    cpu.cp1_mut()
        .write_doubleword(4, (2.0f64.powi(100)).to_bits());

    memory.write_word(0, encode_r(0x11, 0x11, 4, 2, 6, 2));

    cpu.step(&mut memory);

    assert_eq!(
        cpu.cp1().read_doubleword(6),
        Some((2.0f64.powi(200)).to_bits()),
    );

    assert!(!cpu.cp0().exception_level());
}

#[test]
fn divd_overlapping_registers() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    cpu.cp1_mut().write_doubleword(2, (-12.0f64).to_bits());

    cpu.cp1_mut().write_doubleword(4, 4.0f64.to_bits());

    // DIV.D $f4, $f2, $f4
    memory.write_word(0, encode_r(0x11, 0x11, 4, 2, 4, 3));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp1().read_doubleword(4), Some((-3.0f64).to_bits()),);

    assert_eq!(cpu.cp1().read_doubleword(2), Some((-12.0f64).to_bits()),);

    assert!(!cpu.cp0().exception_level());
}

#[test]
fn cp1_double_mul_div_access_validation() {
    for funct in [2, 3] {
        for (enabled, fs, ft, fd, expected) in [
            (false, 2, 4, 6, 11),
            (true, 3, 4, 6, 10),
            (true, 2, 5, 6, 10),
            (true, 2, 4, 7, 10),
        ] {
            let mut cpu = Cpu::new();
            let mut memory = Memory::new(4);

            if enabled {
                cpu.cp0_mut().write_reg(12, 0, 1 << 29);
            }

            let sentinel = 0x1234_5678_9ABC_DEF0;

            cpu.cp1_mut().write_doubleword(6, sentinel);

            memory.write_word(0, encode_r(0x11, 0x11, ft, fs, fd, funct));

            cpu.step(&mut memory);

            assert_eq!(cpu.cp0().exception_code(), expected,);

            if !enabled {
                assert_eq!(cpu.cp0().coprocessor_error(), 1,);
            }

            assert_eq!(cpu.cp1().read_doubleword(6), Some(sentinel),);
        }
    }
}

#[test]
fn cp1_double_arithmetic_chain() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(16);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    cpu.cp1_mut().write_doubleword(2, 8.0f64.to_bits());

    cpu.cp1_mut().write_doubleword(4, 4.0f64.to_bits());

    cpu.cp1_mut().write_doubleword(6, 2.0f64.to_bits());

    let program = [
        // ADD.D $f8, $f2, $f4
        encode_r(0x11, 0x11, 4, 2, 8, 0),
        // MUL.D $f8, $f8, $f6
        encode_r(0x11, 0x11, 6, 8, 8, 2),
        // SUB.D $f8, $f8, $f4
        encode_r(0x11, 0x11, 4, 8, 8, 1),
        // DIV.D $f8, $f8, $f6
        encode_r(0x11, 0x11, 6, 8, 8, 3),
    ];

    for (index, word) in program.into_iter().enumerate() {
        memory.write_word((index * 4) as u32, word);
    }

    for _ in 0..4 {
        cpu.step(&mut memory);
    }

    assert_eq!(cpu.cp1().read_doubleword(8), Some(10.0f64.to_bits()),);

    assert!(!cpu.cp0().exception_level());
}

#[test]
fn fpu_double_abs() {
    use crate::fpu::compute_double_unary;
    use crate::isa::Operation;

    assert_eq!(
        compute_double_unary(&Operation::AbsD, (-2.5f64).to_bits(),),
        2.5f64.to_bits(),
    );
}

#[test]
fn fpu_double_neg() {
    use crate::fpu::compute_double_unary;
    use crate::isa::Operation;

    for value in [2.5f64, -2.5, 0.0, -0.0] {
        assert_eq!(
            compute_double_unary(&Operation::NegD, value.to_bits(),),
            (-value).to_bits(),
        );
    }
}

#[test]
fn fpu_double_mov_preserves_bits() {
    use crate::fpu::compute_double_unary;
    use crate::isa::Operation;

    for value in [
        0x0000_0000_0000_0000,
        0x8000_0000_0000_0000,
        0x7FF8_1234_5678_9ABC,
        u64::MAX,
    ] {
        assert_eq!(compute_double_unary(&Operation::MovD, value,), value,);
    }
}

#[test]
fn fpu_double_sqrt() {
    use crate::fpu::compute_double_unary;
    use crate::isa::Operation;

    assert_eq!(
        compute_double_unary(&Operation::SqrtD, 6.25f64.to_bits(),),
        2.5f64.to_bits(),
    );
}

#[test]
fn absd_execution() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    cpu.cp1_mut().write_doubleword(2, (-3.5f64).to_bits());

    memory.write_word(0, encode_r(0x11, 0x11, 0, 2, 4, 5));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp1().read_doubleword(4), Some(3.5f64.to_bits()),);

    assert!(!cpu.cp0().exception_level());
}

#[test]
fn negd_execution() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    cpu.cp1_mut().write_doubleword(2, 3.5f64.to_bits());

    memory.write_word(0, encode_r(0x11, 0x11, 0, 2, 4, 7));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp1().read_doubleword(4), Some((-3.5f64).to_bits()),);

    assert!(!cpu.cp0().exception_level());
}

#[test]
fn movd_execution() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    let value = 0x7FF8_1234_5678_9ABC;

    cpu.cp1_mut().write_doubleword(2, value);

    memory.write_word(0, encode_r(0x11, 0x11, 0, 2, 4, 6));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp1().read_doubleword(4), Some(value),);

    assert_eq!(cpu.cp1().read_doubleword(2), Some(value),);

    assert!(!cpu.cp0().exception_level());
}

#[test]
fn sqrtd_execution() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    cpu.cp1_mut().write_doubleword(2, 144.0f64.to_bits());

    memory.write_word(0, encode_r(0x11, 0x11, 0, 2, 4, 4));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp1().read_doubleword(4), Some(12.0f64.to_bits()),);

    assert!(!cpu.cp0().exception_level());
}

#[test]
fn sqrtd_extended_exponent_range() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    cpu.cp1_mut()
        .write_doubleword(2, 2.0f64.powi(200).to_bits());

    memory.write_word(0, encode_r(0x11, 0x11, 0, 2, 4, 4));

    cpu.step(&mut memory);

    assert_eq!(
        cpu.cp1().read_doubleword(4),
        Some(2.0f64.powi(100).to_bits()),
    );

    assert!(!cpu.cp0().exception_level());
}

#[test]
fn cp1_double_unary_invalid_register() {
    for funct in [4, 5, 6, 7] {
        for (fs, fd) in [(3, 4), (2, 5)] {
            let mut cpu = Cpu::new();
            let mut memory = Memory::new(4);

            cpu.cp0_mut().write_reg(12, 0, 1 << 29);

            let sentinel = 0x1234_5678_9ABC_DEF0;

            cpu.cp1_mut().write_doubleword(4, sentinel);

            memory.write_word(0, encode_r(0x11, 0x11, 0, fs, fd, funct));

            cpu.step(&mut memory);

            assert_eq!(cpu.cp0().exception_code(), 10);

            assert_eq!(cpu.cp1().read_doubleword(4), Some(sentinel),);
        }
    }
}

#[test]
fn cp1_double_unary_disabled() {
    for funct in [4, 5, 6, 7] {
        let mut cpu = Cpu::new();
        let mut memory = Memory::new(4);

        let sentinel = 0x1234_5678_9ABC_DEF0;

        cpu.cp1_mut().write_doubleword(4, sentinel);

        memory.write_word(0, encode_r(0x11, 0x11, 0, 2, 4, funct));

        cpu.step(&mut memory);

        assert_eq!(cpu.cp0().exception_code(), 11);
        assert_eq!(cpu.cp0().coprocessor_error(), 1);

        assert_eq!(cpu.cp1().read_doubleword(4), Some(sentinel),);
    }
}

#[test]
fn cp1_double_unary_chain() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(16);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    cpu.cp1_mut().write_doubleword(2, (-81.0f64).to_bits());

    let program = [
        // ABS.D $f4, $f2
        encode_r(0x11, 0x11, 0, 2, 4, 5),
        // SQRT.D $f4, $f4
        encode_r(0x11, 0x11, 0, 4, 4, 4),
        // NEG.D $f4, $f4
        encode_r(0x11, 0x11, 0, 4, 4, 7),
        // MOV.D $f30, $f4
        encode_r(0x11, 0x11, 0, 4, 30, 6),
    ];

    for (index, word) in program.into_iter().enumerate() {
        memory.write_word((index * 4) as u32, word);
    }

    for _ in 0..4 {
        cpu.step(&mut memory);
    }

    assert_eq!(cpu.cp1().read_doubleword(30), Some((-9.0f64).to_bits()),);

    assert_eq!(cpu.cp1().read_doubleword(2), Some((-81.0f64).to_bits()),);

    assert!(!cpu.cp0().exception_level());
}

#[test]
fn fpu_convert_single_to_double() {
    use crate::fpu::convert_single_to_double;

    assert_eq!(convert_single_to_double(1.5f32.to_bits()), 1.5f64.to_bits(),);
}

#[test]
fn fpu_convert_double_to_single() {
    use crate::fpu::convert_double_to_single;

    assert_eq!(
        convert_double_to_single(2.25f64.to_bits()),
        2.25f32.to_bits(),
    );
}

#[test]
fn fpu_convert_single_preserves_numeric_value() {
    use crate::fpu::convert_single_to_double;

    let source = 0.1f32;

    let result = f64::from_bits(convert_single_to_double(source.to_bits()));

    assert_eq!(result, source as f64);
    assert_ne!(result, 0.1f64);
}

#[test]
fn fpu_convert_double_precision_loss() {
    use crate::fpu::convert_double_to_single;

    let result = convert_double_to_single(16_777_217.0f64.to_bits());

    assert_eq!(result, 16_777_216.0f32.to_bits(),);
}

#[test]
fn cvt_d_s_execution() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    cpu.cp1_mut().write_word(3, 1.5f32.to_bits());

    // CVT.D.S $f4, $f3
    memory.write_word(0, encode_r(0x11, 0x10, 0, 3, 4, 0x21));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp1().read_doubleword(4), Some(1.5f64.to_bits()),);

    assert!(!cpu.cp0().exception_level());
}

#[test]
fn cvt_s_d_execution() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    cpu.cp1_mut().write_doubleword(2, 2.25f64.to_bits());

    // CVT.S.D $f5, $f2
    memory.write_word(0, encode_r(0x11, 0x11, 0, 2, 5, 0x20));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp1().read_word(5), 2.25f32.to_bits(),);

    assert!(!cpu.cp0().exception_level());
}

#[test]
fn cvt_d_s_invalid_destination() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    cpu.cp1_mut().write_word(2, 1.5f32.to_bits());
    cpu.cp1_mut().write_word(31, 0xDEAD_BEEF);

    memory.write_word(0, encode_r(0x11, 0x10, 0, 2, 31, 0x21));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp0().exception_code(), 10);
    assert_eq!(cpu.cp1().read_word(31), 0xDEAD_BEEF);
}

#[test]
fn cvt_s_d_invalid_source() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    cpu.cp1_mut().write_word(6, 0xDEAD_BEEF);

    memory.write_word(0, encode_r(0x11, 0x11, 0, 3, 6, 0x20));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp0().exception_code(), 10);
    assert_eq!(cpu.cp1().read_word(6), 0xDEAD_BEEF);
}

#[test]
fn cp1_float_conversion_disabled() {
    for (fmt, funct) in [(0x10, 0x21), (0x11, 0x20)] {
        let mut cpu = Cpu::new();
        let mut memory = Memory::new(4);

        let sentinel = 0x1234_5678_9ABC_DEF0;

        cpu.cp1_mut().write_doubleword(6, sentinel);

        memory.write_word(0, encode_r(0x11, fmt, 0, 2, 6, funct));

        cpu.step(&mut memory);

        assert_eq!(cpu.cp0().exception_code(), 11);
        assert_eq!(cpu.cp0().coprocessor_error(), 1);

        assert_eq!(cpu.cp1().read_doubleword(6), Some(sentinel),);
    }
}

#[test]
fn cp1_float_conversion_round_trip() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(12);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    let value = 1.5f32.to_bits();

    cpu.cp1_mut().write_word(3, value);

    let program = [
        // CVT.D.S $f4, $f3
        encode_r(0x11, 0x10, 0, 3, 4, 0x21),
        // ADD.D $f6, $f4, $f4
        encode_r(0x11, 0x11, 4, 4, 6, 0),
        // CVT.S.D $f7, $f6
        encode_r(0x11, 0x11, 0, 6, 7, 0x20),
    ];

    for (index, word) in program.into_iter().enumerate() {
        memory.write_word((index * 4) as u32, word);
    }

    for _ in 0..3 {
        cpu.step(&mut memory);
    }

    assert_eq!(cpu.cp1().read_doubleword(4), Some(1.5f64.to_bits()),);

    assert_eq!(cpu.cp1().read_word(7), 3.0f32.to_bits(),);

    assert_eq!(cpu.cp1().read_word(3), value);
    assert!(!cpu.cp0().exception_level());
}

#[test]
fn fpu_convert_word_to_single() {
    use crate::fpu::convert_word_to_single;

    assert_eq!(convert_word_to_single(42), 42.0f32.to_bits(),);
}

#[test]
fn fpu_convert_word_to_double() {
    use crate::fpu::convert_word_to_double;

    assert_eq!(convert_word_to_double(42), 42.0f64.to_bits(),);
}

#[test]
fn fpu_word_conversion_signed() {
    use crate::fpu::{convert_word_to_double, convert_word_to_single};

    for value in [-1i32, -42, -1024] {
        assert_eq!(
            convert_word_to_single(value as u32),
            (value as f32).to_bits(),
        );

        assert_eq!(
            convert_word_to_double(value as u32),
            (value as f64).to_bits(),
        );
    }
}

#[test]
fn fpu_word_conversion_boundaries() {
    use crate::fpu::{convert_word_to_double, convert_word_to_single};

    for value in [i32::MIN, i32::MAX, 0] {
        assert_eq!(
            convert_word_to_single(value as u32),
            (value as f32).to_bits(),
        );

        assert_eq!(
            convert_word_to_double(value as u32),
            (value as f64).to_bits(),
        );
    }
}

#[test]
fn fpu_word_conversion_precision() {
    use crate::fpu::{convert_word_to_double, convert_word_to_single};

    let value = 16_777_217u32;

    assert_eq!(convert_word_to_single(value), 16_777_216.0f32.to_bits(),);

    assert_eq!(convert_word_to_double(value), 16_777_217.0f64.to_bits(),);
}

#[test]
fn cvt_s_w_execution() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    cpu.cp1_mut().write_word(3, (-42i32) as u32);

    // CVT.S.W $f5, $f3
    memory.write_word(0, encode_r(0x11, 0x14, 0, 3, 5, 0x20));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp1().read_word(5), (-42.0f32).to_bits(),);

    assert!(!cpu.cp0().exception_level());
}

#[test]
fn cvt_d_w_execution() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    cpu.cp1_mut().write_word(3, (-42i32) as u32);

    // CVT.D.W $f4, $f3
    memory.write_word(0, encode_r(0x11, 0x14, 0, 3, 4, 0x21));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp1().read_doubleword(4), Some((-42.0f64).to_bits()),);

    assert!(!cpu.cp0().exception_level());
}

#[test]
fn cvt_d_w_invalid_destination() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    cpu.cp1_mut().write_word(2, 42);
    cpu.cp1_mut().write_word(31, 0xDEAD_BEEF);

    memory.write_word(0, encode_r(0x11, 0x14, 0, 2, 31, 0x21));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp0().exception_code(), 10);
    assert_eq!(cpu.cp1().read_word(31), 0xDEAD_BEEF);
}

#[test]
fn cp1_word_conversion_disabled() {
    for funct in [0x20, 0x21] {
        let mut cpu = Cpu::new();
        let mut memory = Memory::new(4);

        let sentinel = 0x1234_5678_9ABC_DEF0;

        cpu.cp1_mut().write_word(2, 42);
        cpu.cp1_mut().write_doubleword(4, sentinel);

        memory.write_word(0, encode_r(0x11, 0x14, 0, 2, 4, funct));

        cpu.step(&mut memory);

        assert_eq!(cpu.cp0().exception_code(), 11);
        assert_eq!(cpu.cp0().coprocessor_error(), 1);

        assert_eq!(cpu.cp1().read_doubleword(4), Some(sentinel),);
    }
}

#[test]
fn cp1_word_conversion_integration() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(20);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    cpu.write_reg(8, (-21i32) as u32);

    let program = [
        // MTC1 $8, $f2
        encode_r(0x11, 4, 8, 2, 0, 0),
        // CVT.S.W $f4, $f2
        encode_r(0x11, 0x14, 0, 2, 4, 0x20),
        // CVT.D.W $f6, $f2
        encode_r(0x11, 0x14, 0, 2, 6, 0x21),
        // ADD.D $f8, $f6, $f6
        encode_r(0x11, 0x11, 6, 6, 8, 0),
        // CVT.S.D $f10, $f8
        encode_r(0x11, 0x11, 0, 8, 10, 0x20),
    ];

    for (index, word) in program.into_iter().enumerate() {
        memory.write_word((index * 4) as u32, word);
    }

    for _ in 0..5 {
        cpu.step(&mut memory);
    }

    assert_eq!(cpu.cp1().read_word(4), (-21.0f32).to_bits(),);

    assert_eq!(cpu.cp1().read_doubleword(6), Some((-21.0f64).to_bits()),);

    assert_eq!(cpu.cp1().read_doubleword(8), Some((-42.0f64).to_bits()),);

    assert_eq!(cpu.cp1().read_word(10), (-42.0f32).to_bits(),);

    assert!(!cpu.cp0().exception_level());
}

#[test]
fn fpu_word_nearest_even() {
    use crate::fpu::{RoundingMode, convert_double_to_word};

    for (value, expected) in [(2.5f64, 2i32), (3.5, 4), (-2.5, -2), (-3.5, -4)] {
        let result = convert_double_to_word(value.to_bits(), RoundingMode::NearestEven);

        assert_eq!(result.value, expected as u32);
    }
}

#[test]
fn fpu_word_toward_zero() {
    use crate::fpu::{RoundingMode, convert_double_to_word};

    for (value, expected) in [(2.9f64, 2i32), (-2.9, -2)] {
        let result = convert_double_to_word(value.to_bits(), RoundingMode::TowardZero);

        assert_eq!(result.value, expected as u32);
    }
}

#[test]
fn fpu_word_toward_positive() {
    use crate::fpu::{RoundingMode, convert_double_to_word};

    for (value, expected) in [(2.1f64, 3i32), (-2.9, -2)] {
        let result = convert_double_to_word(value.to_bits(), RoundingMode::TowardPositive);

        assert_eq!(result.value, expected as u32);
    }
}

#[test]
fn fpu_word_toward_negative() {
    use crate::fpu::{RoundingMode, convert_double_to_word};

    for (value, expected) in [(2.9f64, 2i32), (-2.1, -3)] {
        let result = convert_double_to_word(value.to_bits(), RoundingMode::TowardNegative);

        assert_eq!(result.value, expected as u32);
    }
}

#[test]
fn fpu_word_conversion_exact() {
    use crate::fpu::{RoundingMode, convert_double_to_word};

    for value in [i32::MIN, i32::MAX, -42, 0, 42] {
        let result = convert_double_to_word((value as f64).to_bits(), RoundingMode::NearestEven);

        assert_eq!(result.value, value as u32);
        assert_eq!(result.exceptions, 0);
    }
}

#[test]
fn fpu_word_conversion_inexact() {
    use crate::fpu::{FP_INEXACT, RoundingMode, convert_single_to_word};

    let result = convert_single_to_word(2.5f32.to_bits(), RoundingMode::NearestEven);

    assert_eq!(result.value, 2);
    assert_eq!(result.exceptions, FP_INEXACT);
}

#[test]
fn fpu_word_conversion_invalid() {
    use crate::fpu::{FP_INVALID, RoundingMode, convert_double_to_word};

    for value in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        2_147_483_648.0,
        -2_147_483_649.0,
    ] {
        let result = convert_double_to_word(value.to_bits(), RoundingMode::NearestEven);

        assert_eq!(result.value, 0x7FFF_FFFF);
        assert_eq!(result.exceptions, FP_INVALID);
    }
}

#[test]
fn fpu_word_conversion_rounded_boundary() {
    use crate::fpu::{FP_INEXACT, FP_INVALID, RoundingMode, convert_double_to_word};

    let value = 2_147_483_647.5f64.to_bits();

    let nearest = convert_double_to_word(value, RoundingMode::NearestEven);

    let truncated = convert_double_to_word(value, RoundingMode::TowardZero);

    assert_eq!(nearest.exceptions, FP_INVALID);

    assert_eq!(truncated.value, 0x7FFF_FFFF);
    assert_eq!(truncated.exceptions, FP_INEXACT);
}

#[test]
fn cvt_w_s_execution() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    cpu.cp1_mut().write_word(3, 3.5f32.to_bits());

    // CVT.W.S $f5, $f3
    memory.write_word(0, encode_r(0x11, 0x10, 0, 3, 5, 0x24));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp1().read_word(5), 4);

    // Cause.I and Flag.I
    assert_eq!(cpu.cp1().read_fcsr(), (1 << 12) | (1 << 2),);

    assert!(!cpu.cp0().exception_level());
}

#[test]
fn cvt_w_d_all_rounding_modes() {
    for (rm, expected) in [(0, -2i32), (1, -2), (2, -2), (3, -3)] {
        let mut cpu = Cpu::new();
        let mut memory = Memory::new(4);

        cpu.cp0_mut().write_reg(12, 0, 1 << 29);

        cpu.cp1_mut().write_control(31, rm);

        cpu.cp1_mut().write_doubleword(2, (-2.5f64).to_bits());

        // CVT.W.D $f5, $f2
        memory.write_word(0, encode_r(0x11, 0x11, 0, 2, 5, 0x24));

        cpu.step(&mut memory);

        assert_eq!(cpu.cp1().read_word(5), expected as u32,);

        assert_eq!(cpu.cp1().read_fcsr(), rm | (1 << 12) | (1 << 2),);

        assert!(!cpu.cp0().exception_level());
    }
}

#[test]
fn cvt_w_invalid_disabled() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    cpu.cp1_mut().write_doubleword(2, f64::INFINITY.to_bits());

    memory.write_word(0, encode_r(0x11, 0x11, 0, 2, 4, 0x24));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp1().read_word(4), 0x7FFF_FFFF);

    assert_eq!(cpu.cp1().read_fcsr(), (1 << 16) | (1 << 6),);

    assert!(!cpu.cp0().exception_level());
}

#[test]
fn cvt_w_enabled_exception_no_writeback() {
    for (value, enable, cause) in [(2.5f64, 1 << 7, 1 << 12), (f64::INFINITY, 1 << 11, 1 << 16)] {
        let mut cpu = Cpu::new();
        let mut memory = Memory::new(4);

        cpu.cp0_mut().write_reg(12, 0, 1 << 29);

        cpu.cp1_mut().write_control(31, enable);

        cpu.cp1_mut().write_doubleword(2, value.to_bits());

        cpu.cp1_mut().write_word(4, 0xDEAD_BEEF);

        memory.write_word(0, encode_r(0x11, 0x11, 0, 2, 4, 0x24));

        cpu.step(&mut memory);

        assert_eq!(cpu.cp0().exception_code(), 15);
        assert_eq!(cpu.cp0().read_epc(), 0);

        assert_eq!(cpu.cp1().read_word(4), 0xDEAD_BEEF,);

        assert_eq!(cpu.cp1().read_fcsr(), enable | cause,);
    }
}

#[test]
fn cp1_fp_flags_sticky() {
    use crate::fpu::FP_INEXACT;

    let mut cpu = Cpu::new();

    assert!(!cpu.cp1_mut().apply_fp_exceptions(FP_INEXACT));

    assert_eq!(cpu.cp1().read_fcsr(), (1 << 12) | (1 << 2),);

    assert!(!cpu.cp1_mut().apply_fp_exceptions(0));

    assert_eq!(cpu.cp1().read_fcsr(), 1 << 2,);
}

#[test]
fn cvt_w_access_validation() {
    for (enabled, fmt, fs, expected) in [
        (false, 0x10, 2, 11),
        (false, 0x11, 2, 11),
        (true, 0x11, 3, 10),
    ] {
        let mut cpu = Cpu::new();
        let mut memory = Memory::new(4);

        if enabled {
            cpu.cp0_mut().write_reg(12, 0, 1 << 29);
        }

        cpu.cp1_mut().write_word(4, 0xDEAD_BEEF);

        memory.write_word(0, encode_r(0x11, fmt, 0, fs, 4, 0x24));

        cpu.step(&mut memory);

        assert_eq!(cpu.cp0().exception_code(), expected,);

        assert_eq!(cpu.cp1().read_word(4), 0xDEAD_BEEF,);

        assert_eq!(cpu.cp1().read_fcsr(), 0);
    }
}
