use super::{Pipeline, PipelineRegisters, Stall, Writeback};
use crate::{
    assembler::assemble, cpu::Cpu, exception::Exception, loader::load_raw, memory::Memory,
};

fn machine(source: &str, size: usize) -> (Cpu, Memory) {
    let mut memory = Memory::new(size);
    let image = assemble(source, 0).unwrap();
    load_raw(&mut memory, 0, &image.bytes).unwrap();
    (Cpu::new(), memory)
}

#[test]
fn pipeline_fills_drains_and_retires_at_writeback() {
    let (cpu, memory) = machine("addiu $t0, $zero, 42", 64);
    let mut pipeline = Pipeline::new(cpu, memory);
    assert!(pipeline.is_empty());
    assert_eq!(pipeline.cycle().unwrap().registers.if_id, Some(0));
    pipeline.pause_fetch();
    assert_eq!(pipeline.cycle().unwrap().registers.id_ex, Some(0));
    assert_eq!(pipeline.cycle().unwrap().registers.ex_mem, Some(0));
    assert_eq!(pipeline.cycle().unwrap().registers.mem_wb, Some(0));
    assert_eq!(pipeline.cpu().read_reg(8), 0);
    let event = pipeline.cycle().unwrap();
    assert_eq!(
        event.writeback,
        Some(Writeback::Retired {
            pc: 0,
            word: 0x2408_002a
        })
    );
    assert_eq!(event.registers, PipelineRegisters::default());
    assert_eq!(pipeline.cpu().read_reg(8), 42);
    assert_eq!(pipeline.retired(), 1);
    assert_eq!(pipeline.cycles(), 5);
    pipeline.resume_fetch();
    assert_eq!(pipeline.cycle().unwrap().fetched, Some(4));
}

#[test]
fn conservative_raw_stall_holds_the_front_end_until_wb() {
    let (cpu, memory) = machine("addiu $t0, $zero, 21\naddu $t1, $t0, $t0", 64);
    let mut pipeline = Pipeline::new(cpu, memory);
    pipeline.cycle().unwrap();
    pipeline.cycle().unwrap();
    pipeline.pause_fetch();
    for _ in 0..2 {
        let event = pipeline.cycle().unwrap();
        assert_eq!(
            event.stall,
            Some(Stall::RegisterDependency { pc: 4, register: 8 })
        );
        assert_eq!(event.registers.if_id, Some(4));
        assert_eq!(event.registers.id_ex, None);
        assert_eq!(event.fetched, None);
    }
    let event = pipeline.cycle().unwrap();
    assert_eq!(event.stall, None);
    assert_eq!(event.registers.id_ex, Some(4));
    while !pipeline.is_empty() {
        pipeline.cycle().unwrap();
    }
    assert_eq!(pipeline.cpu().read_reg(9), 42);
    assert_eq!(pipeline.cycles(), 8);
}

#[test]
fn pipeline_store_commits_before_younger_load_and_honors_widths() {
    let (cpu, memory) = machine(
        "addiu $t0, $zero, -128\nsw $t0, 64($zero)\nlb $t1, 64($zero)\nlbu $t2, 64($zero)\nsyscall",
        128,
    );
    let mut pipeline = Pipeline::new(cpu, memory);
    loop {
        let event = pipeline.cycle().unwrap();
        match event.writeback {
            Some(Writeback::Retired { pc: 0, .. }) => {
                assert_eq!(pipeline.memory().read_word(64), 0)
            }
            Some(Writeback::Retired { pc: 4, .. }) => {
                assert_eq!(pipeline.memory().read_word(64), 0xffff_ff80)
            }
            Some(Writeback::Exception {
                exception: Exception::Syscall,
                ..
            }) => break,
            Some(Writeback::Retired { .. }) | None => {}
            other => panic!("unexpected writeback {other:?}"),
        }
    }
    assert_eq!(pipeline.cpu().read_reg(9), 0xffff_ff80);
    assert_eq!(pipeline.cpu().read_reg(10), 0x80);
}

#[test]
fn zero_destinations_do_not_stall_and_variable_shift_masks_count() {
    let (mut cpu, memory) = machine("addiu $zero, $zero, 8\nsllv $t0, $t1, $t2\nnop", 64);
    cpu.write_reg(9, 3);
    cpu.write_reg(10, 37);
    let mut pipeline = Pipeline::new(cpu, memory);
    while pipeline.retired() < 3 {
        assert!(pipeline.cycle().unwrap().stall.is_none());
    }
    assert_eq!(pipeline.cpu().read_reg(0), 0);
    assert_eq!(pipeline.cpu().read_reg(8), 96);
}

#[test]
fn invalid_store_never_writes_and_flushes_younger_effects() {
    let (mut cpu, memory) = machine("sw $t0, 65($zero)\naddiu $t1, $zero, 7", 128);
    cpu.write_reg(8, 42);
    let mut pipeline = Pipeline::new(cpu, memory);
    for _ in 0..4 {
        assert!(!pipeline.cycle().unwrap().flushed);
    }
    let event = pipeline.cycle().unwrap();
    assert_eq!(
        event.writeback,
        Some(Writeback::Exception {
            pc: 0,
            epc: 0,
            exception: Exception::AddressErrorStore,
            in_delay_slot: false,
        })
    );
    assert!(event.flushed && pipeline.is_empty());
    assert_eq!(pipeline.cpu().cp0().read_bad_vaddr(), 65);
    assert_eq!(pipeline.memory().read_word(64), 0);
    assert_eq!(pipeline.cpu().read_reg(9), 0);
    assert_eq!(pipeline.retired(), 0);
}
