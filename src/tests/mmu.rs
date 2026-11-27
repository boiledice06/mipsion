use super::*;

#[test]
fn mmu_direct_mapping() {
    use crate::cp0::PrivilegeMode::Kernel;
    use crate::mmu::{AccessType, Mmu};

    for (virtual_addr, physical_addr) in [
        (0x8000_0000, 0),
        (0xA000_0000, 0),
        (0x8000_1234, 0x1234),
        (0xA000_1234, 0x1234),
        (0x9FFF_FFFF, 0x1FFF_FFFF),
        (0xBFFF_FFFF, 0x1FFF_FFFF),
    ] {
        assert_eq!(
            Mmu::translate(virtual_addr, Kernel, AccessType::Load,),
            Ok(physical_addr)
        );
    }
}

#[test]
fn mmu_segment_permissions() {
    use crate::cp0::PrivilegeMode::*;
    use crate::mmu::{AccessType, Mmu};

    for (mode, address, allowed) in [
        (Kernel, 0x8000_0000, true),
        (Kernel, 0xE000_0000, true),
        (Supervisor, 0x0000_1000, true),
        (Supervisor, 0x8000_0000, false),
        (Supervisor, 0xA000_0000, false),
        (Supervisor, 0xC000_0000, true),
        (Supervisor, 0xE000_0000, false),
        (User, 0x7FFF_FFFF, true),
        (User, 0x8000_0000, false),
        (User, 0xA000_0000, false),
        (User, 0xC000_0000, false),
        (User, 0xE000_0000, false),
    ] {
        assert_eq!(
            Mmu::translate(address, mode, AccessType::Load,).is_ok(),
            allowed
        );
    }
}

#[test]
fn user_kernel_fetch_denied() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.cp0_mut().write_reg(12, 0, 0x10);
    cpu.write_pc(0x8000_0100);

    memory.write_word(0x100, i(9, 0, 8, 100));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp0().exception_code(), 4);
    assert_eq!(cpu.cp0().read_bad_vaddr(), 0x8000_0100);

    assert_eq!(cpu.cp0().read_epc(), 0x8000_0100);

    assert_eq!(cpu.read_pc(), 0x8000_0180);
    assert_eq!(cpu.read_reg(8), 0);
}

#[test]
fn user_kernel_load_denied() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.cp0_mut().write_reg(12, 0, 0x10);

    cpu.write_reg(8, 0x8000_0100);
    cpu.write_reg(9, 0xDEAD_BEEF);

    memory.write_word(0, i(0x23, 8, 9, 0));
    memory.write_word(0x100, 100);

    cpu.step(&mut memory);

    assert_eq!(cpu.cp0().exception_code(), 4);
    assert_eq!(cpu.cp0().read_bad_vaddr(), 0x8000_0100);

    assert_eq!(cpu.read_reg(9), 0xDEAD_BEEF);
    assert_eq!(cpu.read_pc(), 0x8000_0180);
}

#[test]
fn user_kernel_store_denied() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.cp0_mut().write_reg(12, 0, 0x10);

    cpu.write_reg(8, 0xA000_0100);
    cpu.write_reg(9, 0xDEAD_BEEF);

    memory.write_word(0, i(0x2B, 8, 9, 0));
    memory.write_word(0x100, 100);

    cpu.step(&mut memory);

    assert_eq!(cpu.cp0().exception_code(), 5);
    assert_eq!(cpu.cp0().read_bad_vaddr(), 0xA000_0100);

    assert_eq!(memory.read_word(0x100), 100);
    assert_eq!(cpu.read_pc(), 0x8000_0180);
}

#[test]
fn kernel_segment_load_store() {
    let cpu = run_kernel_alias_program();

    assert_eq!(cpu.read_reg(10), 0xDEAD_BEEF);
    assert!(!cpu.cp0().exception_level());
}

#[test]
fn atomic_segment_alias() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.write_reg(8, 0x8000_0100);
    cpu.write_reg(11, 0xA000_0100);

    memory.write_word(0x100, 10);

    memory.write_word(0, i(0x30, 8, 9, 0));
    memory.write_word(4, i(9, 9, 9, 1));
    memory.write_word(8, i(0x38, 11, 9, 0));

    cpu.step(&mut memory);

    assert!(cpu.check_reservation(0x100));
    assert_eq!(cpu.read_reg(9), 10);

    cpu.step(&mut memory);
    cpu.step(&mut memory);

    assert_eq!(memory.read_word(0x100), 11);
    assert_eq!(cpu.read_reg(9), 1);
    assert!(!cpu.check_reservation(0x100));
}

#[test]
fn mmu_delay_slot_exception() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.cp0_mut().write_reg(12, 0, 0x10);

    cpu.write_reg(8, 0x8000_0100);
    cpu.write_reg(9, 0xDEAD_BEEF);

    memory.write_word(0, i(4, 0, 0, 3));
    memory.write_word(4, i(0x23, 8, 9, 0));

    cpu.step(&mut memory);
    cpu.step(&mut memory);

    assert_eq!(cpu.cp0().exception_code(), 4);
    assert!(cpu.cp0().branch_delay());
    assert_eq!(cpu.cp0().read_epc(), 0);

    assert_eq!(cpu.cp0().read_bad_vaddr(), 0x8000_0100);

    assert_eq!(cpu.read_reg(9), 0xDEAD_BEEF);
    assert_eq!(cpu.read_pc(), 0x8000_0180);
}
