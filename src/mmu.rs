use crate::cp0::PrivilegeMode;
use crate::exception::Exception;

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
}
