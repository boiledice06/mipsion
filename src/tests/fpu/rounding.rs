use super::*;

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

#[test]
fn cp1_explicit_rounding_single() {
    for (funct, value, expected) in [
        (0x0C, 2.5f32, 2i32),
        (0x0C, 3.5, 4),
        (0x0C, -3.5, -4),
        (0x0D, 2.9, 2),
        (0x0D, -2.9, -2),
        (0x0E, 2.1, 3),
        (0x0E, -2.9, -2),
        (0x0F, 2.9, 2),
        (0x0F, -2.1, -3),
    ] {
        let mut cpu = Cpu::new();
        let mut memory = Memory::new(4);

        cpu.cp0_mut().write_reg(12, 0, 1 << 29);

        cpu.cp1_mut().write_word(3, value.to_bits());

        memory.write_word(0, encode_r(0x11, 0x10, 0, 3, 5, funct));

        cpu.step(&mut memory);

        assert_eq!(cpu.cp1().read_word(5), expected as u32,);

        assert_eq!(cpu.cp1().read_fcsr(), (1 << 12) | (1 << 2),);

        assert!(!cpu.cp0().exception_level());
    }
}

#[test]
fn cp1_explicit_rounding_double() {
    for (funct, value, expected) in [
        (0x0C, 2.5f64, 2i32),
        (0x0C, 3.5, 4),
        (0x0C, -3.5, -4),
        (0x0D, 2.9, 2),
        (0x0D, -2.9, -2),
        (0x0E, 2.1, 3),
        (0x0E, -2.9, -2),
        (0x0F, 2.9, 2),
        (0x0F, -2.1, -3),
    ] {
        let mut cpu = Cpu::new();
        let mut memory = Memory::new(4);

        cpu.cp0_mut().write_reg(12, 0, 1 << 29);

        cpu.cp1_mut().write_doubleword(2, value.to_bits());

        memory.write_word(0, encode_r(0x11, 0x11, 0, 2, 5, funct));

        cpu.step(&mut memory);

        assert_eq!(cpu.cp1().read_word(5), expected as u32,);

        assert_eq!(cpu.cp1().read_fcsr(), (1 << 12) | (1 << 2),);

        assert!(!cpu.cp0().exception_level());
    }
}

#[test]
fn cp1_explicit_rounding_ignores_fcsr_rm() {
    for fmt in [0x10, 0x11] {
        for rm in 0..4 {
            for (funct, expected) in [(0x0C, 4u32), (0x0D, 3), (0x0E, 4), (0x0F, 3)] {
                let mut cpu = Cpu::new();
                let mut memory = Memory::new(4);

                cpu.cp0_mut().write_reg(12, 0, 1 << 29);

                cpu.cp1_mut().write_control(31, rm);

                if fmt == 0x10 {
                    cpu.cp1_mut().write_word(2, 3.5f32.to_bits());
                } else {
                    cpu.cp1_mut().write_doubleword(2, 3.5f64.to_bits());
                }

                memory.write_word(0, encode_r(0x11, fmt, 0, 2, 4, funct));

                cpu.step(&mut memory);

                assert_eq!(cpu.cp1().read_word(4), expected,);

                assert_eq!(cpu.cp1().read_fcsr(), rm | (1 << 12) | (1 << 2),);

                assert!(!cpu.cp0().exception_level());
            }
        }
    }
}

#[test]
fn cp1_explicit_rounding_invalid() {
    for fmt in [0x10, 0x11] {
        for funct in 0x0C..=0x0F {
            let mut cpu = Cpu::new();
            let mut memory = Memory::new(4);

            cpu.cp0_mut().write_reg(12, 0, 1 << 29);

            if fmt == 0x10 {
                cpu.cp1_mut().write_word(2, f32::INFINITY.to_bits());
            } else {
                cpu.cp1_mut().write_doubleword(2, f64::INFINITY.to_bits());
            }

            memory.write_word(0, encode_r(0x11, fmt, 0, 2, 4, funct));

            cpu.step(&mut memory);

            assert_eq!(cpu.cp1().read_word(4), 0x7FFF_FFFF,);

            assert_eq!(cpu.cp1().read_fcsr(), (1 << 16) | (1 << 6),);

            assert!(!cpu.cp0().exception_level());
        }
    }
}

#[test]
fn cp1_explicit_rounding_enabled_exception() {
    for fmt in [0x10, 0x11] {
        for funct in 0x0C..=0x0F {
            for (invalid, enable, cause) in [(false, 1 << 7, 1 << 12), (true, 1 << 11, 1 << 16)] {
                let mut cpu = Cpu::new();
                let mut memory = Memory::new(4);

                cpu.cp0_mut().write_reg(12, 0, 1 << 29);

                cpu.cp1_mut().write_control(31, enable);

                let value = if invalid { f64::INFINITY } else { 2.5 };

                if fmt == 0x10 {
                    cpu.cp1_mut().write_word(2, (value as f32).to_bits());
                } else {
                    cpu.cp1_mut().write_doubleword(2, value.to_bits());
                }

                cpu.cp1_mut().write_word(4, 0xDEAD_BEEF);

                memory.write_word(0, encode_r(0x11, fmt, 0, 2, 4, funct));

                cpu.step(&mut memory);

                assert_eq!(cpu.cp0().exception_code(), 15,);

                assert_eq!(cpu.cp0().read_epc(), 0,);

                assert_eq!(cpu.cp1().read_word(4), 0xDEAD_BEEF,);

                assert_eq!(cpu.cp1().read_fcsr(), enable | cause,);
            }
        }
    }
}

#[test]
fn cp1_explicit_rounding_disabled() {
    for fmt in [0x10, 0x11] {
        for funct in 0x0C..=0x0F {
            let mut cpu = Cpu::new();
            let mut memory = Memory::new(4);

            cpu.cp1_mut().write_word(4, 0xDEAD_BEEF);

            memory.write_word(0, encode_r(0x11, fmt, 0, 2, 4, funct));

            cpu.step(&mut memory);

            assert_eq!(cpu.cp0().exception_code(), 11,);

            assert_eq!(cpu.cp0().coprocessor_error(), 1,);

            assert_eq!(cpu.cp1().read_word(4), 0xDEAD_BEEF,);

            assert_eq!(cpu.cp1().read_fcsr(), 0);
        }
    }
}

#[test]
fn cp1_explicit_rounding_invalid_double_source() {
    for funct in 0x0C..=0x0F {
        let mut cpu = Cpu::new();
        let mut memory = Memory::new(4);

        cpu.cp0_mut().write_reg(12, 0, 1 << 29);

        cpu.cp1_mut().write_word(4, 0xDEAD_BEEF);

        memory.write_word(0, encode_r(0x11, 0x11, 0, 3, 4, funct));

        cpu.step(&mut memory);

        assert_eq!(cpu.cp0().exception_code(), 10);

        assert_eq!(cpu.cp1().read_word(4), 0xDEAD_BEEF,);

        assert_eq!(cpu.cp1().read_fcsr(), 0);
    }
}

#[test]
fn cp1_explicit_rounding_exact() {
    for fmt in [0x10, 0x11] {
        for funct in 0x0C..=0x0F {
            let mut cpu = Cpu::new();
            let mut memory = Memory::new(4);

            cpu.cp0_mut().write_reg(12, 0, 1 << 29);

            if fmt == 0x10 {
                cpu.cp1_mut().write_word(2, (-42.0f32).to_bits());
            } else {
                cpu.cp1_mut().write_doubleword(2, (-42.0f64).to_bits());
            }

            memory.write_word(0, encode_r(0x11, fmt, 0, 2, 4, funct));

            cpu.step(&mut memory);

            assert_eq!(cpu.cp1().read_word(4), (-42i32) as u32,);

            assert_eq!(cpu.cp1().read_fcsr(), 0);
            assert!(!cpu.cp0().exception_level());
        }
    }
}

#[test]
fn fpu_single_directed_add() {
    use crate::fpu::*;
    use crate::isa::Operation;

    let modes = [
        RoundingMode::NearestEven,
        RoundingMode::TowardZero,
        RoundingMode::TowardPositive,
        RoundingMode::TowardNegative,
    ];

    let half_ulp = 2.0f32.powi(-24);

    for (mode, positive, negative) in [
        (modes[0], 0x3F80_0000, 0xBF80_0000),
        (modes[1], 0x3F80_0000, 0xBF80_0000),
        (modes[2], 0x3F80_0001, 0xBF80_0000),
        (modes[3], 0x3F80_0000, 0xBF80_0001),
    ] {
        for (lhs, rhs, expected) in [(1.0f32, half_ulp, positive), (-1.0f32, -half_ulp, negative)] {
            let result = rounded_single(&Operation::AddS, lhs.to_bits(), rhs.to_bits(), mode);

            assert_eq!(result.value, expected);
            assert_eq!(result.exceptions, FP_INEXACT);
        }
    }
}

#[test]
fn fpu_single_directed_sub() {
    use crate::fpu::*;
    use crate::isa::Operation;

    for (mode, expected) in [
        (RoundingMode::NearestEven, 0x3F80_0000),
        (RoundingMode::TowardZero, 0x3F7F_FFFF),
        (RoundingMode::TowardPositive, 0x3F80_0000),
        (RoundingMode::TowardNegative, 0x3F7F_FFFF),
    ] {
        let result = rounded_single(
            &Operation::SubS,
            1.0f32.to_bits(),
            2.0f32.powi(-100).to_bits(),
            mode,
        );

        assert_eq!(result.value, expected);
        assert_eq!(result.exceptions, FP_INEXACT);
    }
}

#[test]
fn fpu_single_directed_mul_div() {
    use crate::fpu::*;
    use crate::isa::Operation;

    let a = f32::from_bits(0x3F80_0001);

    for (mode, expected_mul, expected_div) in [
        (RoundingMode::NearestEven, 0x3F80_0002, 0x3EAA_AAAB),
        (RoundingMode::TowardZero, 0x3F80_0002, 0x3EAA_AAAA),
        (RoundingMode::TowardPositive, 0x3F80_0003, 0x3EAA_AAAB),
        (RoundingMode::TowardNegative, 0x3F80_0002, 0x3EAA_AAAA),
    ] {
        let mul = rounded_single(&Operation::MulS, a.to_bits(), a.to_bits(), mode);

        let div = rounded_single(&Operation::DivS, 1.0f32.to_bits(), 3.0f32.to_bits(), mode);

        assert_eq!(mul.value, expected_mul);
        assert_eq!(div.value, expected_div);

        assert_eq!(mul.exceptions, FP_INEXACT);
        assert_eq!(div.exceptions, FP_INEXACT);
    }
}

#[test]
fn fpu_single_directed_sqrt() {
    use crate::fpu::*;
    use crate::isa::Operation;

    for (mode, expected) in [
        (RoundingMode::NearestEven, 0x3FB5_04F3),
        (RoundingMode::TowardZero, 0x3FB5_04F3),
        (RoundingMode::TowardPositive, 0x3FB5_04F4),
        (RoundingMode::TowardNegative, 0x3FB5_04F3),
    ] {
        let result = rounded_single_unary(&Operation::SqrtS, 2.0f32.to_bits(), mode);

        assert_eq!(result.value, expected);
        assert_eq!(result.exceptions, FP_INEXACT);
    }
}

#[test]
fn fpu_single_directed_overflow() {
    use crate::fpu::*;
    use crate::isa::Operation;

    for (mode, positive, negative) in [
        (RoundingMode::NearestEven, 0x7F80_0000, 0xFF80_0000),
        (RoundingMode::TowardZero, 0x7F7F_FFFF, 0xFF7F_FFFF),
        (RoundingMode::TowardPositive, 0x7F80_0000, 0xFF7F_FFFF),
        (RoundingMode::TowardNegative, 0x7F7F_FFFF, 0xFF80_0000),
    ] {
        for (lhs, expected) in [(f32::MAX, positive), (-f32::MAX, negative)] {
            let result = rounded_single(&Operation::MulS, lhs.to_bits(), 2.0f32.to_bits(), mode);

            assert_eq!(result.value, expected);

            assert_eq!(result.exceptions, FP_OVERFLOW | FP_INEXACT,);
        }
    }

    let boundary = rounded_single(
        &Operation::AddS,
        f32::MAX.to_bits(),
        2.0f32.powi(103).to_bits(),
        RoundingMode::TowardZero,
    );

    assert_eq!(boundary.value, f32::MAX.to_bits());
    assert_eq!(boundary.exceptions, FP_INEXACT);

    let boundary = rounded_single(
        &Operation::AddS,
        f32::MAX.to_bits(),
        1.0f32.to_bits(),
        RoundingMode::TowardPositive,
    );

    assert_eq!(boundary.value, f32::INFINITY.to_bits());

    assert_eq!(boundary.exceptions, FP_OVERFLOW | FP_INEXACT,);
}

#[test]
fn fpu_single_directed_underflow() {
    use crate::fpu::*;
    use crate::isa::Operation;

    for (mode, positive, negative) in [
        (RoundingMode::NearestEven, 0, 0x8000_0000),
        (RoundingMode::TowardZero, 0, 0x8000_0000),
        (RoundingMode::TowardPositive, 1, 0x8000_0000),
        (RoundingMode::TowardNegative, 0, 0x8000_0001),
    ] {
        for (operand, expected) in [(1u32, positive), (0x8000_0001, negative)] {
            let result = rounded_single(&Operation::MulS, operand, 0.5f32.to_bits(), mode);

            assert_eq!(result.value, expected);

            assert_eq!(result.exceptions, FP_UNDERFLOW | FP_INEXACT,);
        }
    }
}

#[test]
fn fpu_single_directed_signed_zero() {
    use crate::fpu::*;
    use crate::isa::Operation;

    for (mode, expected) in [
        (RoundingMode::NearestEven, 0),
        (RoundingMode::TowardZero, 0),
        (RoundingMode::TowardPositive, 0),
        (RoundingMode::TowardNegative, 0x8000_0000),
    ] {
        for (operation, lhs, rhs) in [
            (Operation::AddS, 1.0f32, -1.0f32),
            (Operation::SubS, 1.0, 1.0),
            (Operation::AddS, 0.0, -0.0),
        ] {
            let result = rounded_single(&operation, lhs.to_bits(), rhs.to_bits(), mode);

            assert_eq!(result.value, expected);
            assert_eq!(result.exceptions, 0);
        }
    }
}

#[test]
fn cp1_single_directed_rounding_integration() {
    for (rm, expected) in [
        (0u32, 0x3EAA_AAAB),
        (1, 0x3EAA_AAAA),
        (2, 0x3EAA_AAAB),
        (3, 0x3EAA_AAAA),
    ] {
        let mut cpu = Cpu::new();
        let mut memory = Memory::new(8);

        cpu.cp0_mut().write_reg(12, 0, 1 << 29);

        cpu.write_reg(8, rm);

        cpu.cp1_mut().write_word(2, 1.0f32.to_bits());

        cpu.cp1_mut().write_word(4, 3.0f32.to_bits());

        // CTC1 $8, $31
        memory.write_word(0, encode_r(0x11, 6, 8, 31, 0, 0));

        // DIV.S $f6, $f2, $f4
        memory.write_word(4, encode_r(0x11, 0x10, 4, 2, 6, 3));

        cpu.step(&mut memory);
        cpu.step(&mut memory);

        assert_eq!(cpu.cp1().read_word(6), expected,);

        assert_eq!(cpu.cp1().read_fcsr(), rm | (1 << 12) | (1 << 2),);

        assert!(!cpu.cp0().exception_level());
    }
}

#[test]
fn fpu_double_directed_add() {
    use crate::fpu::*;
    use crate::isa::Operation;

    let half_ulp = 2.0f64.powi(-53);

    for (mode, positive, negative) in [
        (
            RoundingMode::NearestEven,
            0x3FF0_0000_0000_0000,
            0xBFF0_0000_0000_0000,
        ),
        (
            RoundingMode::TowardZero,
            0x3FF0_0000_0000_0000,
            0xBFF0_0000_0000_0000,
        ),
        (
            RoundingMode::TowardPositive,
            0x3FF0_0000_0000_0001,
            0xBFF0_0000_0000_0000,
        ),
        (
            RoundingMode::TowardNegative,
            0x3FF0_0000_0000_0000,
            0xBFF0_0000_0000_0001,
        ),
    ] {
        for (lhs, rhs, expected) in [(1.0f64, half_ulp, positive), (-1.0f64, -half_ulp, negative)] {
            let result = rounded_double(&Operation::AddD, lhs.to_bits(), rhs.to_bits(), mode);

            assert_eq!(result.value, expected);
            assert_eq!(result.exceptions, FP_INEXACT);
        }
    }
}

#[test]
fn fpu_double_directed_sub() {
    use crate::fpu::*;
    use crate::isa::Operation;

    for (mode, expected) in [
        (RoundingMode::NearestEven, 0x3FF0_0000_0000_0000),
        (RoundingMode::TowardZero, 0x3FEF_FFFF_FFFF_FFFF),
        (RoundingMode::TowardPositive, 0x3FF0_0000_0000_0000),
        (RoundingMode::TowardNegative, 0x3FEF_FFFF_FFFF_FFFF),
    ] {
        let result = rounded_double(
            &Operation::SubD,
            1.0f64.to_bits(),
            1, // Minimum subnormal: 2^-1074
            mode,
        );

        assert_eq!(result.value, expected);
        assert_eq!(result.exceptions, FP_INEXACT);
    }
}

#[test]
fn fpu_double_directed_mul_div() {
    use crate::fpu::*;
    use crate::isa::Operation;

    let a = 0x3FF0_0000_0000_0001u64;

    for (mode, expected_mul, expected_div) in [
        (
            RoundingMode::NearestEven,
            0x3FF0_0000_0000_0002,
            0x3FD5_5555_5555_5555,
        ),
        (
            RoundingMode::TowardZero,
            0x3FF0_0000_0000_0002,
            0x3FD5_5555_5555_5555,
        ),
        (
            RoundingMode::TowardPositive,
            0x3FF0_0000_0000_0003,
            0x3FD5_5555_5555_5556,
        ),
        (
            RoundingMode::TowardNegative,
            0x3FF0_0000_0000_0002,
            0x3FD5_5555_5555_5555,
        ),
    ] {
        let mul = rounded_double(&Operation::MulD, a, a, mode);

        let div = rounded_double(&Operation::DivD, 1.0f64.to_bits(), 3.0f64.to_bits(), mode);

        assert_eq!(mul.value, expected_mul);
        assert_eq!(div.value, expected_div);

        assert_eq!(mul.exceptions, FP_INEXACT);
        assert_eq!(div.exceptions, FP_INEXACT);
    }
}

#[test]
fn fpu_double_directed_sqrt() {
    use crate::fpu::*;
    use crate::isa::Operation;

    for (mode, expected) in [
        (RoundingMode::NearestEven, 0x3FF6_A09E_667F_3BCD),
        (RoundingMode::TowardZero, 0x3FF6_A09E_667F_3BCC),
        (RoundingMode::TowardPositive, 0x3FF6_A09E_667F_3BCD),
        (RoundingMode::TowardNegative, 0x3FF6_A09E_667F_3BCC),
    ] {
        let result = rounded_double_unary(&Operation::SqrtD, 2.0f64.to_bits(), mode);

        assert_eq!(result.value, expected);
        assert_eq!(result.exceptions, FP_INEXACT);
    }
}

#[test]
fn fpu_double_directed_overflow() {
    use crate::fpu::*;
    use crate::isa::Operation;

    for (mode, positive, negative) in [
        (
            RoundingMode::NearestEven,
            0x7FF0_0000_0000_0000,
            0xFFF0_0000_0000_0000,
        ),
        (
            RoundingMode::TowardZero,
            0x7FEF_FFFF_FFFF_FFFF,
            0xFFEF_FFFF_FFFF_FFFF,
        ),
        (
            RoundingMode::TowardPositive,
            0x7FF0_0000_0000_0000,
            0xFFEF_FFFF_FFFF_FFFF,
        ),
        (
            RoundingMode::TowardNegative,
            0x7FEF_FFFF_FFFF_FFFF,
            0xFFF0_0000_0000_0000,
        ),
    ] {
        for (lhs, expected) in [(f64::MAX, positive), (-f64::MAX, negative)] {
            let result = rounded_double(&Operation::MulD, lhs.to_bits(), 2.0f64.to_bits(), mode);

            assert_eq!(result.value, expected);

            assert_eq!(result.exceptions, FP_OVERFLOW | FP_INEXACT,);
        }
    }

    let boundary = rounded_double(
        &Operation::AddD,
        f64::MAX.to_bits(),
        2.0f64.powi(970).to_bits(),
        RoundingMode::TowardZero,
    );

    assert_eq!(boundary.value, f64::MAX.to_bits());
    assert_eq!(boundary.exceptions, FP_INEXACT);

    let boundary = rounded_double(
        &Operation::AddD,
        f64::MAX.to_bits(),
        1.0f64.to_bits(),
        RoundingMode::TowardPositive,
    );

    assert_eq!(boundary.value, f64::INFINITY.to_bits(),);

    assert_eq!(boundary.exceptions, FP_OVERFLOW | FP_INEXACT,);
}

#[test]
fn fpu_double_directed_underflow() {
    use crate::fpu::*;
    use crate::isa::Operation;

    for (mode, positive, negative) in [
        (RoundingMode::NearestEven, 0, 0x8000_0000_0000_0000),
        (RoundingMode::TowardZero, 0, 0x8000_0000_0000_0000),
        (RoundingMode::TowardPositive, 1, 0x8000_0000_0000_0000),
        (RoundingMode::TowardNegative, 0, 0x8000_0000_0000_0001),
    ] {
        for (operand, expected) in [(1u64, positive), (0x8000_0000_0000_0001, negative)] {
            let result = rounded_double(&Operation::MulD, operand, 0.5f64.to_bits(), mode);

            assert_eq!(result.value, expected);

            assert_eq!(result.exceptions, FP_UNDERFLOW | FP_INEXACT,);
        }
    }
}

#[test]
fn fpu_double_directed_signed_zero() {
    use crate::fpu::*;
    use crate::isa::Operation;

    for (mode, expected) in [
        (RoundingMode::NearestEven, 0),
        (RoundingMode::TowardZero, 0),
        (RoundingMode::TowardPositive, 0),
        (RoundingMode::TowardNegative, 0x8000_0000_0000_0000),
    ] {
        for (operation, lhs, rhs) in [
            (Operation::AddD, 1.0f64, -1.0f64),
            (Operation::SubD, 1.0, 1.0),
            (Operation::AddD, 0.0, -0.0),
        ] {
            let result = rounded_double(&operation, lhs.to_bits(), rhs.to_bits(), mode);

            assert_eq!(result.value, expected);
            assert_eq!(result.exceptions, 0);
        }
    }
}

#[test]
fn cp1_double_directed_rounding_integration() {
    for (rm, expected) in [
        (0u32, 0x3FD5_5555_5555_5555),
        (1, 0x3FD5_5555_5555_5555),
        (2, 0x3FD5_5555_5555_5556),
        (3, 0x3FD5_5555_5555_5555),
    ] {
        let mut cpu = Cpu::new();
        let mut memory = Memory::new(8);

        cpu.cp0_mut().write_reg(12, 0, 1 << 29);

        cpu.write_reg(8, rm);

        cpu.cp1_mut().write_doubleword(2, 1.0f64.to_bits());

        cpu.cp1_mut().write_doubleword(4, 3.0f64.to_bits());

        // CTC1 $8, $31
        memory.write_word(0, encode_r(0x11, 6, 8, 31, 0, 0));

        // DIV.D $f6, $f2, $f4
        memory.write_word(4, encode_r(0x11, 0x11, 4, 2, 6, 3));

        cpu.step(&mut memory);
        cpu.step(&mut memory);

        assert_eq!(cpu.cp1().read_doubleword(6), Some(expected),);

        assert_eq!(cpu.cp1().read_fcsr(), rm | (1 << 12) | (1 << 2),);

        assert!(!cpu.cp0().exception_level());
    }
}
