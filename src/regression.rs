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
    assert!(cpu.is_halted());
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

        assert!(cpu.is_halted());
        assert_eq!(cpu.cp0().exception_code(), 12);
        assert_eq!(cpu.cp0().read_epc(), 0);
        assert_eq!(cpu.read_reg(10), 0xDEADBEEF);
    }
}

#[test]
fn system_exception() {
    for (word, code) in [(0x0000000C, 8), (0x0000000D, 9)] {
        let cpu = run(&[word], &[]);

        assert!(cpu.is_halted());
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

        assert_eq!(cpu.is_halted(), expected);
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

        assert_eq!(cpu.is_halted(), expected);
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

    assert!(cpu.is_halted());
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

        assert!(cpu.is_halted());
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

    assert!(cpu.is_halted());
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

    assert!(cpu.is_halted());
    assert_eq!(cpu.cp0().exception_code(), 10);
    assert_eq!(cpu.cp0().read_epc(), 0);
    assert_eq!(cpu.read_reg(8), 0xDEADBEEF);
}
