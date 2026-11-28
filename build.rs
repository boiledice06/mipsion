use std::{collections::HashSet, env, fmt::Write, fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=ISA_CHECKLIST.md");

    let checklist = fs::read_to_string("ISA_CHECKLIST.md").unwrap();
    let mut operations =
        String::from("#[allow(clippy::upper_case_acronyms)]\npub enum Operation {\n");
    let mut instructions = String::from("pub static INSTRUCTIONS: &[InstructionDef] = &[\n");
    let mut assembly_formats =
        String::from("pub(super) static ASSEMBLY_FORMATS: &[(&str, &str)] = &[\n");
    let mut mnemonics = HashSet::new();
    let mut encodings = Vec::<(u32, u32, &str)>::new();

    for line in checklist.lines().filter(|line| line.starts_with("| `")) {
        let fields: Vec<_> = line.trim_matches('|').split('|').map(str::trim).collect();
        let [
            mnemonic,
            _,
            implementation,
            _,
            opcode,
            funct,
            mask,
            pattern,
            operands,
            _,
            _,
            _,
        ] = fields.as_slice()
        else {
            panic!("Invalid ISA row: {line}");
        };
        let mnemonic = mnemonic.trim_matches('`');
        assert!(mnemonics.insert(mnemonic), "Duplicate mnemonic: {mnemonic}");
        let opcode = parse_hex(opcode);
        let mask = parse_hex(mask);
        let pattern = parse_hex(pattern);
        assert_eq!(pattern & !mask, 0, "{mnemonic}");
        assert_eq!(mask >> 26, 0x3f, "{mnemonic}");
        assert_eq!(pattern >> 26, opcode, "{mnemonic}");
        if *funct != "—" {
            assert_eq!(mask & 0x3f, 0x3f, "{mnemonic}");
            assert_eq!(pattern & 0x3f, parse_hex(funct), "{mnemonic}");
        }

        if matches!(*implementation, "구현" | "부분") {
            writeln!(
                assembly_formats,
                "    ({:?}, {:?}),",
                mnemonic.to_ascii_lowercase(),
                operands.trim_matches('`')
            )
            .unwrap();
        }
        match *implementation {
            "구현" | "부분" | "대기" => encodings.push((mask, pattern, mnemonic)),
            "별칭" => {}
            _ => panic!("Invalid implementation status: {line}"),
        }
    }

    encodings.sort_by_key(|&(mask, _, _)| std::cmp::Reverse(mask.count_ones()));
    for (index, &(mask, pattern, mnemonic)) in encodings.iter().enumerate() {
        for &(other_mask, other_pattern, other_mnemonic) in &encodings[..index] {
            let common_mask = mask & other_mask;
            assert!(
                (pattern ^ other_pattern) & common_mask != 0
                    || (mask != other_mask && common_mask == mask),
                "Ambiguous runtime encoding: {mnemonic}, {other_mnemonic}"
            );
        }
        let operation: String = mnemonic
            .split('.')
            .map(|part| part[..1].to_ascii_uppercase() + &part[1..].to_ascii_lowercase())
            .collect();
        let mnemonic = mnemonic.to_ascii_lowercase();
        writeln!(operations, "    {operation},").unwrap();
        writeln!(
            instructions,
            "    InstructionDef {{ mnemonic: {mnemonic:?}, mask: {mask:#010x}, pattern: {pattern:#010x}, operation: Operation::{operation} }},"
        )
        .unwrap();
    }

    assert!(!encodings.is_empty(), "No runtime ISA entries");
    operations.push_str("}\n");
    instructions.push_str("];\n");

    assembly_formats.push_str("];\n");
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    fs::write(out_dir.join("assembly_formats.rs"), assembly_formats).unwrap();
    fs::write(out_dir.join("operations.rs"), operations).unwrap();
    fs::write(out_dir.join("instructions.rs"), instructions).unwrap();
}

fn parse_hex(field: &str) -> u32 {
    let digits = field.trim_matches('`').strip_prefix("0x").unwrap();
    u32::from_str_radix(digits, 16).unwrap()
}
