use crate::cpu::Cpu;
use crate::instruction::RawInstruction;
use crate::isa::Operation;

pub fn execute(cpu: &mut Cpu, inst: &RawInstruction, operation: &Operation) {
    match operation {
        Operation::Add => {
            let rs = inst.rs() as usize;
            let rt = inst.rt() as usize;
            let rd = inst.rd() as usize;

            let a = cpu.read_reg(rs);
            let b = cpu.read_reg(rt);

            cpu.write_reg(rd, a + b);
        }
        Operation::Addu => {
            let rs = inst.rs() as usize;
            let rt = inst.rt() as usize;
            let rd = inst.rd() as usize;

            let a = cpu.read_reg(rs);
            let b = cpu.read_reg(rt);

            cpu.write_reg(rd, a.wrapping_add(b));
        }
        Operation::Addi => {
            let rs = inst.rs() as usize;
            let rt = inst.rt() as usize;

            let imm = inst.immediate() as i16 as i32;
            let rreg = cpu.read_reg(rs) as i32;

            cpu.write_reg(rt, rreg.wrapping_add(imm) as u32);
        }

        _ => todo!("Instruction not implemented"),
    }
}
