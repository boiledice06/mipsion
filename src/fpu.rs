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
