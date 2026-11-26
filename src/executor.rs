use crate::cpu::Cpu;
use crate::instruction::RawInstruction;
use crate::isa::Operation;

use crate::alu::compute;

pub fn execute(cpu: &mut Cpu, inst: &RawInstruction, operation: &Operation) {
    match operation {
        Operation::Add
        | Operation::Addu
        | Operation::Sub
        | Operation::Subu
        | Operation::And
        | Operation::Or
        | Operation::Xor
        | Operation::Nor => {
            let rs = inst.rs() as usize;
            let rt = inst.rt() as usize;
            let rd = inst.rd() as usize;

            let a = cpu.read_reg(rs);
            let b = cpu.read_reg(rt);

            let result = compute(a, b, operation);

            cpu.write_reg(rd, result);
        }

        Operation::Addi | Operation::Addiu => {
            let rs = inst.rs() as usize;
            let rt = inst.rt() as usize;

            let a = cpu.read_reg(rs);
            let b = inst.immediate() as i16 as i32 as u32;

            let result = compute(a, b, operation);

            cpu.write_reg(rt, result);
        }
    }
}
