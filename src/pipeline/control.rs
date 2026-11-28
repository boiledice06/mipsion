use super::state::Decoded;
use super::trace::Redirect;
use crate::{instruction::RawInstruction, isa::Operation};

pub(super) fn source_registers(
    operation: &Operation,
    raw: &RawInstruction,
) -> Option<([Option<u8>; 2], Option<u8>)> {
    use Operation::*;
    let rs = Some(raw.rs());
    let rt = Some(raw.rt());
    Some(match operation {
        J => ([None, None], None),
        Jal => ([None, None], Some(31)),
        Jr | JrHb => ([rs, None], None),
        Jalr | JalrHb => ([rs, None], Some(raw.rd())),
        Beq | Bne | Beql | Bnel => ([rs, rt], None),
        Bgezal | Bltzal | Bgezall | Bltzall => ([rs, None], Some(31)),
        Blez | Bgtz | Bgez | Bltz | Blezl | Bgtzl | Bgezl | Bltzl => ([rs, None], None),
        _ => return None,
    })
}

pub(super) fn resolve(decoded: Decoded, a: u32, b: u32, raw: &RawInstruction) -> Redirect {
    use Operation::*;
    let branch_pc = decoded.instruction.pc;
    let delay_pc = branch_pc.wrapping_add(4);
    let fallthrough = delay_pc.wrapping_add(4);
    let branch_target = delay_pc.wrapping_add(((raw.immediate() as i16 as i32) << 2) as u32);
    let operation = decoded.operation;
    let (target, taken) = match operation {
        J | Jal => ((delay_pc & 0xf000_0000) | (raw.target() << 2), true),
        Jr | JrHb | Jalr | JalrHb => (a, true),
        _ => {
            let taken = match operation {
                Beq | Beql => a == b,
                Bne | Bnel => a != b,
                Blez | Blezl => (a as i32) <= 0,
                Bgtz | Bgtzl => (a as i32) > 0,
                Bgez | Bgezl | Bgezal | Bgezall => (a as i32) >= 0,
                Bltz | Bltzl | Bltzal | Bltzall => (a as i32) < 0,
                _ => unreachable!("decoded control instruction"),
            };
            (if taken { branch_target } else { fallthrough }, taken)
        }
    };
    let likely = matches!(
        operation,
        Beql | Bnel | Blezl | Bgtzl | Bgezl | Bltzl | Bgezall | Bltzall
    );
    Redirect {
        branch_pc,
        delay_slot: (taken || !likely).then_some(delay_pc),
        target,
        taken,
    }
}
