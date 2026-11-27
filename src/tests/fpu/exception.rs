use super::*;

#[test]
fn fpu_arithmetic_invalid_single() {
    use crate::fpu::*;
    use crate::isa::Operation;

    for (operation, lhs, rhs) in [
        (Operation::AddS, f32::INFINITY, f32::NEG_INFINITY),
        (Operation::SubS, f32::INFINITY, f32::INFINITY),
        (Operation::MulS, 0.0, f32::INFINITY),
        (Operation::DivS, 0.0, 0.0),
        (Operation::DivS, f32::INFINITY, f32::INFINITY),
    ] {
        let result = checked_single(&operation, lhs.to_bits(), rhs.to_bits());

        assert_eq!(result.value, LEGACY_QNAN_SINGLE);
        assert_eq!(result.exceptions, FP_INVALID);
    }
}

#[test]
fn fpu_arithmetic_invalid_double() {
    use crate::fpu::*;
    use crate::isa::Operation;

    for (operation, lhs, rhs) in [
        (Operation::AddD, f64::INFINITY, f64::NEG_INFINITY),
        (Operation::SubD, f64::INFINITY, f64::INFINITY),
        (Operation::MulD, 0.0, f64::INFINITY),
        (Operation::DivD, 0.0, 0.0),
        (Operation::DivD, f64::INFINITY, f64::INFINITY),
    ] {
        let result = checked_double(&operation, lhs.to_bits(), rhs.to_bits());

        assert_eq!(result.value, LEGACY_QNAN_DOUBLE);
        assert_eq!(result.exceptions, FP_INVALID);
    }
}

#[test]
fn fpu_arithmetic_divide_by_zero() {
    use crate::fpu::*;
    use crate::isa::Operation;

    for (lhs, rhs, negative) in [
        (1.0f64, 0.0f64, false),
        (1.0, -0.0, true),
        (-1.0, 0.0, true),
        (-1.0, -0.0, false),
    ] {
        let single = checked_single(
            &Operation::DivS,
            (lhs as f32).to_bits(),
            (rhs as f32).to_bits(),
        );

        let double = checked_double(&Operation::DivD, lhs.to_bits(), rhs.to_bits());

        let expected_single = if negative {
            f32::NEG_INFINITY
        } else {
            f32::INFINITY
        };

        let expected_double = if negative {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };

        assert_eq!(single.value, expected_single.to_bits(),);

        assert_eq!(double.value, expected_double.to_bits(),);

        assert_eq!(single.exceptions, FP_DIVIDE_BY_ZERO,);

        assert_eq!(double.exceptions, FP_DIVIDE_BY_ZERO,);
    }
}

#[test]
fn fpu_arithmetic_legacy_nan() {
    use crate::fpu::*;
    use crate::isa::Operation;

    for (bits, exceptions) in [(0x7FBF_FFFFu32, 0), (0x7FC0_0000, FP_INVALID)] {
        let result = checked_single(&Operation::AddS, bits, 1.0f32.to_bits());

        assert_eq!(result.value, LEGACY_QNAN_SINGLE);
        assert_eq!(result.exceptions, exceptions);
    }

    for (bits, exceptions) in [
        (0x7FF7_FFFF_FFFF_FFFFu64, 0),
        (0x7FF8_0000_0000_0000, FP_INVALID),
    ] {
        let result = checked_double(&Operation::MulD, bits, 1.0f64.to_bits());

        assert_eq!(result.value, LEGACY_QNAN_DOUBLE);
        assert_eq!(result.exceptions, exceptions);
    }
}

#[test]
fn fpu_sqrt_special_values() {
    use crate::fpu::*;
    use crate::isa::Operation;

    for value in [-1.0f32, f32::NEG_INFINITY] {
        let result = checked_single_unary(&Operation::SqrtS, value.to_bits());

        assert_eq!(result.value, LEGACY_QNAN_SINGLE);
        assert_eq!(result.exceptions, FP_INVALID);
    }

    for value in [-1.0f64, f64::NEG_INFINITY] {
        let result = checked_double_unary(&Operation::SqrtD, value.to_bits());

        assert_eq!(result.value, LEGACY_QNAN_DOUBLE);
        assert_eq!(result.exceptions, FP_INVALID);
    }

    for value in [0.0f32, -0.0, f32::INFINITY] {
        let result = checked_single_unary(&Operation::SqrtS, value.to_bits());

        assert_eq!(result.value, value.to_bits());
        assert_eq!(result.exceptions, 0);
    }

    for value in [0.0f64, -0.0, f64::INFINITY] {
        let result = checked_double_unary(&Operation::SqrtD, value.to_bits());

        assert_eq!(result.value, value.to_bits());
        assert_eq!(result.exceptions, 0);
    }
}

#[test]
fn cp1_arithmetic_exception_flags() {
    for fmt in [0x10, 0x11] {
        for invalid in [false, true] {
            let mut cpu = Cpu::new();
            let mut memory = Memory::new(4);

            cpu.cp0_mut().write_reg(12, 0, 1 << 29);

            let lhs = if invalid { 0.0f64 } else { 1.0 };

            if fmt == 0x10 {
                cpu.cp1_mut().write_word(2, (lhs as f32).to_bits());
            } else {
                cpu.cp1_mut().write_doubleword(2, lhs.to_bits());
            }

            // DIV.S / DIV.D
            memory.write_word(0, encode_r(0x11, fmt, 4, 2, 6, 3));

            cpu.step(&mut memory);

            let expected_fcsr = if invalid {
                (1 << 16) | (1 << 6)
            } else {
                (1 << 15) | (1 << 5)
            };

            assert_eq!(cpu.cp1().read_fcsr(), expected_fcsr,);

            if fmt == 0x10 {
                let expected = if invalid {
                    0x7FBF_FFFF
                } else {
                    f32::INFINITY.to_bits()
                };

                assert_eq!(cpu.cp1().read_word(6), expected,);
            } else {
                let expected = if invalid {
                    0x7FF7_FFFF_FFFF_FFFF
                } else {
                    f64::INFINITY.to_bits()
                };

                assert_eq!(cpu.cp1().read_doubleword(6), Some(expected),);
            }

            assert!(!cpu.cp0().exception_level());
        }
    }
}

#[test]
fn cp1_arithmetic_exception_trap() {
    for fmt in [0x10, 0x11] {
        for (lhs, enable, cause) in [(0.0f64, 1 << 11, 1 << 16), (1.0f64, 1 << 10, 1 << 15)] {
            let mut cpu = Cpu::new();
            let mut memory = Memory::new(4);

            cpu.cp0_mut().write_reg(12, 0, 1 << 29);

            cpu.cp1_mut().write_control(31, enable);

            if fmt == 0x10 {
                cpu.cp1_mut().write_word(2, (lhs as f32).to_bits());
            } else {
                cpu.cp1_mut().write_doubleword(2, lhs.to_bits());
            }

            let sentinel = 0x1234_5678_DEAD_BEEFu64;

            cpu.cp1_mut().write_doubleword(6, sentinel);

            memory.write_word(0, encode_r(0x11, fmt, 4, 2, 6, 3));

            cpu.step(&mut memory);

            assert_eq!(cpu.cp0().exception_code(), 15);
            assert_eq!(cpu.cp0().read_epc(), 0);

            assert_eq!(cpu.cp1().read_doubleword(6), Some(sentinel),);

            assert_eq!(cpu.cp1().read_fcsr(), enable | cause,);
        }
    }
}

#[test]
fn cp1_arithmetic_cause_and_move_preservation() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(12);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    cpu.cp1_mut().write_word(2, 1.0f32.to_bits());

    let program = [
        // DIV.S $f6, $f2, $f4
        // Divide by zero
        encode_r(0x11, 0x10, 4, 2, 6, 3),
        // MOV.S $f8, $f6
        encode_r(0x11, 0x10, 0, 6, 8, 6),
        // ADD.S $f10, $f2, $f2
        encode_r(0x11, 0x10, 2, 2, 10, 0),
    ];

    for (index, word) in program.into_iter().enumerate() {
        memory.write_word((index * 4) as u32, word);
    }

    cpu.step(&mut memory);

    assert_eq!(cpu.cp1().read_fcsr(), (1 << 15) | (1 << 5),);

    cpu.step(&mut memory);

    assert_eq!(cpu.cp1().read_fcsr(), (1 << 15) | (1 << 5),);

    assert_eq!(cpu.cp1().read_word(8), f32::INFINITY.to_bits(),);

    cpu.step(&mut memory);

    assert_eq!(cpu.cp1().read_fcsr(), 1 << 5,);

    assert_eq!(cpu.cp1().read_word(10), 2.0f32.to_bits(),);

    assert!(!cpu.cp0().exception_level());
}

#[test]
fn fpu_single_overflow() {
    use crate::fpu::*;
    use crate::isa::Operation;

    for operation in [Operation::AddS, Operation::MulS, Operation::DivS] {
        let rhs = match operation {
            Operation::AddS => f32::MAX,
            Operation::MulS => 2.0f32,
            Operation::DivS => 0.5f32,
            _ => unreachable!(),
        };

        let result = checked_single(&operation, f32::MAX.to_bits(), rhs.to_bits());

        assert_eq!(result.value, f32::INFINITY.to_bits(),);

        assert_eq!(result.exceptions, FP_OVERFLOW | FP_INEXACT,);
    }
}

#[test]
fn fpu_single_underflow() {
    use crate::fpu::*;
    use crate::isa::Operation;

    let smallest = f32::from_bits(1);

    let result = checked_single(&Operation::MulS, smallest.to_bits(), 0.5f32.to_bits());

    assert_eq!(result.value, 0);

    assert_eq!(result.exceptions, FP_UNDERFLOW | FP_INEXACT,);

    // Exact subnormal result.
    let result = checked_single(
        &Operation::MulS,
        f32::MIN_POSITIVE.to_bits(),
        0.5f32.to_bits(),
    );

    assert_eq!(result.value, 0x0040_0000);
    assert_eq!(result.exceptions, 0);
}

#[test]
fn fpu_single_arithmetic_inexact() {
    use crate::fpu::*;
    use crate::isa::Operation;

    for (operation, lhs, rhs) in [
        (Operation::AddS, 1.0f32, 2.0f32.powi(-24)),
        (Operation::AddS, 1.0, 2.0f32.powi(-100)),
        (Operation::SubS, 1.0, 2.0f32.powi(-100)),
        (Operation::MulS, 1.1, 1.1),
        (Operation::DivS, 1.0, 3.0),
    ] {
        let result = checked_single(&operation, lhs.to_bits(), rhs.to_bits());

        assert_eq!(result.exceptions, FP_INEXACT);
    }
}

#[test]
fn fpu_single_arithmetic_exact() {
    use crate::fpu::*;
    use crate::isa::Operation;

    for (operation, lhs, rhs, expected) in [
        (Operation::AddS, 1.5f32, 2.5f32, 4.0f32),
        (Operation::SubS, 4.0, 1.5, 2.5),
        (Operation::MulS, 1.5, 2.0, 3.0),
        (Operation::DivS, 3.0, 2.0, 1.5),
    ] {
        let result = checked_single(&operation, lhs.to_bits(), rhs.to_bits());

        assert_eq!(result.value, expected.to_bits(),);

        assert_eq!(result.exceptions, 0);
    }
}

#[test]
fn fpu_single_sqrt_inexact() {
    use crate::fpu::*;
    use crate::isa::Operation;

    for (value, exceptions) in [
        (2.0f32, FP_INEXACT),
        (3.0, FP_INEXACT),
        (4.0, 0),
        (9.0, 0),
        (0.0, 0),
        (-0.0, 0),
    ] {
        let result = checked_single_unary(&Operation::SqrtS, value.to_bits());

        assert_eq!(result.value, value.sqrt().to_bits(),);

        assert_eq!(result.exceptions, exceptions,);
    }
}

#[test]
fn cp1_single_rounding_exception_integration() {
    for (lhs, rhs, funct, exceptions) in [
        (f32::MAX, 2.0f32, 2u32, 0b00101u32),
        (f32::from_bits(1), 0.5, 2, 0b00011),
        (1.0, 3.0, 3, 0b00001),
    ] {
        for enable in [0, exceptions] {
            let mut cpu = Cpu::new();
            let mut memory = Memory::new(4);

            cpu.cp0_mut().write_reg(12, 0, 1 << 29);

            cpu.cp1_mut().write_control(31, enable << 7);

            cpu.cp1_mut().write_word(2, lhs.to_bits());

            cpu.cp1_mut().write_word(4, rhs.to_bits());

            cpu.cp1_mut().write_word(6, 0xDEAD_BEEF);

            memory.write_word(0, encode_r(0x11, 0x10, 4, 2, 6, funct));

            cpu.step(&mut memory);

            if enable == 0 {
                assert_eq!(
                    cpu.cp1().read_fcsr(),
                    (exceptions << 12) | (exceptions << 2),
                );

                assert!(!cpu.cp0().exception_level());
            } else {
                assert_eq!(cpu.cp0().exception_code(), 15,);

                assert_eq!(cpu.cp1().read_word(6), 0xDEAD_BEEF,);

                assert_eq!(cpu.cp1().read_fcsr(), (exceptions << 12) | (enable << 7),);
            }
        }
    }
}

#[test]
fn fpu_double_overflow() {
    use crate::fpu::*;
    use crate::isa::Operation;

    for (operation, lhs, rhs) in [
        (Operation::AddD, f64::MAX, f64::MAX),
        (Operation::SubD, -f64::MAX, f64::MAX),
        (Operation::MulD, f64::MAX, 2.0),
        (Operation::DivD, f64::MAX, 0.5),
    ] {
        let result = checked_double(&operation, lhs.to_bits(), rhs.to_bits());

        assert!(f64::from_bits(result.value).is_infinite());

        assert_eq!(result.exceptions, FP_OVERFLOW | FP_INEXACT,);
    }
}

#[test]
fn fpu_double_underflow() {
    use crate::fpu::*;
    use crate::isa::Operation;

    let smallest = f64::from_bits(1);

    let result = checked_double(&Operation::MulD, smallest.to_bits(), 0.5f64.to_bits());

    assert_eq!(result.value, 0);

    assert_eq!(result.exceptions, FP_UNDERFLOW | FP_INEXACT,);

    let result = checked_double(
        &Operation::MulD,
        f64::MIN_POSITIVE.to_bits(),
        0.5f64.to_bits(),
    );

    assert_eq!(result.value, 0x0008_0000_0000_0000,);

    assert_eq!(result.exceptions, 0);

    // Nonzero, inexact subnormal.
    let result = checked_double(
        &Operation::MulD,
        f64::from_bits(3).to_bits(),
        0.5f64.to_bits(),
    );

    assert_eq!(result.value, 2);

    assert_eq!(result.exceptions, FP_UNDERFLOW | FP_INEXACT,);
}

#[test]
fn fpu_double_arithmetic_inexact() {
    use crate::fpu::*;
    use crate::isa::Operation;

    for (operation, lhs, rhs) in [
        (Operation::AddD, 1.0f64, 2.0f64.powi(-53)),
        (Operation::AddD, 1.0, 2.0f64.powi(-1000)),
        (Operation::SubD, 1.0, 2.0f64.powi(-1000)),
        (Operation::MulD, 1.1, 1.1),
        (Operation::DivD, 1.0, 3.0),
        (Operation::DivD, 2.0, 7.0),
    ] {
        let result = checked_double(&operation, lhs.to_bits(), rhs.to_bits());

        assert_eq!(result.exceptions, FP_INEXACT);
    }
}

#[test]
fn fpu_double_arithmetic_exact() {
    use crate::fpu::*;
    use crate::isa::Operation;

    for (operation, lhs, rhs, expected) in [
        (Operation::AddD, 1.5f64, 2.5f64, 4.0f64),
        (Operation::SubD, 4.0, 1.5, 2.5),
        (Operation::MulD, 1.5, 2.0, 3.0),
        (Operation::DivD, 3.0, 2.0, 1.5),
        (Operation::AddD, -1.0, 1.0, 0.0),
        (Operation::SubD, -1.0, -1.0, 0.0),
        (Operation::DivD, f64::MAX, f64::MAX, 1.0),
    ] {
        let result = checked_double(&operation, lhs.to_bits(), rhs.to_bits());

        assert_eq!(f64::from_bits(result.value), expected,);

        assert_eq!(result.exceptions, 0);
    }
}

#[test]
fn fpu_double_sqrt_inexact() {
    use crate::fpu::*;
    use crate::isa::Operation;

    for (value, exceptions) in [
        (2.0f64, FP_INEXACT),
        (3.0, FP_INEXACT),
        (4.0, 0),
        (9.0, 0),
        (0.0, 0),
        (-0.0, 0),
        (f64::from_bits(1), 0),
    ] {
        let result = checked_double_unary(&Operation::SqrtD, value.to_bits());

        assert_eq!(result.value, value.sqrt().to_bits(),);

        assert_eq!(result.exceptions, exceptions,);
    }
}

#[test]
fn fpu_double_exponent_boundaries() {
    use crate::fpu::*;
    use crate::isa::Operation;

    let tiny = f64::from_bits(1);

    let result = checked_double(&Operation::AddD, 1.0f64.to_bits(), tiny.to_bits());

    assert_eq!(result.value, 1.0f64.to_bits());
    assert_eq!(result.exceptions, FP_INEXACT);

    let result = checked_double(&Operation::SubD, f64::MAX.to_bits(), f64::MAX.to_bits());

    assert_eq!(result.value, 0);
    assert_eq!(result.exceptions, 0);

    let result = checked_double(&Operation::AddD, tiny.to_bits(), tiny.to_bits());

    assert_eq!(result.value, 2);
    assert_eq!(result.exceptions, 0);

    let result = checked_double(
        &Operation::SubD,
        f64::MIN_POSITIVE.to_bits(),
        f64::from_bits(0x000F_FFFF_FFFF_FFFF).to_bits(),
    );

    assert_eq!(result.value, 1);
    assert_eq!(result.exceptions, 0);
}

#[test]
fn cp1_double_rounding_exception_integration() {
    for (lhs, rhs, funct, exceptions) in [
        (f64::MAX, 2.0f64, 2u32, 0b00101u32),
        (f64::from_bits(1), 0.5, 2, 0b00011),
        (1.0, 3.0, 3, 0b00001),
    ] {
        for enable in [0, exceptions] {
            let mut cpu = Cpu::new();
            let mut memory = Memory::new(4);

            cpu.cp0_mut().write_reg(12, 0, 1 << 29);

            cpu.cp1_mut().write_control(31, enable << 7);

            cpu.cp1_mut().write_doubleword(2, lhs.to_bits());

            cpu.cp1_mut().write_doubleword(4, rhs.to_bits());

            let sentinel = 0x1234_5678_DEAD_BEEFu64;

            cpu.cp1_mut().write_doubleword(6, sentinel);

            memory.write_word(0, encode_r(0x11, 0x11, 4, 2, 6, funct));

            cpu.step(&mut memory);

            if enable == 0 {
                assert_eq!(
                    cpu.cp1().read_fcsr(),
                    (exceptions << 12) | (exceptions << 2),
                );

                assert!(!cpu.cp0().exception_level());
            } else {
                assert_eq!(cpu.cp0().exception_code(), 15,);

                assert_eq!(cpu.cp1().read_doubleword(6), Some(sentinel),);

                assert_eq!(cpu.cp1().read_fcsr(), (exceptions << 12) | (enable << 7),);
            }
        }
    }
}

#[test]
fn cp1_flush_subnormal_control() {
    let mut cpu = Cpu::new();

    assert!(!cpu.cp1().flush_subnormals());

    cpu.cp1_mut().write_control(31, 1 << 24);

    assert!(cpu.cp1().flush_subnormals());

    cpu.cp1_mut().write_control(28, 0);

    assert!(!cpu.cp1().flush_subnormals());

    cpu.cp1_mut().write_control(28, 1 << 2);

    assert!(cpu.cp1().flush_subnormals());
}

#[test]
fn fpu_single_flush_subnormal() {
    use crate::fpu::*;

    for (mode, positive, negative) in [
        (RoundingMode::NearestEven, 0, 0x8000_0000),
        (RoundingMode::TowardZero, 0, 0x8000_0000),
        (RoundingMode::TowardPositive, 0x0080_0000, 0x8000_0000),
        (RoundingMode::TowardNegative, 0, 0x8080_0000),
    ] {
        for (operand, expected) in [(0x0040_0000u32, positive), (0x8040_0000, negative)] {
            let original = FpResult {
                value: operand,
                exceptions: 0,
            };

            assert_eq!(flush_single_result(original, mode, false), original,);

            let flushed = flush_single_result(original, mode, true);

            assert_eq!(flushed.value, expected);

            assert_eq!(flushed.exceptions, FP_UNDERFLOW | FP_INEXACT,);
        }
    }
}

#[test]
fn fpu_double_flush_subnormal() {
    use crate::fpu::*;

    for (mode, positive, negative) in [
        (RoundingMode::NearestEven, 0, 0x8000_0000_0000_0000),
        (RoundingMode::TowardZero, 0, 0x8000_0000_0000_0000),
        (
            RoundingMode::TowardPositive,
            0x0010_0000_0000_0000,
            0x8000_0000_0000_0000,
        ),
        (RoundingMode::TowardNegative, 0, 0x8010_0000_0000_0000),
    ] {
        for (operand, expected) in [
            (0x0008_0000_0000_0000u64, positive),
            (0x8008_0000_0000_0000, negative),
        ] {
            let original = FpResult {
                value: operand,
                exceptions: 0,
            };

            assert_eq!(flush_double_result(original, mode, false), original,);

            let flushed = flush_double_result(original, mode, true);

            assert_eq!(flushed.value, expected);

            assert_eq!(flushed.exceptions, FP_UNDERFLOW | FP_INEXACT,);
        }
    }
}

#[test]
fn fpu_flush_preserves_non_tiny_values() {
    use crate::fpu::*;

    for bits in [
        0u32,
        0x8000_0000,
        0x0080_0000,
        0x3F80_0000,
        0x7F80_0000,
        LEGACY_QNAN_SINGLE,
    ] {
        let original = FpResult {
            value: bits,
            exceptions: 0,
        };

        for mode in [
            RoundingMode::NearestEven,
            RoundingMode::TowardZero,
            RoundingMode::TowardPositive,
            RoundingMode::TowardNegative,
        ] {
            assert_eq!(flush_single_result(original, mode, true), original,);
        }
    }

    for bits in [
        0u64,
        0x8000_0000_0000_0000,
        0x0010_0000_0000_0000,
        0x3FF0_0000_0000_0000,
        0x7FF0_0000_0000_0000,
        LEGACY_QNAN_DOUBLE,
    ] {
        let original = FpResult {
            value: bits,
            exceptions: 0,
        };

        for mode in [
            RoundingMode::NearestEven,
            RoundingMode::TowardZero,
            RoundingMode::TowardPositive,
            RoundingMode::TowardNegative,
        ] {
            assert_eq!(flush_double_result(original, mode, true), original,);
        }
    }
}

#[test]
fn cp1_flush_arithmetic_integration() {
    use crate::fpu::*;

    for fmt in [0x10u32, 0x11] {
        for fs in [false, true] {
            for enable in [0, FP_UNDERFLOW] {
                let mut cpu = Cpu::new();
                let mut memory = Memory::new(4);

                cpu.cp0_mut().write_reg(12, 0, 1 << 29);

                let control = (enable << 7) | if fs { 1 << 24 } else { 0 };

                cpu.cp1_mut().write_control(31, control);

                if fmt == 0x10 {
                    cpu.cp1_mut().write_word(2, f32::MIN_POSITIVE.to_bits());

                    cpu.cp1_mut().write_word(4, 0.5f32.to_bits());
                } else {
                    cpu.cp1_mut()
                        .write_doubleword(2, f64::MIN_POSITIVE.to_bits());

                    cpu.cp1_mut().write_doubleword(4, 0.5f64.to_bits());
                }

                let sentinel = 0x1234_5678_DEAD_BEEFu64;

                cpu.cp1_mut().write_doubleword(6, sentinel);

                // MUL.S / MUL.D
                memory.write_word(0, encode_r(0x11, fmt, 4, 2, 6, 2));

                cpu.step(&mut memory);

                if fs && enable != 0 {
                    assert_eq!(cpu.cp0().exception_code(), 15,);

                    assert_eq!(cpu.cp1().read_doubleword(6), Some(sentinel),);

                    continue;
                }

                if fmt == 0x10 {
                    assert_eq!(cpu.cp1().read_word(6), if fs { 0 } else { 0x0040_0000 },);
                } else {
                    assert_eq!(
                        cpu.cp1().read_doubleword(6),
                        Some(if fs { 0 } else { 0x0008_0000_0000_0000 }),
                    );
                }

                let exceptions = if fs { FP_UNDERFLOW | FP_INEXACT } else { 0 };

                assert_eq!(
                    cpu.cp1().read_fcsr(),
                    control | (exceptions << 12) | (exceptions << 2),
                );

                assert!(!cpu.cp0().exception_level());
            }
        }
    }
}

#[test]
fn cp1_flush_conversion_and_move() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(8);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    // FS = 1, nearest-even.
    cpu.cp1_mut().write_control(31, 1 << 24);

    cpu.cp1_mut()
        .write_doubleword(2, 2.0f64.powi(-149).to_bits());

    cpu.cp1_mut().write_word(4, 1);

    // CVT.S.D $f6, $f2
    memory.write_word(0, encode_r(0x11, 0x11, 0, 2, 6, 0x20));

    // MOV.S $f8, $f4
    memory.write_word(4, encode_r(0x11, 0x10, 0, 4, 8, 6));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp1().read_word(6), 0);

    let fcsr = (1 << 24) | (0b11 << 12) | (0b11 << 2);

    assert_eq!(cpu.cp1().read_fcsr(), fcsr);

    cpu.step(&mut memory);

    // MOV must preserve subnormal bits.
    assert_eq!(cpu.cp1().read_word(8), 1);

    assert_eq!(cpu.cp1().read_fcsr(), fcsr);

    assert!(!cpu.cp0().exception_level());
}

#[test]
fn cp1_exception_mask_exhaustive() {
    use crate::fpu::FP_UNIMPLEMENTED;

    for cause in 0..64u32 {
        for enable in 0..32u32 {
            let mut cp1 = crate::cp1::Cp1::new();

            let initial = (enable << 7) | 3 | (1 << 24);

            cp1.write_control(31, initial);

            let expected_trap = (cause & FP_UNIMPLEMENTED != 0) || (cause & enable != 0);

            let trap = cp1.apply_fp_exceptions(cause);

            assert_eq!(trap, expected_trap);
            assert_eq!(cp1.exception_requested(), expected_trap,);

            let flags = if expected_trap {
                0
            } else {
                (cause & 0x1F) << 2
            };

            assert_eq!(cp1.read_fcsr(), initial | (cause << 12) | flags,);
        }
    }
}

#[test]
fn cp1_exception_sticky_flags() {
    use crate::fpu::*;

    let mut cp1 = crate::cp1::Cp1::new();

    let mut accumulated = 0;

    for exception in [
        FP_INEXACT,
        FP_UNDERFLOW,
        FP_OVERFLOW,
        FP_DIVIDE_BY_ZERO,
        FP_INVALID,
    ] {
        assert!(!cp1.apply_fp_exceptions(exception));

        accumulated |= exception;

        assert_eq!(cp1.read_fcsr(), (exception << 12) | (accumulated << 2),);
    }

    assert!(!cp1.apply_fp_exceptions(0));

    // Cause cleared, all sticky Flags preserved.
    assert_eq!(cp1.read_fcsr(), 0x7C);
}

#[test]
fn cp1_exception_trap_preserves_sticky_flags() {
    use crate::fpu::*;

    let mut cp1 = crate::cp1::Cp1::new();

    let initial = (FP_INVALID << 7) | (FP_DIVIDE_BY_ZERO << 2) | 3 | (1 << 24) | (1 << 23);

    cp1.write_control(31, initial);

    assert!(cp1.apply_fp_exceptions(FP_INVALID));

    assert_eq!(cp1.read_fcsr(), initial | (FP_INVALID << 12),);

    assert!(cp1.read_fcc(0));
    assert_eq!(cp1.rounding_mode(), 3);
    assert!(cp1.flush_subnormals());
}

#[test]
fn cp1_unimplemented_exception_always_traps() {
    use crate::fpu::*;

    for enable in 0..32u32 {
        let mut cp1 = crate::cp1::Cp1::new();

        let initial = (enable << 7) | (FP_INEXACT << 2);

        cp1.write_control(31, initial);

        assert!(cp1.apply_fp_exceptions(FP_UNIMPLEMENTED));

        assert_eq!(cp1.read_fcsr(), initial | (FP_UNIMPLEMENTED << 12),);

        assert!(cp1.exception_requested());
    }
}

#[test]
fn cp1_sticky_flags_do_not_request_exception() {
    let mut cp1 = crate::cp1::Cp1::new();

    // All standard Flags and Enables set,
    // but no current Cause.
    cp1.write_control(31, (0x1F << 2) | (0x1F << 7));

    assert!(!cp1.exception_requested());

    assert!(!cp1.apply_fp_exceptions(0));

    assert_eq!(cp1.read_fcsr(), (0x1F << 2) | (0x1F << 7),);
}

#[test]
fn cp1_arithmetic_trap_in_delay_slot() {
    for (likely, condition) in [(false, true), (false, false), (true, true)] {
        let mut cpu = Cpu::new();
        let mut memory = Memory::new(16);

        cpu.cp0_mut().write_reg(12, 0, 1 << 29);

        // Enable Divide by Zero.
        cpu.cp1_mut().write_control(31, 1 << 10);

        cpu.cp1_mut().write_fcc(0, condition);

        cpu.cp1_mut().write_word(2, 1.0f32.to_bits());

        cpu.cp1_mut().write_word(4, 0);
        cpu.cp1_mut().write_word(6, 0xDEAD_BEEF);

        // BC1T / BC1TL
        memory.write_word(0, encode_fp_branch(0, likely, true, 2));

        // DIV.S in delay slot.
        memory.write_word(4, encode_r(0x11, 0x10, 4, 2, 6, 3));

        cpu.step(&mut memory);
        cpu.step(&mut memory);

        assert_eq!(cpu.cp0().exception_code(), 15);
        assert_eq!(cpu.cp0().read_epc(), 0);

        // CP0.Cause.BD
        let cause = cpu.cp0().read_reg(13, 0).unwrap();

        assert_ne!(cause & (1 << 31), 0);

        assert_eq!(cpu.cp1().read_word(6), 0xDEAD_BEEF,);

        assert_ne!(cpu.cp1().read_fcsr() & (1 << 15), 0,);

        assert!(cpu.cp0().exception_level());
    }
}

#[test]
fn cp1_branch_likely_annuls_arithmetic_trap() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(16);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    cpu.cp1_mut().write_control(31, 1 << 10);

    cpu.cp1_mut().write_fcc(0, false);

    cpu.cp1_mut().write_word(2, 1.0f32.to_bits());

    cpu.cp1_mut().write_word(4, 0);
    cpu.cp1_mut().write_word(6, 0xDEAD_BEEF);

    let initial_fcsr = cpu.cp1().read_fcsr();

    // BC1TL: not taken.
    memory.write_word(0, encode_fp_branch(0, true, true, 2));

    // Must be annulled.
    memory.write_word(4, encode_r(0x11, 0x10, 4, 2, 6, 3));

    // ADDIU $8, $0, 42
    memory.write_word(8, i(9, 0, 8, 42));

    cpu.step(&mut memory);
    cpu.step(&mut memory);

    assert_eq!(cpu.read_reg(8), 42);

    assert_eq!(cpu.cp1().read_word(6), 0xDEAD_BEEF,);

    assert_eq!(cpu.cp1().read_fcsr(), initial_fcsr,);

    assert!(!cpu.cp0().exception_level());
}

#[test]
fn cp1_trap_preserves_unrelated_state() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    let initial_fcsr = (1 << 11) // Invalid Enable
        | (1 << 5) // Divide-by-zero Flag
        | (1 << 24) // FS
        | 2; // Round toward positive

    cpu.cp1_mut().write_control(31, initial_fcsr);

    // Set all FCC bits.
    for cc in 0..8 {
        cpu.cp1_mut().write_fcc(cc, true);
    }

    cpu.write_reg(8, 0x1234_5678);

    cpu.cp1_mut().write_word(2, 0);
    cpu.cp1_mut().write_word(4, 0);
    cpu.cp1_mut().write_word(6, 0xDEAD_BEEF);
    cpu.cp1_mut().write_word(7, 0x1234_ABCD);

    let before = cpu.cp1().read_fcsr();

    // DIV.S: 0 / 0 -> Invalid Operation.
    memory.write_word(0, encode_r(0x11, 0x10, 4, 2, 6, 3));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp0().exception_code(), 15);

    assert_eq!(cpu.read_reg(8), 0x1234_5678);

    assert_eq!(cpu.cp1().read_word(2), 0);
    assert_eq!(cpu.cp1().read_word(4), 0);

    assert_eq!(cpu.cp1().read_word(6), 0xDEAD_BEEF,);

    assert_eq!(cpu.cp1().read_word(7), 0x1234_ABCD,);

    for cc in 0..8 {
        assert!(cpu.cp1().read_fcc(cc));
    }

    assert_eq!(cpu.cp1().read_fcsr(), before | (1 << 16),);
}
