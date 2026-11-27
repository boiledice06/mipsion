use super::*;

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
