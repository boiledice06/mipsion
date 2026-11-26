use crate::exception::Exception;

pub struct Cp0 {
    bad_vaddr: u32,
    status: u32,
    cause: u32,
    epc: u32,
}

impl Cp0 {
    pub fn new() -> Self {
        Self {
            bad_vaddr: 0,
            status: 0,
            cause: 0,
            epc: 0,
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

        self.status |= 1 << 1;

        if let Some(address) = bad_address {
            self.bad_vaddr = address;
        }
    }
}
