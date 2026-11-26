use crate::cpu::Cpu;
use crate::instruction::RawInstruction;
use crate::isa::Operation;

use crate::alu::{accumulate, compute, divide, extract, insert, multiply};

use crate::memory::Memory;

pub fn execute(cpu: &mut Cpu, memory: &mut Memory, inst: &RawInstruction, operation: &Operation) {
    match operation {
        Operation::Add
        | Operation::Addu
        | Operation::Sub
        | Operation::Subu
        | Operation::And
        | Operation::Or
        | Operation::Xor
        | Operation::Nor
        | Operation::Slt
        | Operation::Sltu
        | Operation::Mul => {
            let rs = inst.rs() as usize;
            let rt = inst.rt() as usize;
            let rd = inst.rd() as usize;

            let a = cpu.read_reg(rs);
            let b = cpu.read_reg(rt);

            let result = compute(a, b, operation);

            cpu.write_reg(rd, result);
        }

        Operation::Addi | Operation::Addiu | Operation::Slti | Operation::Sltiu => {
            let rs = inst.rs() as usize;
            let rt = inst.rt() as usize;

            let a = cpu.read_reg(rs);
            let b = inst.immediate() as i16 as i32 as u32;

            let result = compute(a, b, operation);

            cpu.write_reg(rt, result);
        }
        Operation::Sll | Operation::Srl | Operation::Sra | Operation::Rotr => {
            let rt = inst.rt() as usize;
            let rd = inst.rd() as usize;

            let a = cpu.read_reg(rt);
            let b = inst.shamt() as u32;

            let result = compute(a, b, operation);

            cpu.write_reg(rd, result);
        }

        Operation::Andi | Operation::Ori | Operation::Xori => {
            let rs = inst.rs() as usize;
            let rt = inst.rt() as usize;

            let a = cpu.read_reg(rs);
            let b = inst.immediate() as u32;

            let result = compute(a, b, operation);

            cpu.write_reg(rt, result);
        }

        Operation::Sllv | Operation::Srlv | Operation::Srav | Operation::Rotrv => {
            let rs = inst.rs() as usize;
            let rt = inst.rt() as usize;
            let rd = inst.rd() as usize;

            let a = cpu.read_reg(rt);
            let b = cpu.read_reg(rs) & 0x1F;

            let result = compute(a, b, operation);

            cpu.write_reg(rd, result);
        }

        Operation::Lui => {
            let rt = inst.rt() as usize;
            let b = inst.immediate() as u32;

            let result = compute(0, b, operation);

            cpu.write_reg(rt, result);
        }

        Operation::Clz | Operation::Clo => {
            let rs = inst.rs() as usize;
            let rd = inst.rd() as usize;

            let a = cpu.read_reg(rs);
            let result = compute(a, 0, operation);

            cpu.write_reg(rd, result);
        }

        Operation::Seb | Operation::Seh | Operation::Wsbh => {
            let rt = inst.rt() as usize;
            let rd = inst.rd() as usize;

            let a = cpu.read_reg(rt);
            let result = compute(a, 0, operation);

            cpu.write_reg(rd, result);
        }

        Operation::Ext => {
            let rs = inst.rs() as usize;
            let rt = inst.rt() as usize;

            let pos = inst.shamt() as u32;
            let size = inst.rd() as u32 + 1;

            assert!(pos + size <= 32);

            let a = cpu.read_reg(rs);
            let result = extract(a, pos, size);

            cpu.write_reg(rt, result);
        }

        Operation::Ins => {
            let rs = inst.rs() as usize;
            let rt = inst.rt() as usize;

            let pos = inst.shamt() as u32;
            let msb = inst.rd() as u32;

            assert!(msb >= pos);

            let size = msb - pos + 1;

            let a = cpu.read_reg(rs);
            let b = cpu.read_reg(rt);

            let result = insert(a, b, pos, size);

            cpu.write_reg(rt, result);
        }

        Operation::Mult | Operation::Multu => {
            let a = cpu.read_reg(inst.rs() as usize);
            let b = cpu.read_reg(inst.rt() as usize);

            let signed = matches!(operation, Operation::Mult);
            let result = multiply(a, b, signed);

            cpu.write_hi((result >> 32) as u32);
            cpu.write_lo(result as u32);
        }

        Operation::Div | Operation::Divu => {
            let a = cpu.read_reg(inst.rs() as usize);
            let b = cpu.read_reg(inst.rt() as usize);

            let signed = matches!(operation, Operation::Div);

            if let Some((hi, lo)) = divide(a, b, signed) {
                cpu.write_hi(hi);
                cpu.write_lo(lo);
            }
        }

        Operation::Mfhi => {
            let rd = inst.rd() as usize;
            cpu.write_reg(rd, cpu.read_hi());
        }

        Operation::Mflo => {
            let rd = inst.rd() as usize;
            cpu.write_reg(rd, cpu.read_lo());
        }

        Operation::Mthi => {
            let rs = inst.rs() as usize;
            cpu.write_hi(cpu.read_reg(rs));
        }

        Operation::Mtlo => {
            let rs = inst.rs() as usize;
            cpu.write_lo(cpu.read_reg(rs));
        }

        Operation::Madd | Operation::Maddu | Operation::Msub | Operation::Msubu => {
            let a = cpu.read_reg(inst.rs() as usize);
            let b = cpu.read_reg(inst.rt() as usize);

            let signed = matches!(operation, Operation::Madd | Operation::Msub);

            let subtract = matches!(operation, Operation::Msub | Operation::Msubu);

            let acc = cpu.read_hilo();
            let result = accumulate(a, b, acc, signed, subtract);

            cpu.write_hilo(result);
        }

        Operation::Lw | Operation::Lh | Operation::Lhu | Operation::Lb | Operation::Lbu => {
            let rs = inst.rs() as usize;
            let rt = inst.rt() as usize;

            let base = cpu.read_reg(rs);
            let offset = inst.immediate() as i16 as i32 as u32;

            let address = base.wrapping_add(offset);

            let result = match operation {
                Operation::Lw => memory.read_word(address),
                Operation::Lh => memory.read_halfword(address) as i16 as i32 as u32,
                Operation::Lhu => memory.read_halfword(address) as u32,
                Operation::Lb => memory.read_byte(address) as i8 as i32 as u32,
                Operation::Lbu => memory.read_byte(address) as u32,
                _ => unreachable!(),
            };

            cpu.write_reg(rt, result);
        }

        Operation::Sw | Operation::Sh | Operation::Sb => {
            let rs = inst.rs() as usize;
            let rt = inst.rt() as usize;

            let base = cpu.read_reg(rs);
            let offset = inst.immediate() as i16 as i32 as u32;

            let address = base.wrapping_add(offset);
            let value = cpu.read_reg(rt);

            match operation {
                Operation::Sw => memory.write_word(address, value),
                Operation::Sh => memory.write_halfword(address, value as u16),
                Operation::Sb => memory.write_byte(address, value as u8),
                _ => unreachable!(),
            }
        }
        _ => todo!(
            "Instruction not implemented: opcode={:#04x}, funct={:#04x}",
            inst.opcode(),
            inst.funct()
        ),
    }
}
