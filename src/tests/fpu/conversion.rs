use super::*;

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
fn fpu_cvt_sd_directed_rounding() {
    use crate::fpu::*;

    let value = 1.0f64 + 2.0f64.powi(-24);

    for (mode, positive, negative) in [
        (RoundingMode::NearestEven, 0x3F80_0000, 0xBF80_0000),
        (RoundingMode::TowardZero, 0x3F80_0000, 0xBF80_0000),
        (RoundingMode::TowardPositive, 0x3F80_0001, 0xBF80_0000),
        (RoundingMode::TowardNegative, 0x3F80_0000, 0xBF80_0001),
    ] {
        for (operand, expected) in [(value, positive), (-value, negative)] {
            let result = rounded_double_to_single(operand.to_bits(), mode);

            assert_eq!(result.value, expected);
            assert_eq!(result.exceptions, FP_INEXACT);
        }
    }
}

#[test]
fn fpu_cvt_sw_directed_rounding() {
    use crate::fpu::*;

    for (mode, positive, negative) in [
        (RoundingMode::NearestEven, 0x4B80_0000, 0xCB80_0000),
        (RoundingMode::TowardZero, 0x4B80_0000, 0xCB80_0000),
        (RoundingMode::TowardPositive, 0x4B80_0001, 0xCB80_0000),
        (RoundingMode::TowardNegative, 0x4B80_0000, 0xCB80_0001),
    ] {
        for (operand, expected) in [(16_777_217i32, positive), (-16_777_217i32, negative)] {
            let result = rounded_word_to_single(operand as u32, mode);

            assert_eq!(result.value, expected);
            assert_eq!(result.exceptions, FP_INEXACT);
        }
    }
}

#[test]
fn fpu_exact_widening_conversions() {
    use crate::fpu::*;

    for value in [
        0.0f32,
        -0.0,
        1.5,
        -123.75,
        f32::MAX,
        f32::MIN_POSITIVE,
        f32::from_bits(1),
        f32::INFINITY,
        f32::NEG_INFINITY,
    ] {
        let result = checked_single_to_double(value.to_bits());

        assert_eq!(result.value, (value as f64).to_bits(),);

        assert_eq!(result.exceptions, 0);
    }

    for value in [0i32, 1, -1, i32::MAX, i32::MIN, 16_777_217] {
        let result = checked_word_to_double(value as u32);

        assert_eq!(result.value, (value as f64).to_bits(),);

        assert_eq!(result.exceptions, 0);
    }
}

#[test]
fn fpu_cvt_sd_range_exceptions() {
    use crate::fpu::*;

    for (mode, overflow_value, underflow_value) in [
        (RoundingMode::NearestEven, 0x7F80_0000, 0),
        (RoundingMode::TowardZero, 0x7F7F_FFFF, 0),
        (RoundingMode::TowardPositive, 0x7F80_0000, 1),
        (RoundingMode::TowardNegative, 0x7F7F_FFFF, 0),
    ] {
        let overflow = rounded_double_to_single(f64::MAX.to_bits(), mode);

        assert_eq!(overflow.value, overflow_value);

        assert_eq!(overflow.exceptions, FP_OVERFLOW | FP_INEXACT,);

        let underflow = rounded_double_to_single(2.0f64.powi(-150).to_bits(), mode);

        assert_eq!(underflow.value, underflow_value);

        assert_eq!(underflow.exceptions, FP_UNDERFLOW | FP_INEXACT,);
    }

    let exact = rounded_double_to_single(2.0f64.powi(-149).to_bits(), RoundingMode::NearestEven);

    assert_eq!(exact.value, 1);
    assert_eq!(exact.exceptions, 0);

    let boundary = rounded_double_to_single(
        ((f32::MAX as f64) + 2.0f64.powi(103)).to_bits(),
        RoundingMode::TowardZero,
    );

    assert_eq!(boundary.value, f32::MAX.to_bits());
    assert_eq!(boundary.exceptions, FP_INEXACT);
}

#[test]
fn fpu_conversion_special_values() {
    use crate::fpu::*;

    for (operand, exceptions) in [(0x7FBF_FFFFu32, 0), (0x7FC0_0000, FP_INVALID)] {
        let result = checked_single_to_double(operand);

        assert_eq!(result.value, LEGACY_QNAN_DOUBLE,);

        assert_eq!(result.exceptions, exceptions);
    }

    for (operand, exceptions) in [
        (0x7FF7_FFFF_FFFF_FFFFu64, 0),
        (0x7FF8_0000_0000_0000, FP_INVALID),
    ] {
        let result = rounded_double_to_single(operand, RoundingMode::NearestEven);

        assert_eq!(result.value, LEGACY_QNAN_SINGLE,);

        assert_eq!(result.exceptions, exceptions);
    }

    for value in [0.0f64, -0.0, f64::INFINITY, f64::NEG_INFINITY] {
        let result = rounded_double_to_single(value.to_bits(), RoundingMode::TowardZero);

        assert_eq!(result.value, (value as f32).to_bits(),);

        assert_eq!(result.exceptions, 0);
    }
}

#[test]
fn fpu_cvt_sw_integer_boundaries() {
    use crate::fpu::*;

    for (mode, expected) in [
        (RoundingMode::NearestEven, 0x4F00_0000),
        (RoundingMode::TowardZero, 0x4EFF_FFFF),
        (RoundingMode::TowardPositive, 0x4F00_0000),
        (RoundingMode::TowardNegative, 0x4EFF_FFFF),
    ] {
        let result = rounded_word_to_single(i32::MAX as u32, mode);

        assert_eq!(result.value, expected);
        assert_eq!(result.exceptions, FP_INEXACT);

        let result = rounded_word_to_single(i32::MIN as u32, mode);

        assert_eq!(result.value, (i32::MIN as f32).to_bits(),);

        assert_eq!(result.exceptions, 0);
    }
}

#[test]
fn cp1_conversion_rounding_integration() {
    for (rm, expected_sd, expected_sw) in [
        (0u32, 0x3F80_0000, 0x4B80_0000),
        (1, 0x3F80_0000, 0x4B80_0000),
        (2, 0x3F80_0001, 0x4B80_0001),
        (3, 0x3F80_0000, 0x4B80_0000),
    ] {
        let mut cpu = Cpu::new();
        let mut memory = Memory::new(16);

        cpu.cp0_mut().write_reg(12, 0, 1 << 29);

        cpu.cp1_mut().write_control(31, rm);

        cpu.cp1_mut()
            .write_doubleword(2, (1.0f64 + 2.0f64.powi(-24)).to_bits());

        cpu.cp1_mut().write_word(4, 16_777_217);

        let program = [
            // CVT.S.D $f6, $f2
            encode_r(0x11, 0x11, 0, 2, 6, 0x20),
            // CVT.S.W $f8, $f4
            encode_r(0x11, 0x14, 0, 4, 8, 0x20),
            // CVT.D.S $f10, $f6
            encode_r(0x11, 0x10, 0, 6, 10, 0x21),
            // CVT.D.W $f12, $f4
            encode_r(0x11, 0x14, 0, 4, 12, 0x21),
        ];

        for (index, word) in program.into_iter().enumerate() {
            memory.write_word((index * 4) as u32, word);
        }

        cpu.step(&mut memory);

        assert_eq!(cpu.cp1().read_word(6), expected_sd,);

        assert_eq!(cpu.cp1().read_fcsr(), rm | (1 << 12) | (1 << 2),);

        cpu.step(&mut memory);

        assert_eq!(cpu.cp1().read_word(8), expected_sw,);

        cpu.step(&mut memory);

        assert_eq!(
            cpu.cp1().read_doubleword(10),
            Some((f32::from_bits(expected_sd) as f64).to_bits(),),
        );

        assert_eq!(cpu.cp1().read_fcsr(), rm | (1 << 2),);

        cpu.step(&mut memory);

        assert_eq!(
            cpu.cp1().read_doubleword(12),
            Some(16_777_217.0f64.to_bits()),
        );

        assert_eq!(cpu.cp1().read_fcsr(), rm | (1 << 2),);

        assert!(!cpu.cp0().exception_level());
    }
}

#[test]
fn cp1_conversion_exception_traps() {
    use crate::fpu::*;

    for (fmt, funct, source, exceptions, enable) in [
        (
            0x11u32,
            0x20u32,
            f64::MAX.to_bits(),
            FP_OVERFLOW | FP_INEXACT,
            FP_OVERFLOW,
        ),
        (
            0x11,
            0x20,
            2.0f64.powi(-150).to_bits(),
            FP_UNDERFLOW | FP_INEXACT,
            FP_UNDERFLOW,
        ),
        (
            0x11,
            0x20,
            (1.0f64 + 2.0f64.powi(-24)).to_bits(),
            FP_INEXACT,
            FP_INEXACT,
        ),
        (0x10, 0x21, 0x7FC0_0000u64, FP_INVALID, FP_INVALID),
        (0x14, 0x20, 16_777_217u64, FP_INEXACT, FP_INEXACT),
    ] {
        let mut cpu = Cpu::new();
        let mut memory = Memory::new(4);

        cpu.cp0_mut().write_reg(12, 0, 1 << 29);

        cpu.cp1_mut().write_control(31, enable << 7);

        cpu.cp1_mut().write_doubleword(2, source);

        let sentinel = 0x1234_5678_DEAD_BEEFu64;

        cpu.cp1_mut().write_doubleword(6, sentinel);

        memory.write_word(0, encode_r(0x11, fmt, 0, 2, 6, funct));

        cpu.step(&mut memory);

        assert_eq!(cpu.cp0().exception_code(), 15);

        assert_eq!(cpu.cp0().read_epc(), 0);

        assert_eq!(cpu.cp1().read_doubleword(6), Some(sentinel),);

        assert_eq!(cpu.cp1().read_fcsr(), (enable << 7) | (exceptions << 12),);
    }
}
