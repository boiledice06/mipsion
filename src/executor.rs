use crate::cpu::Cpu;
use crate::instruction::RawInstruction;
use crate::isa::Operation;

use crate::alu::{accumulate, compute, divide, extract, insert, multiply};

use crate::memory::Memory;

use crate::alu::checked_compute;
use crate::exception::Exception;

use crate::mmu::AccessType;

pub fn execute(cpu: &mut Cpu, memory: &mut Memory, inst: &RawInstruction, operation: &Operation) {
    // CP0 및 TLB 명령어 접근 권한 검사
    if matches!(
        operation,
        Operation::Mfc0
            | Operation::Mtc0
            | Operation::Ei
            | Operation::Di
            | Operation::Eret
            | Operation::Tlbr
            | Operation::Tlbwi
            | Operation::Tlbwr
            | Operation::Tlbp
    ) && !cpu.cp0().cp0_accessible()
    {
        cpu.raise_exception(Exception::CoprocessorUnusable(0), None);

        return;
    }
    // CP1 및 COP1X 명령어 접근 권한 검사
    let uses_cp1 = matches!(inst.opcode(), 0x11 | 0x13 | 0x31 | 0x35 | 0x39 | 0x3D)
        || matches!(operation, Operation::Movf | Operation::Movt);

    if uses_cp1 && !cpu.cp0().cp1_accessible() {
        cpu.raise_exception(Exception::CoprocessorUnusable(1), None);

        return;
    }

    match operation {
        Operation::Addu
        | Operation::Subu
        | Operation::And
        | Operation::Or
        | Operation::Xor
        | Operation::Nor
        | Operation::Slt
        | Operation::Sltu
        | Operation::Mul => {
            let rs = inst.rs() as usize;
            let rt = inst.rt() as usize;
            let rd = inst.rd() as usize;

            let a = cpu.read_reg(rs);
            let b = cpu.read_reg(rt);

            let result = compute(a, b, operation);

            cpu.write_reg(rd, result);
        }

        Operation::Addiu | Operation::Slti | Operation::Sltiu => {
            let rs = inst.rs() as usize;
            let rt = inst.rt() as usize;

            let a = cpu.read_reg(rs);
            let b = inst.immediate() as i16 as i32 as u32;

            let result = compute(a, b, operation);

            cpu.write_reg(rt, result);
        }

        Operation::Add | Operation::Sub | Operation::Addi => {
            let a = cpu.read_reg(inst.rs() as usize);

            let (b, dest) = if matches!(operation, Operation::Addi) {
                (inst.immediate() as i16 as i32 as u32, inst.rt() as usize)
            } else {
                (cpu.read_reg(inst.rt() as usize), inst.rd() as usize)
            };

            match checked_compute(a, b, operation) {
                Some(result) => cpu.write_reg(dest, result),
                None => cpu.raise_exception(Exception::Overflow, None),
            }
        }

        Operation::Sll | Operation::Srl | Operation::Sra | Operation::Rotr => {
            let rt = inst.rt() as usize;
            let rd = inst.rd() as usize;

            let a = cpu.read_reg(rt);
            let b = inst.shamt() as u32;

            let result = compute(a, b, operation);

            cpu.write_reg(rd, result);
        }

        Operation::Andi | Operation::Ori | Operation::Xori => {
            let rs = inst.rs() as usize;
            let rt = inst.rt() as usize;

            let a = cpu.read_reg(rs);
            let b = inst.immediate() as u32;

            let result = compute(a, b, operation);

            cpu.write_reg(rt, result);
        }

        Operation::Sllv | Operation::Srlv | Operation::Srav | Operation::Rotrv => {
            let rs = inst.rs() as usize;
            let rt = inst.rt() as usize;
            let rd = inst.rd() as usize;

            let a = cpu.read_reg(rt);
            let b = cpu.read_reg(rs) & 0x1F;

            let result = compute(a, b, operation);

            cpu.write_reg(rd, result);
        }

        Operation::Lui => {
            let rt = inst.rt() as usize;
            let b = inst.immediate() as u32;

            let result = compute(0, b, operation);

            cpu.write_reg(rt, result);
        }

        Operation::Clz | Operation::Clo => {
            let rs = inst.rs() as usize;
            let rd = inst.rd() as usize;

            let a = cpu.read_reg(rs);
            let result = compute(a, 0, operation);

            cpu.write_reg(rd, result);
        }

        Operation::Seb | Operation::Seh | Operation::Wsbh => {
            let rt = inst.rt() as usize;
            let rd = inst.rd() as usize;

            let a = cpu.read_reg(rt);
            let result = compute(a, 0, operation);

            cpu.write_reg(rd, result);
        }

        Operation::Ext => {
            let rs = inst.rs() as usize;
            let rt = inst.rt() as usize;

            let pos = inst.shamt() as u32;
            let size = inst.rd() as u32 + 1;

            assert!(pos + size <= 32);

            let a = cpu.read_reg(rs);
            let result = extract(a, pos, size);

            cpu.write_reg(rt, result);
        }

        Operation::Ins => {
            let rs = inst.rs() as usize;
            let rt = inst.rt() as usize;

            let pos = inst.shamt() as u32;
            let msb = inst.rd() as u32;

            assert!(msb >= pos);

            let size = msb - pos + 1;

            let a = cpu.read_reg(rs);
            let b = cpu.read_reg(rt);

            let result = insert(a, b, pos, size);

            cpu.write_reg(rt, result);
        }

        Operation::Mult | Operation::Multu => {
            let a = cpu.read_reg(inst.rs() as usize);
            let b = cpu.read_reg(inst.rt() as usize);

            let signed = matches!(operation, Operation::Mult);
            let result = multiply(a, b, signed);

            cpu.write_hi((result >> 32) as u32);
            cpu.write_lo(result as u32);
        }

        Operation::Div | Operation::Divu => {
            let a = cpu.read_reg(inst.rs() as usize);
            let b = cpu.read_reg(inst.rt() as usize);

            let signed = matches!(operation, Operation::Div);

            if let Some((hi, lo)) = divide(a, b, signed) {
                cpu.write_hi(hi);
                cpu.write_lo(lo);
            }
        }

        Operation::Mfhi => {
            let rd = inst.rd() as usize;
            cpu.write_reg(rd, cpu.read_hi());
        }

        Operation::Mflo => {
            let rd = inst.rd() as usize;
            cpu.write_reg(rd, cpu.read_lo());
        }

        Operation::Mthi => {
            let rs = inst.rs() as usize;
            cpu.write_hi(cpu.read_reg(rs));
        }

        Operation::Mtlo => {
            let rs = inst.rs() as usize;
            cpu.write_lo(cpu.read_reg(rs));
        }

        Operation::Madd | Operation::Maddu | Operation::Msub | Operation::Msubu => {
            let a = cpu.read_reg(inst.rs() as usize);
            let b = cpu.read_reg(inst.rt() as usize);

            let signed = matches!(operation, Operation::Madd | Operation::Msub);

            let subtract = matches!(operation, Operation::Msub | Operation::Msubu);

            let acc = cpu.read_hilo();
            let result = accumulate(a, b, acc, signed, subtract);

            cpu.write_hilo(result);
        }

        Operation::Lw | Operation::Lh | Operation::Lhu | Operation::Lb | Operation::Lbu => {
            let rs = inst.rs() as usize;
            let rt = inst.rt() as usize;

            let base = cpu.read_reg(rs);
            let offset = inst.immediate() as i16 as i32 as u32;

            let address = base.wrapping_add(offset);

            let misaligned = match operation {
                Operation::Lw => address & 3 != 0,
                Operation::Lh | Operation::Lhu => address & 1 != 0,
                _ => false,
            };

            if misaligned {
                cpu.raise_exception(Exception::AddressErrorLoad, Some(address));
                return;
            }

            let Some(address) = cpu.translate_address(address, AccessType::Load) else {
                return;
            };

            let result = match operation {
                Operation::Lw => memory.read_word(address),
                Operation::Lh => memory.read_halfword(address) as i16 as i32 as u32,
                Operation::Lhu => memory.read_halfword(address) as u32,
                Operation::Lb => memory.read_byte(address) as i8 as i32 as u32,
                Operation::Lbu => memory.read_byte(address) as u32,
                _ => unreachable!(),
            };

            cpu.write_reg(rt, result);
        }

        Operation::Sw | Operation::Sh | Operation::Sb => {
            let rs = inst.rs() as usize;
            let rt = inst.rt() as usize;

            let base = cpu.read_reg(rs);
            let offset = inst.immediate() as i16 as i32 as u32;

            let address = base.wrapping_add(offset);

            let misaligned = match operation {
                Operation::Sw => address & 3 != 0,
                Operation::Sh => address & 1 != 0,
                _ => false,
            };

            if misaligned {
                cpu.raise_exception(Exception::AddressErrorStore, Some(address));
                return;
            }

            let value = cpu.read_reg(rt);

            let Some(address) = cpu.translate_address(address, AccessType::Store) else {
                return;
            };

            match operation {
                Operation::Sw => memory.write_word(address, value),
                Operation::Sh => memory.write_halfword(address, value as u16),
                Operation::Sb => memory.write_byte(address, value as u8),
                _ => unreachable!(),
            }

            cpu.clear_reservation();
        }

        Operation::Lwc1 | Operation::Swc1 => {
            let base = inst.rs() as usize;
            let ft = inst.ft() as usize;

            let offset = inst.immediate() as i16 as i32 as u32;

            let address = cpu.read_reg(base).wrapping_add(offset);

            let is_load = matches!(operation, Operation::Lwc1);

            // Word alignment 검사
            if address & 3 != 0 {
                let exception = if is_load {
                    Exception::AddressErrorLoad
                } else {
                    Exception::AddressErrorStore
                };

                cpu.raise_exception(exception, Some(address));

                return;
            }

            // 가상 주소 → 물리 주소
            let access = if is_load {
                AccessType::Load
            } else {
                AccessType::Store
            };

            let Some(physical_address) = cpu.translate_address(address, access) else {
                return;
            };

            match operation {
                Operation::Lwc1 => {
                    let value = memory.read_word(physical_address);

                    cpu.cp1_mut().write_word(ft, value);
                }

                Operation::Swc1 => {
                    let value = cpu.cp1().read_word(ft);

                    memory.write_word(physical_address, value);

                    cpu.clear_reservation();
                }

                _ => unreachable!(),
            }
        }

        Operation::Lwl | Operation::Lwr => {
            let rs = inst.rs() as usize;
            let rt = inst.rt() as usize;

            let base = cpu.read_reg(rs);
            let offset = inst.immediate() as i16 as i32 as u32;
            let address = base.wrapping_add(offset);

            let Some(address) = cpu.translate_address(address, AccessType::Load) else {
                return;
            };

            let value = cpu.read_reg(rt);

            let result = match operation {
                Operation::Lwl => memory.read_word_left(address, value),
                Operation::Lwr => memory.read_word_right(address, value),
                _ => unreachable!(),
            };

            cpu.write_reg(rt, result);
        }

        Operation::Swl | Operation::Swr => {
            let rs = inst.rs() as usize;
            let rt = inst.rt() as usize;

            let base = cpu.read_reg(rs);
            let offset = inst.immediate() as i16 as i32 as u32;
            let address = base.wrapping_add(offset);

            let Some(address) = cpu.translate_address(address, AccessType::Store) else {
                return;
            };

            let value = cpu.read_reg(rt);

            match operation {
                Operation::Swl => memory.write_word_left(address, value),
                Operation::Swr => memory.write_word_right(address, value),
                _ => unreachable!(),
            }

            cpu.clear_reservation();
        }

        Operation::Movn | Operation::Movz => {
            let rs = inst.rs() as usize;
            let rt = inst.rt() as usize;
            let rd = inst.rd() as usize;

            let value = cpu.read_reg(rs);
            let condition = cpu.read_reg(rt);

            let should_move = match operation {
                Operation::Movn => condition != 0,
                Operation::Movz => condition == 0,
                _ => unreachable!(),
            };

            if should_move {
                cpu.write_reg(rd, value);
            }
        }

        Operation::J | Operation::Jal => {
            let target = (cpu.read_pc() & 0xF0000000) | (inst.target() << 2);

            if matches!(operation, Operation::Jal) {
                cpu.write_reg(31, cpu.read_next_pc());
            }

            cpu.write_next_pc(target);
            cpu.mark_delay_slot();
        }

        Operation::Jr | Operation::Jalr | Operation::JrHb | Operation::JalrHb => {
            let rs = inst.rs() as usize;
            let rd = inst.rd() as usize;

            let target = cpu.read_reg(rs);

            if matches!(operation, Operation::Jalr | Operation::JalrHb) {
                cpu.write_reg(rd, cpu.read_next_pc());
            }

            cpu.write_next_pc(target);
            cpu.mark_delay_slot();
        }

        Operation::Beq
        | Operation::Bne
        | Operation::Blez
        | Operation::Bgtz
        | Operation::Bltz
        | Operation::Bgez
        | Operation::Beql
        | Operation::Bnel
        | Operation::Blezl
        | Operation::Bgtzl
        | Operation::Bltzl
        | Operation::Bgezl
        | Operation::Bltzal
        | Operation::Bgezal
        | Operation::Bltzall
        | Operation::Bgezall => {
            let a = cpu.read_reg(inst.rs() as usize);
            let b = cpu.read_reg(inst.rt() as usize);

            let taken = match operation {
                Operation::Beq | Operation::Beql => a == b,
                Operation::Bne | Operation::Bnel => a != b,

                Operation::Blez | Operation::Blezl => (a as i32) <= 0,
                Operation::Bgtz | Operation::Bgtzl => (a as i32) > 0,

                Operation::Bltz | Operation::Bltzl | Operation::Bltzal | Operation::Bltzall => {
                    (a as i32) < 0
                }

                Operation::Bgez | Operation::Bgezl | Operation::Bgezal | Operation::Bgezall => {
                    (a as i32) >= 0
                }

                _ => unreachable!(),
            };

            let likely = matches!(
                operation,
                Operation::Beql
                    | Operation::Bnel
                    | Operation::Blezl
                    | Operation::Bgtzl
                    | Operation::Bltzl
                    | Operation::Bgezl
                    | Operation::Bltzall
                    | Operation::Bgezall
            );

            let link = matches!(
                operation,
                Operation::Bltzal | Operation::Bgezal | Operation::Bltzall | Operation::Bgezall
            );

            if link {
                cpu.write_reg(31, cpu.read_next_pc());
            }

            if taken {
                let offset = (inst.immediate() as i16 as i32) << 2;
                let target = cpu.read_pc().wrapping_add(offset as u32);

                cpu.write_next_pc(target);
                cpu.mark_delay_slot();
            } else if likely {
                cpu.skip_delay_slot();
            } else {
                cpu.mark_delay_slot();
            }
        }

        Operation::Syscall => {
            cpu.raise_exception(Exception::Syscall, None);
        }

        Operation::Break => {
            cpu.raise_exception(Exception::Breakpoint, None);
        }

        Operation::Teq
        | Operation::Tne
        | Operation::Tge
        | Operation::Tgeu
        | Operation::Tlt
        | Operation::Tltu
        | Operation::Teqi
        | Operation::Tnei
        | Operation::Tgei
        | Operation::Tgeiu
        | Operation::Tlti
        | Operation::Tltiu => {
            let a = cpu.read_reg(inst.rs() as usize);

            let immediate = matches!(
                operation,
                Operation::Teqi
                    | Operation::Tnei
                    | Operation::Tgei
                    | Operation::Tgeiu
                    | Operation::Tlti
                    | Operation::Tltiu
            );

            let b = if immediate {
                inst.immediate() as i16 as i32 as u32
            } else {
                cpu.read_reg(inst.rt() as usize)
            };

            let trap = match operation {
                Operation::Teq | Operation::Teqi => a == b,
                Operation::Tne | Operation::Tnei => a != b,

                Operation::Tge | Operation::Tgei => (a as i32) >= (b as i32),

                Operation::Tgeu | Operation::Tgeiu => a >= b,

                Operation::Tlt | Operation::Tlti => (a as i32) < (b as i32),

                Operation::Tltu | Operation::Tltiu => a < b,

                _ => unreachable!(),
            };

            if trap {
                cpu.raise_exception(Exception::Trap, None);
            }
        }

        Operation::Ll => {
            let rs = inst.rs() as usize;
            let rt = inst.rt() as usize;

            let base = cpu.read_reg(rs);
            let offset = inst.immediate() as i16 as i32 as u32;

            let address = base.wrapping_add(offset);

            if address & 3 != 0 {
                cpu.raise_exception(Exception::AddressErrorLoad, Some(address));
                return;
            }

            let Some(address) = cpu.translate_address(address, AccessType::Load) else {
                return;
            };

            let value = memory.read_word(address);

            cpu.write_reg(rt, value);
            cpu.set_reservation(address);
        }

        Operation::Sc => {
            let rs = inst.rs() as usize;
            let rt = inst.rt() as usize;

            let base = cpu.read_reg(rs);
            let offset = inst.immediate() as i16 as i32 as u32;

            let address = base.wrapping_add(offset);

            if address & 3 != 0 {
                cpu.raise_exception(Exception::AddressErrorStore, Some(address));
                return;
            }

            let Some(address) = cpu.translate_address(address, AccessType::Store) else {
                return;
            };

            let value = cpu.read_reg(rt);
            let success = cpu.check_reservation(address);

            if success {
                memory.write_word(address, value);
            }

            cpu.clear_reservation();
            cpu.write_reg(rt, success as u32);
        }

        Operation::Ssnop
        | Operation::Ehb
        | Operation::Sync
        | Operation::Synci
        | Operation::Pref
        | Operation::Pause => {
            // 순차 실행 모델에서의 단순화
        }

        Operation::Rdhwr => {
            let rt = inst.rt() as usize;
            let rd = inst.rd();

            if !cpu.cp0().hardware_register_accessible(rd) {
                cpu.raise_exception(Exception::ReservedInstruction, None);

                return;
            }

            let value = match rd {
                0 => 0,
                1 => 0,
                2 => cpu.cp0().read_count(),
                3 => 1,
                29 => cpu.cp0().read_user_local(),

                _ => {
                    cpu.raise_exception(Exception::ReservedInstruction, None);
                    return;
                }
            };

            cpu.write_reg(rt, value);
        }

        Operation::Mfc1 => {
            let rt = inst.rt() as usize;
            let fs = inst.fs() as usize;

            let value = cpu.cp1().read_word(fs);

            cpu.write_reg(rt, value);
        }

        Operation::Mtc1 => {
            let rt = inst.rt() as usize;
            let fs = inst.fs() as usize;

            let value = cpu.read_reg(rt);

            cpu.cp1_mut().write_word(fs, value);
        }

        Operation::Mfhc1 => {
            let rt = inst.rt() as usize;
            let fs = inst.fs() as usize;

            match cpu.cp1().read_high_word(fs) {
                Some(value) => {
                    cpu.write_reg(rt, value);
                }

                None => {
                    cpu.raise_exception(Exception::ReservedInstruction, None);
                }
            }
        }

        Operation::Mthc1 => {
            let rt = inst.rt() as usize;
            let fs = inst.fs() as usize;

            let value = cpu.read_reg(rt);

            if !cpu.cp1_mut().write_high_word(fs, value) {
                cpu.raise_exception(Exception::ReservedInstruction, None);
            }
        }

        Operation::Cfc1 => {
            let rt = inst.rt() as usize;
            let fs = inst.fs();

            match cpu.cp1().read_control(fs) {
                Some(value) => {
                    cpu.write_reg(rt, value);
                }

                None => {
                    cpu.raise_exception(Exception::ReservedInstruction, None);
                }
            }
        }

        Operation::Ctc1 => {
            let rt = inst.rt() as usize;
            let fs = inst.fs();

            let value = cpu.read_reg(rt);

            if !cpu.cp1_mut().write_control(fs, value) {
                cpu.raise_exception(Exception::ReservedInstruction, None);

                return;
            }

            if cpu.cp1().exception_requested() {
                cpu.raise_exception(Exception::FloatingPoint, None);
            }
        }

        Operation::Mfc0 => {
            let rt = inst.rt() as usize;
            let rd = inst.rd();
            let sel = inst.sel();

            match cpu.cp0().read_reg(rd, sel) {
                Some(value) => cpu.write_reg(rt, value),

                None => {
                    cpu.raise_exception(Exception::ReservedInstruction, None);
                }
            }
        }

        Operation::Mtc0 => {
            let rt = inst.rt() as usize;
            let rd = inst.rd();
            let sel = inst.sel();

            let value = cpu.read_reg(rt);

            if !cpu.cp0_mut().write_reg(rd, sel, value) {
                cpu.raise_exception(Exception::ReservedInstruction, None);
            }
        }

        Operation::Ei | Operation::Di => {
            let rt = inst.rt() as usize;

            let enabled = matches!(operation, Operation::Ei);

            let old_status = cpu.cp0_mut().set_interrupt_enabled(enabled);

            cpu.write_reg(rt, old_status);
        }

        Operation::Eret => {
            cpu.return_from_exception();
        }

        Operation::Tlbr => {
            cpu.cp0_mut().tlb_read();
        }

        Operation::Tlbwi => {
            cpu.cp0_mut().tlb_write_indexed();
        }

        Operation::Tlbwr => {
            cpu.cp0_mut().tlb_write_random();
        }

        Operation::Tlbp => {
            cpu.cp0_mut().tlb_probe();
        }
        _ => todo!(
            "Instruction not implemented: opcode={:#04x}, funct={:#04x}",
            inst.opcode(),
            inst.funct()
        ),
    }
}
