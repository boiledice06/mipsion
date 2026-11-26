use crate::memory::Memory;

use crate::decoder::decode;
use crate::executor::execute;
use crate::instruction::RawInstruction;

pub struct Cpu {
    gpr: [u32; 32], // 범용 register 32
    pc: u32,        // Program Counter
    hi: u32,        // 곱셈 상위 32비트, 나눗셈 나머지
    lo: u32,        // 곱셈 하위 32비트, 나눗셈 몫
    halted: bool,   // 에뮬레이터 실행 중단 여부
}

impl Cpu {
    pub fn new() -> Self {
        Self {
            gpr: [0; 32],
            pc: 0x00000000,
            hi: 0,
            lo: 0,
            halted: false,
        }
    }

    pub fn read_reg(&self, index: usize) -> u32 {
        self.gpr[index]
    }

    pub fn write_reg(&mut self, index: usize, value: u32) {
        if index != 0 {
            self.gpr[index] = value;
        }
    }

    pub fn fetch(&mut self, memory: &Memory) -> u32 {
        let w = memory.read_word(self.pc);
        self.pc += 4;
        w
    }

    pub fn step(&mut self, memory: &Memory) {
        // Fetch -> RawInstruction -> Decode -> Execute
        let w = self.fetch(memory);
        let raw_inst = RawInstruction::new(w);

        let decoded = decode(w).unwrap();

        execute(self, &raw_inst, &decoded.operation);
    }
}
