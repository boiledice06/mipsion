use super::*;

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
fn decode_synchronization() {
    use crate::decoder::decode;

    for (word, mnemonic) in [
        (0x00000040, "ssnop"),
        (0x000000C0, "ehb"),
        (0x00000140, "pause"),
        (0x0000000F, "sync"),
        (0x051F0000, "synci"),
        (0xCD000000, "pref"),
        (0x01000408, "jr.hb"),
        (0x01004C09, "jalr.hb"),
    ] {
        assert_eq!(
            decode(word).map(|inst| inst.mnemonic),
            Some(mnemonic),
            "word: {word:#010x}"
        );
    }
}

#[test]
fn decode_cp0_transfer() {
    use crate::decoder::decode;

    for (word, mnemonic) in [
        (0x40086000, "mfc0"),
        (0x40886000, "mtc0"),
        (0x40082002, "mfc0"),
        (0x40882002, "mtc0"),
    ] {
        assert_eq!(decode(word).map(|inst| inst.mnemonic), Some(mnemonic));
    }
}

#[test]
fn decode_interrupt_control() {
    use crate::decoder::decode;

    for (word, mnemonic) in [
        (0x41606000, "di"),
        (0x41606020, "ei"),
        (0x41686000, "di"),
        (0x41686020, "ei"),
    ] {
        assert_eq!(decode(word).map(|inst| inst.mnemonic), Some(mnemonic));
    }
}

#[test]
fn decode_eret() {
    use crate::decoder::decode;

    assert_eq!(decode(0x42000018).map(|inst| inst.mnemonic), Some("eret"));

    assert_ne!(decode(0x42010018).map(|inst| inst.mnemonic), Some("eret"));
}

#[test]
fn decode_tlb_instructions() {
    use crate::decoder::decode;

    for (word, mnemonic) in [
        (0x42000001, "tlbr"),
        (0x42000002, "tlbwi"),
        (0x42000006, "tlbwr"),
        (0x42000008, "tlbp"),
    ] {
        assert_eq!(decode(word).map(|inst| inst.mnemonic), Some(mnemonic));
    }
}
