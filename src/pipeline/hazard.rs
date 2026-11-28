use super::state::{Decoded, Effect, Executed, Prepared};
use super::trace::{Forwarded, ForwardingSource, Stall};
use crate::cpu::Cpu;

pub(super) fn operands(
    decoded: Decoded,
    cpu: &Cpu,
    producers: [Option<u8>; 2],
    ex: Option<Executed>,
    mem: Option<Prepared>,
    forwarding: bool,
) -> Result<([u32; 2], [Option<Forwarded>; 2]), Stall> {
    let mut values = [0; 2];
    let mut events = [None; 2];
    let pc = decoded.instruction.pc;
    for (index, register) in decoded.sources.into_iter().enumerate() {
        let Some(register) = register else { continue };
        let dependency = Stall::RegisterDependency { pc, register };
        let forwarding_value = if producers[0] == Some(register) {
            if !forwarding {
                return Err(dependency);
            }
            match ex.expect("issue-stage producer moved to EX").effect {
                Effect::Ready(action) => action
                    .register_write()
                    .map(|(_, value)| (value, ForwardingSource::ExMem))
                    .ok_or(dependency)?,
                Effect::Load { .. } => return Err(Stall::LoadUse { pc, register }),
                Effect::Store(_) => unreachable!("stores have no register destination"),
            }
        } else if producers[1] == Some(register) {
            if !forwarding {
                return Err(dependency);
            }
            mem.expect("memory-stage producer moved to WB")
                .action
                .register_write()
                .map(|(_, value)| (value, ForwardingSource::MemWb))
                .ok_or(dependency)?
        } else {
            values[index] = cpu.read_reg(usize::from(register));
            continue;
        };
        let (value, source) = forwarding_value;
        values[index] = value;
        events[index] = Some(Forwarded {
            pc,
            operand: index as u8,
            register,
            value,
            source,
        });
    }
    Ok((values, events))
}
