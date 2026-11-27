use crate::cp0::PrivilegeMode;
use crate::exception::Exception;

use crate::cp0::Cp0;
use crate::tlb::TlbFault;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranslationMode {
    Bare,
    Tlb,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessType {
    Fetch,
    Load,
    Store,
}

pub struct Mmu;

impl Mmu {
    fn address_error(access: AccessType) -> Exception {
        match access {
            AccessType::Fetch | AccessType::Load => Exception::AddressErrorLoad,

            AccessType::Store => Exception::AddressErrorStore,
        }
    }

    pub fn translate(
        address: u32,
        mode: PrivilegeMode,
        access: AccessType,
    ) -> Result<u32, Exception> {
        let allowed = match mode {
            PrivilegeMode::Kernel => true,

            PrivilegeMode::Supervisor => {
                matches!(
                    address,
                    0x0000_0000..=0x7FFF_FFFF
                        | 0xC000_0000..=0xDFFF_FFFF
                )
            }

            PrivilegeMode::User => address <= 0x7FFF_FFFF,

            PrivilegeMode::Reserved => false,
        };

        if !allowed {
            return Err(Self::address_error(access));
        }

        let physical = match address {
            // KSEG0 / KSEG1 Direct Mapping
            0x8000_0000..=0xBFFF_FFFF => address & 0x1FFF_FFFF,

            // TLB 구현 전 임시 Identity Mapping
            _ => address,
        };

        Ok(physical)
    }

    pub fn translate_with_tlb(
        address: u32,
        cp0: &Cp0,
        access: AccessType,
    ) -> Result<u32, Exception> {
        // Segment 접근 권한 검사
        let physical = Self::translate(address, cp0.privilege_mode(), access)?;

        // KSEG0 / KSEG1 Direct Mapping
        if matches!(address, 0x8000_0000..=0xBFFF_FFFF) {
            return Ok(physical);
        }

        // ERL 상태의 KUSEG는 Direct Mapping
        if cp0.error_level() && address <= 0x7FFF_FFFF {
            return Ok(address);
        }

        let store = access == AccessType::Store;

        cp0.tlb()
            .translate(address, cp0.asid(), store)
            .map_err(|fault| match fault {
                TlbFault::Miss => {
                    if store {
                        Exception::TlbRefillStore
                    } else {
                        Exception::TlbRefillLoad
                    }
                }

                TlbFault::Invalid => {
                    if store {
                        Exception::TlbInvalidStore
                    } else {
                        Exception::TlbInvalidLoad
                    }
                }

                TlbFault::Modified => Exception::TlbModified,

                TlbFault::InvalidPageMask => Exception::MachineCheck,
            })
    }
}
