use crate::memory::Memory;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadError {
    EmptyProgram,
    UnalignedAddress,
    InvalidProgramSize,
    AddressOverflow,
    OutOfBounds,
}

pub fn load_raw(memory: &mut Memory, address: u32, bytes: &[u8]) -> Result<(), LoadError> {
    if bytes.is_empty() {
        return Err(LoadError::EmptyProgram);
    }

    if address & 3 != 0 {
        return Err(LoadError::UnalignedAddress);
    }

    if !bytes.len().is_multiple_of(4) {
        return Err(LoadError::InvalidProgramSize);
    }

    let size = u32::try_from(bytes.len()).map_err(|_| LoadError::AddressOverflow)?;

    address
        .checked_add(size - 1)
        .ok_or(LoadError::AddressOverflow)?;

    let start = address as usize;

    let end = start
        .checked_add(bytes.len())
        .ok_or(LoadError::AddressOverflow)?;

    if end > memory.size() {
        return Err(LoadError::OutOfBounds);
    }

    for (offset, &byte) in bytes.iter().enumerate() {
        memory.write_byte(address + offset as u32, byte);
    }

    Ok(())
}
