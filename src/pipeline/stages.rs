use super::control;
use super::state::{
    Action, Decoded, Effect, Executed, Fault, Instruction, Issued, Prepared, Store, Width,
};
use crate::cache::CacheAccess;
use crate::{
    alu, cpu::Cpu, decoder::decode, exception::Exception, instruction::RawInstruction,
    isa::Operation, memory::Memory, mmu::AccessType,
};

pub(super) fn fetch(
    cpu: &Cpu,
    memory: &Memory,
    pc: u32,
    read: impl FnOnce(u32) -> u32,
) -> Instruction {
    let address = cpu.instruction_address(memory, pc).map_err(|exception| {
        let address = (exception != Exception::InstructionBusError).then_some(pc);
        Fault::new(exception, address)
    });
    Instruction {
        pc,
        physical: address.ok(),
        delay_slot: false,
        word: address.map(read),
    }
}

pub(super) fn decode_stage(instruction: Instruction) -> Result<Decoded, Action> {
    use Operation::*;
    let word = instruction.word.map_err(Action::Fault)?;
    let Some(definition) = decode(word) else {
        return Err(Action::Fault(Fault::new(
            Exception::ReservedInstruction,
            None,
        )));
    };
    let operation = &definition.operation;
    let raw = RawInstruction::new(word);
    let rs = Some(raw.rs());
    let rt = Some(raw.rt());
    let rd = Some(raw.rd());
    let control_operands = control::source_registers(operation, &raw);
    if control_operands.is_some() && instruction.delay_slot {
        return Err(Action::Unsupported(Some(definition.mnemonic)));
    }
    let (sources, destination) = if let Some(operands) = control_operands {
        operands
    } else {
        match operation {
            Add | Addu | Sub | Subu | And | Or | Xor | Nor | Slt | Sltu | Mul => ([rs, rt], rd),
            Addi | Addiu | Andi | Ori | Xori | Slti | Sltiu => ([rs, None], rt),
            Sll | Sra | Srl | Rotr => ([rt, None], rd),
            Sllv | Srav | Srlv | Rotrv => ([rt, rs], rd),
            Lui => ([None, None], rt),
            Lb | Lbu | Lh | Lhu | Lw => ([rs, None], rt),
            Sb | Sh | Sw => ([rs, rt], None),
            Syscall => return Err(Action::Fault(Fault::new(Exception::Syscall, None))),
            Break => return Err(Action::Fault(Fault::new(Exception::Breakpoint, None))),
            _ => return Err(Action::Unsupported(Some(definition.mnemonic))),
        }
    };
    Ok(Decoded {
        instruction,
        operation,
        control: control_operands.is_some(),
        sources: sources.map(|source| source.filter(|&reg| reg != 0)),
        destination: destination.filter(|&reg| reg != 0),
    })
}

pub(super) fn execute(issued: Issued) -> Executed {
    let instruction = issued.instruction();
    let effect = match issued {
        Issued::Ready(_, action) => Effect::Ready(action),
        Issued::Operation(decoded, [a, b]) => execution_effect(decoded, a, b),
    };
    Executed {
        instruction,
        effect,
    }
}

fn execution_effect(decoded: Decoded, a: u32, b: u32) -> Effect {
    use Operation::*;
    let raw = RawInstruction::new(decoded.instruction.word.unwrap());
    if decoded.control {
        let redirect = control::resolve(decoded, a, b, &raw);
        let link = decoded
            .destination
            .map(|register| (register, decoded.instruction.pc.wrapping_add(8)));
        return Effect::Ready(Action::Control { redirect, link });
    }
    let signed = raw.immediate() as i16 as i32 as u32;
    let dest = decoded.destination.unwrap_or(0);
    let op = decoded.operation;
    let result = match op {
        Add | Sub | Addi => {
            let b = if matches!(op, Addi) { signed } else { b };
            let Some(value) = alu::checked_compute(a, b, op) else {
                return Effect::Ready(Action::Fault(Fault::new(Exception::Overflow, None)));
            };
            value
        }
        Addu | Subu | And | Or | Xor | Nor | Slt | Sltu | Mul => alu::compute(a, b, op),
        Sllv | Srav | Srlv | Rotrv => alu::compute(a, b & 31, op),
        Addiu | Slti | Sltiu => alu::compute(a, signed, op),
        Andi | Ori | Xori => alu::compute(a, u32::from(raw.immediate()), op),
        Sll | Sra | Srl | Rotr => alu::compute(a, u32::from(raw.shamt()), op),
        Lui => u32::from(raw.immediate()) << 16,
        Lb | Lbu | Lh | Lhu | Lw => {
            let width = match op {
                Lb | Lbu => Width::Byte,
                Lh | Lhu => Width::Half,
                _ => Width::Word,
            };
            return Effect::Load {
                destination: dest,
                address: a.wrapping_add(signed),
                width,
                signed: matches!(op, Lb | Lh),
            };
        }
        Sb | Sh | Sw => {
            let width = match op {
                Sb => Width::Byte,
                Sh => Width::Half,
                _ => Width::Word,
            };
            return Effect::Store(Store {
                address: a.wrapping_add(signed),
                width,
                value: b,
            });
        }
        _ => unreachable!("only supported operations reach EX"),
    };
    Effect::Ready(Action::Write(dest, result))
}

pub(super) fn memory_stage(
    executed: Executed,
    cpu: &Cpu,
    memory: &Memory,
    mut access: impl FnMut(u32, Width, CacheAccess) -> u32,
) -> Prepared {
    let action = match executed.effect {
        Effect::Ready(action) => action,
        Effect::Load {
            destination,
            address,
            width,
            signed,
        } => match translate(cpu, memory, address, width, AccessType::Load) {
            Ok(physical) => {
                let raw = access(physical, width, CacheAccess::Read);
                let value = match (width, signed) {
                    (Width::Byte, true) => raw as i8 as i32 as u32,
                    (Width::Half, true) => raw as i16 as i32 as u32,
                    _ => raw,
                };
                Action::Write(destination, value)
            }
            Err(fault) => Action::Fault(fault),
        },
        Effect::Store(store) => {
            match translate(cpu, memory, store.address, store.width, AccessType::Store) {
                Ok(address) => {
                    access(address, store.width, CacheAccess::Write);
                    Action::Store(Store { address, ..store })
                }
                Err(fault) => Action::Fault(fault),
            }
        }
    };
    Prepared {
        instruction: executed.instruction,
        action,
    }
}

fn translate(
    cpu: &Cpu,
    memory: &Memory,
    address: u32,
    width: Width,
    access: AccessType,
) -> Result<u32, Fault> {
    let bytes = width.bytes();
    if address as usize & (bytes - 1) != 0 {
        let exception = if access == AccessType::Store {
            Exception::AddressErrorStore
        } else {
            Exception::AddressErrorLoad
        };
        return Err(Fault::new(exception, Some(address)));
    }
    let physical = cpu
        .resolve_address(address, access)
        .map_err(|exception| Fault::new(exception, Some(address)))?;
    if !memory.contains_range(physical, bytes) {
        return Err(Fault::new(Exception::DataBusError, None));
    }
    Ok(physical)
}

pub(super) fn read_memory(memory: &Memory, address: u32, width: Width) -> u32 {
    match width {
        Width::Byte => u32::from(memory.read_byte(address)),
        Width::Half => u32::from(memory.read_halfword(address)),
        Width::Word => memory.read_word(address),
    }
}

pub(super) fn write_store(memory: &mut Memory, store: Store) {
    match store.width {
        Width::Byte => memory.write_byte(store.address, store.value as u8),
        Width::Half => memory.write_halfword(store.address, store.value as u16),
        Width::Word => memory.write_word(store.address, store.value),
    }
}
