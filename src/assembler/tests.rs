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
        // Zero operands in the ISA patterns give valid pairs and aligned targets.
        let source = disassemble(definition.pattern, 0);
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
