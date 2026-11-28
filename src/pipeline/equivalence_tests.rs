use super::{Pipeline, Writeback};
use crate::{
    assembler::assemble, cpu::Cpu, exception::Exception, loader::load_raw, memory::Memory,
};

fn initialize(source: &str, words: usize) -> (Cpu, Memory) {
    let image = assemble(source, 0).unwrap();
    let mut memory = Memory::new(words * 4);
    load_raw(&mut memory, 0, &image.bytes).unwrap();
    (Cpu::new(), memory)
}

fn compare_at_writeback(pipeline: &Pipeline, reference: &Cpu, memory: &Memory) {
    let cpu = pipeline.cpu();
    for register in 0..32 {
        assert_eq!(
            cpu.read_reg(register),
            reference.read_reg(register),
            "GPR {register}"
        );
    }
    assert_eq!(cpu.read_pc(), reference.read_pc());
    assert_eq!(cpu.read_next_pc(), reference.read_next_pc());
    assert_eq!(cpu.read_hilo(), reference.read_hilo());
    assert_eq!(cpu.exception_count(), reference.exception_count());
    assert_eq!(cpu.last_instruction(), reference.last_instruction());
    assert_eq!(cpu.cp0().read_count(), reference.cp0().read_count());
    assert_eq!(cpu.cp0().read_epc(), reference.cp0().read_epc());
    assert_eq!(cpu.cp0().branch_delay(), reference.cp0().branch_delay());
    assert_eq!(cpu.cp0().exception_code(), reference.cp0().exception_code());
    assert_eq!(cpu.cp0().read_bad_vaddr(), reference.cp0().read_bad_vaddr());
    for address in (0..memory.size() as u32).step_by(4) {
        assert_eq!(
            pipeline.memory().read_word(address),
            memory.read_word(address),
            "word {address:#x}"
        );
    }
}

fn run_equivalent(mut cpu: Cpu, mut memory: Memory) -> Pipeline {
    let mut pipeline = Pipeline::new(cpu.clone(), memory.clone());
    for _ in 0..250 {
        let event = pipeline.cycle().unwrap();
        if let Some(writeback) = event.writeback {
            assert!(!matches!(writeback, Writeback::Unsupported { .. }));
            cpu.step(&mut memory);
            compare_at_writeback(&pipeline, &cpu, &memory);
            if matches!(writeback, Writeback::Exception { .. }) {
                assert!(event.flushed);
                return pipeline;
            }
        }
    }
    panic!("test program did not terminate with an exception");
}

#[test]
fn dependent_loop_likely_annul_jal_jr_and_link_values_match() {
    let (cpu, memory) = initialize(
        "addiu $t0, $zero, 3\nloop: addiu $t0, $t0, -1\n         bne $t0, $zero, loop\naddiu $t1, $t1, 1\n         bnel $zero, $zero, fail\nsyscall\n         jal callee\nmove $t2, $ra\nj finish\nnop\n         callee: jr $ra\naddiu $t3, $zero, 9\n         fail: break\nfinish: sw $t2, 128($zero)\nlw $t4, 128($zero)\nsyscall",
        64,
    );
    let pipeline = run_equivalent(cpu, memory);
    assert_eq!(pipeline.cpu().read_reg(9), 3);
    assert_eq!(pipeline.cpu().read_reg(10), 32);
    assert_eq!(pipeline.cpu().read_reg(11), 9);
    assert_eq!(pipeline.cpu().read_reg(12), 32);
}

#[test]
fn overflow_in_a_delay_slot_preserves_older_state_and_squashes_younger_store() {
    let (cpu, memory) = initialize(
        "li $t0, 0x7fffffff\nb target\naddi $t0, $t0, 1\ntarget: sw $t0, 64($zero)",
        32,
    );
    let pipeline = run_equivalent(cpu, memory);
    let cpu = pipeline.cpu();
    assert_eq!(cpu.read_reg(8), 0x7fff_ffff);
    assert_eq!(cpu.cp0().read_epc(), 8);
    assert!(cpu.cp0().branch_delay());
    assert_eq!(cpu.cp0().exception_code(), Exception::Overflow.code());
    assert_eq!(pipeline.memory().read_word(64), 0);
}

#[test]
fn not_taken_nonlikely_branch_executes_the_faulting_delay_slot() {
    let (cpu, memory) = initialize("bne $zero, $zero, stop\nlw $t0, 1($zero)\nstop: break", 16);
    let pipeline = run_equivalent(cpu, memory);
    assert!(pipeline.cpu().cp0().branch_delay());
    assert_eq!(pipeline.cpu().cp0().read_bad_vaddr(), 1);
}

#[test]
fn misaligned_register_jump_reports_instruction_address_only_after_its_slot() {
    let (cpu, memory) = initialize("addiu $t0, $zero, 3\njr $t0\naddiu $t1, $zero, 42", 16);
    let pipeline = run_equivalent(cpu, memory);
    let cpu = pipeline.cpu();
    assert_eq!(cpu.read_reg(9), 42);
    assert!(!cpu.cp0().branch_delay());
    assert_eq!(cpu.cp0().read_epc(), 3);
    assert_eq!(cpu.cp0().read_bad_vaddr(), 3);
}

#[test]
fn store_to_an_inflight_instruction_refetches_the_new_word() {
    let (mut cpu, memory) = initialize("sw $t0, 8($zero)\nnop\naddiu $t1, $zero, 1\nsyscall", 16);
    cpu.write_reg(8, 0x2409_002a);
    let pipeline = run_equivalent(cpu, memory);
    assert_eq!(pipeline.cpu().read_reg(9), 42);
}

#[test]
fn paused_fetch_keeps_pending_slot_and_can_resume_at_the_target() {
    let (cpu, memory) = initialize("b target\naddiu $t0, $zero, 42\nbreak\ntarget: syscall", 16);
    let mut pipeline = Pipeline::new(cpu, memory);
    pipeline.cycle().unwrap();
    pipeline.pause_fetch();
    for _ in 0..4 {
        pipeline.cycle().unwrap();
    }
    assert!(pipeline.is_empty());
    assert_eq!(pipeline.cpu().read_pc(), 4);
    assert_eq!(pipeline.cpu().read_next_pc(), 12);
    pipeline.resume_fetch();
    assert_eq!(pipeline.cycle().unwrap().fetched, Some(4));
    assert_eq!(pipeline.cycle().unwrap().fetched, Some(12));
    while pipeline.cycle().unwrap().writeback.is_none() {}
    assert_eq!(pipeline.cpu().read_reg(8), 42);
}

#[test]
fn unsupported_operations_stop_without_commit_or_younger_side_effects() {
    let (cpu, memory) = initialize("mfc0 $t0, $12\naddiu $t1, $zero, 42", 16);
    let mut pipeline = Pipeline::new(cpu, memory);
    let outcome = loop {
        if let Some(outcome) = pipeline.cycle().unwrap().writeback {
            break outcome;
        }
    };
    assert_eq!(
        outcome,
        Writeback::Unsupported {
            pc: 0,
            word: 0x4008_6000,
            mnemonic: Some("mfc0")
        }
    );
    assert_eq!(pipeline.retired(), 0);
    assert_eq!(pipeline.cpu().read_pc(), 0);
    assert_eq!(pipeline.cpu().read_reg(9), 0);
    assert!(pipeline.cycle().is_none());
}

#[test]
fn illegal_control_in_delay_slot_is_reported_explicitly() {
    let (cpu, memory) = initialize("b target\nj 0\ntarget: syscall", 16);
    let mut pipeline = Pipeline::new(cpu, memory);
    loop {
        if let Some(Writeback::Unsupported { pc, .. }) = pipeline.cycle().unwrap().writeback {
            assert_eq!(pc, 4);
            break;
        }
    }
    assert_eq!(pipeline.retired(), 1);
    assert_eq!(pipeline.cpu().read_pc(), 4);
}
