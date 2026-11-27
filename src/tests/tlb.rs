use super::*;

#[test]
fn tlb_register_initialization() {
    use crate::cp0::{Cp0, TLB_SIZE};

    let cp0 = Cp0::new();

    assert_eq!(cp0.read_index(), 0);
    assert_eq!(cp0.read_random(), (TLB_SIZE - 1) as u32);

    assert_eq!(cp0.read_entry_lo0(), 0);
    assert_eq!(cp0.read_entry_lo1(), 0);
    assert_eq!(cp0.read_context(), 0);
    assert_eq!(cp0.read_page_mask(), 0);
    assert_eq!(cp0.read_wired(), 0);
    assert_eq!(cp0.read_entry_hi(), 0);
}

#[test]
fn tlb_register_transfer() {
    for (rd, value) in [
        (0, 15),
        (2, 0x0000_0047),
        (3, 0x0000_0087),
        (4, 0x8000_0000),
        (5, 0x0000_6000),
        (6, 4),
        (10, 0x0040_0001),
    ] {
        let cpu = run(
            &[encode_cp0(4, 8, rd, 0), encode_cp0(0, 9, rd, 0)],
            &[(8, value)],
        );

        assert_eq!(cpu.read_reg(9), value, "CP0 Register {rd}");

        assert!(!cpu.cp0().exception_level());
    }
}

#[test]
fn tlb_register_write_mask() {
    use crate::cp0::Cp0;

    let mut cp0 = Cp0::new();

    for (rd, expected) in [
        (0, 0x0000_001F),
        (2, 0x03FF_FFFF),
        (3, 0x03FF_FFFF),
        (4, 0xFF80_0000),
        (5, 0x1FFF_E000),
        (6, 0x0000_001F),
        (10, 0xFFFF_E0FF),
    ] {
        assert!(cp0.write_reg(rd, 0, u32::MAX));

        assert_eq!(cp0.read_reg(rd, 0), Some(expected));
    }
}

#[test]
fn tlb_random_read_only() {
    use crate::cp0::Cp0;

    let mut cp0 = Cp0::new();

    let old = cp0.read_random();

    assert!(cp0.write_reg(1, 0, 5));
    assert_eq!(cp0.read_random(), old);
}

#[test]
fn tlb_random_cycle() {
    use crate::cp0::Cp0;

    let mut cp0 = Cp0::new();

    for expected in (0..31u32).rev() {
        cp0.tick();

        assert_eq!(cp0.read_random(), expected);
    }

    assert_eq!(cp0.read_random(), 0);

    cp0.tick();

    assert_eq!(cp0.read_random(), 31);
}

#[test]
fn tlb_wired_boundary() {
    use crate::cp0::Cp0;

    let mut cp0 = Cp0::new();

    cp0.write_reg(6, 0, 28);

    assert_eq!(cp0.read_wired(), 28);
    assert_eq!(cp0.read_random(), 31);

    for expected in [30, 29, 28, 31, 30] {
        cp0.tick();

        assert_eq!(cp0.read_random(), expected);
    }

    cp0.write_reg(6, 0, 31);

    for _ in 0..10 {
        cp0.tick();

        assert_eq!(cp0.read_random(), 31);
    }
}

#[test]
fn tlb_asid_extraction() {
    use crate::cp0::Cp0;

    let mut cp0 = Cp0::new();

    cp0.write_reg(10, 0, 0x0040_00AB);

    assert_eq!(cp0.asid(), 0xAB);
    assert_eq!(cp0.read_entry_hi(), 0x0040_00AB);
}

#[test]
fn tlb_context_select_independence() {
    let cpu = run(
        &[
            encode_cp0(4, 8, 4, 0),
            encode_cp0(4, 9, 4, 2),
            encode_cp0(0, 10, 4, 0),
            encode_cp0(0, 11, 4, 2),
        ],
        &[(8, 0x8000_0000), (9, 0x1234_5678)],
    );

    assert_eq!(cpu.read_reg(10), 0x8000_0000);
    assert_eq!(cpu.read_reg(11), 0x1234_5678);

    assert!(!cpu.cp0().exception_level());
}

#[test]
fn tlb_initialization() {
    use crate::cp0::TLB_SIZE;
    use crate::tlb::Tlb;

    let tlb = Tlb::new();

    for index in 0..TLB_SIZE {
        assert_eq!(tlb.read(index), None);
    }

    assert_eq!(tlb.probe(0, 0), None);
}

#[test]
fn tlb_indexed_write() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.cp0_mut().write_reg(0, 0, 5);
    cpu.cp0_mut().write_reg(10, 0, 0x0040_0001);
    cpu.cp0_mut().write_reg(2, 0, 0x0000_0046);
    cpu.cp0_mut().write_reg(3, 0, 0x0000_0086);
    cpu.cp0_mut().write_reg(5, 0, 0);

    memory.write_word(0, 0x42000002);

    cpu.step(&mut memory);

    let entry = cpu.cp0().tlb().read(5).unwrap();

    assert_eq!(entry.entry_hi, 0x0040_0001);
    assert_eq!(entry.entry_lo0, 0x0000_0046);
    assert_eq!(entry.entry_lo1, 0x0000_0086);
    assert_eq!(entry.page_mask, 0);

    assert_eq!(cpu.cp0().tlb().read(4), None);
    assert!(!cpu.cp0().exception_level());
}

#[test]
fn tlb_indexed_read() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.cp0_mut().write_reg(0, 0, 7);
    cpu.cp0_mut().write_reg(10, 0, 0x0080_0002);
    cpu.cp0_mut().write_reg(2, 0, 0x0000_00C6);
    cpu.cp0_mut().write_reg(3, 0, 0x0000_0106);
    cpu.cp0_mut().write_reg(5, 0, 0x6000);

    memory.write_word(0, 0x42000002);
    memory.write_word(4, 0x42000001);

    // TLBWI
    cpu.step(&mut memory);

    // CP0 Register 초기화
    cpu.cp0_mut().write_reg(10, 0, 0);
    cpu.cp0_mut().write_reg(2, 0, 0);
    cpu.cp0_mut().write_reg(3, 0, 0);
    cpu.cp0_mut().write_reg(5, 0, 0);

    // TLBR
    cpu.step(&mut memory);

    assert_eq!(cpu.cp0().read_entry_hi(), 0x0080_0002);
    assert_eq!(cpu.cp0().read_entry_lo0(), 0x0000_00C6);
    assert_eq!(cpu.cp0().read_entry_lo1(), 0x0000_0106);
    assert_eq!(cpu.cp0().read_page_mask(), 0x6000);
}

#[test]
fn tlb_probe_hit() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.cp0_mut().write_reg(0, 0, 9);
    cpu.cp0_mut().write_reg(10, 0, 0x0040_0003);
    cpu.cp0_mut().write_reg(2, 0, 0x46);
    cpu.cp0_mut().write_reg(3, 0, 0x86);

    memory.write_word(0, 0x42000002);
    memory.write_word(4, 0x42000008);

    cpu.step(&mut memory);

    cpu.cp0_mut().write_reg(0, 0, 0);

    cpu.step(&mut memory);

    assert_eq!(cpu.cp0().read_index(), 9);
    assert_eq!(cpu.cp0().read_index() >> 31, 0);
}

#[test]
fn tlb_probe_asid_miss() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.cp0_mut().write_reg(0, 0, 5);
    cpu.cp0_mut().write_reg(10, 0, 0x0040_0001);
    cpu.cp0_mut().write_reg(2, 0, 0x46);
    cpu.cp0_mut().write_reg(3, 0, 0x86);

    memory.write_word(0, 0x42000002);
    memory.write_word(4, 0x42000008);
    memory.write_word(8, 0x42000008);

    cpu.step(&mut memory);

    // VPN2는 동일하지만 ASID 불일치
    cpu.cp0_mut().write_reg(10, 0, 0x0040_0002);

    cpu.step(&mut memory);

    assert_ne!(cpu.cp0().read_index() & 0x8000_0000, 0);

    // ASID 복원
    cpu.cp0_mut().write_reg(10, 0, 0x0040_0001);

    cpu.step(&mut memory);

    assert_eq!(cpu.cp0().read_index(), 5);
}

#[test]
fn tlb_global_mapping() {
    use crate::tlb::TlbEntry;

    let mut cpu = Cpu::new();

    for (lo0, lo1, global) in [
        (0x47, 0x87, true),
        (0x47, 0x86, false),
        (0x46, 0x87, false),
        (0x46, 0x86, false),
    ] {
        cpu.cp0_mut().write_reg(0, 0, 0);
        cpu.cp0_mut().write_reg(10, 0, 0x0040_0001);
        cpu.cp0_mut().write_reg(2, 0, lo0);
        cpu.cp0_mut().write_reg(3, 0, lo1);

        cpu.cp0_mut().tlb_write_indexed();

        let entry: TlbEntry = cpu.cp0().tlb().read(0).unwrap();

        assert_eq!(entry.global(), global);
        assert_eq!(entry.matches(0x0040_0000, 2), global);

        // 동일 ASID는 항상 일치
        assert!(entry.matches(0x0040_0000, 1));
    }
}

#[test]
fn tlb_page_mask_probe() {
    use crate::tlb::TlbEntry;

    let entry = TlbEntry {
        entry_hi: 0x0040_0001,
        entry_lo0: 0x46,
        entry_lo1: 0x86,
        page_mask: 0x6000,
    };

    // 16 KiB Page 2개 = 32 KiB 범위
    for address in [0x0040_0000, 0x0040_2000, 0x0040_4000, 0x0040_7FFF] {
        assert!(entry.matches(address, 1));
    }

    assert!(!entry.matches(0x0040_8000, 1));
    assert!(!entry.matches(0x0040_0000, 2));
}

#[test]
fn tlb_random_write() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(1024);

    cpu.cp0_mut().write_reg(6, 0, 28);

    cpu.cp0_mut().write_reg(10, 0, 0x0040_0001);
    cpu.cp0_mut().write_reg(2, 0, 0x46);
    cpu.cp0_mut().write_reg(3, 0, 0x86);

    memory.write_word(0, 0x42000006);

    assert_eq!(cpu.cp0().read_random(), 31);

    cpu.step(&mut memory);

    // step()에서 Random이 먼저 감소
    assert_eq!(cpu.cp0().read_random(), 30);

    let entry = cpu.cp0().tlb().read(30).unwrap();

    assert_eq!(entry.entry_hi, 0x0040_0001);

    for index in 0..28 {
        assert_eq!(cpu.cp0().tlb().read(index), None);
    }
}

#[test]
fn user_tlb_access_denied() {
    for word in [0x42000001, 0x42000002, 0x42000006, 0x42000008] {
        let mut cpu = Cpu::new();
        let mut memory = Memory::new(1024);

        cpu.cp0_mut().write_reg(12, 0, 0x10);
        cpu.cp0_mut().write_reg(0, 0, 5);

        memory.write_word(0, word);

        cpu.step(&mut memory);

        assert_eq!(cpu.cp0().exception_code(), 11);
        assert_eq!(cpu.read_pc(), 0x8000_0180);
        assert_eq!(cpu.cp0().read_epc(), 0);

        assert_eq!(cpu.cp0().tlb().read(5), None);
        assert_eq!(cpu.cp0().read_index(), 5);

        assert!(cpu.cp0().is_kernel_mode());
    }
}

#[test]
fn tlb_translate_even_odd() {
    use crate::tlb::TlbEntry;

    let entry = TlbEntry {
        entry_hi: 0x0040_0001,
        entry_lo0: (0x10 << 6) | 6,
        entry_lo1: (0x30 << 6) | 6,
        page_mask: 0,
    };

    assert_eq!(entry.translate(0x0040_0123, false), Ok(0x0001_0123));

    assert_eq!(entry.translate(0x0040_1456, false), Ok(0x0003_0456));
}

#[test]
fn tlb_translate_large_page() {
    use crate::tlb::TlbEntry;

    let entry = TlbEntry {
        entry_hi: 0x0040_0001,
        entry_lo0: (0x13 << 6) | 6,
        entry_lo1: (0x33 << 6) | 6,
        page_mask: 0x6000,
    };

    assert_eq!(entry.page_size(), Ok(16384));

    assert_eq!(entry.translate(0x0040_2345, false), Ok(0x0001_2345));

    assert_eq!(entry.translate(0x0040_4567, false), Ok(0x0003_0567));
}

#[test]
fn tlb_translate_miss() {
    use crate::tlb::{Tlb, TlbEntry, TlbFault};

    let mut tlb = Tlb::new();

    tlb.write(
        0,
        TlbEntry {
            entry_hi: 0x0040_0001,
            entry_lo0: 0x46,
            entry_lo1: 0x86,
            page_mask: 0,
        },
    );

    assert_eq!(tlb.translate(0x0040_0123, 1, false), Ok(0x1123));

    assert_eq!(tlb.translate(0x0040_0123, 2, false), Err(TlbFault::Miss));

    assert_eq!(tlb.translate(0x0080_0000, 1, false), Err(TlbFault::Miss));
}

#[test]
fn tlb_translate_invalid() {
    use crate::tlb::{TlbEntry, TlbFault};

    let entry = TlbEntry {
        entry_hi: 0x0040_0001,
        entry_lo0: 0x44,
        entry_lo1: 0x86,
        page_mask: 0,
    };

    assert_eq!(entry.translate(0x0040_0000, false), Err(TlbFault::Invalid));

    assert_eq!(entry.translate(0x0040_0000, true), Err(TlbFault::Invalid));

    assert_eq!(entry.translate(0x0040_1000, false), Ok(0x2000));
}

#[test]
fn tlb_translate_modified() {
    use crate::tlb::{TlbEntry, TlbFault};

    let entry = TlbEntry {
        entry_hi: 0x0040_0001,
        entry_lo0: 0x42,
        entry_lo1: 0x86,
        page_mask: 0,
    };

    // Read-only Page
    assert_eq!(entry.translate(0x0040_0000, false), Ok(0x1000));

    assert_eq!(entry.translate(0x0040_0000, true), Err(TlbFault::Modified));

    // Writable Page
    assert_eq!(entry.translate(0x0040_1000, true), Ok(0x2000));
}

#[test]
fn tlb_invalid_page_mask() {
    use crate::tlb::{TlbEntry, TlbFault};

    let entry = TlbEntry {
        entry_hi: 0,
        entry_lo0: 0x46,
        entry_lo1: 0x86,
        page_mask: 0x2000,
    };

    assert_eq!(entry.page_size(), Err(TlbFault::InvalidPageMask));

    assert_eq!(entry.translate(0, false), Err(TlbFault::InvalidPageMask));
}

#[test]
fn tlb_mapped_instruction_fetch() {
    let mut cpu = Cpu::new_with_tlb();
    let mut memory = Memory::new(8192);

    cpu.cp0_mut().write_reg(10, 0, 0x0040_0001);
    cpu.cp0_mut().write_reg(2, 0, 0x46);
    cpu.cp0_mut().write_reg(3, 0, 0);
    cpu.cp0_mut().tlb_write_indexed();

    cpu.write_pc(0x0040_0000);

    memory.write_word(0x1000, i(9, 0, 8, 100));

    cpu.step(&mut memory);

    assert_eq!(cpu.read_reg(8), 100);
    assert_eq!(cpu.read_pc(), 0x0040_0004);
    assert!(!cpu.cp0().exception_level());
}

#[test]
fn tlb_mapped_load_store() {
    let mut cpu = Cpu::new_with_tlb();
    let mut memory = Memory::new(8192);

    cpu.cp0_mut().write_reg(10, 0, 0x0040_0001);
    cpu.cp0_mut().write_reg(2, 0, 0x46);
    cpu.cp0_mut().tlb_write_indexed();

    cpu.write_pc(0x8000_0000);

    cpu.write_reg(8, 0x0040_0100);
    cpu.write_reg(9, 0xDEAD_BEEF);

    memory.write_word(0, i(0x2B, 8, 9, 0));
    memory.write_word(4, i(0x23, 8, 10, 0));

    cpu.step(&mut memory);
    cpu.step(&mut memory);

    assert_eq!(memory.read_word(0x1100), 0xDEAD_BEEF);

    assert_eq!(cpu.read_reg(10), 0xDEAD_BEEF);
    assert!(!cpu.cp0().exception_level());
}

#[test]
fn tlb_refill_exception() {
    let mut cpu = Cpu::new_with_tlb();
    let mut memory = Memory::new(1024);

    let address = 0x0040_1234;

    cpu.cp0_mut().write_reg(10, 0, 0xAB);
    cpu.cp0_mut().write_reg(4, 0, 0x8000_0000);

    cpu.write_pc(address);
    cpu.step(&mut memory);

    assert_eq!(cpu.cp0().exception_code(), 2);
    assert_eq!(cpu.cp0().read_bad_vaddr(), address);
    assert_eq!(cpu.cp0().read_epc(), address);

    assert_eq!(cpu.cp0().read_entry_hi(), 0x0040_00AB);

    assert_eq!(cpu.cp0().read_context(), 0x8000_2000);

    assert_eq!(cpu.read_pc(), 0x8000_0000);
    assert!(cpu.cp0().exception_level());
}

#[test]
fn nested_tlb_refill_exception() {
    let mut cpu = Cpu::new_with_tlb();
    let mut memory = Memory::new(1024);

    cpu.cp0_mut().write_reg(14, 0, 0x1234);
    cpu.cp0_mut().write_reg(12, 0, 2);

    cpu.write_pc(0x0040_0000);
    cpu.step(&mut memory);

    assert_eq!(cpu.cp0().exception_code(), 2);
    assert_eq!(cpu.cp0().read_epc(), 0x1234);
    assert_eq!(cpu.read_pc(), 0x8000_0180);
}

#[test]
fn tlb_invalid_exception() {
    let mut cpu = Cpu::new_with_tlb();
    let mut memory = Memory::new(1024);

    cpu.cp0_mut().write_reg(10, 0, 0x0040_0001);
    cpu.cp0_mut().write_reg(2, 0, 0x44);
    cpu.cp0_mut().tlb_write_indexed();

    cpu.write_pc(0x0040_0000);
    cpu.step(&mut memory);

    assert_eq!(cpu.cp0().exception_code(), 2);
    assert_eq!(cpu.read_pc(), 0x8000_0180);

    assert_eq!(cpu.cp0().read_bad_vaddr(), 0x0040_0000);
}

#[test]
fn tlb_modified_exception() {
    let mut cpu = Cpu::new_with_tlb();
    let mut memory = Memory::new(8192);

    cpu.cp0_mut().write_reg(10, 0, 0x0040_0001);
    cpu.cp0_mut().write_reg(2, 0, 0x42);
    cpu.cp0_mut().tlb_write_indexed();

    cpu.write_pc(0x8000_0000);

    cpu.write_reg(8, 0x0040_0000);
    cpu.write_reg(9, 0xDEAD_BEEF);

    memory.write_word(0, i(0x2B, 8, 9, 0));
    memory.write_word(0x1000, 123);

    cpu.step(&mut memory);

    assert_eq!(cpu.cp0().exception_code(), 1);
    assert_eq!(cpu.read_pc(), 0x8000_0180);
    assert_eq!(cpu.cp0().read_epc(), 0x8000_0000);

    assert_eq!(memory.read_word(0x1000), 123);
}

#[test]
fn tlb_store_refill_exception() {
    let mut cpu = Cpu::new_with_tlb();
    let mut memory = Memory::new(1024);

    cpu.write_pc(0x8000_0000);

    cpu.write_reg(8, 0x0040_0000);
    cpu.write_reg(9, 100);

    memory.write_word(0, i(0x2B, 8, 9, 0));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp0().exception_code(), 3);

    assert_eq!(cpu.cp0().read_bad_vaddr(), 0x0040_0000);

    assert_eq!(cpu.read_pc(), 0x8000_0000);
    assert_eq!(memory.read_word(0x100), 0);
    assert!(cpu.cp0().exception_level());
}

#[test]
fn complete_tlb_refill_cycle() {
    let mut cpu = Cpu::new_with_tlb();
    let mut memory = Memory::new(8192);

    // User Mode
    cpu.cp0_mut().write_reg(12, 0, 0x10);
    cpu.write_pc(0x0040_0000);

    // User Program
    memory.write_word(0x1000, i(9, 0, 8, 100));

    // TLB Refill Handler
    memory.write_word(0x00, i(9, 0, 26, 0x46));
    memory.write_word(0x04, encode_cp0(4, 26, 2, 0));
    memory.write_word(0x08, 0x42000002); // TLBWI
    memory.write_word(0x0C, 0x42000018); // ERET

    // TLB Miss
    cpu.step(&mut memory);

    assert_eq!(cpu.read_pc(), 0x8000_0000);
    assert!(cpu.cp0().is_kernel_mode());

    // Refill Handler 실행
    for _ in 0..4 {
        cpu.step(&mut memory);
    }

    assert_eq!(cpu.read_pc(), 0x0040_0000);
    assert!(!cpu.cp0().exception_level());
    assert!(!cpu.cp0().is_kernel_mode());

    // 실패했던 명령어 재실행
    cpu.step(&mut memory);

    assert_eq!(cpu.read_reg(8), 100);
    assert_eq!(cpu.read_pc(), 0x0040_0004);
}
