use mipsion::assembler::assemble;
use mipsion::cache::{AccessResult, CacheAccess, CacheConfig};
use mipsion::cpu::Cpu;
use mipsion::exception::Exception;
use mipsion::loader::load_raw;
use mipsion::memory::Memory;
use mipsion::pipeline::{
    AluSnapshot, CacheModelConfig, CommittedWrite, Pipeline, PipelineOptions, StageAddress,
    StageEffect, Writeback,
};

fn machine(source: &str) -> (Cpu, Memory) {
    let mut memory = Memory::new(256);
    load_raw(&mut memory, 0, &assemble(source, 0).unwrap().bytes).unwrap();
    (Cpu::new(), memory)
}

fn timing(
    instruction: (u32, u32, u32),
    data: (u32, u32, u32),
    miss_latency: u32,
) -> PipelineOptions {
    let geometry = |(bytes, line, ways)| CacheConfig::new(bytes, line, ways).unwrap();
    PipelineOptions {
        caches: Some(CacheModelConfig {
            instruction: geometry(instruction),
            data: geometry(data),
            miss_latency,
        }),
        forwarding: true,
    }
}

#[test]
fn observed_cycles_match_plain_cycles_and_snapshot_reads_are_pure() {
    let (cpu, memory) = machine("lw $t0, 64($zero)\naddiu $t1, $t0, 5\nsw $t1, 68($zero)\nsyscall");
    let options = timing((32, 8, 1), (32, 8, 2), 2);
    let mut observed = Pipeline::with_options(cpu.clone(), memory.clone(), options);
    let mut plain = Pipeline::with_options(cpu.clone(), memory.clone(), options);
    let initial = observed.snapshot();
    let mut exception = false;
    for _ in 0..100 {
        let before = observed.snapshot();
        assert_eq!(observed.snapshot(), before);
        let actual = observed.cycle_observed().unwrap();
        assert_eq!(actual.event, plain.cycle().unwrap());
        assert_eq!(actual.snapshot, observed.snapshot());
        assert_eq!(actual.snapshot, plain.snapshot());
        assert_eq!(
            observed.memory().read_word(68),
            plain.memory().read_word(68)
        );
        if matches!(actual.event.writeback, Some(Writeback::Exception { .. })) {
            exception = true;
            break;
        }
    }
    assert!(exception);
    assert_eq!(observed.memory().read_word(68), 5);
    assert_eq!(
        Pipeline::with_options(cpu, memory, options).snapshot(),
        initial
    );
}

#[test]
fn observes_actual_alu_inputs_memory_requests_and_only_committed_writes() {
    let (cpu, mut memory) = machine(
        "addiu $t0, $zero, -2\nsb $t0, 64($zero)\nlb $t1, 64($zero)\naddiu $zero, $zero, 1\nsyscall",
    );
    memory.write_word(64, 0xabcd_00a5);
    let mut pipeline = Pipeline::with_options(
        cpu,
        memory,
        PipelineOptions {
            forwarding: true,
            caches: None,
        },
    );
    let events: Vec<_> = (0..9).map(|_| pipeline.cycle_observed().unwrap()).collect();

    assert_eq!(events[0].snapshot.if_id.unwrap().sequence, 1);
    assert_eq!(events[1].snapshot.id_ex.unwrap().instruction.sequence, 1);
    assert_eq!(
        events[1]
            .snapshot
            .id_ex
            .unwrap()
            .inputs
            .unwrap()
            .register_values,
        [0, 0]
    );
    let arithmetic = events[2].executed.unwrap();
    assert_eq!(arithmetic.instruction.sequence, 1);
    assert_eq!(
        arithmetic.alu,
        Some(AluSnapshot {
            lhs: 0,
            rhs: 0xffff_fffe,
            result: Some(0xffff_fffe)
        })
    );
    assert_eq!(events[2].snapshot.ex_mem, Some(arithmetic));
    assert_eq!(
        events[3].executed.unwrap().effect,
        StageEffect::Store {
            address: StageAddress::Virtual(64),
            bytes: 1,
            value: 0xffff_fffe
        }
    );
    assert_eq!(events[3].snapshot.mem_wb.unwrap().instruction.sequence, 1);

    assert_eq!(events[4].writeback_instruction.unwrap().sequence, 1);
    assert_eq!(
        events[4].committed_write,
        Some(CommittedWrite::Register {
            register: 8,
            before: 0,
            after: 0xffff_fffe
        })
    );
    assert_eq!(
        events[4].snapshot.mem_wb.unwrap().effect,
        StageEffect::Store {
            address: StageAddress::Physical(64),
            bytes: 1,
            value: 0xffff_fffe
        }
    );
    let store_request = events[4].memory_request.unwrap();
    assert_eq!(
        (
            store_request.kind,
            store_request.physical_address,
            store_request.bytes,
            store_request.read_value
        ),
        (CacheAccess::Write, 64, 1, None)
    );

    assert_eq!(
        events[5].committed_write,
        Some(CommittedWrite::Memory {
            physical_address: 64,
            bytes: 1,
            before: 0xa5,
            after: 0xfe
        })
    );
    let load_request = events[5].memory_request.unwrap();
    assert_eq!(
        (
            load_request.kind,
            load_request.virtual_address,
            load_request.read_value
        ),
        (CacheAccess::Read, 64, Some(0xfe))
    );
    assert_eq!(
        events[6].committed_write,
        Some(CommittedWrite::Register {
            register: 9,
            before: 0,
            after: 0xffff_fffe
        })
    );
    assert_eq!(events[7].committed_write, None);
    assert!(matches!(
        events[7].event.writeback,
        Some(Writeback::Retired { pc: 12, .. })
    ));
    assert_eq!(events[8].committed_write, None);
    assert_eq!(events[8].snapshot.architectural_pc, 0x8000_0180);
    assert_eq!(events[8].snapshot.cp0.epc, 16);
    assert_eq!(events[8].snapshot.cp0.count, 5);
    assert_eq!(pipeline.memory().read_word(64), 0xabcd_00fe);
}

#[test]
fn looping_instructions_have_distinct_ids_and_pending_fetch_counts_down() {
    let (cpu, memory) = machine("loop: b loop\nnop");
    let mut pipeline = Pipeline::with_options(cpu, memory, timing((16, 4, 1), (16, 4, 1), 2));
    let first = pipeline.cycle_observed().unwrap();
    let pending = first.snapshot.pending_fetch.unwrap();
    assert_eq!(pending.cycles_remaining, 2);
    assert_eq!(pending.entry.pc, 0);
    assert_eq!(first.snapshot.if_id, None);
    assert_eq!(
        pipeline
            .cycle_observed()
            .unwrap()
            .snapshot
            .pending_fetch
            .unwrap()
            .cycles_remaining,
        1
    );
    let accepted = pipeline.cycle_observed().unwrap();
    assert_eq!(accepted.snapshot.if_id, Some(pending.entry));
    assert_eq!(accepted.snapshot.pending_fetch, None);

    let mut branches = Vec::new();
    let mut saw_slot = false;
    for _ in 0..30 {
        let event = pipeline.cycle_observed().unwrap();
        if let Some(instruction) = event.writeback_instruction {
            if instruction.pc == 0 {
                branches.push(instruction.sequence);
                if branches.len() == 2 {
                    break;
                }
            } else {
                assert_eq!(instruction.pc, 4);
                assert!(instruction.in_delay_slot);
                saw_slot = true;
            }
        }
    }
    assert!(saw_slot);
    assert_eq!(branches.len(), 2);
    assert_eq!(branches[0], pending.entry.sequence);
    assert!(branches[1] > branches[0]);
}

#[test]
fn cache_locations_reflect_fills_and_reads_do_not_change_replacement() {
    let (cpu, memory) = machine("lw $t0, 64($zero)\nlw $t1, 72($zero)\nlw $t2, 80($zero)\nsyscall");
    let mut pipeline = Pipeline::with_options(cpu, memory, timing((32, 32, 1), (16, 8, 2), 2));
    let mut seen = Vec::new();
    for _ in 0..25 {
        let event = pipeline.cycle_observed().unwrap();
        if let Some(access) = event.event.instruction_cache {
            let line = event.instruction_cache_line.unwrap();
            assert_eq!(line.parts.line_base, access.address & !31);
        }
        if let Some(access) = event.event.data_cache {
            let line = event.data_cache_line.unwrap();
            assert_eq!(line.parts.set, 0);
            assert_eq!(line.parts.tag, access.address / 8);
            assert!(line.state.valid && !line.state.dirty);
            seen.push((line.way, access.result));
            if seen.len() == 2 {
                let before = pipeline.statistics();
                for _ in 0..3 {
                    assert_eq!(
                        pipeline
                            .data_cache()
                            .unwrap()
                            .resident_line(65)
                            .unwrap()
                            .way,
                        0
                    );
                }
                assert_eq!(pipeline.statistics(), before);
            }
        } else if let Some(pending) = event.snapshot.pending_memory {
            assert!(pending.cycles_remaining > 0);
            assert!(event.executed.is_none());
        }
        if matches!(event.event.writeback, Some(Writeback::Exception { .. })) {
            break;
        }
    }
    assert_eq!(
        seen,
        [
            (0, AccessResult::Miss { evicted: None }),
            (1, AccessResult::Miss { evicted: None }),
            (0, AccessResult::Miss { evicted: Some(64) }),
        ]
    );
    assert!(pipeline.data_cache().unwrap().resident_line(64).is_none());
}

#[test]
fn observes_coherent_self_modification_and_refetch_with_distinct_identity() {
    let (mut cpu, memory) = machine("sw $t0, 8($t2)\nnop\naddiu $t1, $zero, 1\nsyscall");
    cpu.write_reg(8, 0x2409_002a);
    cpu.write_reg(10, 0x8000_0000);
    let mut pipeline = Pipeline::with_options(cpu, memory, timing((32, 16, 2), (32, 16, 2), 2));
    let mut original = None;
    let mut updated = None;
    let mut invalidations = 0;
    for _ in 0..40 {
        let event = pipeline.cycle_observed().unwrap();
        if let Some(instruction) = event
            .snapshot
            .if_id
            .filter(|instruction| instruction.pc == 8)
        {
            match instruction.word {
                Ok(0x2409_0001) => original = Some(instruction.sequence),
                Ok(0x2409_002a) => updated = Some(instruction.sequence),
                other => panic!("{other:?}"),
            }
        }
        if let Some(request) = event.memory_request {
            assert_eq!(
                (request.virtual_address, request.physical_address),
                (0x8000_0008, 8)
            );
        }
        if let Some(line) = event.invalidated_line {
            invalidations += 1;
            assert!(event.event.flushed);
            assert_eq!(line.parts.line_base, 0);
            assert!(line.state.valid);
            assert!(
                !pipeline
                    .instruction_cache()
                    .unwrap()
                    .line(line.parts.set, line.way)
                    .unwrap()
                    .valid
            );
            assert_eq!(
                event.committed_write,
                Some(CommittedWrite::Memory {
                    physical_address: 8,
                    bytes: 4,
                    before: 0x2409_0001,
                    after: 0x2409_002a
                })
            );
        }
        if matches!(event.event.writeback, Some(Writeback::Exception { .. })) {
            break;
        }
    }
    assert_eq!(invalidations, 1);
    assert!(updated.unwrap() > original.unwrap());
    assert_eq!(pipeline.cpu().read_reg(9), 42);
}

#[test]
fn observes_checked_overflow_without_a_result_or_committed_write() {
    let (mut cpu, memory) = machine("addi $t0, $t0, 1\nsw $t0, 64($zero)");
    cpu.write_reg(8, 0x7fff_ffff);
    let mut pipeline = Pipeline::new(cpu, memory);
    let events: Vec<_> = (0..5).map(|_| pipeline.cycle_observed().unwrap()).collect();
    let alu = events[2].executed.unwrap().alu.unwrap();
    assert_eq!(
        alu,
        AluSnapshot {
            lhs: 0x7fff_ffff,
            rhs: 1,
            result: None
        }
    );
    assert!(matches!(
        events[4].event.writeback,
        Some(Writeback::Exception {
            pc: 0,
            exception: Exception::Overflow,
            ..
        })
    ));
    assert!(events[4].event.flushed);
    assert!(events[4].snapshot.ex_mem.is_none() && events[4].snapshot.pending_memory.is_none());
    assert!(
        events
            .iter()
            .all(|event| event.committed_write.is_none() && event.memory_request.is_none())
    );
    assert_eq!(pipeline.cpu().read_reg(8), 0x7fff_ffff);
    assert_eq!(pipeline.memory().read_word(64), 0);
}
