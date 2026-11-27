#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exception {
    Interrupt,
    TlbModified,
    TlbRefillLoad,
    TlbRefillStore,
    TlbInvalidLoad,
    TlbInvalidStore,
    AddressErrorLoad,
    AddressErrorStore,
    Syscall,
    Breakpoint,
    ReservedInstruction,
    CoprocessorUnusable(u8),
    Overflow,
    Trap,
    FloatingPoint,
    MachineCheck,
    InstructionBusError,
    DataBusError,
}

impl Exception {
    pub fn code(self) -> u32 {
        match self {
            Self::Interrupt => 0,
            Self::TlbModified => 1,
            Self::TlbRefillLoad | Self::TlbInvalidLoad => 2,
            Self::TlbRefillStore | Self::TlbInvalidStore => 3,
            Self::AddressErrorLoad => 4,
            Self::AddressErrorStore => 5,
            Self::Syscall => 8,
            Self::Breakpoint => 9,
            Self::ReservedInstruction => 10,
            Self::CoprocessorUnusable(_) => 11,
            Self::Overflow => 12,
            Self::Trap => 13,
            Self::FloatingPoint => 15,
            Self::MachineCheck => 24,
            Self::InstructionBusError => 6,
            Self::DataBusError => 7,
        }
    }

    pub fn is_tlb_refill(self) -> bool {
        matches!(self, Self::TlbRefillLoad | Self::TlbRefillStore)
    }

    pub fn is_tlb_exception(self) -> bool {
        matches!(
            self,
            Self::TlbModified
                | Self::TlbRefillLoad
                | Self::TlbRefillStore
                | Self::TlbInvalidLoad
                | Self::TlbInvalidStore
        )
    }
}
