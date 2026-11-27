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

pub const FP_DIVIDE_BY_ZERO: u32 = 1 << 3;

pub const LEGACY_QNAN_SINGLE: u32 = 0x7FBF_FFFF;

pub const LEGACY_QNAN_DOUBLE: u64 = 0x7FF7_FFFF_FFFF_FFFF;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FpResult<T> {
    pub value: T,
    pub exceptions: u32,
}

fn binary_special_exceptions(
    operation: &Operation,
    lhs: f64,
    rhs: f64,
    signaling_nan: bool,
) -> u32 {
    if signaling_nan {
        return FP_INVALID;
    }

    if lhs.is_nan() || rhs.is_nan() {
        return 0;
    }

    let invalid = match operation {
        Operation::AddS | Operation::AddD => {
            lhs.is_infinite()
                && rhs.is_infinite()
                && lhs.is_sign_negative() != rhs.is_sign_negative()
        }

        Operation::SubS | Operation::SubD => {
            lhs.is_infinite()
                && rhs.is_infinite()
                && lhs.is_sign_negative() == rhs.is_sign_negative()
        }

        Operation::MulS | Operation::MulD => {
            (lhs == 0.0 && rhs.is_infinite()) || (lhs.is_infinite() && rhs == 0.0)
        }

        Operation::DivS | Operation::DivD => {
            (lhs == 0.0 && rhs == 0.0) || (lhs.is_infinite() && rhs.is_infinite())
        }

        _ => unreachable!(),
    };

    if invalid {
        return FP_INVALID;
    }

    let division = matches!(operation, Operation::DivS | Operation::DivD);

    if division && lhs.is_finite() && lhs != 0.0 && rhs == 0.0 {
        return FP_DIVIDE_BY_ZERO;
    }

    0
}

pub fn checked_single(operation: &Operation, lhs: u32, rhs: u32) -> FpResult<u32> {
    let a = f32::from_bits(lhs);
    let b = f32::from_bits(rhs);

    let signaling = is_signaling_nan_single(lhs) || is_signaling_nan_single(rhs);

    let mut exceptions = binary_special_exceptions(operation, a as f64, b as f64, signaling);

    let value = if exceptions & FP_INVALID != 0 || a.is_nan() || b.is_nan() {
        LEGACY_QNAN_SINGLE
    } else {
        compute_single(operation, lhs, rhs)
    };

    if exceptions == 0 {
        exceptions |= single_binary_rounding_exceptions(operation, a, b, f32::from_bits(value));
    }

    FpResult { value, exceptions }
}

pub fn checked_double(operation: &Operation, lhs: u64, rhs: u64) -> FpResult<u64> {
    let a = f64::from_bits(lhs);
    let b = f64::from_bits(rhs);

    let signaling = is_signaling_nan_double(lhs) || is_signaling_nan_double(rhs);

    let mut exceptions = binary_special_exceptions(operation, a, b, signaling);

    let value = if exceptions & FP_INVALID != 0 || a.is_nan() || b.is_nan() {
        LEGACY_QNAN_DOUBLE
    } else {
        compute_double(operation, lhs, rhs)
    };

    if exceptions == 0 {
        exceptions |= double_binary_rounding_exceptions(operation, a, b, f64::from_bits(value));
    }

    FpResult { value, exceptions }
}

pub fn checked_single_unary(operation: &Operation, operand: u32) -> FpResult<u32> {
    // MOV.S는 순수 비트열 복사
    if matches!(operation, Operation::MovS) {
        return FpResult {
            value: operand,
            exceptions: 0,
        };
    }

    let value = f32::from_bits(operand);

    let invalid =
        is_signaling_nan_single(operand) || (matches!(operation, Operation::SqrtS) && value < 0.0);

    if invalid || value.is_nan() {
        return FpResult {
            value: LEGACY_QNAN_SINGLE,
            exceptions: if invalid { FP_INVALID } else { 0 },
        };
    }

    let result = compute_single_unary(operation, operand);

    let exceptions = if matches!(operation, Operation::SqrtS) && value.is_finite() {
        let r = f32::from_bits(result) as f64;

        if r * r != value as f64 { FP_INEXACT } else { 0 }
    } else {
        0
    };

    FpResult {
        value: result,
        exceptions,
    }
}

pub fn checked_double_unary(operation: &Operation, operand: u64) -> FpResult<u64> {
    // MOV.D는 순수 비트열 복사
    if matches!(operation, Operation::MovD) {
        return FpResult {
            value: operand,
            exceptions: 0,
        };
    }

    let value = f64::from_bits(operand);

    let invalid =
        is_signaling_nan_double(operand) || (matches!(operation, Operation::SqrtD) && value < 0.0);

    if invalid || value.is_nan() {
        return FpResult {
            value: LEGACY_QNAN_DOUBLE,
            exceptions: if invalid { FP_INVALID } else { 0 },
        };
    }

    let result = compute_double_unary(operation, operand);

    let exceptions = if matches!(operation, Operation::SqrtD) && value.is_finite() {
        let r = Dyadic::from_f64(f64::from_bits(result));

        let x = Dyadic::from_f64(value);

        if r.multiply(r) != x { FP_INEXACT } else { 0 }
    } else {
        0
    };

    FpResult {
        value: result,
        exceptions,
    }
}

pub const FP_UNDERFLOW: u32 = 1 << 1;
pub const FP_OVERFLOW: u32 = 1 << 2;

fn single_binary_rounding_exceptions(
    operation: &Operation,
    lhs: f32,
    rhs: f32,
    result: f32,
) -> u32 {
    if !lhs.is_finite() || !rhs.is_finite() {
        return 0;
    }

    if matches!(operation, Operation::DivS) && rhs == 0.0 {
        return 0;
    }

    // Finite operands producing infinity.
    if result.is_infinite() {
        return FP_OVERFLOW | FP_INEXACT;
    }

    let a = lhs as f64;
    let b = rhs as f64;
    let r = result as f64;

    let inexact = match operation {
        Operation::AddS => {
            // Error-free TwoSum residual.
            let s = a + b;
            let z = s - a;
            let error = (a - (s - z)) + (b - z);

            s != r || error != 0.0
        }

        Operation::SubS => {
            let b = -b;
            let s = a + b;
            let z = s - a;
            let error = (a - (s - z)) + (b - z);

            s != r || error != 0.0
        }

        Operation::MulS => a * b != r,

        Operation::DivS => r * b != a,

        _ => unreachable!(),
    };

    if !inexact {
        return 0;
    }

    let tiny = result.abs() < f32::MIN_POSITIVE;

    if tiny {
        FP_UNDERFLOW | FP_INEXACT
    } else {
        FP_INEXACT
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Dyadic {
    negative: bool,
    significand: u128,
    exponent: i32,
}

impl Dyadic {
    fn new(negative: bool, significand: u128, exponent: i32) -> Self {
        if significand == 0 {
            return Self {
                negative: false,
                significand: 0,
                exponent: 0,
            };
        }

        let shift = significand.trailing_zeros();

        Self {
            negative,
            significand: significand >> shift,
            exponent: exponent + shift as i32,
        }
    }

    fn from_f64(value: f64) -> Self {
        assert!(value.is_finite());

        let bits = value.to_bits();

        let negative = bits >> 63 != 0;

        let exponent = ((bits >> 52) & 0x7FF) as i32;

        let fraction = bits & 0x000F_FFFF_FFFF_FFFF;

        if exponent == 0 {
            // Zero or subnormal.
            Self::new(negative, fraction as u128, -1074)
        } else {
            // Normal number: implicit leading 1.
            Self::new(
                negative,
                ((1u64 << 52) | fraction) as u128,
                exponent - 1023 - 52,
            )
        }
    }

    fn negate(self) -> Self {
        Self::new(!self.negative, self.significand, self.exponent)
    }

    fn multiply(self, rhs: Self) -> Self {
        Self::new(
            self.negative ^ rhs.negative,
            self.significand * rhs.significand,
            self.exponent + rhs.exponent,
        )
    }
}

fn exact_dyadic_sum(lhs: Dyadic, rhs: Dyadic, result: Dyadic) -> bool {
    if lhs.significand == 0 {
        return rhs == result;
    }

    if rhs.significand == 0 {
        return lhs == result;
    }

    let exponent = lhs.exponent.min(rhs.exponent);

    let lhs_shift = lhs.exponent - exponent;
    let rhs_shift = rhs.exponent - exponent;

    // A sufficiently large exponent gap makes
    // the exact sum unrepresentable in binary64.
    if lhs_shift > 54 || rhs_shift > 54 {
        return false;
    }

    let a = (lhs.significand << lhs_shift as u32) as i128;

    let b = (rhs.significand << rhs_shift as u32) as i128;

    let a = if lhs.negative { -a } else { a };
    let b = if rhs.negative { -b } else { b };

    let sum = a + b;

    let exact = Dyadic::new(sum < 0, sum.unsigned_abs(), exponent);

    exact == result
}

fn double_binary_rounding_exceptions(
    operation: &Operation,
    lhs: f64,
    rhs: f64,
    result: f64,
) -> u32 {
    if !lhs.is_finite() || !rhs.is_finite() {
        return 0;
    }

    if matches!(operation, Operation::DivD) && rhs == 0.0 {
        return 0;
    }

    if result.is_infinite() {
        return FP_OVERFLOW | FP_INEXACT;
    }

    let a = Dyadic::from_f64(lhs);
    let b = Dyadic::from_f64(rhs);
    let r = Dyadic::from_f64(result);

    let exact = match operation {
        Operation::AddD => exact_dyadic_sum(a, b, r),

        Operation::SubD => exact_dyadic_sum(a, b.negate(), r),

        Operation::MulD => a.multiply(b) == r,

        Operation::DivD => r.multiply(b) == a,

        _ => unreachable!(),
    };

    if exact {
        return 0;
    }

    if result.abs() < f64::MIN_POSITIVE {
        FP_UNDERFLOW | FP_INEXACT
    } else {
        FP_INEXACT
    }
}

fn next_up_single(bits: u32) -> u32 {
    let value = f32::from_bits(bits);

    if value.is_nan() || value == f32::INFINITY {
        return bits;
    }

    if value == 0.0 {
        return 1;
    }

    if value.is_sign_negative() {
        bits - 1
    } else {
        bits + 1
    }
}

fn next_down_single(bits: u32) -> u32 {
    let value = f32::from_bits(bits);

    if value.is_nan() || value == f32::NEG_INFINITY {
        return bits;
    }

    if value == 0.0 {
        return 0x8000_0001;
    }

    if value.is_sign_negative() {
        bits + 1
    } else {
        bits - 1
    }
}
fn compare_exact_single(
    operation: &Operation,
    lhs: f64,
    rhs: f64,
    candidate: f64,
) -> std::cmp::Ordering {
    use std::cmp::Ordering;

    match operation {
        Operation::AddS | Operation::SubS => {
            let b = if matches!(operation, Operation::SubS) {
                -rhs
            } else {
                rhs
            };

            // TwoSum: exact result = sum + error
            let sum = lhs + b;
            let z = sum - lhs;

            let error = (lhs - (sum - z)) + (b - z);

            if sum < candidate {
                Ordering::Less
            } else if sum > candidate {
                Ordering::Greater
            } else {
                error.partial_cmp(&0.0).unwrap()
            }
        }

        Operation::MulS => (lhs * rhs).partial_cmp(&candidate).unwrap(),

        Operation::DivS => {
            let ordering = lhs.partial_cmp(&(candidate * rhs)).unwrap();

            if rhs.is_sign_negative() {
                ordering.reverse()
            } else {
                ordering
            }
        }

        Operation::SqrtS => lhs.partial_cmp(&(candidate * candidate)).unwrap(),

        _ => unreachable!(),
    }
}

fn direct_single_rounding(bits: u32, ordering: std::cmp::Ordering, mode: RoundingMode) -> u32 {
    use std::cmp::Ordering;

    if ordering == Ordering::Equal {
        return bits;
    }

    let negative = bits & 0x8000_0000 != 0;

    match mode {
        RoundingMode::NearestEven => bits,

        RoundingMode::TowardPositive => {
            if ordering == Ordering::Greater {
                next_up_single(bits)
            } else {
                bits
            }
        }

        RoundingMode::TowardNegative => {
            if ordering == Ordering::Less {
                next_down_single(bits)
            } else {
                bits
            }
        }

        RoundingMode::TowardZero => {
            if negative {
                if ordering == Ordering::Greater {
                    next_up_single(bits)
                } else {
                    bits
                }
            } else if ordering == Ordering::Less {
                next_down_single(bits)
            } else {
                bits
            }
        }
    }
}

pub fn rounded_single(
    operation: &Operation,
    lhs: u32,
    rhs: u32,
    mode: RoundingMode,
) -> FpResult<u32> {
    use std::cmp::Ordering;

    let mut result = checked_single(operation, lhs, rhs);

    if matches!(mode, RoundingMode::NearestEven) {
        return result;
    }

    let a = f32::from_bits(lhs);
    let b = f32::from_bits(rhs);

    // Special values are handled by checked_single.
    if !a.is_finite() || !b.is_finite() || result.exceptions & (FP_INVALID | FP_DIVIDE_BY_ZERO) != 0
    {
        return result;
    }

    let native = f32::from_bits(result.value);

    // Exact cancellation under roundTowardNegative.
    if native == 0.0 && result.exceptions == 0 {
        let opposite_signs = match operation {
            Operation::AddS => a.is_sign_negative() != b.is_sign_negative(),

            Operation::SubS => a.is_sign_negative() == b.is_sign_negative(),

            _ => false,
        };

        if opposite_signs && matches!(mode, RoundingMode::TowardNegative) {
            result.value = 0x8000_0000;
        }

        return result;
    }

    if result.exceptions & FP_INEXACT == 0 {
        return result;
    }

    let negative = native.is_sign_negative();

    // Handle nearest-even overflow separately.
    if native.is_infinite() {
        let saturate = match mode {
            RoundingMode::TowardZero => true,

            RoundingMode::TowardPositive => negative,

            RoundingMode::TowardNegative => !negative,

            RoundingMode::NearestEven => false,
        };

        if saturate {
            result.value = if negative { 0xFF7F_FFFF } else { 0x7F7F_FFFF };

            // Overflow threshold for rounding
            // toward smaller magnitude: 2^128.
            let limit = 2.0f64.powi(128);

            let threshold = if negative { -limit } else { limit };

            let ordering = compare_exact_single(operation, a as f64, b as f64, threshold);

            let overflow = if negative {
                ordering != Ordering::Greater
            } else {
                ordering != Ordering::Less
            };

            result.exceptions = FP_INEXACT;

            if overflow {
                result.exceptions |= FP_OVERFLOW;
            }
        }

        return result;
    }

    let ordering = compare_exact_single(operation, a as f64, b as f64, native as f64);

    result.value = direct_single_rounding(result.value, ordering, mode);

    let rounded = f32::from_bits(result.value);

    // Recompute range-related exceptions
    // from the directed result.
    result.exceptions = FP_INEXACT;

    if rounded.is_infinite() {
        result.exceptions |= FP_OVERFLOW;
    } else if rounded.abs() < f32::MIN_POSITIVE {
        result.exceptions |= FP_UNDERFLOW;
    }

    result
}

pub fn rounded_single_unary(
    operation: &Operation,
    operand: u32,
    mode: RoundingMode,
) -> FpResult<u32> {
    let mut result = checked_single_unary(operation, operand);

    if !matches!(operation, Operation::SqrtS)
        || matches!(mode, RoundingMode::NearestEven)
        || result.exceptions & FP_INEXACT == 0
    {
        return result;
    }

    let x = f32::from_bits(operand) as f64;
    let r = f32::from_bits(result.value) as f64;

    let ordering = compare_exact_single(operation, x, 0.0, r);

    result.value = direct_single_rounding(result.value, ordering, mode);

    result
}

impl Dyadic {
    fn compare(self, rhs: Self) -> std::cmp::Ordering {
        use std::cmp::Ordering;

        if self.significand == 0 && rhs.significand == 0 {
            return Ordering::Equal;
        }

        if self.negative != rhs.negative {
            return if self.negative {
                Ordering::Less
            } else {
                Ordering::Greater
            };
        }

        let magnitude = if self.significand == 0 {
            Ordering::Less
        } else if rhs.significand == 0 {
            Ordering::Greater
        } else {
            let lhs_top = self.exponent + (128 - self.significand.leading_zeros()) as i32;

            let rhs_top = rhs.exponent + (128 - rhs.significand.leading_zeros()) as i32;

            match lhs_top.cmp(&rhs_top) {
                Ordering::Equal => {
                    let exponent = self.exponent.min(rhs.exponent);

                    let a = self.significand << (self.exponent - exponent) as u32;

                    let b = rhs.significand << (rhs.exponent - exponent) as u32;

                    a.cmp(&b)
                }

                ordering => ordering,
            }
        };

        if self.negative {
            magnitude.reverse()
        } else {
            magnitude
        }
    }
}

fn compare_dyadic_sum(lhs: Dyadic, rhs: Dyadic, candidate: Dyadic) -> std::cmp::Ordering {
    use std::cmp::Ordering;

    // Covers binary64 finite values and
    // the overflow threshold 2^1024.
    const WORDS: usize = 34;
    const BASE_EXPONENT: i32 = -1074;

    fn accumulate(words: &mut [u64; WORDS], value: Dyadic) {
        if value.significand == 0 {
            return;
        }

        let mut significand = value.significand;
        let mut bit = (value.exponent - BASE_EXPONENT) as usize;

        while significand != 0 {
            if significand & 1 != 0 {
                let mut index = bit / 64;
                let mut carry = 1u64 << (bit % 64);

                loop {
                    let (sum, overflow) = words[index].overflowing_add(carry);

                    words[index] = sum;

                    if !overflow {
                        break;
                    }

                    index += 1;
                    carry = 1;
                }
            }

            significand >>= 1;
            bit += 1;
        }
    }

    let mut positive = [0u64; WORDS];
    let mut negative = [0u64; WORDS];

    for value in [lhs, rhs, candidate.negate()] {
        if value.negative {
            accumulate(&mut negative, value);
        } else {
            accumulate(&mut positive, value);
        }
    }

    for index in (0..WORDS).rev() {
        match positive[index].cmp(&negative[index]) {
            Ordering::Equal => {}
            ordering => return ordering,
        }
    }

    Ordering::Equal
}

fn compare_exact_double(
    operation: &Operation,
    lhs: Dyadic,
    rhs: Dyadic,
    candidate: Dyadic,
) -> std::cmp::Ordering {
    match operation {
        Operation::AddD => compare_dyadic_sum(lhs, rhs, candidate),

        Operation::SubD => compare_dyadic_sum(lhs, rhs.negate(), candidate),

        Operation::MulD => lhs.multiply(rhs).compare(candidate),

        Operation::DivD => {
            let ordering = lhs.compare(candidate.multiply(rhs));

            if rhs.negative {
                ordering.reverse()
            } else {
                ordering
            }
        }

        Operation::SqrtD => lhs.compare(candidate.multiply(candidate)),

        _ => unreachable!(),
    }
}

fn next_up_double(bits: u64) -> u64 {
    let value = f64::from_bits(bits);

    if value.is_nan() || value == f64::INFINITY {
        return bits;
    }

    if value == 0.0 {
        return 1;
    }

    if value.is_sign_negative() {
        bits - 1
    } else {
        bits + 1
    }
}

fn next_down_double(bits: u64) -> u64 {
    let value = f64::from_bits(bits);

    if value.is_nan() || value == f64::NEG_INFINITY {
        return bits;
    }

    if value == 0.0 {
        return 0x8000_0000_0000_0001;
    }

    if value.is_sign_negative() {
        bits + 1
    } else {
        bits - 1
    }
}

fn direct_double_rounding(bits: u64, ordering: std::cmp::Ordering, mode: RoundingMode) -> u64 {
    use std::cmp::Ordering;

    if ordering == Ordering::Equal {
        return bits;
    }

    let negative = bits & 0x8000_0000_0000_0000 != 0;

    match mode {
        RoundingMode::NearestEven => bits,

        RoundingMode::TowardPositive => {
            if ordering == Ordering::Greater {
                next_up_double(bits)
            } else {
                bits
            }
        }

        RoundingMode::TowardNegative => {
            if ordering == Ordering::Less {
                next_down_double(bits)
            } else {
                bits
            }
        }

        RoundingMode::TowardZero => {
            if negative {
                if ordering == Ordering::Greater {
                    next_up_double(bits)
                } else {
                    bits
                }
            } else if ordering == Ordering::Less {
                next_down_double(bits)
            } else {
                bits
            }
        }
    }
}

pub fn rounded_double(
    operation: &Operation,
    lhs: u64,
    rhs: u64,
    mode: RoundingMode,
) -> FpResult<u64> {
    use std::cmp::Ordering;

    let mut result = checked_double(operation, lhs, rhs);

    if matches!(mode, RoundingMode::NearestEven) {
        return result;
    }

    let a = f64::from_bits(lhs);
    let b = f64::from_bits(rhs);

    if !a.is_finite() || !b.is_finite() || result.exceptions & (FP_INVALID | FP_DIVIDE_BY_ZERO) != 0
    {
        return result;
    }

    let native = f64::from_bits(result.value);

    // Signed zero from exact cancellation.
    if native == 0.0 && result.exceptions == 0 {
        let opposite_signs = match operation {
            Operation::AddD => a.is_sign_negative() != b.is_sign_negative(),

            Operation::SubD => a.is_sign_negative() == b.is_sign_negative(),

            _ => false,
        };

        if opposite_signs && matches!(mode, RoundingMode::TowardNegative) {
            result.value = 0x8000_0000_0000_0000;
        }

        return result;
    }

    if result.exceptions & FP_INEXACT == 0 {
        return result;
    }

    let exact_a = Dyadic::from_f64(a);
    let exact_b = Dyadic::from_f64(b);

    let negative = native.is_sign_negative();

    // Native nearest-even overflow.
    if native.is_infinite() {
        let saturate = match mode {
            RoundingMode::TowardZero => true,

            RoundingMode::TowardPositive => negative,

            RoundingMode::TowardNegative => !negative,

            RoundingMode::NearestEven => false,
        };

        if saturate {
            result.value = if negative {
                0xFFEF_FFFF_FFFF_FFFF
            } else {
                0x7FEF_FFFF_FFFF_FFFF
            };

            // 2^1024 cannot be represented as f64.
            // Construct it directly as a Dyadic.
            let threshold = Dyadic::new(negative, 1, 1024);

            let ordering = compare_exact_double(operation, exact_a, exact_b, threshold);

            let overflow = if negative {
                ordering != Ordering::Greater
            } else {
                ordering != Ordering::Less
            };

            result.exceptions = FP_INEXACT;

            if overflow {
                result.exceptions |= FP_OVERFLOW;
            }
        }

        return result;
    }

    let candidate = Dyadic::from_f64(native);

    let ordering = compare_exact_double(operation, exact_a, exact_b, candidate);

    result.value = direct_double_rounding(result.value, ordering, mode);

    let rounded = f64::from_bits(result.value);

    result.exceptions = FP_INEXACT;

    if rounded.is_infinite() {
        result.exceptions |= FP_OVERFLOW;
    } else if rounded.abs() < f64::MIN_POSITIVE {
        result.exceptions |= FP_UNDERFLOW;
    }

    result
}

pub fn rounded_double_unary(
    operation: &Operation,
    operand: u64,
    mode: RoundingMode,
) -> FpResult<u64> {
    let mut result = checked_double_unary(operation, operand);

    if !matches!(operation, Operation::SqrtD)
        || matches!(mode, RoundingMode::NearestEven)
        || result.exceptions & FP_INEXACT == 0
    {
        return result;
    }

    let x = Dyadic::from_f64(f64::from_bits(operand));

    let r = Dyadic::from_f64(f64::from_bits(result.value));

    let ordering = x.compare(r.multiply(r));

    result.value = direct_double_rounding(result.value, ordering, mode);

    result
}

fn round_f64_to_single(value: f64, mode: RoundingMode) -> FpResult<u32> {
    use std::cmp::Ordering;

    debug_assert!(!value.is_nan());

    let native = value as f32;

    if value.is_infinite() {
        return FpResult {
            value: native.to_bits(),
            exceptions: 0,
        };
    }

    // Exactly representable in binary32.
    if native as f64 == value {
        return FpResult {
            value: native.to_bits(),
            exceptions: 0,
        };
    }

    let mut bits = native.to_bits();

    // Native nearest-even overflow.
    if native.is_infinite() {
        let negative = value.is_sign_negative();

        let saturate = match mode {
            RoundingMode::NearestEven => false,
            RoundingMode::TowardZero => true,
            RoundingMode::TowardPositive => negative,
            RoundingMode::TowardNegative => !negative,
        };

        if saturate {
            bits = if negative { 0xFF7F_FFFF } else { 0x7F7F_FFFF };

            let overflow = value.abs() >= 2.0f64.powi(128);

            return FpResult {
                value: bits,
                exceptions: FP_INEXACT | if overflow { FP_OVERFLOW } else { 0 },
            };
        }

        return FpResult {
            value: bits,
            exceptions: FP_OVERFLOW | FP_INEXACT,
        };
    }

    let ordering = value
        .partial_cmp(&(native as f64))
        .unwrap_or(Ordering::Equal);

    bits = direct_single_rounding(bits, ordering, mode);

    let rounded = f32::from_bits(bits);

    let exceptions = if rounded.is_infinite() {
        FP_OVERFLOW | FP_INEXACT
    } else if rounded.abs() < f32::MIN_POSITIVE {
        FP_UNDERFLOW | FP_INEXACT
    } else {
        FP_INEXACT
    };

    FpResult {
        value: bits,
        exceptions,
    }
}

pub fn rounded_double_to_single(operand: u64, mode: RoundingMode) -> FpResult<u32> {
    let value = f64::from_bits(operand);

    if value.is_nan() {
        return FpResult {
            value: LEGACY_QNAN_SINGLE,

            exceptions: if is_signaling_nan_double(operand) {
                FP_INVALID
            } else {
                0
            },
        };
    }

    round_f64_to_single(value, mode)
}

pub fn checked_single_to_double(operand: u32) -> FpResult<u64> {
    let value = f32::from_bits(operand);

    if value.is_nan() {
        return FpResult {
            value: LEGACY_QNAN_DOUBLE,

            exceptions: if is_signaling_nan_single(operand) {
                FP_INVALID
            } else {
                0
            },
        };
    }

    FpResult {
        value: convert_single_to_double(operand),
        exceptions: 0,
    }
}

pub fn rounded_word_to_single(operand: u32, mode: RoundingMode) -> FpResult<u32> {
    let value = operand as i32 as f64;

    round_f64_to_single(value, mode)
}

pub fn checked_word_to_double(operand: u32) -> FpResult<u64> {
    FpResult {
        value: convert_word_to_double(operand),
        exceptions: 0,
    }
}

pub const FP_UNIMPLEMENTED: u32 = 1 << 5;

pub fn flush_single_result(
    mut result: FpResult<u32>,
    mode: RoundingMode,
    fs: bool,
) -> FpResult<u32> {
    if !fs {
        return result;
    }

    let value = f32::from_bits(result.value);

    let tiny = value.is_subnormal() || (value == 0.0 && result.exceptions & FP_UNDERFLOW != 0);

    if !tiny {
        return result;
    }

    let negative = value.is_sign_negative();

    result.value = match (mode, negative) {
        (RoundingMode::TowardPositive, false) => 0x0080_0000,

        (RoundingMode::TowardNegative, true) => 0x8080_0000,

        (_, true) => 0x8000_0000,

        (_, false) => 0,
    };

    result.exceptions |= FP_UNDERFLOW | FP_INEXACT;

    result
}

pub fn flush_double_result(
    mut result: FpResult<u64>,
    mode: RoundingMode,
    fs: bool,
) -> FpResult<u64> {
    if !fs {
        return result;
    }

    let value = f64::from_bits(result.value);

    let tiny = value.is_subnormal() || (value == 0.0 && result.exceptions & FP_UNDERFLOW != 0);

    if !tiny {
        return result;
    }

    let negative = value.is_sign_negative();

    result.value = match (mode, negative) {
        (RoundingMode::TowardPositive, false) => 0x0010_0000_0000_0000,

        (RoundingMode::TowardNegative, true) => 0x8010_0000_0000_0000,

        (_, true) => 0x8000_0000_0000_0000,

        (_, false) => 0,
    };

    result.exceptions |= FP_UNDERFLOW | FP_INEXACT;

    result
}
