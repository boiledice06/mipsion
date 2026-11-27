use super::*;

#[test]
fn load_store() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.write_reg(8, 256);
    cpu.write_reg(9, 0x12348081);

    memory.write_word(0, 0xAD090000); // sw
    memory.write_word(4, 0x8D0A0000); // lw
    memory.write_word(8, 0x810B0000); // lb
    memory.write_word(12, 0x910C0000); // lbu
    memory.write_word(16, 0x850D0000); // lh
    memory.write_word(20, 0x950E0000); // lhu
    memory.write_word(24, 0xA5090004); // sh
    memory.write_word(28, 0xA1090006); // sb

    for _ in 0..8 {
        cpu.step(&mut memory);
    }

    assert_eq!(cpu.read_reg(10), 0x12348081);
    assert_eq!(cpu.read_reg(11), 0xFFFFFF81);
    assert_eq!(cpu.read_reg(12), 0x00000081);
    assert_eq!(cpu.read_reg(13), 0xFFFF8081);
    assert_eq!(cpu.read_reg(14), 0x00008081);

    assert_eq!(memory.read_halfword(260), 0x8081);
    assert_eq!(memory.read_byte(262), 0x81);
}

#[test]
fn memory_negative_offset() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.write_reg(8, 256);
    cpu.write_reg(9, 0xDEADBEEF);

    memory.write_word(0, 0xAD09FFFC); // sw -4
    memory.write_word(4, 0x8D0AFFFC); // lw -4

    cpu.step(&mut memory);
    cpu.step(&mut memory);

    assert_eq!(memory.read_word(252), 0xDEADBEEF);
    assert_eq!(cpu.read_reg(10), 0xDEADBEEF);
}

#[test]
fn unaligned_load() {
    for offset in 0..4u32 {
        let mut cpu = Cpu::new();
        let mut memory = Memory::new(1024);

        let address = 256 + offset;

        cpu.write_reg(8, address);
        cpu.write_reg(9, 0xDEADBEEF);

        memory.write_byte(address, 0x11);
        memory.write_byte(address + 1, 0x22);
        memory.write_byte(address + 2, 0x33);
        memory.write_byte(address + 3, 0x44);

        memory.write_word(0, 0x89090003); // lwl
        memory.write_word(4, 0x99090000); // lwr

        cpu.step(&mut memory);
        cpu.step(&mut memory);

        assert_eq!(cpu.read_reg(9), 0x44332211, "offset: {offset}");
    }
}

#[test]
fn unaligned_store() {
    for offset in 0..4u32 {
        let mut cpu = Cpu::new();
        let mut memory = Memory::new(1024);

        let address = 256 + offset;

        cpu.write_reg(8, address);
        cpu.write_reg(9, 0x44332211);

        for i in 255..264 {
            memory.write_byte(i, 0xAA);
        }

        memory.write_word(0, 0xA9090003); // swl
        memory.write_word(4, 0xB9090000); // swr

        cpu.step(&mut memory);
        cpu.step(&mut memory);

        assert_eq!(memory.read_byte(address), 0x11);
        assert_eq!(memory.read_byte(address + 1), 0x22);
        assert_eq!(memory.read_byte(address + 2), 0x33);
        assert_eq!(memory.read_byte(address + 3), 0x44);

        assert_eq!(memory.read_byte(address - 1), 0xAA);
        assert_eq!(memory.read_byte(address + 4), 0xAA);
    }
}

#[test]
fn unaligned_partial_word() {
    let mut memory = Memory::new(1024);

    memory.write_word(256, 0x44332211);

    let old = 0xAABBCCDD;

    assert_eq!(memory.read_word_left(257, old), 0x2211CCDD);

    assert_eq!(memory.read_word_right(257, old), 0xAA443322);

    memory.write_word_left(257, 0xAABBCCDD);
    assert_eq!(memory.read_word(256), 0x4433AABB);

    memory.write_word(256, 0x44332211);

    memory.write_word_right(257, 0xAABBCCDD);
    assert_eq!(memory.read_word(256), 0xBBCCDD11);
}

#[test]
fn memory_alignment_exception() {
    for (opcode, address, code) in [
        (0x23, 257, 4), // lw
        (0x21, 257, 4), // lh
        (0x25, 257, 4), // lhu
        (0x2B, 258, 5), // sw
        (0x29, 257, 5), // sh
        (0x30, 257, 4), // ll
        (0x38, 257, 5), // sc
    ] {
        let mut cpu = Cpu::new();
        let mut memory = Memory::new(1024);

        cpu.write_reg(8, address);
        cpu.write_reg(9, 0xDEADBEEF);

        memory.write_word(0, i(opcode, 8, 9, 0));
        memory.write_word(256, 0x12345678);

        cpu.step(&mut memory);

        assert!(!cpu.is_halted());
        assert_eq!(cpu.read_pc(), 0x8000_0180);
        assert_eq!(cpu.cp0().exception_code(), code);
        assert_eq!(cpu.cp0().read_bad_vaddr(), address);
        assert_eq!(cpu.cp0().read_epc(), 0);

        assert_eq!(cpu.read_reg(9), 0xDEADBEEF);
        assert_eq!(memory.read_word(256), 0x12345678);
    }
}

#[test]
fn instruction_alignment_exception() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.write_pc(3);
    cpu.step(&mut memory);

    assert!(!cpu.is_halted());
    assert_eq!(cpu.read_pc(), 0x8000_0180);
    assert_eq!(cpu.cp0().exception_code(), 4);
    assert_eq!(cpu.cp0().read_bad_vaddr(), 3);
    assert_eq!(cpu.cp0().read_epc(), 3);
}

#[test]
fn atomic_increment() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.write_reg(8, 256);
    memory.write_word(256, 10);

    memory.write_word(0, i(0x30, 8, 9, 0));
    memory.write_word(4, i(0x09, 9, 9, 1));
    memory.write_word(8, i(0x38, 8, 9, 0));

    cpu.step(&mut memory);
    assert_eq!(cpu.read_reg(9), 10);
    assert!(cpu.check_reservation(256));

    cpu.step(&mut memory);
    assert_eq!(cpu.read_reg(9), 11);

    cpu.step(&mut memory);

    assert_eq!(memory.read_word(256), 11);
    assert_eq!(cpu.read_reg(9), 1);
    assert!(!cpu.check_reservation(256));
}

#[test]
fn atomic_store_failure() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.write_reg(8, 256);
    cpu.write_reg(10, 99);

    memory.write_word(256, 10);

    memory.write_word(0, i(0x30, 8, 9, 0));
    memory.write_word(4, i(0x2B, 8, 10, 0));
    memory.write_word(8, i(0x38, 8, 9, 0));

    for _ in 0..3 {
        cpu.step(&mut memory);
    }

    assert_eq!(memory.read_word(256), 99);
    assert_eq!(cpu.read_reg(9), 0);
    assert!(!cpu.check_reservation(256));
}

#[test]
fn store_conditional_without_ll() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.write_reg(8, 256);
    cpu.write_reg(9, 123);

    memory.write_word(256, 456);
    memory.write_word(0, i(0x38, 8, 9, 0));

    cpu.step(&mut memory);

    assert_eq!(memory.read_word(256), 456);
    assert_eq!(cpu.read_reg(9), 0);
}

#[test]
fn memory_range_exact_boundary() {
    let memory = Memory::new(64);

    assert!(memory.contains_range(60, 4));
    assert!(memory.contains_range(63, 1));
    assert!(memory.contains_range(0, 64));
}

#[test]
fn memory_range_out_of_bounds() {
    let memory = Memory::new(64);

    assert!(!memory.contains_range(60, 8));
    assert!(!memory.contains_range(64, 1));
    assert!(!memory.contains_range(0, 65));
}

#[test]
fn memory_range_overflow() {
    let memory = Memory::new(64);

    assert!(!memory.contains_range(u32::MAX, 4));
    assert!(!memory.contains_range(0, 0));
}

#[test]
fn memory_range_empty_memory() {
    let memory = Memory::new(0);

    assert!(!memory.contains_range(0, 1));
}

#[test]
fn instruction_bus_error_out_of_bounds() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(64);

    cpu.write_pc(64);
    cpu.step(&mut memory);

    assert_eq!((cpu.cp0().read_cause() >> 2) & 0x1F, 6);
    assert_eq!(cpu.read_pc(), 0x8000_0180);
    assert_eq!(cpu.cp0().read_reg(14, 0), Some(64));
}

#[test]
fn instruction_bus_error_partial_fetch() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(6);

    cpu.write_pc(4);
    cpu.step(&mut memory);

    assert_eq!((cpu.cp0().read_cause() >> 2) & 0x1F, 6);
    assert_eq!(cpu.cp0().read_reg(14, 0), Some(4));
}

#[test]
fn data_bus_error_integer_loads() {
    for opcode in [0x20, 0x21, 0x23, 0x24, 0x25] {
        let mut cpu = Cpu::new();
        let mut memory = Memory::new(64);

        memory.write_word(0, i(opcode, 8, 9, 0));

        cpu.write_reg(8, 64);
        cpu.write_reg(9, 0x1234_5678);

        cpu.step(&mut memory);

        assert_eq!((cpu.cp0().read_cause() >> 2) & 0x1F, 7);
        assert_eq!(cpu.read_reg(9), 0x1234_5678);
    }
}

#[test]
fn data_bus_error_integer_stores() {
    for opcode in [0x28, 0x29, 0x2B] {
        let mut cpu = Cpu::new();
        let mut memory = Memory::new(64);

        memory.write_word(0, i(opcode, 8, 9, 0));

        cpu.write_reg(8, 64);
        cpu.write_reg(9, u32::MAX);

        cpu.step(&mut memory);

        assert_eq!((cpu.cp0().read_cause() >> 2) & 0x1F, 7);
    }
}

#[test]
fn data_bus_error_no_partial_store() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(10);

    memory.write_word(0, i(0x2B, 8, 9, 0));
    memory.write_halfword(8, 0x1234);

    cpu.write_reg(8, 8);
    cpu.write_reg(9, u32::MAX);

    cpu.step(&mut memory);

    assert_eq!((cpu.cp0().read_cause() >> 2) & 0x1F, 7);
    assert_eq!(memory.read_halfword(8), 0x1234);
}

#[test]
fn data_bus_error_preserves_bad_vaddr() {
    use crate::exception::Exception;

    let mut cpu = Cpu::new();
    let mut memory = Memory::new(64);

    cpu.raise_exception(Exception::AddressErrorLoad, Some(0x1234_5678));

    cpu.return_from_exception();

    memory.write_word(0, i(0x23, 8, 9, 0));
    cpu.write_pc(0);
    cpu.write_reg(8, 64);

    cpu.step(&mut memory);

    assert_eq!((cpu.cp0().read_cause() >> 2) & 0x1F, 7);
    assert_eq!(cpu.cp0().read_bad_vaddr(), 0x1234_5678);
}

#[test]
fn alignment_exception_precedes_bus_error() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(64);

    memory.write_word(0, i(0x23, 8, 9, 0));
    cpu.write_reg(8, 65);

    cpu.step(&mut memory);

    assert_eq!((cpu.cp0().read_cause() >> 2) & 0x1F, 4);
    assert_eq!(cpu.cp0().read_bad_vaddr(), 65);
}

#[test]
fn data_bus_error_ll() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(64);

    memory.write_word(0, i(0x30, 8, 9, 0));

    cpu.write_reg(8, 64);
    cpu.write_reg(9, 123);

    cpu.step(&mut memory);

    assert_eq!((cpu.cp0().read_cause() >> 2) & 0x1F, 7);
    assert_eq!(cpu.read_reg(9), 123);
    assert!(!cpu.check_reservation(64));
}

#[test]
fn data_bus_error_sc() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(64);

    memory.write_word(0, i(0x38, 8, 9, 0));

    cpu.write_reg(8, 64);
    cpu.write_reg(9, 123);
    cpu.set_reservation(64);

    cpu.step(&mut memory);

    assert_eq!((cpu.cp0().read_cause() >> 2) & 0x1F, 7);
    assert_eq!(cpu.read_reg(9), 123);
    assert!(!cpu.check_reservation(64));
}

#[test]
fn data_bus_error_fpu_loads() {
    for opcode in [0x31, 0x35] {
        let mut cpu = Cpu::new();
        let mut memory = Memory::new(64);

        // Enable CP1
        cpu.cp0_mut().write_reg(12, 0, 1 << 29);

        memory.write_word(0, i(opcode, 8, 2, 0));

        cpu.write_reg(8, 64);
        cpu.cp1_mut().write_word(2, 0x1234_5678);
        cpu.cp1_mut().write_word(3, 0x8765_4321);

        cpu.step(&mut memory);

        assert_eq!(cpu.cp0().exception_code(), 7);
        assert_eq!(cpu.cp1().read_word(2), 0x1234_5678);
        assert_eq!(cpu.cp1().read_word(3), 0x8765_4321);
    }
}

#[test]
fn data_bus_error_fpu_stores() {
    for opcode in [0x39, 0x3D] {
        let mut cpu = Cpu::new();
        let mut memory = Memory::new(64);

        cpu.cp0_mut().write_reg(12, 0, 1 << 29);

        memory.write_word(0, i(opcode, 8, 2, 0));

        cpu.write_reg(8, 64);
        cpu.cp1_mut().write_word(2, u32::MAX);

        cpu.step(&mut memory);

        assert_eq!(cpu.cp0().exception_code(), 7);
    }
}

#[test]
fn data_bus_error_no_partial_doubleword_store() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(12);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    memory.write_word(0, i(0x3D, 8, 2, 0));
    memory.write_word(8, 0x1234_5678);

    cpu.write_reg(8, 8);
    cpu.cp1_mut().write_word(2, u32::MAX);
    cpu.cp1_mut().write_word(3, u32::MAX);

    cpu.step(&mut memory);

    assert_eq!(cpu.cp0().exception_code(), 7);
    assert_eq!(memory.read_word(8), 0x1234_5678);
}

#[test]
fn unaligned_left_load_at_memory_boundary() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(10);

    memory.write_word(0, i(0x22, 8, 9, 0));
    memory.write_halfword(8, 0xABCD);

    cpu.write_reg(8, 9);
    cpu.write_reg(9, 0x1234_5678);

    cpu.step(&mut memory);

    assert_eq!(cpu.read_reg(9), 0xABCD_5678);
    assert!(!cpu.cp0().exception_level());
}

#[test]
fn unaligned_left_store_at_memory_boundary() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(10);

    memory.write_word(0, i(0x2A, 8, 9, 0));

    cpu.write_reg(8, 9);
    cpu.write_reg(9, 0xABCD_1234);

    cpu.step(&mut memory);

    assert_eq!(memory.read_halfword(8), 0xABCD);
    assert!(!cpu.cp0().exception_level());
}

#[test]
fn data_bus_error_unaligned_right_load() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(10);

    memory.write_word(0, i(0x26, 8, 9, 0));

    cpu.write_reg(8, 9);
    cpu.write_reg(9, 0x1234_5678);

    cpu.step(&mut memory);

    assert_eq!(cpu.cp0().exception_code(), 7);
    assert_eq!(cpu.read_reg(9), 0x1234_5678);
}

#[test]
fn data_bus_error_unaligned_right_store() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(10);

    memory.write_word(0, i(0x2E, 8, 9, 0));
    memory.write_halfword(8, 0xABCD);

    cpu.write_reg(8, 9);
    cpu.write_reg(9, u32::MAX);

    cpu.step(&mut memory);

    assert_eq!(cpu.cp0().exception_code(), 7);
    assert_eq!(memory.read_halfword(8), 0xABCD);
}

#[test]
fn instruction_bus_error_in_delay_slot() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    // BEQ $zero, $zero, 1
    memory.write_word(0, i(0x04, 0, 0, 1));

    cpu.step(&mut memory);
    cpu.step(&mut memory);

    assert_eq!(cpu.cp0().exception_code(), 6);
    assert_eq!(cpu.cp0().read_epc(), 0);
    assert!(cpu.cp0().branch_delay());
}

#[test]
fn data_bus_error_in_delay_slot() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(64);

    memory.write_word(0, i(0x04, 0, 0, 1));
    memory.write_word(4, i(0x23, 8, 9, 0));

    cpu.write_reg(8, 64);
    cpu.write_reg(9, 123);

    cpu.step(&mut memory);
    cpu.step(&mut memory);

    assert_eq!(cpu.cp0().exception_code(), 7);
    assert_eq!(cpu.cp0().read_epc(), 0);
    assert!(cpu.cp0().branch_delay());
    assert_eq!(cpu.read_reg(9), 123);
}

#[test]
fn data_bus_error_after_segment_translation() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(64);

    memory.write_word(0, i(0x23, 8, 9, 0));

    cpu.write_reg(8, 0x8000_0040);
    cpu.write_reg(9, 123);

    cpu.step(&mut memory);

    assert_eq!(cpu.cp0().exception_code(), 7);
    assert_eq!(cpu.read_reg(9), 123);
}

#[test]
fn data_bus_error_after_tlb_translation() {
    let mut cpu = Cpu::new_with_tlb();
    let mut memory = Memory::new(4096);

    cpu.cp0_mut().write_reg(10, 0, 0x0040_0001);
    cpu.cp0_mut().write_reg(2, 0, 0x46);
    cpu.cp0_mut().tlb_write_indexed();

    cpu.write_pc(0x8000_0000);

    memory.write_word(0, i(0x23, 8, 9, 0));

    cpu.write_reg(8, 0x0040_0000);
    cpu.write_reg(9, 123);

    cpu.step(&mut memory);

    assert_eq!(cpu.cp0().exception_code(), 7);
    assert_eq!(cpu.read_reg(9), 123);
}
