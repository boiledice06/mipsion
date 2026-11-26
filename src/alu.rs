use crate::isa::Operation;

pub fn compute(a: u32, b: u32, operation: &Operation) -> u32 {
    match operation {
        Operation::Add | Operation::Addi | Operation::Addu | Operation::Addiu => a.wrapping_add(b),
        Operation::Sub | Operation::Subu => {
            let b_neg = (!b).wrapping_add(1);
            a.wrapping_add(b_neg)
        }
        Operation::And | Operation::Andi => a & b,
        Operation::Or | Operation::Ori => a | b,

        Operation::Xor | Operation::Xori => a ^ b,
        Operation::Nor => !(a | b),

        Operation::Slt | Operation::Slti => ((a as i32) < (b as i32)) as u32,
        Operation::Sltu | Operation::Sltiu => (a < b) as u32,
        Operation::Sll | Operation::Sllv => a << b,
        Operation::Srl | Operation::Srlv => a >> b,
        Operation::Sra | Operation::Srav => ((a as i32) >> b) as u32,

        Operation::Rotr | Operation::Rotrv => a.rotate_right(b),

        Operation::Lui => b << 16,

        Operation::Clz => a.leading_zeros(),
        Operation::Clo => a.leading_ones(),

        Operation::Seb => a as i8 as i32 as u32,
        Operation::Seh => a as i16 as i32 as u32,

        Operation::Wsbh => ((a & 0x00FF00FF) << 8) | ((a & 0xFF00FF00) >> 8),

        Operation::Mul => a.wrapping_mul(b),
        _ => todo!("Instruction not implemented"),
    }
}

fn bit_mask(size: u32) -> u32 {
    u32::MAX >> (32 - size)
}

pub fn extract(value: u32, pos: u32, size: u32) -> u32 {
    (value >> pos) & bit_mask(size)
}

pub fn insert(src: u32, dst: u32, pos: u32, size: u32) -> u32 {
    let mask = bit_mask(size) << pos;

    (dst & !mask) | ((src << pos) & mask)
}

pub fn multiply(a: u32, b: u32, signed: bool) -> u64 {
    if signed {
        ((a as i32 as i64) * (b as i32 as i64)) as u64
    } else {
        (a as u64) * (b as u64)
    }
}

pub fn divide(a: u32, b: u32, signed: bool) -> Option<(u32, u32)> {
    if b == 0 {
        return None;
    }

    if signed {
        let a = a as i32;
        let b = b as i32;

        Some((a.wrapping_rem(b) as u32, a.wrapping_div(b) as u32))
    } else {
        Some((a % b, a / b))
    }
}

pub fn accumulate(a: u32, b: u32, acc: u64, signed: bool, subtract: bool) -> u64 {
    let product = multiply(a, b, signed);

    if subtract {
        acc.wrapping_sub(product)
    } else {
        acc.wrapping_add(product)
    }
}
