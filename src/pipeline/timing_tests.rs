use super::equivalence_tests::{compare_at_writeback, initialize};
use super::{
    CacheModelConfig, CycleEvent, ForwardingSource, Pipeline, PipelineOptions, Stall, Writeback,
};
use crate::{
    cache::{AccessResult, CacheAccess, CacheConfig},
    cpu::Cpu,
    exception::Exception,
    memory::Memory,
};

fn options(line_size: u32, latency: u32) -> PipelineOptions {
    PipelineOptions {
        caches: Some(CacheModelConfig {
            instruction: CacheConfig::new(64, line_size, 1).unwrap(),
            data: CacheConfig::new(64, 16, 2).unwrap(),
            miss_latency: latency,
        }),
        forwarding: true,
    }
}

fn run(mut cpu: Cpu, mut memory: Memory, options: PipelineOptions) -> (Pipeline, Vec<CycleEvent>) {
    let mut pipeline = Pipeline::with_options(cpu.clone(), memory.clone(), options);
    let mut events = Vec::new();
    for _ in 0..500 {
        let event = pipeline.cycle().unwrap();
        events.push(event);
        if let Some(outcome) = event.writeback {
            cpu.step(&mut memory);
            compare_at_writeback(&pipeline, &cpu, &memory);
            match outcome {
                Writeback::Retired { .. } => {}
                Writeback::Exception { .. } => return (pipeline, events),
                Writeback::Unsupported { .. } => panic!("{outcome:?}"),
            }
        }
    }
    panic!("program did not terminate");
}

#[test]
fn forwarding_uses_the_youngest_matching_writer_for_both_sources() {
    let (cpu, memory) = initialize(
        "addiu $t0, $0, 20\naddiu $t0, $t0, 1\naddu $t1, $t0, $t0\nsyscall",
        32,
    );
    let (pipeline, events) = run(
        cpu,
        memory,
        PipelineOptions {
            forwarding: true,
            ..PipelineOptions::default()
        },
    );
    assert!(events.iter().all(|event| event.stall.is_none()));
    let forwards: Vec<_> = events
        .iter()
        .flat_map(|event| event.forwarded.into_iter().flatten())
        .filter(|forward| forward.pc == 8)
        .collect();
    assert_eq!(forwards.len(), 2);
    for (index, forward) in forwards.iter().enumerate() {
        assert_eq!(forward.operand, index as u8);
        assert_eq!(forward.register, 8);
        assert_eq!(forward.value, 21);
        assert_eq!(forward.source, ForwardingSource::ExMem);
    }
    assert_eq!(pipeline.cpu().read_reg(9), 42);
}

#[test]
fn cold_misses_hold_the_correct_stages_and_load_use_issues_after_mem() {
    let (cpu, mut memory) = initialize("lw $t0, 64($0)\naddu $t1, $t0, $t0\nsyscall", 32);
    memory.write_word(64, 21);
    let (pipeline, events) = run(cpu, memory, options(16, 2));
    assert_eq!(events[0].stall, Some(Stall::InstructionMemory { pc: 0 }));
    assert_eq!(events[0].instruction_cache.unwrap().latency, 2);
    assert_eq!(events[1].stall, Some(Stall::InstructionMemory { pc: 0 }));
    assert_eq!(events[2].fetched, Some(0));
    assert_eq!(
        events
            .iter()
            .filter(|event| event.stall == Some(Stall::LoadUse { pc: 4, register: 8 }))
            .count(),
        1
    );
    let waits: Vec<_> = events
        .iter()
        .filter(|event| event.stall == Some(Stall::DataMemory { pc: 0 }))
        .collect();
    assert_eq!(waits.len(), 2);
    assert_eq!(
        waits[0].data_cache.unwrap().result,
        AccessResult::Miss { evicted: None }
    );
    assert!(waits[1].data_cache.is_none());
    assert!(
        waits
            .iter()
            .all(|event| event.registers.ex_mem == Some(0) && event.registers.if_id == Some(4))
    );
    assert!(events.iter().any(|event| {
        event
            .forwarded
            .into_iter()
            .flatten()
            .any(|forward| forward.pc == 4 && forward.source == ForwardingSource::MemWb)
    }));
    assert_eq!(pipeline.data_cache().unwrap().stats().reads, 1);
    assert_eq!(pipeline.cpu().read_reg(9), 42);
}

#[test]
fn written_data_is_visible_in_both_cache_and_backing_memory() {
    let (cpu, memory) = initialize(
        "addiu $t0, $0, -128\nsh $t0, 66($0)\nlh $t1, 66($0)\nlbu $t2, 66($0)\nsyscall",
        32,
    );
    let (pipeline, events) = run(cpu, memory, options(16, 3));
    let accesses: Vec<_> = events.iter().filter_map(|event| event.data_cache).collect();
    assert_eq!(accesses.len(), 3);
    assert_eq!(accesses[0].kind, CacheAccess::Write);
    assert_eq!(accesses[0].result, AccessResult::Miss { evicted: None });
    assert!(
        accesses[1..]
            .iter()
            .all(|event| event.result == AccessResult::Hit)
    );
    assert_eq!(pipeline.memory().read_halfword(66), 0xff80);
    assert_eq!(pipeline.cpu().read_reg(9), 0xffff_ff80);
    assert_eq!(pipeline.cpu().read_reg(10), 0x80);
}

#[test]
fn physical_alias_store_invalidates_and_refetches_an_instruction() {
    let (cpu, memory) = initialize(
        "li $t0, 0x2409002a\nlui $t1, 0x8000\nsw $t0, 24($t1)\nnop\nnop\naddiu $t1, $0, 1\nsyscall",
        32,
    );
    let (pipeline, events) = run(cpu, memory, options(32, 2));
    assert!(
        events
            .iter()
            .any(|event| event.invalidated_instruction_line == Some(0) && event.flushed)
    );
    assert_eq!(pipeline.cpu().read_reg(9), 42);
    assert_eq!(
        pipeline.instruction_cache().unwrap().stats().invalidations,
        1
    );
}

#[test]
fn likely_annul_cancels_an_outstanding_if_miss() {
    let (cpu, memory) = initialize(
        "bnel $0, $0, target\nbreak\naddiu $t0, $0, 42\ntarget: syscall",
        16,
    );
    let (pipeline, events) = run(cpu, memory, options(4, 3));
    assert!(events.iter().any(|event| {
        event
            .instruction_cache
            .is_some_and(|access| access.address == 4)
    }));
    assert!(events.iter().all(|event| event.fetched != Some(4)));
    assert!(events.iter().any(|event| {
        event
            .redirect
            .is_some_and(|redirect| redirect.delay_slot.is_none())
            && event.flushed
    }));
    assert_eq!(pipeline.cpu().read_reg(8), 42);
    assert_eq!(
        pipeline.cpu().cp0().exception_code(),
        Exception::Syscall.code()
    );
}

#[test]
fn delay_slot_fault_squashes_younger_memory_access_and_flushes_timing() {
    let (cpu, memory) = initialize("b target\nlw $t0, 1($0)\ntarget: sw $t0, 64($0)\nbreak", 32);
    let (pipeline, events) = run(cpu, memory, options(4, 2));
    assert!(events.iter().all(|event| event.data_cache.is_none()));
    assert!(pipeline.is_empty());
    assert_eq!(pipeline.memory().read_word(64), 0);
    assert!(pipeline.cpu().cp0().branch_delay());
    assert_eq!(pipeline.cpu().cp0().read_bad_vaddr(), 1);
}

#[test]
fn zero_miss_penalty_and_paused_fetch_preserve_validity_and_counts() {
    let (cpu, memory) = initialize("nop\nsyscall", 16);
    let (pipeline, events) = run(cpu.clone(), memory.clone(), options(4, 0));
    assert!(events.iter().all(|event| event.stall.is_none()));
    assert_eq!(pipeline.cycles(), 6);
    assert!(pipeline.instruction_cache().unwrap().stats().misses >= 2);

    let mut pipeline = Pipeline::with_options(cpu, memory, options(4, 2));
    pipeline.cycle().unwrap();
    pipeline.pause_fetch();
    assert!(!pipeline.is_empty());
    while !pipeline.is_empty() {
        pipeline.cycle().unwrap();
    }
    assert_eq!(pipeline.retired(), 1);
    assert_eq!(pipeline.instruction_cache().unwrap().stats().reads, 1);
    assert_eq!(pipeline.cpu().read_pc(), 4);
}
