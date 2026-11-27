use crate::memory::Memory;

use crate::decoder::decode;
use crate::executor::execute;
use crate::instruction::RawInstruction;

use crate::cp0::Cp0;
use crate::cp1::Cp1;
use crate::exception::Exception;

use crate::mmu::{AccessType, Mmu, TranslationMode};

pub struct Cpu {
    gpr: [u32; 32], // 범용 register 32
    pc: u32,        // Program Counter
    next_pc: u32,
    hi: u32,      // 곱셈 상위 32비트, 나눗셈 나머지
    lo: u32,      // 곱셈 하위 32비트, 나눗셈 몫
    halted: bool, // 에뮬레이터 실행 중단 여부
    cp0: Cp0,
    cp1: Cp1,
    current_pc: u32,
    in_delay_slot: bool,
    next_delay_slot: bool,
    reservation: Option<u32>,
    translation_mode: TranslationMode,
    exception_count: u64,
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
            cp1: Cp1::new(),
            current_pc: 0,
            in_delay_slot: false,
            next_delay_slot: false,
            reservation: None,
            translation_mode: TranslationMode::Bare,
            exception_count: 0,
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

    pub fn fetch(&mut self, memory: &Memory) -> Option<u32> {
        self.current_pc = self.pc;

        self.in_delay_slot = self.next_delay_slot;
        self.next_delay_slot = false;

        let address = self.translate_address(self.pc, AccessType::Fetch)?;

        if !self.check_memory_access(memory, address, 4, AccessType::Fetch) {
            return None;
        }

        let word = memory.read_word(address);

        self.pc = self.next_pc;
        self.next_pc = self.next_pc.wrapping_add(4);

        Some(word)
    }

    pub fn step(&mut self, memory: &mut Memory) {
        if self.halted {
            return;
        }

        self.cp0.tick();

        // Interrupt 검사
        if self.cp0.interrupt_requested() {
            self.current_pc = self.pc;
            self.in_delay_slot = self.next_delay_slot;

            self.raise_exception(Exception::Interrupt, None);
            return;
        }

        // Instruction Alignment 검사
        if self.pc & 3 != 0 {
            self.current_pc = self.pc;
            self.in_delay_slot = self.next_delay_slot;

            self.raise_exception(Exception::AddressErrorLoad, Some(self.pc));
            return;
        }

        // 주소 변환 실패 시 실행 중단
        let Some(word) = self.fetch(memory) else {
            return;
        };

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

    pub fn cp0_mut(&mut self) -> &mut Cp0 {
        &mut self.cp0
    }

    pub fn cp1(&self) -> &Cp1 {
        &self.cp1
    }

    pub fn cp1_mut(&mut self) -> &mut Cp1 {
        &mut self.cp1
    }

    pub fn mark_delay_slot(&mut self) {
        self.next_delay_slot = true;
    }

    pub fn raise_exception(&mut self, exception: Exception, bad_address: Option<u32>) {
        self.exception_count = self.exception_count.wrapping_add(1);

        // Exception 진입 이전의 EXL로 Vector 결정
        let vector = self.cp0.exception_vector(exception);

        self.cp0
            .enter_exception(exception, self.current_pc, self.in_delay_slot, bad_address);

        self.write_pc(vector);

        self.in_delay_slot = false;
        self.next_delay_slot = false;

        self.clear_reservation();
    }

    pub fn return_from_exception(&mut self) {
        let address = self.cp0.return_from_exception();

        // 복귀 주소로 즉시 이동
        self.write_pc(address);

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

    pub fn translate_address(&mut self, address: u32, access: AccessType) -> Option<u32> {
        let result = match self.translation_mode {
            TranslationMode::Bare => Mmu::translate(address, self.cp0.privilege_mode(), access),

            TranslationMode::Tlb => Mmu::translate_with_tlb(address, &self.cp0, access),
        };

        match result {
            Ok(physical) => Some(physical),

            Err(exception) => {
                self.raise_exception(exception, Some(address));

                None
            }
        }
    }

    pub fn new_with_tlb() -> Self {
        let mut cpu = Self::new();
        cpu.translation_mode = TranslationMode::Tlb;
        cpu
    }

    pub fn check_memory_access(
        &mut self,
        memory: &Memory,
        address: u32,
        size: usize,
        access: AccessType,
    ) -> bool {
        if memory.contains_range(address, size) {
            return true;
        }

        let exception = match access {
            AccessType::Fetch => Exception::InstructionBusError,
            AccessType::Load | AccessType::Store => Exception::DataBusError,
        };

        self.raise_exception(exception, None);

        false
    }

    pub fn exception_count(&self) -> u64 {
        self.exception_count
    }
}
