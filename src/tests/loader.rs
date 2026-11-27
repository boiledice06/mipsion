use super::*;

#[test]
fn loader_raw_single_instruction() {
    use crate::loader::load_raw;

    let mut memory = Memory::new(64);
    let bytes = 0x2408_002Au32.to_le_bytes();

    assert_eq!(load_raw(&mut memory, 0, &bytes), Ok(()));
    assert_eq!(memory.read_word(0), 0x2408_002A);
}

#[test]
fn loader_raw_multiple_instructions() {
    use crate::loader::load_raw;

    let mut memory = Memory::new(64);

    let bytes = [0x2A, 0x00, 0x08, 0x24, 0x01, 0x00, 0x09, 0x25];

    assert_eq!(load_raw(&mut memory, 0, &bytes), Ok(()));

    assert_eq!(memory.read_word(0), 0x2408_002A);
    assert_eq!(memory.read_word(4), 0x2509_0001);
}

#[test]
fn loader_raw_nonzero_address() {
    use crate::loader::load_raw;

    let mut memory = Memory::new(64);
    let bytes = 0x2408_002Au32.to_le_bytes();

    assert_eq!(load_raw(&mut memory, 16, &bytes), Ok(()));

    assert_eq!(memory.read_word(0), 0);
    assert_eq!(memory.read_word(16), 0x2408_002A);
}

#[test]
fn loader_raw_empty_program() {
    use crate::loader::{LoadError, load_raw};

    let mut memory = Memory::new(64);

    assert_eq!(load_raw(&mut memory, 0, &[]), Err(LoadError::EmptyProgram));
}

#[test]
fn loader_raw_unaligned_address() {
    use crate::loader::{LoadError, load_raw};

    let mut memory = Memory::new(64);

    assert_eq!(
        load_raw(&mut memory, 2, &[0; 4]),
        Err(LoadError::UnalignedAddress)
    );
}

#[test]
fn loader_raw_invalid_size() {
    use crate::loader::{LoadError, load_raw};

    let mut memory = Memory::new(64);

    assert_eq!(
        load_raw(&mut memory, 0, &[0; 3]),
        Err(LoadError::InvalidProgramSize)
    );
}

#[test]
fn loader_raw_out_of_bounds() {
    use crate::loader::{LoadError, load_raw};

    let mut memory = Memory::new(64);

    assert_eq!(
        load_raw(&mut memory, 60, &[0; 8]),
        Err(LoadError::OutOfBounds)
    );
}

#[test]
fn loader_raw_address_overflow() {
    use crate::loader::{LoadError, load_raw};

    let mut memory = Memory::new(64);

    assert_eq!(
        load_raw(&mut memory, 0xFFFF_FFFC, &[0; 8]),
        Err(LoadError::AddressOverflow)
    );
}

#[test]
fn loader_raw_exact_boundary() {
    use crate::loader::load_raw;

    let mut memory = Memory::new(64);
    let bytes = 0x1234_5678u32.to_le_bytes();

    assert_eq!(load_raw(&mut memory, 60, &bytes), Ok(()));
    assert_eq!(memory.read_word(60), 0x1234_5678);
}

#[test]
fn loader_raw_failure_preserves_memory() {
    use crate::loader::{LoadError, load_raw};

    let mut memory = Memory::new(64);
    memory.write_word(60, 0x1234_5678);

    assert_eq!(
        load_raw(&mut memory, 60, &[0xFF; 8]),
        Err(LoadError::OutOfBounds)
    );

    assert_eq!(memory.read_word(60), 0x1234_5678);
}

#[test]
fn loader_raw_execute_program() {
    use crate::loader::load_raw;

    let mut cpu = Cpu::new();
    let mut memory = Memory::new(64);

    let bytes = [0x2A, 0x00, 0x08, 0x24, 0x01, 0x00, 0x09, 0x25];

    load_raw(&mut memory, 16, &bytes).unwrap();

    cpu.write_pc(16);

    cpu.step(&mut memory);
    cpu.step(&mut memory);

    assert_eq!(cpu.read_reg(8), 42);
    assert_eq!(cpu.read_reg(9), 43);
    assert_eq!(cpu.read_pc(), 24);
}
