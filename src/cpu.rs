use crate::memory::Memory;

use crate::decoder::decode;
use crate::executor::execute;
use crate::instruction::RawInstruction;

use crate::cp0::Cp0;
use crate::exception::Exception;

pub struct Cpu {
    gpr: [u32; 32], // 범용 register 32
    pc: u32,        // Program Counter
    next_pc: u32,
    hi: u32,      // 곱셈 상위 32비트, 나눗셈 나머지
    lo: u32,      // 곱셈 하위 32비트, 나눗셈 몫
    halted: bool, // 에뮬레이터 실행 중단 여부
    cp0: Cp0,
    current_pc: u32,
    in_delay_slot: bool,
    next_delay_slot: bool,
    reservation: Option<u32>,
}

impl Cpu {
    pub fn new() -> Self {
        Self {
            gpr: [0; 32],
            pc: 0x00000000,
            next_pc: 4,
            hi: 0,
            lo: 0,
            halted: false,
            cp0: Cp0::new(),
            current_pc: 0,
            in_delay_slot: false,
            next_delay_slot: false,
            reservation: None,
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
        self.current_pc = self.pc;

        self.in_delay_slot = self.next_delay_slot;
        self.next_delay_slot = false;

        let word = memory.read_word(self.pc);

        self.pc = self.next_pc;
        self.next_pc = self.next_pc.wrapping_add(4);

        word
    }

    pub fn step(&mut self, memory: &mut Memory) {
        if self.halted {
            return;
        }

        self.cp0.tick();

        if self.pc & 3 != 0 {
            self.current_pc = self.pc;
            self.in_delay_slot = self.next_delay_slot;

            self.raise_exception(Exception::AddressErrorLoad, Some(self.pc));
            return;
        }

        let word = self.fetch(memory);
        let raw_inst = RawInstruction::new(word);

        match decode(word) {
            Some(decoded) => {
                execute(self, memory, &raw_inst, &decoded.operation);
            }
            None => {
                self.raise_exception(Exception::ReservedInstruction, None);
            }
        }
    }

    pub fn read_hi(&self) -> u32 {
        self.hi
    }

    pub fn read_lo(&self) -> u32 {
        self.lo
    }

    pub fn write_hi(&mut self, value: u32) {
        self.hi = value;
    }

    pub fn write_lo(&mut self, value: u32) {
        self.lo = value;
    }

    pub fn read_hilo(&self) -> u64 {
        ((self.hi as u64) << 32) | self.lo as u64
    }

    pub fn write_hilo(&mut self, value: u64) {
        self.hi = (value >> 32) as u32;
        self.lo = value as u32;
    }

    pub fn read_pc(&self) -> u32 {
        self.pc
    }

    pub fn read_next_pc(&self) -> u32 {
        self.next_pc
    }

    pub fn write_pc(&mut self, address: u32) {
        self.pc = address;
        self.next_pc = address.wrapping_add(4);
    }

    pub fn write_next_pc(&mut self, address: u32) {
        self.next_pc = address;
    }

    pub fn skip_delay_slot(&mut self) {
        self.pc = self.next_pc;
        self.next_pc = self.next_pc.wrapping_add(4);
    }

    pub fn cp0(&self) -> &Cp0 {
        &self.cp0
    }

    pub fn mark_delay_slot(&mut self) {
        self.next_delay_slot = true;
    }

    pub fn raise_exception(&mut self, exception: Exception, bad_address: Option<u32>) {
        self.cp0
            .enter_exception(exception, self.current_pc, self.in_delay_slot, bad_address);

        self.halted = true;
        self.in_delay_slot = false;
        self.next_delay_slot = false;

        self.clear_reservation();
    }

    pub fn is_halted(&self) -> bool {
        self.halted
    }

    pub fn set_reservation(&mut self, address: u32) {
        self.reservation = Some(address);
    }

    pub fn check_reservation(&self, address: u32) -> bool {
        self.reservation == Some(address)
    }

    pub fn clear_reservation(&mut self) {
        self.reservation = None;
    }
}
