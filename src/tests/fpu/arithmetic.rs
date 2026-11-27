use super::*;

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
