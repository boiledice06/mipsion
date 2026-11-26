#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum Exception {
    AddressErrorLoad = 4,
    AddressErrorStore = 5,
    Syscall = 8,
    Breakpoint = 9,
    ReservedInstruction = 10,
    CoprocessorUnusable = 11,
    Overflow = 12,
    Trap = 13,
}

impl Exception {
    pub fn code(self) -> u32 {
        self as u32
    }
}
