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

#[test]
fn independent_schedule_and_cache_counters_match_four_timing_modes() {
    use crate::cache::CacheStats;

    let (cpu, mut memory) = initialize(
        "lw $t0, 128($0)\naddu $t1, $t0, $t0\nsw $t1, 132($0)\nlw $t2, 132($0)\nsyscall",
        64,
    );
    memory.write_word(128, 21);
    let split = CacheModelConfig {
        instruction: CacheConfig::new(64, 32, 1).unwrap(),
        data: CacheConfig::new(64, 16, 2).unwrap(),
        miss_latency: 2,
    };
    for (caches, forwarding, retire, trap) in [
        (None, false, [5, 8, 11, 12], 13),
        (None, true, [5, 7, 8, 9], 10),
        (Some(split), false, [9, 12, 15, 16], 17),
        (Some(split), true, [9, 11, 12, 13], 14),
    ] {
        let (pipeline, events) = run(
            cpu.clone(),
            memory.clone(),
            PipelineOptions { caches, forwarding },
        );
        let wb: Vec<_> = events
            .iter()
            .filter_map(|event| event.writeback.map(|writeback| (event.cycle, writeback)))
            .collect();
        let words = [0x8c08_0080, 0x0108_4821, 0xac09_0084, 0x8c0a_0084];
        for (index, ((cycle, actual), word)) in wb.iter().zip(words).enumerate() {
            assert_eq!(
                *cycle,
                retire[index],
                "forwarding={forwarding} cache={}",
                caches.is_some()
            );
            assert_eq!(
                *actual,
                Writeback::Retired {
                    pc: index as u32 * 4,
                    word
                }
            );
        }
        assert_eq!(wb.len(), 5);
        assert_eq!(
            wb[4],
            (
                trap,
                Writeback::Exception {
                    pc: 16,
                    exception: Exception::Syscall,
                    epc: 16,
                    in_delay_slot: false
                }
            )
        );
        assert_eq!(pipeline.memory().read_word(132), 42);
        assert_eq!(pipeline.cpu().read_reg(10), 42);
        let stats = pipeline.statistics();
        assert_eq!(stats.cycles, trap);
        assert_eq!(stats.retired, 4);
        assert_eq!(stats.cpi(), Some(trap as f64 / 4.0));
        assert_eq!(stats.ipc(), Some(4.0 / trap as f64));
        if caches.is_some() {
            assert_eq!(
                stats.instruction_cache,
                Some(CacheStats {
                    reads: 8,
                    hits: 7,
                    misses: 1,
                    ..CacheStats::default()
                })
            );
            assert_eq!(
                stats.data_cache,
                Some(CacheStats {
                    reads: 2,
                    writes: 1,
                    hits: 2,
                    misses: 1,
                    ..CacheStats::default()
                })
            );
            assert_eq!(
                events
                    .iter()
                    .filter(|event| matches!(event.stall, Some(Stall::InstructionMemory { .. })))
                    .count(),
                2
            );
            assert_eq!(
                events
                    .iter()
                    .filter(|event| matches!(event.stall, Some(Stall::DataMemory { .. })))
                    .count(),
                2
            );
            let simultaneous: Vec<_> = events
                .iter()
                .filter_map(|event| {
                    Some((event.cycle, event.instruction_cache?, event.data_cache?))
                })
                .collect();
            assert_eq!(
                simultaneous
                    .iter()
                    .map(|(cycle, _, _)| *cycle)
                    .collect::<Vec<_>>(),
                if forwarding { [11, 12] } else { [14, 15] }
            );
            assert!(simultaneous.iter().all(|(_, i, d)| i.result == AccessResult::Hit && d.result == AccessResult::Hit));
        } else {
            assert_eq!(stats.instruction_cache, None);
            assert_eq!(stats.data_cache, None);
        }
    }
}

#[test]
fn branch_and_store_dependencies_preserve_retirement_across_geometry_and_latency() {
    let (cpu, mut memory) = initialize(
        "lw $t0, 128($0)\naddiu $zero, $t0, 2\naddiu $t2, $0, 132\nbne $t0, $0, taken\nsw $t0, 0($t2)\nbreak\ntaken: lw $t1, 132($0)\nbnel $t0, $t1, finish\nbreak\nfinish: syscall",
        64,
    );
    memory.write_word(128, 0x8000_00a5);
    for forwarding in [false, true] {
        for caches in [
            None,
            Some(CacheModelConfig {
                instruction: CacheConfig::new(8, 4, 1).unwrap(),
                data: CacheConfig::new(8, 4, 2).unwrap(),
                miss_latency: 0,
            }),
            Some(CacheModelConfig {
                instruction: CacheConfig::new(48, 8, 3).unwrap(),
                data: CacheConfig::new(32, 8, 1).unwrap(),
                miss_latency: 1,
            }),
            Some(CacheModelConfig {
                instruction: CacheConfig::new(32, 16, 2).unwrap(),
                data: CacheConfig::new(64, 16, 2).unwrap(),
                miss_latency: 7,
            }),
        ] {
            let (pipeline, events) = run(
                cpu.clone(),
                memory.clone(),
                PipelineOptions { caches, forwarding },
            );
            let pcs: Vec<_> = events
                .iter()
                .filter_map(|event| match event.writeback {
                    Some(Writeback::Retired { pc, .. }) => Some(pc),
                    _ => None,
                })
                .collect();
            assert_eq!(pcs, [0, 4, 8, 12, 16, 24, 28]);
            assert_eq!(pipeline.cpu().read_reg(0), 0);
            assert_eq!(pipeline.cpu().read_reg(9), 0x8000_00a5);
            assert_eq!(pipeline.memory().read_word(132), 0x8000_00a5);
            assert_eq!(pipeline.cpu().cp0().read_epc(), 36);
            assert_eq!(pipeline.cpu().cp0().exception_code(), 8);
            assert!(
                events
                    .iter()
                    .flat_map(|event| event.forwarded.into_iter().flatten())
                    .all(|event| event.register != 0)
            );
        }
    }
}

#[test]
fn precise_exception_cancels_outstanding_fetch_and_keeps_request_accounting() {
    let (cpu, mut memory) = initialize("syscall\naddiu $t0, $0, 7", 128);
    memory.write_word(0x180, 0x2409_002a);
    let mut pipeline = Pipeline::with_options(cpu, memory, options(4, 4));
    let pristine = pipeline.statistics();
    assert_eq!(pristine.cpi(), None);
    assert_eq!(pristine.ipc(), None);
    let events: Vec<_> = (0..9).map(|_| pipeline.cycle().unwrap()).collect();
    assert_eq!(events[4].fetched, Some(0));
    assert_eq!(events[5].instruction_cache.unwrap().address, 4);
    assert!(events.iter().all(|event| event.fetched != Some(4)));
    assert_eq!(
        events[8].writeback,
        Some(Writeback::Exception {
            pc: 0,
            exception: Exception::Syscall,
            epc: 0,
            in_delay_slot: false
        })
    );
    assert!(events[8].flushed && pipeline.is_empty());
    assert_eq!(pipeline.cpu().read_reg(8), 0);
    assert_eq!(pipeline.statistics().cpi(), None);
    assert_eq!(pipeline.statistics().ipc(), Some(0.0));
    assert_eq!(pipeline.instruction_cache().unwrap().stats().misses, 2);
    let restart = pipeline.cycle().unwrap();
    assert_eq!(restart.cycle, 10);
    assert_eq!(restart.instruction_cache.unwrap().address, 0x180);
    assert_eq!(
        restart.stall,
        Some(Stall::InstructionMemory { pc: 0x8000_0180 })
    );
}
