use super::*;

#[test]
fn exception_state() {
    use crate::exception::Exception;

    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    memory.write_word(0, 0x00000000);
    cpu.step(&mut memory);

    cpu.raise_exception(Exception::Overflow, None);

    assert_eq!(cpu.cp0().exception_code(), 12);
    assert_eq!(cpu.cp0().read_epc(), 0);
    assert!(cpu.cp0().exception_level());
    assert!(!cpu.cp0().branch_delay());
    assert!(!cpu.is_halted());
    assert_eq!(cpu.read_pc(), 0x8000_0180);
}

#[test]
fn delay_slot_exception() {
    use crate::exception::Exception;

    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    memory.write_word(0, i(4, 0, 0, 3));
    memory.write_word(4, 0x00000000);

    cpu.step(&mut memory);
    cpu.step(&mut memory);

    cpu.raise_exception(Exception::Overflow, None);

    assert_eq!(cpu.cp0().exception_code(), 12);
    assert_eq!(cpu.cp0().read_epc(), 0);
    assert!(cpu.cp0().branch_delay());
}

#[test]
fn nested_exception() {
    use crate::cp0::Cp0;
    use crate::exception::Exception;

    let mut cp0 = Cp0::new();

    cp0.enter_exception(Exception::Overflow, 0x104, true, None);

    cp0.enter_exception(Exception::AddressErrorLoad, 0x200, false, Some(0x123));

    assert_eq!(cp0.read_epc(), 0x100);
    assert!(cp0.branch_delay());
    assert_eq!(cp0.exception_code(), 4);
    assert_eq!(cp0.read_bad_vaddr(), 0x123);
}

#[test]
fn arithmetic_overflow() {
    for (word, a, b) in [
        (encode_r(0, 8, 9, 10, 0, 0x20), 0x7FFFFFFF, 1),
        (encode_r(0, 8, 9, 10, 0, 0x22), 0x80000000, 1),
        (i(8, 8, 10, 1), 0x7FFFFFFF, 0),
        (i(8, 8, 10, 0xFFFF), 0x80000000, 0),
    ] {
        let cpu = run(&[word], &[(8, a), (9, b), (10, 0xDEADBEEF)]);

        assert!(!cpu.is_halted());
        assert_eq!(cpu.read_pc(), 0x8000_0180);
        assert_eq!(cpu.cp0().exception_code(), 12);
        assert_eq!(cpu.cp0().read_epc(), 0);
        assert_eq!(cpu.read_reg(10), 0xDEADBEEF);
    }
}

#[test]
fn system_exception() {
    for (word, code) in [(0x0000000C, 8), (0x0000000D, 9)] {
        let cpu = run(&[word], &[]);

        assert!(!cpu.is_halted());
        assert_eq!(cpu.read_pc(), 0x8000_0180);
        assert_eq!(cpu.cp0().exception_code(), code);
        assert!(cpu.cp0().exception_level());
        assert_eq!(cpu.cp0().read_epc(), 0);
    }
}

#[test]
fn register_trap() {
    for (funct, a, b, expected) in [
        (0x34, 10, 10, true),
        (0x34, 10, 20, false),
        (0x36, 10, 20, true),
        (0x36, 10, 10, false),
        (0x30, 10, 10, true),
        (0x30, u32::MAX, 0, false),
        (0x31, u32::MAX, 0, true),
        (0x31, 0, 1, false),
        (0x32, u32::MAX, 0, true),
        (0x32, 10, 10, false),
        (0x33, 0, u32::MAX, true),
        (0x33, u32::MAX, 0, false),
    ] {
        let cpu = run(&[encode_r(0, 8, 9, 0, 0, funct)], &[(8, a), (9, b)]);

        assert!(!cpu.is_halted());

        if expected {
            assert_eq!(cpu.read_pc(), 0x8000_0180);
        } else {
            assert_eq!(cpu.read_pc(), 4);
        }
        assert_eq!(cpu.cp0().exception_code(), if expected { 13 } else { 0 });
    }
}

#[test]
fn immediate_trap() {
    for (rt, a, imm, expected) in [
        (0x0C, 10, 10, true),
        (0x0C, 10, 20, false),
        (0x0E, 10, 20, true),
        (0x0E, 10, 10, false),
        (0x08, 0, 0xFFFF, true),
        (0x08, u32::MAX, 0, false),
        (0x09, u32::MAX, 0xFFFF, true),
        (0x09, 0x00010000, 0xFFFF, false),
        (0x0A, u32::MAX, 0, true),
        (0x0A, 10, 10, false),
        (0x0B, 0x00010000, 0xFFFF, true),
        (0x0B, u32::MAX, 0xFFFF, false),
    ] {
        let cpu = run(&[i(1, 8, rt, imm)], &[(8, a)]);

        assert!(!cpu.is_halted());

        if expected {
            assert_eq!(cpu.read_pc(), 0x8000_0180);
        } else {
            assert_eq!(cpu.read_pc(), 4);
        }
        assert_eq!(cpu.cp0().exception_code(), if expected { 13 } else { 0 });
    }
}

#[test]
fn arithmetic_delay_slot_exception() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.write_reg(8, 0x7FFFFFFF);
    cpu.write_reg(9, 1);
    cpu.write_reg(10, 123);

    memory.write_word(0, i(4, 0, 0, 3));
    memory.write_word(4, encode_r(0, 8, 9, 10, 0, 0x20));

    cpu.step(&mut memory);
    cpu.step(&mut memory);

    assert!(!cpu.is_halted());
    assert_eq!(cpu.read_pc(), 0x8000_0180);
    assert!(cpu.cp0().branch_delay());
    assert_eq!(cpu.cp0().read_epc(), 0);
    assert_eq!(cpu.cp0().exception_code(), 12);
    assert_eq!(cpu.read_reg(10), 123);
}

#[test]
fn decoded_unsupported_wait_raises_reserved_instruction_at_guest_pc() {
    let cpu = run(&[0x4200_0020], &[]);
    assert_eq!(cpu.cp0().exception_code(), 10);
    assert_eq!(cpu.cp0().read_epc(), 0);
    assert!(!cpu.cp0().branch_delay());
    assert_eq!(cpu.read_pc(), 0x8000_0180);
}

#[test]
fn decoded_unsupported_wait_in_delay_slot_preserves_branch_epc() {
    let cpu = run(&[0x1000_0002, 0x4200_0020], &[]);
    assert_eq!(cpu.cp0().exception_code(), 10);
    assert_eq!(cpu.cp0().read_epc(), 0);
    assert!(cpu.cp0().branch_delay());
}
