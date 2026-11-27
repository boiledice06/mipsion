use super::*;

#[test]
fn cp1_register_reset() {
    let cp1 = crate::cp1::Cp1::new();

    for index in 0..32 {
        assert_eq!(cp1.read_raw(index), 0);
    }
}

#[test]
fn cp1_raw_register_transfer() {
    let mut cp1 = crate::cp1::Cp1::new();

    let value = 0x1234_5678_9ABC_DEF0u64;

    cp1.write_raw(5, value);

    assert_eq!(cp1.read_raw(5), value);
}

#[test]
fn cp1_fpr_zero_is_writable() {
    let mut cp1 = crate::cp1::Cp1::new();

    cp1.write_raw(0, 0xDEAD_BEEF);

    assert_eq!(cp1.read_raw(0), 0xDEAD_BEEF);
}

#[test]
fn cp1_register_independence() {
    let mut cp1 = crate::cp1::Cp1::new();

    cp1.write_raw(30, 0xAAAA_AAAA_AAAA_AAAA);
    cp1.write_raw(31, 0xBBBB_BBBB_BBBB_BBBB);

    assert_eq!(cp1.read_raw(30), 0xAAAA_AAAA_AAAA_AAAA);
    assert_eq!(cp1.read_raw(31), 0xBBBB_BBBB_BBBB_BBBB);
    assert_eq!(cp1.read_raw(29), 0);
}

#[test]
fn cp1_control_register_reset() {
    let cp1 = crate::cp1::Cp1::new();

    assert_eq!(cp1.read_fir(), 0x0013_0000);
    assert_eq!(cp1.read_fcsr(), 0);
}

#[test]
fn cpu_cp1_integration() {
    let mut cpu = crate::cpu::Cpu::new();

    cpu.cp1_mut().write_raw(7, 0x1234_5678);

    assert_eq!(cpu.cp1().read_raw(7), 0x1234_5678);
    assert_eq!(cpu.read_reg(7), 0);
}

#[test]
fn cp1_disabled_on_reset() {
    let cpu = Cpu::new();

    assert!(!cpu.cp0().cp1_accessible());
}

#[test]
fn cp1_status_cu1_write() {
    let mut cpu = Cpu::new();

    assert!(cpu.cp0_mut().write_reg(12, 0, 1 << 29));
    assert!(cpu.cp0().cp1_accessible());

    assert!(cpu.cp0_mut().write_reg(12, 0, 0));
    assert!(!cpu.cp0().cp1_accessible());
}

#[test]
fn cp1_unusable_exception() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    // MFC1 $8, $f2
    memory.write_word(0, encode_r(0x11, 0, 8, 2, 0, 0));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp0().exception_code(), 11);
    assert_eq!(cpu.cp0().coprocessor_error(), 1);
    assert_eq!(cpu.cp0().read_epc(), 0);
    assert!(cpu.cp0().exception_level());
}

#[test]
fn cp1_unusable_prevents_register_write() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    cpu.write_reg(8, 0x1234_5678);
    cpu.cp1_mut().write_raw(2, 0xDEAD_BEEF);

    // MFC1 $8, $f2
    memory.write_word(0, encode_r(0x11, 0, 8, 2, 0, 0));

    cpu.step(&mut memory);

    assert_eq!(cpu.read_reg(8), 0x1234_5678);
    assert_eq!(cpu.cp1().read_raw(2), 0xDEAD_BEEF);
    assert_eq!(cpu.cp0().exception_code(), 11);
}

#[test]
fn cp1_memory_access_disabled() {
    for opcode in [0x31, 0x35, 0x39, 0x3D] {
        let mut cpu = Cpu::new();
        let mut memory = Memory::new(64);

        cpu.write_reg(8, 32);

        memory.write_word(0, i(opcode, 8, 2, 0));

        cpu.step(&mut memory);

        assert_eq!(cpu.cp0().exception_code(), 11);
        assert_eq!(cpu.cp0().coprocessor_error(), 1);
    }
}

#[test]
fn coprocessor_exception_number_update() {
    use crate::exception::Exception;

    let mut cpu = Cpu::new();

    cpu.raise_exception(Exception::CoprocessorUnusable(1), None);

    assert_eq!(cpu.cp0().coprocessor_error(), 1);

    cpu.raise_exception(Exception::CoprocessorUnusable(0), None);

    assert_eq!(cpu.cp0().coprocessor_error(), 0);
    assert_eq!(cpu.cp0().exception_code(), 11);
}

#[test]
fn cp1_word_read() {
    let mut cpu = Cpu::new();

    cpu.cp1_mut().write_raw(2, 0x1234_5678_9ABC_DEF0);

    assert_eq!(cpu.cp1().read_word(2), 0x9ABC_DEF0,);
}

#[test]
fn cp1_word_write() {
    let mut cpu = Cpu::new();

    cpu.cp1_mut().write_raw(2, 0x1234_5678_9ABC_DEF0);

    cpu.cp1_mut().write_word(2, 0x3F80_0000);

    assert_eq!(cpu.cp1().read_raw(2), 0x1234_5678_3F80_0000,);
}

#[test]
fn mtc1_register_transfer() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);
    cpu.write_reg(8, 0x3F80_0000);

    // MTC1 $8, $f2
    memory.write_word(0, encode_r(0x11, 4, 8, 2, 0, 0));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp1().read_word(2), 0x3F80_0000);
    assert_eq!(cpu.read_reg(8), 0x3F80_0000);
    assert!(!cpu.cp0().exception_level());
}

#[test]
fn mfc1_register_transfer() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);
    cpu.cp1_mut().write_word(2, 0xBF80_0000);

    // MFC1 $8, $f2
    memory.write_word(0, encode_r(0x11, 0, 8, 2, 0, 0));

    cpu.step(&mut memory);

    assert_eq!(cpu.read_reg(8), 0xBF80_0000);
    assert_eq!(cpu.cp1().read_word(2), 0xBF80_0000);
    assert!(!cpu.cp0().exception_level());
}

#[test]
fn cp1_transfer_round_trip() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(8);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    // NaN을 포함할 수 있는 비트열도 그대로 전달
    let value = 0x7FC0_1234;

    cpu.write_reg(8, value);

    // MTC1 $8, $f3
    memory.write_word(0, encode_r(0x11, 4, 8, 3, 0, 0));

    // MFC1 $9, $f3
    memory.write_word(4, encode_r(0x11, 0, 9, 3, 0, 0));

    cpu.step(&mut memory);
    cpu.step(&mut memory);

    assert_eq!(cpu.read_reg(9), value);
    assert_eq!(cpu.cp1().read_word(3), value);
    assert!(!cpu.cp0().exception_level());
}

#[test]
fn mfc1_preserves_gpr_zero() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);
    cpu.cp1_mut().write_word(2, 0xFFFF_FFFF);

    // MFC1 $zero, $f2
    memory.write_word(0, encode_r(0x11, 0, 0, 2, 0, 0));

    cpu.step(&mut memory);

    assert_eq!(cpu.read_reg(0), 0);
    assert!(!cpu.cp0().exception_level());
}

#[test]
fn mtc1_disabled_preserves_fpr() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    cpu.write_reg(8, 0xFFFF_FFFF);
    cpu.cp1_mut().write_word(2, 0x1234_5678);

    // MTC1 $8, $f2
    memory.write_word(0, encode_r(0x11, 4, 8, 2, 0, 0));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp1().read_word(2), 0x1234_5678,);

    assert_eq!(cpu.cp0().exception_code(), 11);
    assert_eq!(cpu.cp0().coprocessor_error(), 1);
}

#[test]
fn cp1_control_register_read() {
    let cpu = Cpu::new();

    assert_eq!(cpu.cp1().read_control(0), Some(0x0013_0000));

    for index in [25, 26, 28, 31] {
        assert_eq!(cpu.cp1().read_control(index), Some(0),);
    }

    assert_eq!(cpu.cp1().read_control(1), None);
}

#[test]
fn cp1_fcsr_write_mask() {
    let mut cpu = Cpu::new();

    assert!(cpu.cp1_mut().write_control(31, u32::MAX));

    assert_eq!(cpu.cp1().read_fcsr(), 0xFF83_FFFF,);
}

#[test]
fn cp1_fccr_alias() {
    let mut cpu = Cpu::new();

    cpu.cp1_mut().write_control(31, 0x0100_0003);
    cpu.cp1_mut().write_control(25, 0xA5);

    assert_eq!(cpu.cp1().read_control(25), Some(0xA5),);

    assert_eq!(cpu.cp1().read_fcsr(), 0xA580_0003,);
}

#[test]
fn cp1_fexr_alias() {
    let mut cpu = Cpu::new();

    cpu.cp1_mut().write_control(31, 3);
    cpu.cp1_mut().write_control(26, 0x0001_0040);

    assert_eq!(cpu.cp1().read_control(26), Some(0x0001_0040),);

    assert_eq!(cpu.cp1().read_fcsr(), 0x0001_0043,);
}

#[test]
fn cp1_fenr_alias() {
    let mut cpu = Cpu::new();

    cpu.cp1_mut().write_control(31, 0x40);
    cpu.cp1_mut().write_control(28, 0x0000_0807);

    assert_eq!(cpu.cp1().read_control(28), Some(0x0000_0807),);

    assert_eq!(cpu.cp1().read_fcsr(), 0x0100_0843,);
}

#[test]
fn cp1_fir_is_read_only() {
    let mut cpu = Cpu::new();

    let old_fir = cpu.cp1().read_fir();

    assert!(!cpu.cp1_mut().write_control(0, u32::MAX));

    assert_eq!(cpu.cp1().read_fir(), old_fir);
}

#[test]
fn cp1_control_transfer_round_trip() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(8);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);
    cpu.write_reg(8, 3);

    // CTC1 $8, $31
    memory.write_word(0, encode_r(0x11, 6, 8, 31, 0, 0));

    // CFC1 $9, $31
    memory.write_word(4, encode_r(0x11, 2, 9, 31, 0, 0));

    cpu.step(&mut memory);
    cpu.step(&mut memory);

    assert_eq!(cpu.read_reg(9), 3);
    assert_eq!(cpu.cp1().read_fcsr(), 3);
    assert!(!cpu.cp0().exception_level());
}

#[test]
fn ctc1_enabled_fp_exception() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    // Invalid Operation Cause + Enable
    let value = (1 << 16) | (1 << 11);

    cpu.write_reg(8, value);

    memory.write_word(0, encode_r(0x11, 6, 8, 31, 0, 0));

    cpu.step(&mut memory);

    // FCSR must be written before the exception
    assert_eq!(cpu.cp1().read_fcsr(), value);
    assert_eq!(cpu.cp0().exception_code(), 15);
    assert_eq!(cpu.cp0().read_epc(), 0);
    assert!(cpu.cp0().exception_level());
}

#[test]
fn ctc1_unimplemented_operation_exception() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);
    cpu.write_reg(8, 1 << 17);

    memory.write_word(0, encode_r(0x11, 6, 8, 31, 0, 0));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp1().read_fcsr(), 1 << 17);
    assert_eq!(cpu.cp0().exception_code(), 15);
    assert!(cpu.cp0().exception_level());
}

#[test]
fn cp1_doubleword_read() {
    let mut cpu = Cpu::new();

    cpu.cp1_mut().write_word(2, 0x9ABC_DEF0);
    cpu.cp1_mut().write_word(3, 0x1234_5678);

    assert_eq!(cpu.cp1().read_doubleword(2), Some(0x1234_5678_9ABC_DEF0),);
}

#[test]
fn cp1_doubleword_write() {
    let mut cpu = Cpu::new();

    assert!(cpu.cp1_mut().write_doubleword(2, 0x1234_5678_9ABC_DEF0,));

    assert_eq!(cpu.cp1().read_word(2), 0x9ABC_DEF0);
    assert_eq!(cpu.cp1().read_word(3), 0x1234_5678);
}

#[test]
fn cp1_doubleword_last_register_pair() {
    let mut cpu = Cpu::new();

    let value = 0x4009_21FB_5444_2D18;

    assert!(cpu.cp1_mut().write_doubleword(30, value));

    assert_eq!(cpu.cp1().read_doubleword(30), Some(value),);

    assert_eq!(cpu.cp1().read_word(31), 0x4009_21FB);
}

#[test]
fn cp1_invalid_doubleword_register() {
    let mut cpu = Cpu::new();

    for index in [1, 3, 31, 32] {
        assert_eq!(cpu.cp1().read_doubleword(index), None,);

        assert!(!cpu.cp1_mut().write_doubleword(index, u64::MAX));

        assert_eq!(cpu.cp1().read_high_word(index), None,);

        assert!(!cpu.cp1_mut().write_high_word(index, u32::MAX));
    }

    for index in 0..32 {
        assert_eq!(cpu.cp1().read_raw(index), 0);
    }
}

#[test]
fn mfhc1_register_transfer() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    cpu.cp1_mut().write_doubleword(2, 0x1234_5678_9ABC_DEF0);

    // MFHC1 $8, $f2
    memory.write_word(0, encode_r(0x11, 3, 8, 2, 0, 0));

    cpu.step(&mut memory);

    assert_eq!(cpu.read_reg(8), 0x1234_5678);
    assert!(!cpu.cp0().exception_level());
}

#[test]
fn mthc1_preserves_low_word() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);
    cpu.write_reg(8, 0x3FF0_0000);

    cpu.cp1_mut().write_word(2, 0x1234_5678);

    // MTHC1 $8, $f2
    memory.write_word(0, encode_r(0x11, 7, 8, 2, 0, 0));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp1().read_doubleword(2), Some(0x3FF0_0000_1234_5678),);

    assert!(!cpu.cp0().exception_level());
}

#[test]
fn cp1_doubleword_transfer_round_trip() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(16);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    cpu.write_reg(8, 0x5444_2D18);
    cpu.write_reg(9, 0x4009_21FB);

    // MTC1 $8, $f2
    memory.write_word(0, encode_r(0x11, 4, 8, 2, 0, 0));

    // MTHC1 $9, $f2
    memory.write_word(4, encode_r(0x11, 7, 9, 2, 0, 0));

    // MFC1 $10, $f2
    memory.write_word(8, encode_r(0x11, 0, 10, 2, 0, 0));

    // MFHC1 $11, $f2
    memory.write_word(12, encode_r(0x11, 3, 11, 2, 0, 0));

    for _ in 0..4 {
        cpu.step(&mut memory);
    }

    assert_eq!(cpu.read_reg(10), 0x5444_2D18);
    assert_eq!(cpu.read_reg(11), 0x4009_21FB);

    assert_eq!(cpu.cp1().read_doubleword(2), Some(0x4009_21FB_5444_2D18),);

    assert!(!cpu.cp0().exception_level());
}

#[test]
fn mfhc1_odd_register_rejected() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);
    cpu.write_reg(8, 0x1234_5678);

    // MFHC1 $8, $f3
    memory.write_word(0, encode_r(0x11, 3, 8, 3, 0, 0));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp0().exception_code(), 10);
    assert_eq!(cpu.read_reg(8), 0x1234_5678);
}

#[test]
fn mthc1_odd_register_rejected() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);
    cpu.write_reg(8, u32::MAX);

    // MTHC1 $8, $f31
    memory.write_word(0, encode_r(0x11, 7, 8, 31, 0, 0));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp0().exception_code(), 10);

    for index in 0..32 {
        assert_eq!(cpu.cp1().read_raw(index), 0);
    }
}

#[test]
fn cp1_high_word_access_disabled() {
    for rs in [3, 7] {
        let mut cpu = Cpu::new();
        let mut memory = Memory::new(4);

        cpu.write_reg(8, 0x1234_5678);

        cpu.cp1_mut().write_doubleword(2, 0xAAAA_BBBB_CCCC_DDDD);

        memory.write_word(0, encode_r(0x11, rs, 8, 2, 0, 0));

        cpu.step(&mut memory);

        assert_eq!(cpu.cp0().exception_code(), 11);
        assert_eq!(cpu.cp0().coprocessor_error(), 1);

        assert_eq!(cpu.read_reg(8), 0x1234_5678);

        assert_eq!(cpu.cp1().read_doubleword(2), Some(0xAAAA_BBBB_CCCC_DDDD),);
    }
}

#[test]
fn lwc1_word_load() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(64);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);
    cpu.write_reg(8, 32);

    memory.write_word(32, 0x3F80_0000);

    // LWC1 $f2, 0($8)
    memory.write_word(0, i(0x31, 8, 2, 0));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp1().read_word(2), 0x3F80_0000);
    assert_eq!(cpu.read_reg(8), 32);
    assert!(!cpu.cp0().exception_level());
}

#[test]
fn swc1_word_store() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(64);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);
    cpu.write_reg(8, 32);

    cpu.cp1_mut().write_word(2, 0xBF80_0000);

    // SWC1 $f2, 0($8)
    memory.write_word(0, i(0x39, 8, 2, 0));

    cpu.step(&mut memory);

    assert_eq!(memory.read_word(32), 0xBF80_0000);
    assert_eq!(cpu.cp1().read_word(2), 0xBF80_0000);
    assert!(!cpu.cp0().exception_level());
}

#[test]
fn cp1_memory_round_trip() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(64);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);
    cpu.write_reg(8, 32);

    let value = 0x7FC0_1234;

    cpu.cp1_mut().write_word(2, value);

    // SWC1 $f2, 0($8)
    memory.write_word(0, i(0x39, 8, 2, 0));

    // LWC1 $f3, 0($8)
    memory.write_word(4, i(0x31, 8, 3, 0));

    cpu.step(&mut memory);
    cpu.step(&mut memory);

    assert_eq!(memory.read_word(32), value);
    assert_eq!(cpu.cp1().read_word(3), value);
    assert!(!cpu.cp0().exception_level());
}

#[test]
fn lwc1_negative_offset() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(64);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);
    cpu.write_reg(8, 36);

    memory.write_word(32, 0x1234_5678);

    // LWC1 $f0, -4($8)
    memory.write_word(0, i(0x31, 8, 0, (-4i16) as u16));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp1().read_word(0), 0x1234_5678);
    assert!(!cpu.cp0().exception_level());
}

#[test]
fn lwc1_alignment_exception() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(64);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);
    cpu.write_reg(8, 33);

    cpu.cp1_mut().write_word(2, 0x1234_5678);

    memory.write_word(0, i(0x31, 8, 2, 0));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp0().exception_code(), 4);
    assert_eq!(cpu.cp0().read_bad_vaddr(), 33);
    assert_eq!(cpu.cp1().read_word(2), 0x1234_5678);
}

#[test]
fn swc1_alignment_exception() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(64);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);
    cpu.write_reg(8, 33);

    cpu.cp1_mut().write_word(2, u32::MAX);

    memory.write_word(0, i(0x39, 8, 2, 0));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp0().exception_code(), 5);
    assert_eq!(cpu.cp0().read_bad_vaddr(), 33);

    assert_eq!(memory.read_word(32), 0);
    assert_eq!(memory.read_word(36), 0);
}

#[test]
fn cp1_memory_unusable_priority() {
    for opcode in [0x31, 0x39] {
        let mut cpu = Cpu::new();
        let mut memory = Memory::new(64);

        // CU1 = 0, 동시에 misaligned address
        cpu.write_reg(8, 33);

        cpu.cp1_mut().write_word(2, 0x1234_5678);

        memory.write_word(0, i(opcode, 8, 2, 0));

        cpu.step(&mut memory);

        assert_eq!(cpu.cp0().exception_code(), 11);
        assert_eq!(cpu.cp0().coprocessor_error(), 1);

        assert_eq!(cpu.cp1().read_word(2), 0x1234_5678);
        assert_eq!(memory.read_word(32), 0);
        assert_eq!(memory.read_word(36), 0);
    }
}

#[test]
fn lwc1_tlb_refill_exception() {
    let mut cpu = Cpu::new_with_tlb();
    let mut memory = Memory::new(64);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    // kseg0: instruction fetch는 TLB 우회
    cpu.write_pc(0x8000_0000);

    // 매핑되지 않은 가상 주소
    cpu.write_reg(8, 0x0040_0000);

    cpu.cp1_mut().write_word(2, 0x1234_5678);

    memory.write_word(0, i(0x31, 8, 2, 0));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp0().exception_code(), 2);

    assert_eq!(cpu.cp0().read_bad_vaddr(), 0x0040_0000,);

    assert_eq!(cpu.cp0().read_epc(), 0x8000_0000,);

    assert_eq!(cpu.cp1().read_word(2), 0x1234_5678);
}

#[test]
fn swc1_tlb_refill_exception() {
    let mut cpu = Cpu::new_with_tlb();
    let mut memory = Memory::new(64);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    cpu.write_pc(0x8000_0000);
    cpu.write_reg(8, 0x0040_0000);

    cpu.cp1_mut().write_word(2, u32::MAX);

    memory.write_word(0, i(0x39, 8, 2, 0));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp0().exception_code(), 3);

    assert_eq!(cpu.cp0().read_bad_vaddr(), 0x0040_0000,);

    assert_eq!(cpu.cp0().read_epc(), 0x8000_0000,);

    assert_eq!(memory.read_word(32), 0);
    assert_eq!(cpu.cp1().read_word(2), u32::MAX);
}

#[test]
fn swc1_clears_reservation() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(64);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    cpu.write_reg(8, 32);
    cpu.set_reservation(32);

    cpu.cp1_mut().write_word(2, 0xDEAD_BEEF);

    memory.write_word(0, i(0x39, 8, 2, 0));

    assert!(cpu.check_reservation(32));

    cpu.step(&mut memory);

    assert_eq!(memory.read_word(32), 0xDEAD_BEEF);
    assert!(!cpu.check_reservation(32));
    assert!(!cpu.cp0().exception_level());
}

#[test]
fn memory_doubleword_read() {
    let mut memory = Memory::new(64);

    memory.write_word(32, 0x5444_2D18);
    memory.write_word(36, 0x4009_21FB);

    assert_eq!(memory.read_doubleword(32), 0x4009_21FB_5444_2D18,);
}

#[test]
fn memory_doubleword_write() {
    let mut memory = Memory::new(64);

    memory.write_doubleword(32, 0x1234_5678_9ABC_DEF0);

    assert_eq!(memory.read_word(32), 0x9ABC_DEF0);
    assert_eq!(memory.read_word(36), 0x1234_5678);

    let expected = [0xF0, 0xDE, 0xBC, 0x9A, 0x78, 0x56, 0x34, 0x12];

    for (index, value) in expected.into_iter().enumerate() {
        assert_eq!(memory.read_byte(32 + index as u32), value,);
    }
}

#[test]
fn ldc1_doubleword_load() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(64);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);
    cpu.write_reg(8, 32);

    memory.write_doubleword(32, 0x4009_21FB_5444_2D18);

    // LDC1 $f2, 0($8)
    memory.write_word(0, i(0x35, 8, 2, 0));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp1().read_word(2), 0x5444_2D18);
    assert_eq!(cpu.cp1().read_word(3), 0x4009_21FB);
    assert!(!cpu.cp0().exception_level());
}

#[test]
fn sdc1_doubleword_store() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(64);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);
    cpu.write_reg(8, 32);

    cpu.cp1_mut().write_doubleword(2, 0x1234_5678_9ABC_DEF0);

    // SDC1 $f2, 0($8)
    memory.write_word(0, i(0x3D, 8, 2, 0));

    cpu.step(&mut memory);

    assert_eq!(memory.read_doubleword(32), 0x1234_5678_9ABC_DEF0,);

    assert!(!cpu.cp0().exception_level());
}

#[test]
fn cp1_doubleword_memory_round_trip() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(64);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);
    cpu.write_reg(8, 40);

    let value = 0x7FF8_1234_5678_9ABC;

    cpu.cp1_mut().write_doubleword(2, value);

    // SDC1 $f2, -8($8)
    memory.write_word(0, i(0x3D, 8, 2, (-8i16) as u16));

    // LDC1 $f30, -8($8)
    memory.write_word(4, i(0x35, 8, 30, (-8i16) as u16));

    cpu.step(&mut memory);
    cpu.step(&mut memory);

    assert_eq!(memory.read_doubleword(32), value);

    assert_eq!(cpu.cp1().read_doubleword(30), Some(value),);

    assert!(!cpu.cp0().exception_level());
}

#[test]
fn ldc1_alignment_exception() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(64);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    // Word aligned, but not doubleword aligned
    cpu.write_reg(8, 36);

    let value = 0x1234_5678_9ABC_DEF0;

    cpu.cp1_mut().write_doubleword(2, value);

    memory.write_word(0, i(0x35, 8, 2, 0));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp0().exception_code(), 4);
    assert_eq!(cpu.cp0().read_bad_vaddr(), 36);

    assert_eq!(cpu.cp1().read_doubleword(2), Some(value),);
}

#[test]
fn sdc1_alignment_exception() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(64);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);
    cpu.write_reg(8, 36);

    cpu.cp1_mut().write_doubleword(2, u64::MAX);

    memory.write_word(0, i(0x3D, 8, 2, 0));

    cpu.step(&mut memory);

    assert_eq!(cpu.cp0().exception_code(), 5);
    assert_eq!(cpu.cp0().read_bad_vaddr(), 36);

    assert_eq!(memory.read_doubleword(32), 0);
    assert_eq!(memory.read_doubleword(40), 0);
}

#[test]
fn cp1_doubleword_memory_odd_register_rejected() {
    for opcode in [0x35, 0x3D] {
        let mut cpu = Cpu::new();
        let mut memory = Memory::new(64);

        cpu.cp0_mut().write_reg(12, 0, 1 << 29);
        cpu.write_reg(8, 32);

        memory.write_doubleword(32, 0x1234_5678_9ABC_DEF0);

        // Invalid doubleword register in FR=0
        memory.write_word(0, i(opcode, 8, 31, 0));

        cpu.step(&mut memory);

        assert_eq!(cpu.cp0().exception_code(), 10);

        assert_eq!(cpu.cp1().read_raw(31), 0);

        assert_eq!(memory.read_doubleword(32), 0x1234_5678_9ABC_DEF0,);
    }
}

#[test]
fn cp1_doubleword_memory_tlb_refill() {
    for (opcode, exception_code) in [(0x35, 2), (0x3D, 3)] {
        let mut cpu = Cpu::new_with_tlb();
        let mut memory = Memory::new(64);

        cpu.cp0_mut().write_reg(12, 0, 1 << 29);
        cpu.write_pc(0x8000_0000);
        cpu.write_reg(8, 0x0040_0000);

        let value = 0xAAAA_BBBB_CCCC_DDDD;

        cpu.cp1_mut().write_doubleword(2, value);

        memory.write_word(0, i(opcode, 8, 2, 0));

        cpu.step(&mut memory);

        assert_eq!(cpu.cp0().exception_code(), exception_code,);

        assert_eq!(cpu.cp0().read_bad_vaddr(), 0x0040_0000,);

        assert_eq!(cpu.cp0().read_epc(), 0x8000_0000,);

        assert_eq!(cpu.cp1().read_doubleword(2), Some(value),);

        assert_eq!(memory.read_doubleword(32), 0);
    }
}

#[test]
fn sdc1_clears_reservation() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(64);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);
    cpu.write_reg(8, 32);

    cpu.set_reservation(32);

    cpu.cp1_mut().write_doubleword(2, 0x1234_5678_9ABC_DEF0);

    memory.write_word(0, i(0x3D, 8, 2, 0));

    assert!(cpu.check_reservation(32));

    cpu.step(&mut memory);

    assert!(!cpu.check_reservation(32));

    assert_eq!(memory.read_doubleword(32), 0x1234_5678_9ABC_DEF0,);

    assert!(!cpu.cp0().exception_level());
}

#[test]
fn cp1_fir_feature_advertisement() {
    let cpu = Cpu::new();

    let fir = cpu.cp1().read_fir();

    // Supported formats: S, D, W.
    for bit in [16, 17, 20] {
        assert_ne!(fir & (1 << bit), 0);
    }

    // Unsupported formats: PS, 3D, L, F64.
    for bit in [18, 19, 21, 22] {
        assert_eq!(fir & (1 << bit), 0);
    }

    assert_eq!(fir, 0x0013_0000);
}

#[test]
fn cp0_cp1_feature_consistency() {
    let cpu = Cpu::new();

    let config1 = cpu.cp0().read_config1();
    let fir = cpu.cp1().read_fir();

    // Config1.FP advertises CP1 presence.
    assert_eq!(config1 & 1, 1);

    // Basic S/D/W formats are available.
    assert_eq!(fir & 0x0013_0000, 0x0013_0000,);

    // FR=1 register model is not available.
    assert_eq!(fir & (1 << 22), 0);

    // FPU presence does not imply access permission.
    assert!(!cpu.cp0().cp1_accessible());
}

#[test]
fn cp0_config1_fpu_discovery_instruction() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    // MFC0 $8, $16, 1
    memory.write_word(0, encode_r(0x10, 0, 8, 16, 0, 1));

    cpu.step(&mut memory);

    assert_eq!(cpu.read_reg(8), 0x3E00_0001,);

    assert!(!cpu.cp0().exception_level());
}

#[test]
fn cp1_fir_discovery_instruction() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    // CFC1 $8, $0
    memory.write_word(0, encode_r(0x11, 2, 8, 0, 0, 0));

    cpu.step(&mut memory);

    assert_eq!(cpu.read_reg(8), 0x0013_0000,);

    assert!(!cpu.cp0().exception_level());
}

#[test]
fn cp1_discovery_requires_cu1() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(4);

    // CP1 exists, but Status.CU1 is disabled.
    assert_eq!(cpu.cp0().read_config1() & 1, 1,);

    assert!(!cpu.cp0().cp1_accessible());

    cpu.write_reg(8, 0xDEAD_BEEF);

    // CFC1 $8, $0
    memory.write_word(0, encode_r(0x11, 2, 8, 0, 0, 0));

    cpu.step(&mut memory);

    // Coprocessor Unusable.
    assert_eq!(cpu.cp0().exception_code(), 11);

    // Destination must not be modified.
    assert_eq!(cpu.read_reg(8), 0xDEAD_BEEF,);

    // The exception does not remove the FPU.
    assert_eq!(cpu.cp0().read_config1() & 1, 1,);

    assert_eq!(cpu.cp1().read_fir(), 0x0013_0000,);
}
