//! Regression tests for the current integer core. Include from main.rs with
//! #[cfg(test)] mod regression;
use crate::test_helpers::{encode_i as i, encode_r, run};

#[test]
fn register_alu() {
    for (funct, expected) in [
        (0x20, 22),
        (0x21, 22),
        (0x22, 2),
        (0x23, 2),
        (0x24, 8),
        (0x25, 14),
        (0x26, 6),
        (0x27, 0xfffffff1),
    ] {
        let cpu = run(&[encode_r(0, 8, 9, 10, 0, funct)], &[(8, 12), (9, 10)]);
        assert_eq!(cpu.read_reg(10), expected, "funct {funct:#x}");
    }
}
#[test]
fn negative_subtraction() {
    for funct in [0x22, 0x23] {
        assert_eq!(
            run(&[encode_r(0, 8, 9, 10, 0, funct)], &[(8, 10), (9, 12)]).read_reg(10),
            0xfffffffe
        );
    }
}
#[test]
fn immediate_add_sign_extension() {
    for op in [8, 9] {
        assert_eq!(
            run(&[i(op, 8, 10, 0xffff)], &[(8, 0)]).read_reg(10),
            0xffffffff
        );
    }
}
#[test]
fn comparison_signed_unsigned() {
    for (a, b, s, u) in [
        (0xffffffff, 1, 1, 0),
        (1, 0xffffffff, 0, 1),
        (10, 10, 0, 0),
        (10, 20, 1, 1),
    ] {
        let cpu = run(
            &[
                encode_r(0, 8, 9, 10, 0, 0x2a),
                encode_r(0, 8, 9, 11, 0, 0x2b),
            ],
            &[(8, a), (9, b)],
        );
        assert_eq!(cpu.read_reg(10), s);
        assert_eq!(cpu.read_reg(11), u);
    }
}
#[test]
fn shifts() {
    for (funct, expected) in [(0, 0x20), (2, 0x20000002), (3, 0xe0000002)] {
        assert_eq!(
            run(&[encode_r(0, 0, 8, 9, 2, funct)], &[(8, 0x80000008)]).read_reg(9),
            expected
        );
    }
    for funct in [0, 2, 3] {
        assert_eq!(
            run(&[encode_r(0, 0, 8, 9, 0, funct)], &[(8, 0x80000008)]).read_reg(9),
            0x80000008
        );
    }
}
#[test]
fn zero_register() {
    let cpu = run(&[i(9, 0, 0, 123)], &[(0, 42)]);
    assert_eq!(cpu.read_reg(0), 0);
}
#[test]
fn unsigned_wrap() {
    assert_eq!(
        run(&[encode_r(0, 8, 9, 10, 0, 0x21)], &[(8, u32::MAX), (9, 1)]).read_reg(10),
        0
    );
}

#[test]
fn decode_current_instructions() {
    use crate::decoder::decode;

    for (word, mnemonic) in [
        (0x0109_5020, "add"),
        (0x0109_5021, "addu"),
        (0x210a_ffff, "addi"),
        (0x250a_ffff, "addiu"),
        (0x0109_5022, "sub"),
        (0x0109_5023, "subu"),
        (0x0109_5024, "and"),
        (0x0109_5025, "or"),
        (0x0109_5026, "xor"),
        (0x0109_5027, "nor"),
        (0x0109_502a, "slt"),
        (0x0109_502b, "sltu"),
        (0x0008_4880, "sll"),
        (0x0008_4882, "srl"),
        (0x0008_4883, "sra"),
    ] {
        assert_eq!(decode(word).map(|inst| inst.mnemonic), Some(mnemonic));
    }
}

#[test]
fn decode_reserved_fields() {
    use crate::decoder::decode;

    for word in [0x0109_5060, 0x0028_4880, 0x0028_4883] {
        assert!(decode(word).is_none(), "{word:#010x}");
    }
    assert_ne!(decode(0x0028_4882).map(|inst| inst.mnemonic), Some("srl"));
}

#[test]
fn machine_word_encodings() {
    use crate::test_helpers::encode_j;

    assert_eq!(encode_r(0, 8, 9, 10, 0, 0x20), 0x0109_5020);
    assert_eq!(encode_r(0x1c, 8, 9, 10, 0, 2), 0x7109_5002);
    assert_eq!(i(0x09, 8, 10, 0xffff), 0x250a_ffff);
    assert_eq!(encode_j(0x02, 0x0010_0000), 0x0810_0000);
}

#[test]
fn word_boundaries() {
    let cpu = run(
        &[
            encode_r(0, 8, 9, 10, 0, 0x23),
            i(9, 8, 11, 0x8000),
            i(9, 8, 12, 0x7fff),
            encode_r(0, 0, 9, 13, 31, 0),
            encode_r(0, 0, 14, 15, 31, 2),
            encode_r(0, 0, 14, 16, 31, 3),
        ],
        &[(9, 1), (14, 0x8000_0000)],
    );
    assert_eq!(cpu.read_reg(10), u32::MAX);
    assert_eq!(cpu.read_reg(11), 0xffff_8000);
    assert_eq!(cpu.read_reg(12), 0x7fff);
    assert_eq!(cpu.read_reg(13), 0x8000_0000);
    assert_eq!(cpu.read_reg(15), 1);
    assert_eq!(cpu.read_reg(16), u32::MAX);
}
