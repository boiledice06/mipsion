use crate::memory::Memory;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ElfError {
    Truncated,
    InvalidMagic,
    UnsupportedClass,
    UnsupportedEndian,
    UnsupportedVersion,
    UnsupportedType,
    UnsupportedMachine,
    InvalidHeader,
    DynamicExecutable,
    InvalidSegment,
    AddressOverflow,
    OutOfBounds,
    OverlappingSegments,
    NoLoadableSegments,
    InvalidEntry,
    InsufficientStack,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ElfHeader {
    pub entry: u32,
    pub phoff: u32,
    pub phentsize: u16,
    pub phnum: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoadSegment {
    pub offset: u32,
    pub address: u32,
    pub filesz: u32,
    pub memsz: u32,
    pub flags: u32,
    pub align: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ElfLoadResult {
    pub entry: u32,
    pub stack_pointer: u32,
    pub segment_count: usize,
}

fn read_u16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(bytes[offset..offset + 2].try_into().unwrap())
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

pub fn parse_header(bytes: &[u8]) -> Result<ElfHeader, ElfError> {
    if bytes.len() < 52 {
        return Err(ElfError::Truncated);
    }

    if &bytes[..4] != b"\x7FELF" {
        return Err(ElfError::InvalidMagic);
    }

    if bytes[4] != 1 {
        return Err(ElfError::UnsupportedClass);
    }

    if bytes[5] != 1 {
        return Err(ElfError::UnsupportedEndian);
    }

    if bytes[6] != 1 || read_u32(bytes, 20) != 1 {
        return Err(ElfError::UnsupportedVersion);
    }

    // ET_EXEC
    if read_u16(bytes, 16) != 2 {
        return Err(ElfError::UnsupportedType);
    }

    // EM_MIPS
    if read_u16(bytes, 18) != 8 {
        return Err(ElfError::UnsupportedMachine);
    }

    let header = ElfHeader {
        entry: read_u32(bytes, 24),
        phoff: read_u32(bytes, 28),
        phentsize: read_u16(bytes, 42),
        phnum: read_u16(bytes, 44),
    };

    if read_u16(bytes, 40) != 52
        || header.phentsize != 32
        || header.phnum == 0
        || header.phnum == u16::MAX
        || header.phoff < 52
    {
        return Err(ElfError::InvalidHeader);
    }

    let table_size = usize::from(header.phnum) * 32;

    let table_end = (header.phoff as usize)
        .checked_add(table_size)
        .ok_or(ElfError::AddressOverflow)?;

    if table_end > bytes.len() {
        return Err(ElfError::Truncated);
    }

    if header.entry & 3 != 0 {
        return Err(ElfError::InvalidEntry);
    }

    Ok(header)
}

pub fn parse_segments(bytes: &[u8]) -> Result<Vec<LoadSegment>, ElfError> {
    let header = parse_header(bytes)?;
    let mut segments = Vec::new();

    for index in 0..usize::from(header.phnum) {
        let base = header.phoff as usize + index * 32;

        let kind = read_u32(bytes, base);

        // PT_DYNAMIC / PT_INTERP
        if matches!(kind, 2 | 3) {
            return Err(ElfError::DynamicExecutable);
        }

        // PT_LOAD
        if kind != 1 {
            continue;
        }

        let segment = LoadSegment {
            offset: read_u32(bytes, base + 4),
            address: read_u32(bytes, base + 8),
            filesz: read_u32(bytes, base + 16),
            memsz: read_u32(bytes, base + 20),
            flags: read_u32(bytes, base + 24),
            align: read_u32(bytes, base + 28),
        };

        if segment.filesz > segment.memsz {
            return Err(ElfError::InvalidSegment);
        }

        if segment.align > 1
            && (!segment.align.is_power_of_two()
                || segment.address % segment.align != segment.offset % segment.align)
        {
            return Err(ElfError::InvalidSegment);
        }

        let file_end = (segment.offset as usize)
            .checked_add(segment.filesz as usize)
            .ok_or(ElfError::AddressOverflow)?;

        if file_end > bytes.len() {
            return Err(ElfError::Truncated);
        }

        if segment.memsz == 0 {
            continue;
        }

        segment
            .address
            .checked_add(segment.memsz)
            .ok_or(ElfError::AddressOverflow)?;

        segments.push(segment);
    }

    if segments.is_empty() {
        return Err(ElfError::NoLoadableSegments);
    }

    segments.sort_unstable_by_key(|segment| segment.address);

    for pair in segments.windows(2) {
        let end = pair[0].address + pair[0].memsz;

        if end > pair[1].address {
            return Err(ElfError::OverlappingSegments);
        }
    }

    Ok(segments)
}

pub fn load_elf(memory: &mut Memory, bytes: &[u8]) -> Result<ElfLoadResult, ElfError> {
    const STACK_SIZE: u32 = 4096;

    let header = parse_header(bytes)?;
    let segments = parse_segments(bytes)?;

    // Validate all segments before modifying memory.
    for segment in &segments {
        if !memory.contains_range(segment.address, segment.memsz as usize) {
            return Err(ElfError::OutOfBounds);
        }
    }

    // Entry must point to file-backed executable code.
    let valid_entry = segments.iter().any(|segment| {
        segment.flags & 1 != 0
            && header.entry >= segment.address
            && u64::from(header.entry) + 4 <= u64::from(segment.address) + u64::from(segment.filesz)
    });

    if !valid_entry {
        return Err(ElfError::InvalidEntry);
    }

    let memory_end = u32::try_from(memory.size()).map_err(|_| ElfError::AddressOverflow)?;

    // Restrict ELF loading to low, identity-mapped addresses.
    if memory_end > 0x8000_0000 {
        return Err(ElfError::OutOfBounds);
    }

    let stack_top = memory_end & !15;

    let stack_bottom = stack_top
        .checked_sub(STACK_SIZE)
        .ok_or(ElfError::InsufficientStack)?;

    let image_end = segments
        .iter()
        .map(|segment| segment.address + segment.memsz)
        .max()
        .unwrap();

    if image_end > stack_bottom {
        return Err(ElfError::InsufficientStack);
    }

    // Copy initialized data and zero-fill BSS.
    for segment in &segments {
        for offset in 0..segment.filesz {
            memory.write_byte(
                segment.address + offset,
                bytes[(segment.offset + offset) as usize],
            );
        }

        for offset in segment.filesz..segment.memsz {
            memory.write_byte(segment.address + offset, 0);
        }
    }

    // Initialize the reserved stack area.
    for address in stack_bottom..stack_top {
        memory.write_byte(address, 0);
    }

    Ok(ElfLoadResult {
        entry: header.entry,
        stack_pointer: stack_top - 16,
        segment_count: segments.len(),
    })
}
