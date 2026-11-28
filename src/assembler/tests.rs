use super::{ErrorKind, assemble};
use crate::isa::decoder::decode;
use crate::isa::disassembler::disassemble;
use crate::isa::mips32r2::INSTRUCTIONS;

fn words(source: &str, pc: u32) -> Vec<u32> {
    let assembly = assemble(source, pc).unwrap();
    let (words, remainder) = assembly.bytes.as_chunks::<4>();
    assert!(remainder.is_empty());
    words.iter().copied().map(u32::from_le_bytes).collect()
}

#[test]
fn independent_integer_system_and_fpu_words() {
    let source = "addu $t0, $t1, $t2\naddiu $8, $0, -1\nrotr $t0, $t1, 4\nclz $t0, $t1\n                  mfc0 $t0, $12, 1\nadd.d $f0, $f2, $f4\nc.eq.s $fcc3, $f1, $f0\nldc1 $f2, -8($sp)";
    assert_eq!(
        words(source, 0),
        [
            0x012a_4021,
            0x2408_ffff,
            0x0029_4102,
            0x7128_4020,
            0x4008_6001,
            0x4624_1000,
            0x4600_0b32,
            0xd7a2_fff8,
        ]
    );
}

#[test]
fn all_execution_path_definitions_have_canonical_assembly() {
    use super::encoder::ASSEMBLY_FORMATS;
    assert_eq!(ASSEMBLY_FORMATS.len(), 200);
    for &(mnemonic, _) in ASSEMBLY_FORMATS {
        let definition = INSTRUCTIONS
            .iter()
            .find(|entry| entry.mnemonic == mnemonic)
            .unwrap();
        // Select architecturally valid operands where zero is restricted.
        let representative = definition.pattern
            | match mnemonic {
                "jalr" | "jalr.hb" => 9 << 21,
                "ctc1" => 31 << 11,
                _ => 0,
            };
        let source = disassemble(representative, 0);
        assert!(
            !source.starts_with(".word"),
            "missing canonical form for {mnemonic}"
        );
        let word = words(&source, 0)[0];
        assert_eq!(
            decode(word).unwrap().mnemonic,
            decode(definition.pattern).unwrap().mnemonic,
            "{source}"
        );
        assert_eq!(disassemble(word, 0), source, "{mnemonic}");
    }
}

#[test]
fn forward_and_backward_references_and_stable_pseudos() {
    assert_eq!(
        words(
            "top: b end\nnop\nli $t1, -1\nend: b top\nnop\nmove $s8, $t1",
            0x1000
        ),
        [
            0x1000_0003,
            0,
            0x3c09_ffff,
            0x3529_ffff,
            0x1000_fffb,
            0,
            0x0120_f021,
        ]
    );
}

#[test]
fn sections_are_contiguous_and_cross_section_data_is_little_endian() {
    let assembly = assemble(
        ".data\nptr: .word start\n.byte -1, 2\n.align 2\n.half 0xABCD\n.ascii \"a#\"\n.asciiz \"B\", \"C\"\n.space 2\n.text\nstart: la $t0, ptr\n",
        0x1000,
    ).unwrap();
    assert_eq!(assembly.text_size, 8);
    assert_eq!(assembly.data_offset, 8);
    assert_eq!(assembly.symbols["start"], 0x1000);
    assert_eq!(assembly.symbols["ptr"], 0x1008);
    assert_eq!(&assembly.bytes[..8], &[0, 0, 8, 60, 8, 16, 8, 53]);
    assert_eq!(
        &assembly.bytes[8..],
        &[
            0, 16, 0, 0, 255, 2, 0, 0, 0xcd, 0xab, b'a', b'#', b'B', 0, b'C', 0, 0, 0
        ]
    );
}

#[test]
fn rejects_bad_address_immediates_registers_and_directives() {
    for (source, kind) in [
        ("addi $t0, $t1, 32768", ErrorKind::OutOfRange),
        ("andi $t0, $t1, -1", ErrorKind::OutOfRange),
        ("sll $t0, $t1, 32", ErrorKind::OutOfRange),
        ("ext $t0, $t1, 31, 2", ErrorKind::OutOfRange),
        ("beq $t0, $0, 0x20004", ErrorKind::OutOfRange),
        ("j 0x10000000", ErrorKind::OutOfRange),
        ("beq $t0, $0, 3", ErrorKind::Misaligned),
        ("lw $32, -4($sp)", ErrorKind::InvalidOperand),
        ("add.d $f1, $f2, $f4", ErrorKind::InvalidOperand),
        ("add.s $f0, $f32, $f4", ErrorKind::OutOfRange),
        ("j missing", ErrorKind::UndefinedSymbol),
        ("wait", ErrorKind::UnsupportedInstruction),
        ("add.ps $f0, $f2, $f4", ErrorKind::UnsupportedInstruction),
        ("x: nop\nx: nop", ErrorKind::DuplicateSymbol),
        (".byte 256", ErrorKind::OutOfRange),
        (".half 1\nnop", ErrorKind::Misaligned),
        (".data\nnop", ErrorKind::Layout),
        (".space 0x10000001", ErrorKind::Layout),
        (".org 12", ErrorKind::UnknownDirective),
    ] {
        assert_eq!(assemble(source, 0).unwrap_err().kind, kind, "{source}");
    }
}

#[test]
fn branch_and_jump_handle_pc_plus_four_boundaries() {
    assert_eq!(words("j 0x10000000", 0x0fff_fffc), [0x0800_0000]);
    assert_eq!(words("b 0", 0xffff_fffc), [0x1000_0000]);
    assert_eq!(words("b 0x20000", 0), [0x1000_7fff]);
    assert_eq!(words("b 4", 0x20000), [0x1000_8000]);
    assert_eq!(
        assemble("li $t0, 1", 0xffff_fffc).unwrap_err().kind,
        ErrorKind::Layout
    );
    assert_eq!(assemble("nop", 1).unwrap_err().kind, ErrorKind::Misaligned);
}

#[test]
fn all_200_forms_match_independent_gnu_words() {
    use super::encoder::ASSEMBLY_FORMATS;
    use std::collections::BTreeSet;
    let mut source = String::from(".text\n");
    let mut expected = Vec::new();
    let mut names = BTreeSet::new();
    for line in include_str!("../../tests/fixtures/gnu-mips32r2.tsv")
        .lines()
        .filter(|line| !line.starts_with('#'))
    {
        let (word, statement) = line.split_once('\t').unwrap();
        assert!(
            names.insert(statement.split_whitespace().next().unwrap()),
            "{statement}"
        );
        expected.push((u32::from_str_radix(word, 16).unwrap(), statement));
        source.push_str(statement);
        source.push('\n');
    }
    assert_eq!(
        names,
        ASSEMBLY_FORMATS.iter().map(|&(name, _)| name).collect()
    );
    assert_eq!(names.len(), 200);
    source.push_str("target:\n");
    let actual = words(&source, 0);
    assert_eq!(actual.len(), expected.len());
    for (actual, (expected, statement)) in actual.into_iter().zip(expected) {
        assert_eq!(actual, expected, "{statement}");
    }
}

#[test]
fn rejects_unpredictable_link_registers_and_unavailable_fpu_control_registers() {
    for source in [
        "jalr $t0, $t0",
        "jalr $ra",
        "jalr.hb $31, $ra",
        "bgezal $31, 4",
        "bgezall $ra, 4",
        "bltzal $ra, 4",
        "bltzall $31, 4",
        "cfc1 $t0, $1",
        "ctc1 $t0, $0",
        "ctc1 $t0, $27",
    ] {
        let error = assemble(source, 0).unwrap_err();
        assert_eq!(error.kind, ErrorKind::InvalidOperand, "{source}");
        assert_eq!(error.location.line, 1);
    }
    assert_eq!(
        words("jalr $t0\ncfc1 $t0, $0\nctc1 $t0, $31", 0),
        [0x0100_f809, 0x4448_0000, 0x44c8_f800]
    );
}

#[test]
fn windows_utf8_bom_and_crlf_keep_source_locations() {
    assert_eq!(words("\u{feff}.text\r\nnop\r\n", 0), [0]);
    let error = assemble("\u{feff}  addiu $t0, $zero, 32768\r\n", 0).unwrap_err();
    assert_eq!(error.kind, ErrorKind::OutOfRange);
    assert_eq!(
        error.location,
        super::SourceLocation {
            line: 1,
            column: 21
        }
    );
}

#[test]
fn immediate_edges_and_final_word_at_top_of_address_space() {
    assert_eq!(
        words(
            "addiu $t0, $t1, -32768\naddiu $t0, $t1, 32767\nori $t0, $t1, 65535\nins $t0, $t1, 31, 1",
            0
        ),
        [0x2528_8000, 0x2528_7fff, 0x3528_ffff, 0x7d28_ffc4]
    );
    assert_eq!(
        words(".word -2147483648, 0xffffffff", 0),
        [0x8000_0000, 0xffff_ffff]
    );
    assert_eq!(words("nop", 0xffff_fffc), [0]);
    for source in [
        "addiu $t0, $t0, -32769",
        "ori $t0, $t0, 65536",
        ".word 0x100000000",
        "bc1t $fcc8, 0",
        "mfc0 $t0, $12, 8",
    ] {
        assert_eq!(assemble(source, 0).unwrap_err().kind, ErrorKind::OutOfRange);
    }
}
