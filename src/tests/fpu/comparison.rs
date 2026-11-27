use super::*;

#[test]
fn cp1_fcc_register_mapping() {
    let mut cpu = Cpu::new();

    for cc in 0..8u8 {
        cpu.cp1_mut().write_fcc(cc, true);

        assert!(cpu.cp1().read_fcc(cc));
    }

    assert_eq!(cpu.cp1().read_control(25), Some(0xFF),);

    assert_eq!(cpu.cp1().read_fcsr(), 0xFE80_0000,);

    for cc in 0..8u8 {
        cpu.cp1_mut().write_fcc(cc, false);

        assert!(!cpu.cp1().read_fcc(cc));
    }

    assert_eq!(cpu.cp1().read_fcsr(), 0);
}

#[test]
fn cp1_compare_all_conditions() {
    for fmt in [0x10, 0x11] {
        for cond in 0..16u8 {
            for (lhs, rhs, relation) in [
                (1.0f64, 2.0f64, 4u8),
                (2.0, 2.0, 2),
                (3.0, 2.0, 0),
                (-0.0, 0.0, 2),
            ] {
                let mut cpu = Cpu::new();
                let mut memory = Memory::new(4);

                cpu.cp0_mut().write_reg(12, 0, 1 << 29);

                if fmt == 0x10 {
                    cpu.cp1_mut().write_word(2, (lhs as f32).to_bits());

                    cpu.cp1_mut().write_word(4, (rhs as f32).to_bits());
                } else {
                    cpu.cp1_mut().write_doubleword(2, lhs.to_bits());

                    cpu.cp1_mut().write_doubleword(4, rhs.to_bits());
                }

                memory.write_word(0, encode_fp_compare(fmt, 2, 4, 3, cond));

                cpu.step(&mut memory);

                assert_eq!(
                    cpu.cp1().read_fcc(3),
                    cond & relation != 0,
                    "fmt={fmt:#x}, cond={cond:#x}",
                );

                assert!(!cpu.cp0().exception_level());
            }
        }
    }
}

#[test]
fn cp1_compare_all_fcc_destinations() {
    for fmt in [0x10, 0x11] {
        for cc in 0..8u8 {
            let mut cpu = Cpu::new();
            let mut memory = Memory::new(4);

            cpu.cp0_mut().write_reg(12, 0, 1 << 29);

            // 초기 FPR 값은 모두 zero
            // C.EQ.S / C.EQ.D
            memory.write_word(0, encode_fp_compare(fmt, 2, 4, cc, 2));

            cpu.step(&mut memory);

            for other in 0..8u8 {
                assert_eq!(cpu.cp1().read_fcc(other), other == cc,);
            }

            assert!(!cpu.cp0().exception_level());
        }
    }
}

#[test]
fn cp1_compare_legacy_nan() {
    for fmt in [0x10, 0x11] {
        for cond in 0..16u8 {
            for signaling in [false, true] {
                let mut cpu = Cpu::new();
                let mut memory = Memory::new(4);

                cpu.cp0_mut().write_reg(12, 0, 1 << 29);

                if fmt == 0x10 {
                    let nan = if signaling { 0x7FC0_0000 } else { 0x7FBF_FFFF };

                    cpu.cp1_mut().write_word(2, nan);
                } else {
                    let nan = if signaling {
                        0x7FF8_0000_0000_0000
                    } else {
                        0x7FF7_FFFF_FFFF_FFFF
                    };

                    cpu.cp1_mut().write_doubleword(2, nan);
                }

                memory.write_word(0, encode_fp_compare(fmt, 2, 4, 0, cond));

                cpu.step(&mut memory);

                assert_eq!(cpu.cp1().read_fcc(0), cond & 1 != 0,);

                let invalid = signaling || cond & 8 != 0;

                let fcsr = cpu.cp1().read_fcsr();

                assert_eq!(fcsr & (1 << 16) != 0, invalid,);

                assert_eq!(fcsr & (1 << 6) != 0, invalid,);

                assert!(!cpu.cp0().exception_level());
            }
        }
    }
}

#[test]
fn cp1_compare_trap_preserves_fcc() {
    for fmt in [0x10, 0x11] {
        let mut cpu = Cpu::new();
        let mut memory = Memory::new(4);

        cpu.cp0_mut().write_reg(12, 0, 1 << 29);

        cpu.cp1_mut().write_control(31, 1 << 11);
        cpu.cp1_mut().write_fcc(3, true);

        if fmt == 0x10 {
            cpu.cp1_mut().write_word(2, 0x7FC0_0000);
        } else {
            cpu.cp1_mut().write_doubleword(2, 0x7FF8_0000_0000_0000);
        }

        // C.EQ: unordered이면 false이지만
        // trap 발생 시 기존 FCC는 유지되어야 한다.
        memory.write_word(0, encode_fp_compare(fmt, 2, 4, 3, 2));

        cpu.step(&mut memory);

        assert_eq!(cpu.cp0().exception_code(), 15);
        assert_eq!(cpu.cp0().read_epc(), 0);
        assert!(cpu.cp1().read_fcc(3));

        let fcsr = cpu.cp1().read_fcsr();

        assert_ne!(fcsr & (1 << 16), 0);
        assert_eq!(fcsr & (1 << 6), 0);
    }
}

#[test]
fn cp1_compare_disabled() {
    for fmt in [0x10, 0x11] {
        for cond in 0..16u8 {
            let mut cpu = Cpu::new();
            let mut memory = Memory::new(4);

            cpu.cp1_mut().write_fcc(3, true);

            let old_fcsr = cpu.cp1().read_fcsr();

            memory.write_word(0, encode_fp_compare(fmt, 2, 4, 3, cond));

            cpu.step(&mut memory);

            assert_eq!(cpu.cp0().exception_code(), 11,);

            assert_eq!(cpu.cp0().coprocessor_error(), 1,);

            assert_eq!(cpu.cp1().read_fcsr(), old_fcsr,);
        }
    }
}

#[test]
fn cp1_compare_invalid_double_register() {
    for cond in 0..16u8 {
        for (fs, ft) in [(3, 4), (2, 5)] {
            let mut cpu = Cpu::new();
            let mut memory = Memory::new(4);

            cpu.cp0_mut().write_reg(12, 0, 1 << 29);

            cpu.cp1_mut().write_fcc(3, true);

            let old_fcsr = cpu.cp1().read_fcsr();

            memory.write_word(0, encode_fp_compare(0x11, fs, ft, 3, cond));

            cpu.step(&mut memory);

            assert_eq!(cpu.cp0().exception_code(), 10,);

            assert_eq!(cpu.cp1().read_fcsr(), old_fcsr,);
        }
    }
}

#[test]
fn cp1_branch_all_conditions() {
    for cc in 0..8u8 {
        for condition in [false, true] {
            for expected in [false, true] {
                for likely in [false, true] {
                    let mut cpu = Cpu::new();
                    let mut memory = Memory::new(16);

                    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

                    cpu.cp1_mut().write_fcc(cc, condition);

                    let old_fcsr = cpu.cp1().read_fcsr();

                    memory.write_word(0, encode_fp_branch(cc, likely, expected, 2));

                    // Delay slot
                    memory.write_word(4, i(9, 0, 8, 1));

                    // Fall-through
                    memory.write_word(8, i(9, 0, 9, 2));

                    // Branch target
                    memory.write_word(12, i(9, 0, 10, 3));

                    let taken = condition == expected;

                    cpu.step(&mut memory);

                    if taken {
                        assert_eq!(cpu.read_pc(), 4);
                        assert_eq!(cpu.read_next_pc(), 12);

                        cpu.step(&mut memory);

                        assert_eq!(cpu.read_pc(), 12);
                        assert_eq!(cpu.read_reg(8), 1);

                        cpu.step(&mut memory);

                        assert_eq!(cpu.read_reg(9), 0);
                        assert_eq!(cpu.read_reg(10), 3);
                    } else if likely {
                        assert_eq!(cpu.read_pc(), 8);

                        cpu.step(&mut memory);

                        assert_eq!(cpu.read_reg(8), 0);
                        assert_eq!(cpu.read_reg(9), 2);
                    } else {
                        assert_eq!(cpu.read_pc(), 4);

                        cpu.step(&mut memory);

                        assert_eq!(cpu.read_reg(8), 1);
                        assert_eq!(cpu.read_pc(), 8);

                        cpu.step(&mut memory);

                        assert_eq!(cpu.read_reg(9), 2);
                    }

                    assert_eq!(cpu.cp1().read_fcsr(), old_fcsr,);

                    assert!(!cpu.cp0().exception_level());
                }
            }
        }
    }
}

#[test]
fn cp1_branch_backward() {
    for likely in [false, true] {
        for expected in [false, true] {
            let mut cpu = Cpu::new();
            let mut memory = Memory::new(12);

            cpu.cp0_mut().write_reg(12, 0, 1 << 29);

            cpu.cp1_mut().write_fcc(7, expected);

            memory.write_word(0, i(9, 8, 8, 1));

            memory.write_word(4, encode_fp_branch(7, likely, expected, -2));

            memory.write_word(8, i(9, 9, 9, 1));

            for _ in 0..6 {
                cpu.step(&mut memory);
            }

            assert_eq!(cpu.read_reg(8), 2);
            assert_eq!(cpu.read_reg(9), 2);
            assert_eq!(cpu.read_pc(), 0);

            assert!(!cpu.cp0().exception_level());
        }
    }
}

#[test]
fn cp1_branch_disabled() {
    for expected in [false, true] {
        for likely in [false, true] {
            let mut cpu = Cpu::new();
            let mut memory = Memory::new(8);

            cpu.cp1_mut().write_fcc(3, expected);

            let old_fcsr = cpu.cp1().read_fcsr();

            memory.write_word(0, encode_fp_branch(3, likely, expected, 2));

            memory.write_word(4, i(9, 0, 8, 1));

            cpu.step(&mut memory);

            assert_eq!(cpu.cp0().exception_code(), 11);

            assert_eq!(cpu.cp0().coprocessor_error(), 1,);

            assert_eq!(cpu.cp0().read_epc(), 0);
            assert_eq!(cpu.read_reg(8), 0);

            assert_eq!(cpu.cp1().read_fcsr(), old_fcsr,);
        }
    }
}

#[test]
fn cp1_branch_delay_slot_exception() {
    for (likely, condition) in [(false, true), (false, false), (true, true)] {
        let mut cpu = Cpu::new();
        let mut memory = Memory::new(16);

        cpu.cp0_mut().write_reg(12, 0, 1 << 29);

        cpu.cp1_mut().write_fcc(0, condition);

        memory.write_word(0, encode_fp_branch(0, likely, true, 2));

        // SYSCALL in delay slot
        memory.write_word(4, 0x0000_000C);

        cpu.step(&mut memory);
        cpu.step(&mut memory);

        assert_eq!(cpu.cp0().exception_code(), 8);
        assert_eq!(cpu.cp0().read_epc(), 0);
        assert!(cpu.cp0().exception_level());
    }
}

#[test]
fn cp1_branch_likely_annuls_exception() {
    for expected in [false, true] {
        let mut cpu = Cpu::new();
        let mut memory = Memory::new(16);

        cpu.cp0_mut().write_reg(12, 0, 1 << 29);

        cpu.cp1_mut().write_fcc(5, !expected);

        memory.write_word(0, encode_fp_branch(5, true, expected, 2));

        // 실행되면 예외가 발생해야 하는 명령어
        memory.write_word(4, 0x0000_000C);

        memory.write_word(8, i(9, 0, 8, 42));

        cpu.step(&mut memory);

        assert_eq!(cpu.read_pc(), 8);

        cpu.step(&mut memory);

        assert_eq!(cpu.read_reg(8), 42);
        assert!(!cpu.cp0().exception_level());
    }
}

#[test]
fn cp1_compare_branch_integration() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(20);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    cpu.cp1_mut().write_word(2, 1.0f32.to_bits());

    cpu.cp1_mut().write_word(4, 2.0f32.to_bits());

    let program = [
        // C.OLT.S $fcc3, $f2, $f4
        encode_fp_compare(0x10, 2, 4, 3, 4),
        // BC1T $fcc3, target
        encode_fp_branch(3, false, true, 2),
        // Delay slot
        i(9, 0, 8, 1),
        // Fall-through (skipped)
        i(9, 0, 9, 2),
        // Branch target
        i(9, 0, 10, 3),
    ];

    for (index, word) in program.into_iter().enumerate() {
        memory.write_word((index * 4) as u32, word);
    }

    for _ in 0..4 {
        cpu.step(&mut memory);
    }

    assert!(cpu.cp1().read_fcc(3));

    assert_eq!(cpu.read_reg(8), 1);
    assert_eq!(cpu.read_reg(9), 0);
    assert_eq!(cpu.read_reg(10), 3);

    assert!(!cpu.cp0().exception_level());
}

#[test]
fn cp1_fcc_conditional_fpr_move() {
    for fmt in [0x10, 0x11] {
        for cc in 0..8u8 {
            for condition in [false, true] {
                for expected in [false, true] {
                    let mut cpu = Cpu::new();
                    let mut memory = Memory::new(4);

                    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

                    cpu.cp1_mut().write_fcc(cc, condition);

                    let source = 0x7FF7_1234_5678_9ABCu64;
                    let sentinel = 0x1234_5678_DEAD_BEEFu64;

                    cpu.cp1_mut().write_doubleword(2, source);

                    cpu.cp1_mut().write_doubleword(4, sentinel);

                    let old_fcsr = cpu.cp1().read_fcsr();

                    let rt = (cc << 2) | expected as u8;

                    memory.write_word(0, encode_r(0x11, fmt, rt as u32, 2, 4, 0x11));

                    cpu.step(&mut memory);

                    let moved = condition == expected;

                    let result = if !moved {
                        sentinel
                    } else if fmt == 0x10 {
                        (sentinel & 0xFFFF_FFFF_0000_0000) | (source & 0xFFFF_FFFF)
                    } else {
                        source
                    };

                    assert_eq!(cpu.cp1().read_doubleword(4), Some(result),);

                    assert_eq!(cpu.cp1().read_fcsr(), old_fcsr,);

                    assert!(!cpu.cp0().exception_level());
                    assert_eq!(cpu.read_pc(), 4);
                }
            }
        }
    }
}

#[test]
fn cp1_gpr_conditional_fpr_move() {
    for fmt in [0x10, 0x11] {
        for funct in [0x12, 0x13] {
            for condition in [0u32, 1, u32::MAX] {
                let mut cpu = Cpu::new();
                let mut memory = Memory::new(4);

                cpu.cp0_mut().write_reg(12, 0, 1 << 29);

                cpu.write_reg(8, condition);

                let source = 0x8000_0000_1234_5678u64;
                let sentinel = 0xABCD_EF01_DEAD_BEEFu64;

                cpu.cp1_mut().write_doubleword(2, source);

                cpu.cp1_mut().write_doubleword(4, sentinel);

                memory.write_word(0, encode_r(0x11, fmt, 8, 2, 4, funct));

                cpu.step(&mut memory);

                let moved = if funct == 0x12 {
                    condition == 0
                } else {
                    condition != 0
                };

                let result = if !moved {
                    sentinel
                } else if fmt == 0x10 {
                    (sentinel & 0xFFFF_FFFF_0000_0000) | (source & 0xFFFF_FFFF)
                } else {
                    source
                };

                assert_eq!(cpu.cp1().read_doubleword(4), Some(result),);

                assert_eq!(cpu.read_reg(8), condition);
                assert_eq!(cpu.cp1().read_fcsr(), 0);
                assert!(!cpu.cp0().exception_level());
            }
        }
    }
}

#[test]
fn cp1_fcc_conditional_gpr_move() {
    for cc in 0..8u8 {
        for condition in [false, true] {
            for expected in [false, true] {
                let mut cpu = Cpu::new();
                let mut memory = Memory::new(4);

                cpu.cp0_mut().write_reg(12, 0, 1 << 29);

                cpu.cp1_mut().write_fcc(cc, condition);

                cpu.write_reg(8, 0x1234_5678);
                cpu.write_reg(9, 0xDEAD_BEEF);

                let rt = (cc << 2) | expected as u8;

                memory.write_word(0, encode_r(0, 8, rt as u32, 9, 0, 1));

                let old_fcsr = cpu.cp1().read_fcsr();

                cpu.step(&mut memory);

                let result = if condition == expected {
                    0x1234_5678
                } else {
                    0xDEAD_BEEF
                };

                assert_eq!(cpu.read_reg(9), result);
                assert_eq!(cpu.read_reg(8), 0x1234_5678);

                assert_eq!(cpu.cp1().read_fcsr(), old_fcsr,);

                assert!(!cpu.cp0().exception_level());
            }
        }
    }
}

#[test]
fn cp1_conditional_move_preserves_raw_bits() {
    for bits in [
        0x0000_0000u32,
        0x8000_0000,
        0x7FBF_FFFF,
        0x7FC0_0000,
        0xFFFF_FFFF,
    ] {
        let mut cpu = Cpu::new();
        let mut memory = Memory::new(4);

        cpu.cp0_mut().write_reg(12, 0, 1 << 29);

        cpu.cp1_mut().write_word(3, bits);
        cpu.cp1_mut().write_word(4, 0xDEAD_BEEF);

        cpu.cp1_mut().write_fcc(7, true);

        let old_fcsr = cpu.cp1().read_fcsr();

        // MOVT.S $f5, $f3, $fcc7
        memory.write_word(0, encode_r(0x11, 0x10, 29, 3, 5, 0x11));

        cpu.step(&mut memory);

        assert_eq!(cpu.cp1().read_word(5), bits);
        assert_eq!(cpu.cp1().read_word(3), bits);
        assert_eq!(cpu.cp1().read_word(4), 0xDEAD_BEEF);

        assert_eq!(cpu.cp1().read_fcsr(), old_fcsr,);

        assert!(!cpu.cp0().exception_level());
    }
}

#[test]
fn cp1_conditional_move_zero_register() {
    for expected in [false, true] {
        let mut cpu = Cpu::new();
        let mut memory = Memory::new(4);

        cpu.cp0_mut().write_reg(12, 0, 1 << 29);

        cpu.cp1_mut().write_fcc(0, expected);
        cpu.write_reg(8, 0xFFFF_FFFF);

        memory.write_word(0, encode_r(0, 8, expected as u32, 0, 0, 1));

        cpu.step(&mut memory);

        assert_eq!(cpu.read_reg(0), 0);
        assert!(!cpu.cp0().exception_level());
    }
}

#[test]
fn cp1_conditional_move_disabled() {
    let instructions = [
        // MOVF / MOVT
        encode_r(0, 8, 0, 9, 0, 1),
        encode_r(0, 8, 1, 9, 0, 1),
        // MOVF.S / MOVT.S
        encode_r(0x11, 0x10, 0, 2, 4, 0x11),
        encode_r(0x11, 0x10, 1, 2, 4, 0x11),
        // MOVF.D / MOVT.D
        encode_r(0x11, 0x11, 0, 2, 4, 0x11),
        encode_r(0x11, 0x11, 1, 2, 4, 0x11),
        // MOVZ.S / MOVN.S
        encode_r(0x11, 0x10, 8, 2, 4, 0x12),
        encode_r(0x11, 0x10, 8, 2, 4, 0x13),
        // MOVZ.D / MOVN.D
        encode_r(0x11, 0x11, 8, 2, 4, 0x12),
        encode_r(0x11, 0x11, 8, 2, 4, 0x13),
    ];

    for word in instructions {
        let mut cpu = Cpu::new();
        let mut memory = Memory::new(4);

        cpu.write_reg(8, 1);
        cpu.write_reg(9, 0xDEAD_BEEF);

        let sentinel = 0x1234_5678_9ABC_DEF0u64;

        cpu.cp1_mut().write_doubleword(4, sentinel);

        memory.write_word(0, word);

        cpu.step(&mut memory);

        assert_eq!(cpu.cp0().exception_code(), 11);
        assert_eq!(cpu.cp0().coprocessor_error(), 1);

        assert_eq!(cpu.read_reg(9), 0xDEAD_BEEF);

        assert_eq!(cpu.cp1().read_doubleword(4), Some(sentinel),);

        assert_eq!(cpu.cp1().read_fcsr(), 0);
    }
}

#[test]
fn cp1_conditional_move_invalid_double_register() {
    for (rt, funct) in [(0, 0x11), (1, 0x11), (8, 0x12), (8, 0x13)] {
        for (fs, fd) in [(3, 4), (2, 5)] {
            let mut cpu = Cpu::new();
            let mut memory = Memory::new(4);

            cpu.cp0_mut().write_reg(12, 0, 1 << 29);

            let sentinel = 0x1234_5678_DEAD_BEEFu64;

            cpu.cp1_mut().write_doubleword(4, sentinel);

            memory.write_word(0, encode_r(0x11, 0x11, rt, fs, fd, funct));

            cpu.step(&mut memory);

            assert_eq!(cpu.cp0().exception_code(), 10);

            assert_eq!(cpu.cp1().read_doubleword(4), Some(sentinel),);

            assert_eq!(cpu.cp1().read_fcsr(), 0);
        }
    }
}

#[test]
fn cp1_conditional_move_integration() {
    let mut cpu = Cpu::new();
    let mut memory = Memory::new(28);

    cpu.cp0_mut().write_reg(12, 0, 1 << 29);

    cpu.cp1_mut().write_word(2, 3.0f32.to_bits());

    cpu.cp1_mut().write_word(4, 5.0f32.to_bits());

    cpu.write_reg(8, 1);

    let program = [
        // C.OLT.S $fcc3, $f2, $f4
        encode_fp_compare(0x10, 2, 4, 3, 4),
        // MOVT.S $f6, $f4, $fcc3
        encode_r(0x11, 0x10, 13, 4, 6, 0x11),
        // MOVF.S $f6, $f2, $fcc3
        encode_r(0x11, 0x10, 12, 2, 6, 0x11),
        // ADD.S $f8, $f6, $f2
        encode_r(0x11, 0x10, 2, 6, 8, 0),
        // MOVN.S $f10, $f8, $8
        encode_r(0x11, 0x10, 8, 8, 10, 0x13),
        // MFC1 $9, $f10
        encode_r(0x11, 0, 9, 10, 0, 0),
        // MOVT $10, $9, $fcc3
        encode_r(0, 9, 13, 10, 0, 1),
    ];

    for (index, word) in program.into_iter().enumerate() {
        memory.write_word((index * 4) as u32, word);
    }

    for _ in 0..7 {
        cpu.step(&mut memory);
    }

    assert!(cpu.cp1().read_fcc(3));

    assert_eq!(cpu.cp1().read_word(6), 5.0f32.to_bits(),);

    assert_eq!(cpu.cp1().read_word(10), 8.0f32.to_bits(),);

    assert_eq!(cpu.read_reg(10), 8.0f32.to_bits(),);

    assert_eq!(cpu.read_pc(), 28);
    assert!(!cpu.cp0().exception_level());
}
