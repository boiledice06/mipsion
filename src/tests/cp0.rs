use super::*;

#[test]
fn cp0_register_transfer() {
    let cpu = run(
        &[
            encode_cp0(4, 8, 12, 0),
            encode_cp0(0, 9, 12, 0),
            encode_cp0(4, 10, 14, 0),
            encode_cp0(0, 11, 14, 0),
        ],
        &[(8, 0x0000_FF01), (10, 0x8000_1000)],
    );

    assert_eq!(cpu.read_reg(9), 0x0000_FF01);
    assert_eq!(cpu.read_reg(11), 0x8000_1000);
    assert!(!cpu.is_halted());
}

#[test]
fn cp0_write_mask() {
    let cpu = run(
        &[
            encode_cp0(4, 8, 12, 0),
            encode_cp0(0, 9, 12, 0),
            encode_cp0(4, 8, 13, 0),
            encode_cp0(0, 10, 13, 0),
            encode_cp0(4, 8, 7, 0),
            encode_cp0(0, 11, 7, 0),
        ],
        &[(8, u32::MAX)],
    );

    assert_eq!(cpu.read_reg(9), 0x3040_FF1F);
    assert_eq!(cpu.read_reg(10), 0x0080_0300);
    assert_eq!(cpu.read_reg(11), 0x2000_000F);
    assert!(!cpu.is_halted());
}

#[test]
fn cp0_read_only_register() {
    let cpu = run(
        &[encode_cp0(4, 8, 8, 0), encode_cp0(0, 9, 8, 0)],
        &[(8, 0xDEADBEEF)],
    );

    assert_eq!(cpu.read_reg(9), 0);
    assert_eq!(cpu.cp0().read_bad_vaddr(), 0);
    assert!(!cpu.is_halted());
}

#[test]
fn cp0_user_local_transfer() {
    let cpu = run(
        &[
            encode_cp0(4, 8, 4, 2),
            encode_cp0(0, 9, 4, 2),
            encode_r(0x1F, 0, 10, 29, 0, 0x3B),
        ],
        &[(8, 0x1234_5678)],
    );

    assert_eq!(cpu.read_reg(9), 0x1234_5678);
    assert_eq!(cpu.read_reg(10), 0x1234_5678);
    assert!(!cpu.is_halted());
}

#[test]
fn cp0_count_transfer() {
    let cpu = run(
        &[encode_cp0(4, 8, 9, 0), encode_cp0(0, 9, 9, 0)],
        &[(8, 100)],
    );

    assert_eq!(cpu.read_reg(9), 101);
    assert_eq!(cpu.cp0().read_count(), 101);
    assert!(!cpu.is_halted());
}

#[test]
fn invalid_cp0_register() {
    for word in [encode_cp0(0, 8, 31, 7), encode_cp0(4, 8, 31, 7)] {
        let cpu = run(&[word], &[(8, 0xDEADBEEF)]);

        assert!(!cpu.is_halted());
        assert_eq!(cpu.read_pc(), 0x8000_0180);
        assert_eq!(cpu.cp0().exception_code(), 10);
        assert_eq!(cpu.cp0().read_epc(), 0);
        assert_eq!(cpu.read_reg(8), 0xDEADBEEF);
    }
}

#[test]
fn enable_interrupt() {
    let cpu = run(
        &[
            encode_cp0(4, 8, 12, 0),
            0x41696020, // ei $t1
        ],
        &[(8, 0x0000_FF00)],
    );

    assert_eq!(cpu.read_reg(9), 0x0000_FF00);
    assert_eq!(cpu.cp0().read_status(), 0x0000_FF01);
    assert!(cpu.cp0().interrupt_enabled());
    assert!(!cpu.is_halted());
}

#[test]
fn disable_interrupt() {
    let cpu = run(
        &[
            encode_cp0(4, 8, 12, 0),
            0x41696000, // di $t1
        ],
        &[(8, 0x0000_FF01)],
    );

    assert_eq!(cpu.read_reg(9), 0x0000_FF01);
    assert_eq!(cpu.cp0().read_status(), 0x0000_FF00);
    assert!(!cpu.cp0().interrupt_enabled());
    assert!(!cpu.is_halted());
}

#[test]
fn interrupt_control_zero_register() {
    for (word, status, expected) in [(0x41606020, 0xFF00, 0xFF01), (0x41606000, 0xFF01, 0xFF00)] {
        let cpu = run(&[encode_cp0(4, 8, 12, 0), word], &[(8, status)]);

        assert_eq!(cpu.read_reg(0), 0);
        assert_eq!(cpu.cp0().read_status(), expected);
        assert!(!cpu.is_halted());
    }
}

#[test]
fn repeated_interrupt_control() {
    let cpu = run(
        &[
            encode_cp0(4, 8, 12, 0),
            0x41696020, // ei $t1
            0x416A6020, // ei $t2
            0x416B6000, // di $t3
            0x416C6000, // di $t4
        ],
        &[(8, 0x0040_FF06)],
    );

    assert_eq!(cpu.read_reg(9), 0x0040_FF06);
    assert_eq!(cpu.read_reg(10), 0x0040_FF07);
    assert_eq!(cpu.read_reg(11), 0x0040_FF07);
    assert_eq!(cpu.read_reg(12), 0x0040_FF06);

    assert_eq!(cpu.cp0().read_status(), 0x0040_FF06);
    assert!(!cpu.is_halted());
}

#[test]
fn general_exception_vector() {
    let cpu = run(&[0x0000000C], &[]);

    assert_eq!(cpu.read_pc(), 0x8000_0180);
    assert_eq!(cpu.read_next_pc(), 0x8000_0184);

    assert_eq!(cpu.cp0().read_epc(), 0);
    assert_eq!(cpu.cp0().exception_code(), 8);
    assert!(cpu.cp0().exception_level());

    assert!(!cpu.is_halted());
}

#[test]
fn bootstrap_exception_vector() {
    let cpu = run(&[encode_cp0(4, 8, 12, 0), 0x0000000C], &[(8, 1 << 22)]);

    assert_eq!(cpu.read_pc(), 0xBFC0_0380);
    assert_eq!(cpu.read_next_pc(), 0xBFC0_0384);

    assert_eq!(cpu.cp0().read_epc(), 4);
    assert!(cpu.cp0().exception_level());
    assert!(!cpu.is_halted());
}

#[test]
fn relocated_exception_vector() {
    let cpu = run(
        &[encode_cp0(4, 8, 15, 1), encode_cp0(0, 9, 15, 1), 0x0000000C],
        &[(8, 0x8000_2000)],
    );

    assert_eq!(cpu.read_reg(9), 0x8000_2000);
    assert_eq!(cpu.cp0().read_ebase(), 0x8000_2000);

    assert_eq!(cpu.read_pc(), 0x8000_2180);
    assert_eq!(cpu.cp0().read_epc(), 8);
}

#[test]
fn ebase_write_mask() {
    let cpu = run(
        &[encode_cp0(4, 8, 15, 1), encode_cp0(0, 9, 15, 1)],
        &[(8, u32::MAX)],
    );

    assert_eq!(cpu.read_reg(9), 0xBFFF_F000);
    assert!(!cpu.is_halted());
}

#[test]
fn memory_segment_alias() {
    use crate::cp0::PrivilegeMode;
    use crate::mmu::{AccessType, Mmu};

    let mut memory = Memory::new(1024);

    let a = Mmu::translate(0x8000_0180, PrivilegeMode::Kernel, AccessType::Load).unwrap();

    let b = Mmu::translate(0xA000_0180, PrivilegeMode::Kernel, AccessType::Store).unwrap();

    assert_eq!(a, 0x180);
    assert_eq!(b, 0x180);

    memory.write_word(a, 0x1234_5678);
    assert_eq!(memory.read_word(b), 0x1234_5678);

    memory.write_word(b, 0xDEAD_BEEF);
    assert_eq!(memory.read_word(a), 0xDEAD_BEEF);
}

#[test]
fn exception_handler_execution() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    memory.write_word(0, 0x0000000C);

    // Exception Handler
    memory.write_word(0x180, encode_cp0(0, 8, 14, 0));

    memory.write_word(0x184, encode_cp0(0, 9, 13, 0));

    cpu.step(&mut memory);

    assert_eq!(cpu.read_pc(), 0x8000_0180);

    cpu.step(&mut memory);
    cpu.step(&mut memory);

    assert_eq!(cpu.read_reg(8), 0);
    assert_eq!(cpu.read_reg(9), 8 << 2);

    assert_eq!(cpu.read_pc(), 0x8000_0188);
    assert!(!cpu.is_halted());
}

#[test]
fn delay_slot_exception_vector() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    memory.write_word(0, i(4, 0, 0, 3));
    memory.write_word(4, 0x0000000C);

    cpu.step(&mut memory);
    cpu.step(&mut memory);

    assert_eq!(cpu.read_pc(), 0x8000_0180);
    assert_eq!(cpu.read_next_pc(), 0x8000_0184);

    assert!(cpu.cp0().branch_delay());
    assert_eq!(cpu.cp0().read_epc(), 0);

    cpu.step(&mut memory);

    assert_eq!(cpu.read_pc(), 0x8000_0184);
    assert!(!cpu.is_halted());
}

#[test]
fn nested_exception_preserves_epc() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    memory.write_word(4, 0x0000000C);
    memory.write_word(0x180, 0x0000000D);

    cpu.write_pc(4);

    cpu.step(&mut memory);

    assert_eq!(cpu.cp0().read_epc(), 4);
    assert_eq!(cpu.cp0().exception_code(), 8);

    cpu.step(&mut memory);

    assert_eq!(cpu.cp0().read_epc(), 4);
    assert_eq!(cpu.cp0().exception_code(), 9);

    assert_eq!(cpu.read_pc(), 0x8000_0180);
    assert!(cpu.cp0().exception_level());
    assert!(!cpu.is_halted());
}

#[test]
fn error_epc_transfer() {
    let cpu = run(
        &[encode_cp0(4, 8, 30, 0), encode_cp0(0, 9, 30, 0)],
        &[(8, 0x8000_1000)],
    );

    assert_eq!(cpu.read_reg(9), 0x8000_1000);
    assert_eq!(cpu.cp0().read_error_epc(), 0x8000_1000);
    assert!(!cpu.is_halted());
}

#[test]
fn exception_return() {
    let cpu = run(
        &[encode_cp0(4, 8, 14, 0), encode_cp0(4, 9, 12, 0), 0x42000018],
        &[(8, 0x100), (9, 0x0000_FF03)],
    );

    assert_eq!(cpu.read_pc(), 0x100);
    assert_eq!(cpu.read_next_pc(), 0x104);

    assert_eq!(cpu.cp0().read_status(), 0x0000_FF01);
    assert!(!cpu.cp0().exception_level());
    assert!(!cpu.is_halted());
}

#[test]
fn error_level_return() {
    let cpu = run(
        &[
            encode_cp0(4, 8, 14, 0),
            encode_cp0(4, 9, 30, 0),
            encode_cp0(4, 10, 12, 0),
            0x42000018,
        ],
        &[(8, 0x100), (9, 0x200), (10, 0x0000_0006)],
    );

    assert_eq!(cpu.read_pc(), 0x200);
    assert_eq!(cpu.read_next_pc(), 0x204);

    assert!(!cpu.cp0().error_level());
    assert!(cpu.cp0().exception_level());

    assert_eq!(cpu.cp0().read_status(), 2);
    assert!(!cpu.is_halted());
}

#[test]
fn eret_without_delay_slot() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.cp0_mut().write_reg(14, 0, 0x100);
    cpu.cp0_mut().write_reg(12, 0, 2);

    memory.write_word(0, 0x42000018);

    memory.write_word(4, i(9, 0, 8, 100));
    memory.write_word(0x100, i(9, 0, 9, 200));

    cpu.step(&mut memory);

    assert_eq!(cpu.read_pc(), 0x100);

    cpu.step(&mut memory);

    assert_eq!(cpu.read_reg(8), 0);
    assert_eq!(cpu.read_reg(9), 200);

    assert_eq!(cpu.read_pc(), 0x104);
}

#[test]
fn eret_clears_reservation() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.cp0_mut().write_reg(14, 0, 0x100);
    cpu.cp0_mut().write_reg(12, 0, 2);

    cpu.set_reservation(256);

    memory.write_word(0, 0x42000018);

    assert!(cpu.check_reservation(256));

    cpu.step(&mut memory);

    assert!(!cpu.check_reservation(256));
    assert_eq!(cpu.read_pc(), 0x100);
}

#[test]
fn complete_exception_cycle() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    // Main Program
    memory.write_word(0x000, i(9, 0, 8, 100));
    memory.write_word(0x004, 0x0000000C);
    memory.write_word(0x008, i(9, 0, 9, 200));

    // Exception Handler
    memory.write_word(0x180, encode_cp0(0, 26, 14, 0));

    memory.write_word(0x184, i(9, 26, 26, 4));

    memory.write_word(0x188, encode_cp0(4, 26, 14, 0));

    memory.write_word(0x18C, 0x42000018);

    // Main Program 실행
    cpu.step(&mut memory);

    assert_eq!(cpu.read_reg(8), 100);

    // SYSCALL
    cpu.step(&mut memory);

    assert_eq!(cpu.read_pc(), 0x8000_0180);
    assert_eq!(cpu.cp0().read_epc(), 4);
    assert!(cpu.cp0().exception_level());

    // Exception Handler 실행
    for _ in 0..4 {
        cpu.step(&mut memory);
    }

    assert_eq!(cpu.read_pc(), 8);
    assert_eq!(cpu.cp0().read_epc(), 8);
    assert!(!cpu.cp0().exception_level());

    // Main Program 복귀
    cpu.step(&mut memory);

    assert_eq!(cpu.read_reg(8), 100);
    assert_eq!(cpu.read_reg(9), 200);
    assert_eq!(cpu.read_pc(), 12);

    assert!(!cpu.is_halted());
}

#[test]
fn privilege_mode_selection() {
    use crate::cp0::{Cp0, PrivilegeMode};

    let mut cp0 = Cp0::new();

    for (status, expected) in [
        (0x00, PrivilegeMode::Kernel),
        (0x08, PrivilegeMode::Supervisor),
        (0x10, PrivilegeMode::User),
        (0x18, PrivilegeMode::Reserved),
    ] {
        cp0.write_reg(12, 0, status);

        assert_eq!(cp0.privilege_mode(), expected);
    }
}

#[test]
fn exception_privilege_override() {
    use crate::cp0::{Cp0, PrivilegeMode};

    let mut cp0 = Cp0::new();

    for status in [0x12, 0x14, 0x16, 0x0A, 0x0C] {
        cp0.write_reg(12, 0, status);

        assert_eq!(cp0.privilege_mode(), PrivilegeMode::Kernel);
    }
}

#[test]
fn user_cp0_access_denied() {
    for word in [
        encode_cp0(0, 8, 12, 0), // mfc0
        encode_cp0(4, 8, 12, 0), // mtc0
        0x41686020,              // ei
        0x41686000,              // di
        0x42000018,              // eret
    ] {
        let mut cpu = Cpu::new();
        let mut memory = Memory::new(1024);

        cpu.cp0_mut().write_reg(12, 0, 0x10);

        cpu.write_reg(8, 0xDEAD_BEEF);
        memory.write_word(0, word);

        cpu.step(&mut memory);

        assert_eq!(cpu.cp0().exception_code(), 11);
        assert_eq!(cpu.read_pc(), 0x8000_0180);
        assert_eq!(cpu.cp0().read_epc(), 0);
        assert_eq!(cpu.read_reg(8), 0xDEAD_BEEF);

        assert_eq!((cpu.cp0().read_cause() >> 28) & 3, 0);

        assert!(cpu.cp0().is_kernel_mode());
        assert!(!cpu.is_halted());
    }
}

#[test]
fn user_cp0_access_enabled() {
    use crate::cp0::PrivilegeMode;

    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.cp0_mut().write_reg(12, 0, 0x1000_0010);

    memory.write_word(0, encode_cp0(0, 8, 12, 0));

    cpu.step(&mut memory);

    assert_eq!(cpu.read_reg(8), 0x1000_0010);
    assert_eq!(cpu.read_pc(), 4);

    assert_eq!(cpu.cp0().privilege_mode(), PrivilegeMode::User);

    assert!(!cpu.cp0().exception_level());
}

#[test]
fn supervisor_cp0_access() {
    for (status, allowed) in [(0x0000_0008, false), (0x1000_0008, true)] {
        let mut cpu = Cpu::new();
        let mut memory = Memory::new(1024);

        cpu.cp0_mut().write_reg(12, 0, status);

        memory.write_word(0, encode_cp0(0, 8, 12, 0));

        cpu.step(&mut memory);

        if allowed {
            assert_eq!(cpu.read_reg(8), status);
            assert_eq!(cpu.read_pc(), 4);
        } else {
            assert_eq!(cpu.cp0().exception_code(), 11);
            assert_eq!(cpu.read_pc(), 0x8000_0180);
        }
    }
}

#[test]
fn user_rdhwr_access_denied() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.cp0_mut().write_reg(12, 0, 0x10);

    cpu.write_reg(8, 0xDEAD_BEEF);

    memory.write_word(0, encode_r(0x1F, 0, 8, 2, 0, 0x3B));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp0().exception_code(), 10);
    assert_eq!(cpu.read_reg(8), 0xDEAD_BEEF);
    assert_eq!(cpu.read_pc(), 0x8000_0180);
}

#[test]
fn user_rdhwr_access_enabled() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.cp0_mut().write_reg(12, 0, 0x10);
    cpu.cp0_mut().write_hwrena(1 << 2);

    memory.write_word(0, encode_r(0x1F, 0, 8, 2, 0, 0x3B));

    cpu.step(&mut memory);

    assert_eq!(cpu.read_reg(8), 1);
    assert_eq!(cpu.read_pc(), 4);
    assert!(!cpu.cp0().exception_level());
}

#[test]
fn user_kernel_user_transition() {
    use crate::cp0::PrivilegeMode;

    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    // User Mode
    cpu.cp0_mut().write_reg(12, 0, 0x10);

    // User Program
    memory.write_word(0, 0x0000000C);
    memory.write_word(4, i(9, 0, 8, 100));

    // Exception Handler
    memory.write_word(0x180, encode_cp0(0, 26, 14, 0));

    memory.write_word(0x184, i(9, 26, 26, 4));

    memory.write_word(0x188, encode_cp0(4, 26, 14, 0));

    memory.write_word(0x18C, 0x42000018);

    assert_eq!(cpu.cp0().privilege_mode(), PrivilegeMode::User);

    // SYSCALL
    cpu.step(&mut memory);

    assert_eq!(cpu.cp0().privilege_mode(), PrivilegeMode::Kernel);

    assert_eq!(cpu.read_pc(), 0x8000_0180);
    assert!(cpu.cp0().exception_level());

    // Exception Handler 실행
    for _ in 0..4 {
        cpu.step(&mut memory);
    }

    // ERET 이후 User Mode 복귀
    assert_eq!(cpu.cp0().privilege_mode(), PrivilegeMode::User);

    assert_eq!(cpu.read_pc(), 4);
    assert!(!cpu.cp0().exception_level());

    cpu.step(&mut memory);

    assert_eq!(cpu.read_reg(8), 100);
    assert_eq!(cpu.read_pc(), 8);
}

#[test]
fn compare_register_transfer() {
    let cpu = run(
        &[
            encode_cp0(4, 8, 11, 0),
            encode_cp0(0, 9, 11, 0),
            encode_cp0(0, 10, 12, 1),
        ],
        &[(8, 100)],
    );

    assert_eq!(cpu.read_reg(9), 100);
    assert_eq!(cpu.cp0().read_compare(), 100);

    assert_eq!(cpu.read_reg(10) >> 29, 7);

    assert!(!cpu.cp0().timer_interrupt_pending());
}

#[test]
fn timer_interrupt_pending() {
    use crate::cp0::Cp0;

    let mut cp0 = Cp0::new();

    cp0.write_compare(3);

    cp0.tick();
    cp0.tick();

    assert!(!cp0.timer_interrupt_pending());

    cp0.tick();

    assert_eq!(cp0.read_count(), 3);
    assert!(cp0.timer_interrupt_pending());
    assert_eq!(cp0.interrupt_pending(), 0x80);

    cp0.tick();

    assert_eq!(cp0.read_count(), 4);
    assert!(cp0.timer_interrupt_pending());
}

#[test]
fn compare_write_clears_interrupt() {
    use crate::cp0::Cp0;

    let mut cp0 = Cp0::new();

    cp0.write_compare(1);
    cp0.tick();

    assert!(cp0.timer_interrupt_pending());

    cp0.write_reg(13, 0, 0x100);
    cp0.write_compare(10);

    assert!(!cp0.timer_interrupt_pending());
    assert_eq!(cp0.interrupt_pending(), 1);
    assert_eq!(cp0.read_compare(), 10);
}

#[test]
fn interrupt_acceptance_conditions() {
    use crate::cp0::Cp0;

    let mut cp0 = Cp0::new();

    cp0.write_compare(1);
    cp0.tick();

    for (status, expected) in [
        (0x0000, false),
        (0x0001, false),
        (0x8000, false),
        (0x8001, true),
        (0x8003, false),
        (0x8005, false),
        (0x4001, false),
    ] {
        cp0.write_reg(12, 0, status);

        assert_eq!(cp0.interrupt_requested(), expected);

        assert!(cp0.timer_interrupt_pending());
    }
}

#[test]
fn timer_interrupt_delivery() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.cp0_mut().write_compare(2);
    cpu.cp0_mut().write_reg(12, 0, 0x8001);

    memory.write_word(0, i(9, 0, 8, 100));
    memory.write_word(4, i(9, 0, 9, 200));

    cpu.step(&mut memory);

    assert_eq!(cpu.read_reg(8), 100);
    assert_eq!(cpu.read_pc(), 4);

    cpu.step(&mut memory);

    assert_eq!(cpu.read_pc(), 0x8000_0180);
    assert_eq!(cpu.cp0().exception_code(), 0);
    assert_eq!(cpu.cp0().read_epc(), 4);

    assert_eq!(cpu.read_reg(9), 0);
    assert!(cpu.cp0().exception_level());
    assert!(cpu.cp0().timer_interrupt_pending());
    assert!(!cpu.is_halted());
}

#[test]
fn dedicated_interrupt_vector() {
    for (status, vector) in [(0x0000_8001, 0x8000_0200), (0x0040_8001, 0xBFC0_0400)] {
        let mut cpu = Cpu::new();
        let mut memory = Memory::new(1024);

        cpu.cp0_mut().write_compare(1);
        cpu.cp0_mut().write_reg(12, 0, status);
        cpu.cp0_mut().write_reg(13, 0, 1 << 23);

        cpu.step(&mut memory);

        assert_eq!(cpu.read_pc(), vector);
        assert_eq!(cpu.read_next_pc(), vector + 4);
        assert_eq!(cpu.cp0().read_epc(), 0);
        assert!(cpu.cp0().exception_level());
    }
}

#[test]
fn software_interrupt_delivery() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.cp0_mut().write_reg(12, 0, 0x0101);
    cpu.cp0_mut().write_reg(13, 0, 0x0100);

    memory.write_word(0, i(9, 0, 8, 100));

    cpu.step(&mut memory);

    assert_eq!(cpu.read_pc(), 0x8000_0180);
    assert_eq!(cpu.cp0().exception_code(), 0);
    assert_eq!(cpu.cp0().read_epc(), 0);

    assert_eq!(cpu.read_reg(8), 0);
    assert!(!cpu.cp0().timer_interrupt_pending());
}

#[test]
fn interrupt_before_delay_slot() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.cp0_mut().write_compare(2);
    cpu.cp0_mut().write_reg(12, 0, 0x8001);

    memory.write_word(0, i(4, 0, 0, 3));
    memory.write_word(4, i(9, 0, 8, 100));

    cpu.step(&mut memory);

    assert_eq!(cpu.read_pc(), 4);
    assert_eq!(cpu.read_next_pc(), 16);

    cpu.step(&mut memory);

    assert_eq!(cpu.read_pc(), 0x8000_0180);
    assert_eq!(cpu.cp0().read_epc(), 0);
    assert!(cpu.cp0().branch_delay());

    assert_eq!(cpu.read_reg(8), 0);
}

#[test]
fn timer_count_wraparound() {
    use crate::cp0::Cp0;

    let mut cp0 = Cp0::new();

    cp0.write_reg(9, 0, u32::MAX);
    cp0.write_compare(0);

    cp0.tick();

    assert_eq!(cp0.read_count(), 0);
    assert!(cp0.timer_interrupt_pending());
}

#[test]
fn complete_timer_interrupt_cycle() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.cp0_mut().write_compare(2);
    cpu.cp0_mut().write_reg(12, 0, 0x8001);

    // Main Program
    memory.write_word(0, i(9, 0, 8, 100));
    memory.write_word(4, i(9, 0, 9, 200));

    // Timer Interrupt Handler
    memory.write_word(0x180, i(9, 0, 26, 100));

    memory.write_word(0x184, encode_cp0(4, 26, 11, 0));

    memory.write_word(0x188, 0x42000018);

    // Main Program 실행
    cpu.step(&mut memory);

    assert_eq!(cpu.read_reg(8), 100);

    // Timer Interrupt 발생
    cpu.step(&mut memory);

    assert_eq!(cpu.read_pc(), 0x8000_0180);
    assert_eq!(cpu.cp0().read_epc(), 4);
    assert!(cpu.cp0().timer_interrupt_pending());

    // Interrupt Handler 실행
    for _ in 0..3 {
        cpu.step(&mut memory);
    }

    assert_eq!(cpu.read_pc(), 4);
    assert_eq!(cpu.cp0().read_compare(), 100);

    assert!(!cpu.cp0().timer_interrupt_pending());
    assert!(!cpu.cp0().exception_level());

    // 중단된 명령어 실행
    cpu.step(&mut memory);

    assert_eq!(cpu.read_reg(9), 200);
    assert_eq!(cpu.read_pc(), 8);
    assert!(!cpu.is_halted());
}

#[test]
fn config_initialization() {
    use crate::cp0::Cp0;

    let cp0 = Cp0::new();

    assert_eq!(cp0.read_config(), 0x8000_0482);
    assert_eq!(cp0.read_config1(), 0x3E00_0001);
}

#[test]
fn config_architecture_fields() {
    use crate::cp0::Cp0;

    let cp0 = Cp0::new();
    let config = cp0.read_config();

    assert_eq!((config >> 31) & 1, 1);
    assert_eq!((config >> 15) & 1, 0);
    assert_eq!((config >> 13) & 3, 0);
    assert_eq!((config >> 10) & 7, 1);
    assert_eq!((config >> 7) & 7, 1);
    assert_eq!(config & 7, 2);
}

#[test]
fn config_tlb_size() {
    use crate::cp0::{Cp0, TLB_SIZE};

    let cp0 = Cp0::new();
    let config1 = cp0.read_config1();

    let mmu_size = (config1 >> 25) & 0x3F;

    assert_eq!(mmu_size as usize + 1, TLB_SIZE);

    // Config2 미지원
    assert_eq!(config1 >> 31, 0);

    // FPU 미지원
    assert_eq!(config1 & 1, 1);
}

#[test]
fn config_write_mask() {
    use crate::cp0::Cp0;

    let mut cp0 = Cp0::new();

    cp0.write_reg(16, 0, u32::MAX);

    assert_eq!(cp0.read_config(), 0x8000_0487);

    cp0.write_reg(16, 0, 0);

    assert_eq!(cp0.read_config(), 0x8000_0480);

    cp0.write_reg(16, 0, 2);

    assert_eq!(cp0.read_config(), 0x8000_0482);
}

#[test]
fn config1_read_only() {
    use crate::cp0::Cp0;

    let mut cp0 = Cp0::new();
    let old = cp0.read_config1();

    assert!(cp0.write_reg(16, 1, u32::MAX));
    assert_eq!(cp0.read_config1(), old);

    assert!(cp0.write_reg(16, 1, 0));
    assert_eq!(cp0.read_config1(), old);
}

#[test]
fn config_register_transfer() {
    let cpu = run(
        &[
            encode_cp0(0, 8, 16, 0),
            encode_cp0(0, 9, 16, 1),
            encode_cp0(4, 10, 16, 0),
            encode_cp0(0, 11, 16, 0),
            encode_cp0(4, 10, 16, 1),
            encode_cp0(0, 12, 16, 1),
        ],
        &[(10, 3)],
    );

    assert_eq!(cpu.read_reg(8), 0x8000_0482);
    assert_eq!(cpu.read_reg(9), 0x3E00_0001);
    assert_eq!(cpu.read_reg(11), 0x8000_0483);
    assert_eq!(cpu.read_reg(12), 0x3E00_0001);

    assert!(!cpu.cp0().exception_level());
}
