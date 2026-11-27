use crate::isa::Operation;

pub fn compute_single(operation: &Operation, lhs: u32, rhs: u32) -> u32 {
    let lhs = f32::from_bits(lhs);
    let rhs = f32::from_bits(rhs);

    let result = match operation {
        Operation::AddS => lhs + rhs,
        Operation::SubS => lhs - rhs,
        Operation::MulS => lhs * rhs,
        Operation::DivS => lhs / rhs,
        _ => unreachable!(),
    };

    result.to_bits()
}

pub fn compute_single_unary(operation: &Operation, operand: u32) -> u32 {
    match operation {
        Operation::AbsS => operand & 0x7FFF_FFFF,

        Operation::MovS => operand,

        Operation::NegS => operand ^ 0x8000_0000,

        Operation::SqrtS => f32::from_bits(operand).sqrt().to_bits(),

        _ => unreachable!(),
    }
}

pub fn compute_double(operation: &Operation, lhs: u64, rhs: u64) -> u64 {
    let lhs = f64::from_bits(lhs);
    let rhs = f64::from_bits(rhs);

    let result = match operation {
        Operation::AddD => lhs + rhs,
        Operation::SubD => lhs - rhs,
        Operation::MulD => lhs * rhs,
        Operation::DivD => lhs / rhs,
        _ => unreachable!(),
    };

    result.to_bits()
}

pub fn compute_double_unary(operation: &Operation, operand: u64) -> u64 {
    match operation {
        Operation::AbsD => operand & 0x7FFF_FFFF_FFFF_FFFF,

        Operation::MovD => operand,

        Operation::NegD => operand ^ 0x8000_0000_0000_0000,

        Operation::SqrtD => f64::from_bits(operand).sqrt().to_bits(),

        _ => unreachable!(),
    }
}

pub fn convert_single_to_double(operand: u32) -> u64 {
    let value = f32::from_bits(operand);

    (value as f64).to_bits()
}

pub fn convert_double_to_single(operand: u64) -> u32 {
    let value = f64::from_bits(operand);

    (value as f32).to_bits()
}

pub fn convert_word_to_single(operand: u32) -> u32 {
    let value = operand as i32;

    (value as f32).to_bits()
}

pub fn convert_word_to_double(operand: u32) -> u64 {
    let value = operand as i32;

    (value as f64).to_bits()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoundingMode {
    NearestEven,
    TowardZero,
    TowardPositive,
    TowardNegative,
}

impl RoundingMode {
    pub fn from_fcsr(fcsr: u32) -> Self {
        match fcsr & 0b11 {
            0 => Self::NearestEven,
            1 => Self::TowardZero,
            2 => Self::TowardPositive,
            3 => Self::TowardNegative,
            _ => unreachable!(),
        }
    }
}

pub const FP_INEXACT: u32 = 1 << 0;
pub const FP_INVALID: u32 = 1 << 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WordConversion {
    pub value: u32,
    pub exceptions: u32,
}

fn convert_float_to_word(value: f64, mode: RoundingMode) -> WordConversion {
    let rounded = match mode {
        RoundingMode::NearestEven => value.round_ties_even(),
        RoundingMode::TowardZero => value.trunc(),
        RoundingMode::TowardPositive => value.ceil(),
        RoundingMode::TowardNegative => value.floor(),
    };

    if !rounded.is_finite() || rounded < i32::MIN as f64 || rounded > i32::MAX as f64 {
        return WordConversion {
            value: 0x7FFF_FFFF,
            exceptions: FP_INVALID,
        };
    }

    WordConversion {
        value: rounded as i32 as u32,

        exceptions: if rounded != value { FP_INEXACT } else { 0 },
    }
}

pub fn convert_single_to_word(operand: u32, mode: RoundingMode) -> WordConversion {
    convert_float_to_word(f32::from_bits(operand) as f64, mode)
}

pub fn convert_double_to_word(operand: u64, mode: RoundingMode) -> WordConversion {
    convert_float_to_word(f64::from_bits(operand), mode)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompareResult {
    pub condition: bool,
    pub exceptions: u32,
}

fn compare_float(lhs: f64, rhs: f64, signaling_nan: bool, cond: u8) -> CompareResult {
    let unordered = lhs.is_nan() || rhs.is_nan();

    let relation = if unordered {
        0b001
    } else if lhs == rhs {
        0b010
    } else if lhs < rhs {
        0b100
    } else {
        0
    };

    let signaling_compare = cond & 0b1000 != 0;

    let invalid = signaling_nan || (unordered && signaling_compare);

    CompareResult {
        condition: relation & cond & 0b111 != 0,

        exceptions: if invalid { FP_INVALID } else { 0 },
    }
}

// MIPS legacy NaN encoding
fn is_signaling_nan_single(bits: u32) -> bool {
    let exponent = bits & 0x7F80_0000;
    let fraction = bits & 0x007F_FFFF;

    exponent == 0x7F80_0000 && fraction != 0 && fraction & 0x0040_0000 != 0
}

fn is_signaling_nan_double(bits: u64) -> bool {
    let exponent = bits & 0x7FF0_0000_0000_0000;
    let fraction = bits & 0x000F_FFFF_FFFF_FFFF;

    exponent == 0x7FF0_0000_0000_0000 && fraction != 0 && fraction & 0x0008_0000_0000_0000 != 0
}

pub fn compare_single(lhs: u32, rhs: u32, cond: u8) -> CompareResult {
    let signaling = is_signaling_nan_single(lhs) || is_signaling_nan_single(rhs);

    compare_float(
        f32::from_bits(lhs) as f64,
        f32::from_bits(rhs) as f64,
        signaling,
        cond,
    )
}

pub fn compare_double(lhs: u64, rhs: u64, cond: u8) -> CompareResult {
    let signaling = is_signaling_nan_double(lhs) || is_signaling_nan_double(rhs);

    compare_float(f64::from_bits(lhs), f64::from_bits(rhs), signaling, cond)
}
