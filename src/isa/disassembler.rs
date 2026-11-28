use crate::isa::decoder::decode;
use crate::isa::instruction::RawInstruction;
use crate::runtime::inspection::GPR_NAMES;

fn gpr(index: u8) -> &'static str {
    GPR_NAMES[usize::from(index)]
}

fn branch_target(pc: u32, immediate: u16) -> u32 {
    let offset = (immediate as i16 as i32) << 2;

    pc.wrapping_add(4).wrapping_add(offset as u32)
}

fn jump_target(pc: u32, target: u32) -> u32 {
    (pc.wrapping_add(4) & 0xF000_0000) | (target << 2)
}

pub fn disassemble(word: u32, pc: u32) -> String {
    if word == 0 {
        return "nop".into();
    }

    let Some(definition) = decode(word) else {
        return format!(".word 0x{word:08X}");
    };

    let mnemonic = definition.mnemonic;
    let inst = RawInstruction::new(word);

    let rs = gpr(inst.rs());
    let rt = gpr(inst.rt());
    let rd = gpr(inst.rd());

    let imm = inst.immediate();
    let signed = imm as i16;
    let shamt = inst.shamt();

    let operands = match mnemonic {
        // Integer arithmetic and logic
        "add" | "addu" | "sub" | "subu" | "and" | "or" | "xor" | "nor" | "slt" | "sltu" | "mul"
        | "movn" | "movz" => {
            format!("{rd}, {rs}, {rt}")
        }

        // Signed immediates
        "addi" | "addiu" | "slti" | "sltiu" => {
            format!("{rt}, {rs}, {signed}")
        }

        // Zero-extended immediates
        "andi" | "ori" | "xori" => {
            format!("{rt}, {rs}, 0x{imm:04X}")
        }

        "lui" => {
            format!("{rt}, 0x{imm:04X}")
        }

        // Shift and rotate
        "sll" | "srl" | "sra" | "rotr" => {
            format!("{rd}, {rt}, {shamt}")
        }

        "sllv" | "srlv" | "srav" | "rotrv" => {
            format!("{rd}, {rt}, {rs}")
        }

        // HI / LO arithmetic
        "mult" | "multu" | "div" | "divu" | "madd" | "maddu" | "msub" | "msubu" => {
            format!("{rs}, {rt}")
        }

        "mfhi" | "mflo" => rd.to_string(),

        "mthi" | "mtlo" => rs.to_string(),

        // Bit manipulation
        "clz" | "clo" => {
            format!("{rd}, {rs}")
        }

        "seb" | "seh" | "wsbh" => {
            format!("{rd}, {rt}")
        }

        "ext" => {
            let size = u32::from(inst.rd()) + 1;
            format!("{rt}, {rs}, {shamt}, {size}")
        }

        "ins" => {
            let size = i32::from(inst.rd()) - i32::from(shamt) + 1;

            if size <= 0 {
                return format!(".word 0x{word:08X}");
            }

            format!("{rt}, {rs}, {shamt}, {size}")
        }

        // Integer load / store
        "lb" | "lbu" | "lh" | "lhu" | "lw" | "lwl" | "lwr" | "sb" | "sh" | "sw" | "swl" | "swr"
        | "ll" | "sc" => {
            format!("{rt}, {signed}({rs})")
        }

        // Conditional branches
        "beq" | "bne" | "beql" | "bnel" => {
            let target = branch_target(pc, imm);
            format!("{rs}, {rt}, 0x{target:08X}")
        }

        "bgez" | "bgezal" | "bgezall" | "bgezl" | "bltz" | "bltzal" | "bltzall" | "bltzl"
        | "bgtz" | "bgtzl" | "blez" | "blezl" => {
            let target = branch_target(pc, imm);
            format!("{rs}, 0x{target:08X}")
        }

        // Jumps
        "j" | "jal" | "jalx" => {
            let target = jump_target(pc, inst.target());
            format!("0x{target:08X}")
        }

        "jr" | "jr.hb" => rs.to_string(),

        "jalr" | "jalr.hb" => {
            format!("{rd}, {rs}")
        }

        // Trap instructions
        "teq" | "tne" | "tge" | "tgeu" | "tlt" | "tltu" => {
            let code = (word >> 6) & 0x3FF;

            if code == 0 {
                format!("{rs}, {rt}")
            } else {
                format!("{rs}, {rt}, {code}")
            }
        }

        "teqi" | "tnei" | "tgei" | "tgeiu" | "tlti" | "tltiu" => {
            format!("{rs}, {signed}")
        }

        // System instructions
        "syscall" | "break" | "sdbbp" => {
            let code = (word >> 6) & 0xF_FFFF;

            if code == 0 {
                String::new()
            } else {
                code.to_string()
            }
        }

        "sync" => {
            if shamt == 0 {
                String::new()
            } else {
                shamt.to_string()
            }
        }

        "synci" => format!("{signed}({rs})"),

        "pref" | "cache" => {
            format!("{}, {signed}({rs})", inst.rt())
        }

        "rdhwr" => format!("{rt}, ${}", inst.rd()),

        "mfc0" | "mtc0" => {
            format!("{rt}, ${}, {}", inst.rd(), inst.sel())
        }

        "di" | "ei" => rt.to_string(),

        "eret" | "deret" | "tlbp" | "tlbr" | "tlbwi" | "tlbwr" | "ssnop" | "ehb" | "pause" => {
            String::new()
        }

        // Floating-point transfer
        "mfc1" | "mtc1" | "mfhc1" | "mthc1" => {
            format!("{rt}, $f{}", inst.fs())
        }

        "cfc1" | "ctc1" => {
            format!("{rt}, ${}", inst.fs())
        }

        // Floating-point load / store
        "lwc1" | "swc1" | "ldc1" | "sdc1" => {
            format!("$f{}, {signed}({rs})", inst.ft())
        }

        // Floating-point branches
        "bc1f" | "bc1t" | "bc1fl" | "bc1tl" => {
            let target = branch_target(pc, imm);

            format!("{}, 0x{target:08X}", inst.branch_cc())
        }

        // Conditional moves
        "movf" | "movt" => {
            format!("{rd}, {rs}, {}", inst.branch_cc())
        }

        "movf.s" | "movf.d" | "movf.ps" | "movt.s" | "movt.d" | "movt.ps" => {
            format!("$f{}, $f{}, {}", inst.fd(), inst.fs(), inst.branch_cc())
        }

        "movn.s" | "movn.d" | "movn.ps" | "movz.s" | "movz.d" | "movz.ps" => {
            format!("$f{}, $f{}, {rt}", inst.fd(), inst.fs())
        }

        // Floating-point comparisons
        name if name.starts_with("c.") => {
            format!("{}, $f{}, $f{}", inst.compare_cc(), inst.fs(), inst.ft())
        }

        // Floating-point binary arithmetic
        "add.s" | "add.d" | "add.ps" | "sub.s" | "sub.d" | "sub.ps" | "mul.s" | "mul.d"
        | "mul.ps" | "div.s" | "div.d" | "pll.ps" | "plu.ps" | "pul.ps" | "puu.ps" => {
            format!("$f{}, $f{}, $f{}", inst.fd(), inst.fs(), inst.ft())
        }

        // Floating-point unary arithmetic and conversion
        "abs.s" | "abs.d" | "abs.ps" | "neg.s" | "neg.d" | "neg.ps" | "mov.s" | "mov.d"
        | "mov.ps" | "sqrt.s" | "sqrt.d" | "recip.s" | "recip.d" | "rsqrt.s" | "rsqrt.d" => {
            format!("$f{}, $f{}", inst.fd(), inst.fs())
        }

        name if name.starts_with("cvt.")
            || name.starts_with("round.")
            || name.starts_with("trunc.")
            || name.starts_with("ceil.")
            || name.starts_with("floor.") =>
        {
            if name == "cvt.ps.s" {
                format!("$f{}, $f{}, $f{}", inst.fd(), inst.fs(), inst.ft())
            } else {
                format!("$f{}, $f{}", inst.fd(), inst.fs())
            }
        }

        // Known encoding, unsupported operand formatting
        _ => {
            return format!(".word 0x{word:08X} # {mnemonic}");
        }
    };

    if operands.is_empty() {
        mnemonic.to_string()
    } else {
        format!("{mnemonic} {operands}")
    }
}

pub fn format_disassembly(word: u32, pc: u32) -> String {
    format!("{pc:08X}  {word:08X}  {}", disassemble(word, pc))
}
