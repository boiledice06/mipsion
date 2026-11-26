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

fn main() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024 * 1024);

    memory.write_word(0, 0x2008000A);
    memory.write_word(4, 0x20090014);
    memory.write_word(8, 0x01095020);

    for _ in 0..3 {
        cpu.step(&memory);
    }

    assert_eq!(cpu.read_reg(8), 10);
    assert_eq!(cpu.read_reg(9), 20);
    assert_eq!(cpu.read_reg(10), 30);

    println!("First MIPS program executed successfully!");
}
