use crate::exception::Exception;
use crate::tlb::{Tlb, TlbEntry};

pub const TLB_SIZE: usize = 32;

const CONFIG_RESET: u32 = (1 << 31) | (1 << 10) | (1 << 7) | 2;

const CONFIG1: u32 = ((TLB_SIZE - 1) as u32) << 25;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrivilegeMode {
    Kernel,
    Supervisor,
    User,
    Reserved,
}

pub struct Cp0 {
    bad_vaddr: u32,
    status: u32,
    cause: u32,
    epc: u32,
    count: u32,
    hwrena: u32,
    user_local: u32,
    ebase: u32,
    error_epc: u32,
    compare: u32,
    index: u32,
    random: u32,
    entry_lo0: u32,
    entry_lo1: u32,
    context: u32,
    page_mask: u32,
    wired: u32,
    entry_hi: u32,
    tlb: Tlb,
    config: u32,
}

impl Cp0 {
    pub fn new() -> Self {
        Self {
            bad_vaddr: 0,
            status: 0,
            cause: 0,
            epc: 0,
            count: 0,
            hwrena: 0,
            user_local: 0,
            ebase: 0x8000_0000,
            error_epc: 0,
            compare: 0,
            index: 0,
            random: (TLB_SIZE - 1) as u32,
            entry_lo0: 0,
            entry_lo1: 0,
            context: 0,
            page_mask: 0,
            wired: 0,
            entry_hi: 0,
            tlb: Tlb::new(),
            config: CONFIG_RESET,
        }
    }

    pub fn read_bad_vaddr(&self) -> u32 {
        self.bad_vaddr
    }

    pub fn read_status(&self) -> u32 {
        self.status
    }

    pub fn read_cause(&self) -> u32 {
        self.cause
    }

    pub fn read_epc(&self) -> u32 {
        self.epc
    }

    pub fn exception_level(&self) -> bool {
        self.status & (1 << 1) != 0
    }

    pub fn exception_code(&self) -> u32 {
        (self.cause >> 2) & 0x1F
    }

    pub fn branch_delay(&self) -> bool {
        self.cause & (1 << 31) != 0
    }

    pub fn enter_exception(
        &mut self,
        exception: Exception,
        pc: u32,
        in_delay_slot: bool,
        bad_address: Option<u32>,
    ) {
        if !self.exception_level() {
            self.epc = if in_delay_slot {
                pc.wrapping_sub(4)
            } else {
                pc
            };

            self.cause &= !(1 << 31);

            if in_delay_slot {
                self.cause |= 1 << 31;
            }
        }

        self.cause &= !(0x1F << 2);
        self.cause |= exception.code() << 2;

        // CP0 Unusable Exception의 Coprocessor 번호
        if exception == Exception::CoprocessorUnusable {
            self.cause &= !(0b11 << 28);
        }

        self.status |= 1 << 1;

        if let Some(address) = bad_address {
            self.bad_vaddr = address;

            if exception.is_tlb_exception() {
                // Context.BadVPN2 갱신
                self.context = (self.context & 0xFF80_0000) | ((address >> 9) & 0x007F_FFF0);

                // EntryHi.VPN2 갱신, ASID 보존
                self.entry_hi = (address & 0xFFFF_E000) | (self.entry_hi & 0xFF);
            }
        }
    }

    pub fn tick(&mut self) {
        self.count = self.count.wrapping_add(1);

        if self.count == self.compare {
            // Timer Interrupt Pending
            self.cause |= (1 << 30) | (1 << 15);
        }

        // TLB Random Index 순환
        if self.random <= self.wired {
            self.random = (TLB_SIZE - 1) as u32;
        } else {
            self.random -= 1;
        }
    }

    pub fn read_count(&self) -> u32 {
        self.count
    }

    pub fn read_hwrena(&self) -> u32 {
        self.hwrena
    }

    pub fn write_hwrena(&mut self, value: u32) {
        self.hwrena = value & 0x2000000F;
    }

    pub fn read_user_local(&self) -> u32 {
        self.user_local
    }

    pub fn write_user_local(&mut self, value: u32) {
        self.user_local = value;
    }

    pub fn read_reg(&self, rd: u8, sel: u8) -> Option<u32> {
        match (rd, sel) {
            (7, 0) => Some(self.hwrena),
            (8, 0) => Some(self.bad_vaddr),
            (9, 0) => Some(self.count),
            (12, 0) => Some(self.status),
            (13, 0) => Some(self.cause),
            (14, 0) => Some(self.epc),
            (4, 2) => Some(self.user_local),
            (15, 1) => Some(self.ebase),
            (30, 0) => Some(self.error_epc),
            (11, 0) => Some(self.compare),

            // IntCtl.IPTI = 7
            (12, 1) => Some(7u32 << 29),

            (0, 0) => Some(self.index),
            (1, 0) => Some(self.random),
            (2, 0) => Some(self.entry_lo0),
            (3, 0) => Some(self.entry_lo1),
            (4, 0) => Some(self.context),
            (5, 0) => Some(self.page_mask),
            (6, 0) => Some(self.wired),
            (10, 0) => Some(self.entry_hi),

            (16, 0) => Some(self.config),
            (16, 1) => Some(CONFIG1),

            _ => None,
        }
    }

    pub fn write_reg(&mut self, rd: u8, sel: u8, value: u32) -> bool {
        match (rd, sel) {
            (7, 0) => self.write_hwrena(value),

            (8, 0) => {
                // Read-only Register
            }

            (9, 0) => self.count = value,

            (12, 0) => {
                // 기본 System Control 비트만 허용
                const MASK: u32 = 0x1040_FF1F;

                self.status = (self.status & !MASK) | (value & MASK);
            }

            (13, 0) => {
                // Software Interrupt Pending
                const MASK: u32 = 0x0080_0300;

                self.cause = (self.cause & !MASK) | (value & MASK);
            }

            (14, 0) => self.epc = value,

            (4, 2) => self.write_user_local(value),
            (15, 1) => {
                // Exception Base Address 갱신
                self.ebase = 0x8000_0000 | (value & 0x3FFF_F000);
            }
            (30, 0) => self.error_epc = value,

            (11, 0) => self.write_compare(value),

            (12, 1) => {
                // 고정 Interrupt Routing
            }

            (0, 0) => {
                // Index.P 보존
                self.index = (self.index & 0x8000_0000) | (value & ((TLB_SIZE - 1) as u32));
            }

            (1, 0) => {
                // Random은 Read-only Register
            }

            (2, 0) => {
                self.entry_lo0 = value & 0x03FF_FFFF;
            }

            (3, 0) => {
                self.entry_lo1 = value & 0x03FF_FFFF;
            }

            (4, 0) => {
                // PTEBase만 소프트웨어 쓰기 허용
                const MASK: u32 = 0xFF80_0000;

                self.context = (self.context & !MASK) | (value & MASK);
            }

            (5, 0) => {
                self.page_mask = value & 0x1FFF_E000;
            }

            (6, 0) => {
                self.wired = value & ((TLB_SIZE - 1) as u32);

                // Wired 갱신 시 Random 초기화
                self.random = (TLB_SIZE - 1) as u32;
            }

            (10, 0) => {
                self.entry_hi = value & 0xFFFF_E0FF;
            }

            (16, 0) => {
                // Config.K0만 쓰기 허용
                self.config = (self.config & !7) | (value & 7);
            }

            (16, 1) => {
                // Config1은 Read-only Register
            }

            _ => return false,
        }

        true
    }

    pub fn interrupt_enabled(&self) -> bool {
        self.status & 1 != 0
    }

    pub fn set_interrupt_enabled(&mut self, enabled: bool) -> u32 {
        let old_status = self.status;

        // Status.IE 갱신
        if enabled {
            self.status |= 1;
        } else {
            self.status &= !1;
        }

        old_status
    }

    pub fn read_ebase(&self) -> u32 {
        self.ebase
    }

    pub fn bootstrap_exception_vectors(&self) -> bool {
        self.status & (1 << 22) != 0
    }

    pub fn exception_vector(&self, exception: Exception) -> u32 {
        let base = if self.bootstrap_exception_vectors() {
            0xBFC0_0200u32
        } else {
            self.ebase & 0xFFFF_F000
        };

        let offset = if exception.is_tlb_refill() && !self.exception_level() {
            0x000
        } else if exception == Exception::Interrupt && self.cause & (1 << 23) != 0 {
            0x200
        } else {
            0x180
        };

        base.wrapping_add(offset)
    }

    pub fn read_error_epc(&self) -> u32 {
        self.error_epc
    }

    pub fn error_level(&self) -> bool {
        self.status & (1 << 2) != 0
    }

    pub fn return_from_exception(&mut self) -> u32 {
        if self.error_level() {
            // Error Level 해제
            self.status &= !(1 << 2);
            self.error_epc
        } else {
            // Exception Level 해제
            self.status &= !(1 << 1);
            self.epc
        }
    }

    pub fn privilege_mode(&self) -> PrivilegeMode {
        if self.exception_level() || self.error_level() {
            return PrivilegeMode::Kernel;
        }

        match (self.status >> 3) & 0b11 {
            0b00 => PrivilegeMode::Kernel,
            0b01 => PrivilegeMode::Supervisor,
            0b10 => PrivilegeMode::User,
            _ => PrivilegeMode::Reserved,
        }
    }

    pub fn is_kernel_mode(&self) -> bool {
        self.privilege_mode() == PrivilegeMode::Kernel
    }

    pub fn cp0_accessible(&self) -> bool {
        self.is_kernel_mode() || self.status & (1 << 28) != 0
    }

    pub fn hardware_register_accessible(&self, rd: u8) -> bool {
        self.is_kernel_mode() || (rd < 32 && self.hwrena & (1u32 << rd) != 0)
    }

    pub fn read_compare(&self) -> u32 {
        self.compare
    }

    pub fn write_compare(&mut self, value: u32) {
        self.compare = value;

        // Timer Interrupt 해제
        self.cause &= !((1 << 30) | (1 << 15));
    }

    pub fn timer_interrupt_pending(&self) -> bool {
        self.cause & (1 << 30) != 0
    }

    pub fn interrupt_mask(&self) -> u32 {
        (self.status >> 8) & 0xFF
    }

    pub fn interrupt_pending(&self) -> u32 {
        (self.cause >> 8) & 0xFF
    }

    pub fn interrupt_requested(&self) -> bool {
        self.interrupt_enabled()
            && !self.exception_level()
            && !self.error_level()
            && (self.interrupt_mask() & self.interrupt_pending()) != 0
    }

    pub fn read_index(&self) -> u32 {
        self.index
    }

    pub fn read_random(&self) -> u32 {
        self.random
    }

    pub fn read_entry_lo0(&self) -> u32 {
        self.entry_lo0
    }

    pub fn read_entry_lo1(&self) -> u32 {
        self.entry_lo1
    }

    pub fn read_context(&self) -> u32 {
        self.context
    }

    pub fn read_page_mask(&self) -> u32 {
        self.page_mask
    }

    pub fn read_wired(&self) -> u32 {
        self.wired
    }

    pub fn read_entry_hi(&self) -> u32 {
        self.entry_hi
    }

    pub fn asid(&self) -> u8 {
        (self.entry_hi & 0xFF) as u8
    }

    fn current_tlb_entry(&self) -> TlbEntry {
        let global = self.entry_lo0 & self.entry_lo1 & 1;

        TlbEntry {
            entry_hi: self.entry_hi,
            entry_lo0: (self.entry_lo0 & !1) | global,
            entry_lo1: (self.entry_lo1 & !1) | global,
            page_mask: self.page_mask,
        }
    }

    pub fn tlb_read(&mut self) {
        let index = (self.index & ((TLB_SIZE - 1) as u32)) as usize;

        let entry = self.tlb.read(index).unwrap_or_default();

        // TLB Entry를 CP0 Register로 복사
        self.entry_hi = entry.entry_hi;
        self.entry_lo0 = entry.entry_lo0;
        self.entry_lo1 = entry.entry_lo1;
        self.page_mask = entry.page_mask;
    }

    pub fn tlb_write_indexed(&mut self) {
        let index = (self.index & ((TLB_SIZE - 1) as u32)) as usize;

        let entry = self.current_tlb_entry();

        self.tlb.write(index, entry);
    }

    pub fn tlb_write_random(&mut self) {
        let index = self.random as usize;

        let entry = self.current_tlb_entry();

        self.tlb.write(index, entry);
    }

    pub fn tlb_probe(&mut self) {
        let address = self.entry_hi;
        let asid = self.asid();

        match self.tlb.probe(address, asid) {
            Some(index) => {
                // Probe 성공
                self.index = index as u32;
            }

            None => {
                // Index.P 설정
                self.index |= 1 << 31;
            }
        }
    }

    pub fn tlb(&self) -> &Tlb {
        &self.tlb
    }

    pub fn read_config(&self) -> u32 {
        self.config
    }

    pub fn read_config1(&self) -> u32 {
        CONFIG1
    }
}
