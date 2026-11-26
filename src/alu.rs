use crate::isa::Operation;

pub fn compute(a: u32, b: u32, operation: &Operation) -> u32 {
    match operation {
        Operation::Add | Operation::Addi | Operation::Addu | Operation::Addiu => a.wrapping_add(b),
        Operation::Sub | Operation::Subu => {
            let b_neg = (!b).wrapping_add(1);
            a.wrapping_add(b_neg)
        }
        Operation::And => a & b,
        Operation::Or => a | b,

        Operation::Xor => a ^ b,
        Operation::Nor => !(a | b),

        Operation::Slt => ((a as i32) < (b as i32)) as u32,
        Operation::Sltu => (a < b) as u32,
        Operation::Sll => a << b,
        Operation::Srl => a >> b,
        Operation::Sra => ((a as i32) >> b) as u32,
    }
}
