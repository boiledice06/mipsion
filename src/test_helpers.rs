use crate::{cpu::Cpu, memory::Memory};

pub fn encode_r(opcode: u32, rs: u32, rt: u32, rd: u32, shamt: u32, funct: u32) -> u32 {
    assert!(opcode < 64 && funct < 64);
    assert!(rs < 32 && rt < 32 && rd < 32 && shamt < 32);
    (opcode << 26) | (rs << 21) | (rt << 16) | (rd << 11) | (shamt << 6) | funct
}

pub fn encode_i(opcode: u32, rs: u32, rt: u32, immediate: u16) -> u32 {
    assert!(opcode < 64 && rs < 32 && rt < 32);
    (opcode << 26) | (rs << 21) | (rt << 16) | u32::from(immediate)
}

pub fn encode_j(opcode: u32, target: u32) -> u32 {
    assert!(opcode < 64 && target < (1 << 26));
    (opcode << 26) | target
}

pub fn load_program(memory: &mut Memory, address: u32, words: &[u32]) {
    for (index, &word) in words.iter().enumerate() {
        memory.write_word(address + u32::try_from(index * 4).unwrap(), word);
    }
}

pub fn run_steps(cpu: &mut Cpu, memory: &Memory, steps: usize) {
    for _ in 0..steps {
        cpu.step(memory);
    }
}

pub fn run(words: &[u32], regs: &[(usize, u32)]) -> Cpu {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(words.len() * 4);
    for &(index, value) in regs {
        cpu.write_reg(index, value);
    }
    load_program(&mut memory, 0, words);
    run_steps(&mut cpu, &memory, words.len());
    cpu
}
