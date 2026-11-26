mod cpu;
mod memory;
use cpu::Cpu;
use memory::Memory;

mod instruction;
use instruction::RawInstruction;

mod decoder;
mod isa;
use decoder::decode;

mod executor;
use executor::execute;

mod alu;

fn main() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024 * 1024);

    // Initialize registers
    memory.write_word(0, 0x2008000C); // addi $t0, $zero, 12
    memory.write_word(4, 0x2009000A); // addi $t1, $zero, 10

    // ALU instructions
    memory.write_word(8, 0x01095022); // sub $t2, $t0, $t1
    memory.write_word(12, 0x01285822); // sub $t3, $t1, $t0
    memory.write_word(16, 0x01096024); // and $t4, $t0, $t1
    memory.write_word(20, 0x01096825); // or  $t5, $t0, $t1
    memory.write_word(24, 0x01097026); // xor $t6, $t0, $t1
    memory.write_word(28, 0x01097827); // nor $t7, $t0, $t1

    // Execute
    for _ in 0..8 {
        cpu.step(&memory);
    }

    // Verify
    assert_eq!(cpu.read_reg(10), 2);
    println!("SUB test passed!");

    assert_eq!(cpu.read_reg(11), 0xFFFFFFFE);
    println!("SUB negative result test passed!");

    assert_eq!(cpu.read_reg(12), 8);
    println!("AND test passed!");

    assert_eq!(cpu.read_reg(13), 14);
    println!("OR test passed!");

    assert_eq!(cpu.read_reg(14), 6);
    println!("XOR test passed!");

    assert_eq!(cpu.read_reg(15), 0xFFFFFFF1);
    println!("NOR test passed!");

    println!("All ALU tests passed!");
}
