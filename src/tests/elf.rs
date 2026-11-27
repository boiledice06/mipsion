use super::*;

#[test]
fn elf_header_valid() {
    let bytes = elf_fixture();

    let header = crate::elf::parse_header(&bytes).unwrap();

    assert_eq!(header.entry, 0x1000);
    assert_eq!(header.phoff, 52);
    assert_eq!(header.phentsize, 32);
    assert_eq!(header.phnum, 1);
}

#[test]
fn elf_header_truncated() {
    use crate::elf::{ElfError, parse_header};

    for size in 0..52 {
        assert_eq!(parse_header(&vec![0; size]), Err(ElfError::Truncated),);
    }
}

#[test]
fn elf_invalid_magic() {
    use crate::elf::{ElfError, parse_header};

    let mut bytes = elf_fixture();

    bytes[0] = 0;

    assert_eq!(parse_header(&bytes), Err(ElfError::InvalidMagic),);
}

#[test]
fn elf_unsupported_class() {
    use crate::elf::{ElfError, parse_header};

    let mut bytes = elf_fixture();

    bytes[4] = 2;

    assert_eq!(parse_header(&bytes), Err(ElfError::UnsupportedClass),);
}

#[test]
fn elf_unsupported_endian() {
    use crate::elf::{ElfError, parse_header};

    let mut bytes = elf_fixture();

    bytes[5] = 2;

    assert_eq!(parse_header(&bytes), Err(ElfError::UnsupportedEndian),);
}

#[test]
fn elf_unsupported_machine() {
    use crate::elf::{ElfError, parse_header};

    let mut bytes = elf_fixture();

    bytes[18] = 62;

    assert_eq!(parse_header(&bytes), Err(ElfError::UnsupportedMachine),);
}

#[test]
fn elf_unsupported_type() {
    use crate::elf::{ElfError, parse_header};

    let mut bytes = elf_fixture();

    bytes[16] = 3;

    assert_eq!(parse_header(&bytes), Err(ElfError::UnsupportedType),);
}

#[test]
fn elf_truncated_program_header() {
    use crate::elf::{ElfError, parse_header};

    let mut bytes = elf_fixture();

    bytes.truncate(60);

    assert_eq!(parse_header(&bytes), Err(ElfError::Truncated),);
}

#[test]
fn elf_dynamic_segment_rejected() {
    use crate::elf::{ElfError, parse_segments};

    let mut bytes = elf_fixture();

    bytes[52] = 3;

    assert_eq!(parse_segments(&bytes), Err(ElfError::DynamicExecutable),);
}

#[test]
fn elf_parse_load_segment() {
    let bytes = elf_fixture();

    let segments = crate::elf::parse_segments(&bytes).unwrap();

    assert_eq!(segments.len(), 1);
    assert_eq!(segments[0].address, 0x1000);
    assert_eq!(segments[0].offset, 0x100);
    assert_eq!(segments[0].filesz, 12);
    assert_eq!(segments[0].memsz, 32);
}

#[test]
fn elf_invalid_segment_sizes() {
    use crate::elf::{ElfError, parse_segments};

    let mut bytes = elf_fixture();

    bytes[72..76].copy_from_slice(&4u32.to_le_bytes());

    assert_eq!(parse_segments(&bytes), Err(ElfError::InvalidSegment),);
}

#[test]
fn elf_truncated_segment_data() {
    use crate::elf::{ElfError, parse_segments};

    let mut bytes = elf_fixture();

    bytes.pop();

    assert_eq!(parse_segments(&bytes), Err(ElfError::Truncated),);
}

#[test]
fn elf_segment_address_overflow() {
    use crate::elf::{ElfError, parse_segments};

    let mut bytes = elf_fixture();

    bytes[60..64].copy_from_slice(&0xFFFF_FFF0u32.to_le_bytes());

    assert_eq!(parse_segments(&bytes), Err(ElfError::AddressOverflow),);
}

#[test]
fn elf_load_program() {
    let bytes = elf_fixture();
    let mut memory = Memory::new(16384);

    let result = crate::elf::load_elf(&mut memory, &bytes).unwrap();

    assert_eq!(result.entry, 0x1000);
    assert_eq!(result.segment_count, 1);

    assert_eq!(memory.read_word(0x1000), 0x2408_002A);
    assert_eq!(memory.read_word(0x1004), 0x2509_0001);
    assert_eq!(memory.read_word(0x1008), 0x0000_000C);
}

#[test]
fn elf_zero_fills_bss() {
    let bytes = elf_fixture();
    let mut memory = Memory::new(16384);

    for address in 0x100C..0x1020 {
        memory.write_byte(address, 0xFF);
    }

    crate::elf::load_elf(&mut memory, &bytes).unwrap();

    for address in 0x100C..0x1020 {
        assert_eq!(memory.read_byte(address), 0);
    }
}

#[test]
fn elf_load_out_of_bounds() {
    use crate::elf::{ElfError, load_elf};

    let bytes = elf_fixture();
    let mut memory = Memory::new(1024);

    assert_eq!(load_elf(&mut memory, &bytes), Err(ElfError::OutOfBounds),);
}

#[test]
fn elf_invalid_entry() {
    use crate::elf::{ElfError, load_elf};

    let mut bytes = elf_fixture();

    bytes[24..28].copy_from_slice(&0x2000u32.to_le_bytes());

    let mut memory = Memory::new(16384);

    assert_eq!(load_elf(&mut memory, &bytes), Err(ElfError::InvalidEntry),);
}

#[test]
fn elf_stack_initialization() {
    let bytes = elf_fixture();
    let mut memory = Memory::new(16384);

    let result = crate::elf::load_elf(&mut memory, &bytes).unwrap();

    assert_eq!(result.stack_pointer, 0x3FF0);
    assert_eq!(result.stack_pointer & 15, 0);

    assert_eq!(memory.read_word(result.stack_pointer), 0,);
}

#[test]
fn elf_insufficient_stack() {
    use crate::elf::{ElfError, load_elf};

    let bytes = elf_fixture();
    let mut memory = Memory::new(8192);

    assert_eq!(
        load_elf(&mut memory, &bytes),
        Err(ElfError::InsufficientStack),
    );
}

#[test]
fn elf_failure_preserves_memory() {
    let mut bytes = elf_fixture();

    bytes[24..28].copy_from_slice(&0x2000u32.to_le_bytes());

    let mut memory = Memory::new(16384);

    memory.write_word(0x1000, 0xDEAD_BEEF);

    assert!(crate::elf::load_elf(&mut memory, &bytes).is_err());

    assert_eq!(memory.read_word(0x1000), 0xDEAD_BEEF,);
}

#[test]
fn elf_runtime_integration() {
    use crate::runtime::{ExecutionConfig, StopReason};

    let bytes = elf_fixture();

    let mut cpu = Cpu::new();
    let mut memory = Memory::new(16384);

    let loaded = crate::elf::load_elf(&mut memory, &bytes).unwrap();

    cpu.write_pc(loaded.entry);
    cpu.write_reg(29, loaded.stack_pointer);

    let result = crate::runtime::run(&mut cpu, &mut memory, ExecutionConfig::default());

    assert_eq!(cpu.read_reg(8), 42);
    assert_eq!(cpu.read_reg(9), 43);
    assert_eq!(cpu.read_reg(29), 0x3FF0);
    assert_eq!(result.steps, 3);

    assert_eq!(
        result.reason,
        StopReason::Exception {
            code: 8,
            epc: 0x1008,
            in_delay_slot: false,
        },
    );
}

#[test]
fn elf_trace_integration() {
    use crate::runtime::{ExecutionConfig, run_with_trace};

    let bytes = elf_fixture();

    let mut cpu = Cpu::new();
    let mut memory = Memory::new(16384);

    let loaded = crate::elf::load_elf(&mut memory, &bytes).unwrap();

    cpu.write_pc(loaded.entry);

    let mut events = Vec::new();

    run_with_trace(&mut cpu, &mut memory, ExecutionConfig::default(), |event| {
        events.push(event.clone())
    });

    assert_eq!(events.len(), 3);
    assert_eq!(events[0].pc, 0x1000);
    assert_eq!(events[0].mnemonic, Some("addiu"));
    assert_eq!(events[2].mnemonic, Some("syscall"));
}
