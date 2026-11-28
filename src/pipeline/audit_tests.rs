use super::equivalence_tests::{compare_at_writeback, initialize};
use super::{Pipeline, Writeback};
use crate::exception::Exception;
use std::fmt::Write;

#[test]
fn timer_interrupt_preempts_a_delay_slot_unsupported_by_the_pipeline() {
    let (mut cpu, mut memory) = initialize("b target\nmfc0 $t0, $12\ntarget: nop\nnop", 32);
    assert!(cpu.cp0_mut().write_reg(12, 0, 0x8001));
    cpu.cp0_mut().write_compare(2);
    let mut pipeline = Pipeline::new(cpu.clone(), memory.clone());
    let mut boundaries = Vec::new();
    for _ in 0..12 {
        let event = pipeline.cycle().unwrap();
        if let Some(outcome) = event.writeback {
            cpu.step(&mut memory);
            compare_at_writeback(&pipeline, &cpu, &memory);
            boundaries.push((event.cycle, outcome));
            if boundaries.len() == 2 {
                assert!(event.flushed);
                break;
            }
        }
    }
    assert_eq!(
        boundaries,
        [
            (
                5,
                Writeback::Retired {
                    pc: 0,
                    word: 0x1000_0001
                }
            ),
            (
                6,
                Writeback::Exception {
                    pc: 4,
                    exception: Exception::Interrupt,
                    epc: 0,
                    in_delay_slot: true
                }
            ),
        ]
    );
    assert_eq!(pipeline.cpu().cp0().read_count(), 2);
    assert_eq!(pipeline.cpu().last_instruction(), None);
    assert_eq!(pipeline.cpu().read_pc(), 0x8000_0180);
    assert!(
        pipeline.cycle().is_some(),
        "an interrupt does not stop the engine"
    );
}

#[test]
fn branch_delay_slot_and_precise_fault_have_independent_cycle_schedule() {
    let (cpu, memory) = initialize(
        "b target\naddiu $t0, $zero, 7\nsw $t0, 64($zero)\ntarget: addiu $t1, $zero, 8\nsyscall",
        32,
    );
    let mut pipeline = Pipeline::new(cpu, memory);
    let events: Vec<_> = (0..8).map(|_| pipeline.cycle().unwrap()).collect();
    assert_eq!(
        events[..4]
            .iter()
            .map(|event| event.fetched)
            .collect::<Vec<_>>(),
        [Some(0), Some(4), Some(12), Some(16)]
    );
    assert_eq!(events[2].redirect.unwrap().target, 12);
    assert_eq!(events[2].redirect.unwrap().delay_slot, Some(4));
    assert!(
        events[..4]
            .iter()
            .all(|event| event.writeback.is_none() && event.stall.is_none())
    );
    assert_eq!(
        events[4..]
            .iter()
            .map(|event| event.writeback.unwrap())
            .collect::<Vec<_>>(),
        [
            Writeback::Retired {
                pc: 0,
                word: 0x1000_0002
            },
            Writeback::Retired {
                pc: 4,
                word: 0x2408_0007
            },
            Writeback::Retired {
                pc: 12,
                word: 0x2409_0008
            },
            Writeback::Exception {
                pc: 16,
                exception: Exception::Syscall,
                epc: 16,
                in_delay_slot: false
            },
        ]
    );
    assert!(events[7].flushed && pipeline.is_empty());
    assert_eq!(pipeline.cpu().read_reg(8), 7);
    assert_eq!(pipeline.cpu().read_reg(9), 8);
    assert_eq!(pipeline.memory().read_word(64), 0);
    assert_eq!(pipeline.cpu().cp0().read_count(), 4);
}

#[test]
fn pipeline_can_resume_a_committed_functional_branch_at_its_delay_slot() {
    let (mut cpu, mut memory) = initialize("jal target\nmove $t0, $ra\nbreak\ntarget: syscall", 16);
    cpu.step(&mut memory);
    assert_eq!(cpu.read_pc(), 4);
    assert_eq!(cpu.read_next_pc(), 12);

    let mut pipeline = Pipeline::new(cpu.clone(), memory.clone());
    assert_eq!(pipeline.cycle().unwrap().fetched, Some(4));
    assert_eq!(pipeline.cycle().unwrap().fetched, Some(12));
    let mut outcomes = Vec::new();
    for _ in 0..8 {
        if let Some(outcome) = pipeline.cycle().unwrap().writeback {
            cpu.step(&mut memory);
            compare_at_writeback(&pipeline, &cpu, &memory);
            outcomes.push(outcome);
            if outcomes.len() == 2 {
                break;
            }
        }
    }
    assert_eq!(
        outcomes,
        [
            Writeback::Retired {
                pc: 4,
                word: 0x03e0_4021
            },
            Writeback::Exception {
                pc: 12,
                exception: Exception::Syscall,
                epc: 12,
                in_delay_slot: false
            },
        ]
    );
    assert_eq!(pipeline.cpu().read_reg(8), 8);
}

#[test]
fn seeded_defined_integer_programs_match_reference_and_independent_state() {
    const SEEDS: [u32; 4] = [1, 0x243f_6a88, 0xb7e1_5163, 0xdead_beef];
    const WORDS: usize = 128;
    const DATA: u32 = 0x400;
    for mut state in SEEDS {
        let seed = state;
        let mut registers = [0u32; 8];
        let mut data = [0u32; 4];
        let mut expected = Vec::new();
        let mut source = String::new();
        for _ in 0..WORDS {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            let [d, a, b, bits] = state.to_le_bytes();
            let (d, a, b) = (usize::from(d & 7), usize::from(a & 7), usize::from(b & 7));
            let gpr = |index| format!("{}{index}", '$');
            let (rd, ra, rb) = (gpr(8 + d), gpr(8 + a), gpr(8 + b));
            let slot = usize::from((bits >> 4) & 3);
            let addr = DATA + 4 * slot as u32;
            let immediate = bits as i8 as i16;
            let (a, b) = (registers[a], registers[b]);
            match state % 10 {
                0 => {
                    writeln!(source, "addiu {rd}, {ra}, {immediate}").unwrap();
                    registers[d] = a.wrapping_add_signed(i32::from(immediate));
                }
                1 => {
                    writeln!(source, "addu {rd}, {ra}, {rb}").unwrap();
                    registers[d] = a.wrapping_add(b);
                }
                2 => {
                    writeln!(source, "subu {rd}, {ra}, {rb}").unwrap();
                    registers[d] = a.wrapping_sub(b);
                }
                3 => {
                    writeln!(source, "xor {rd}, {ra}, {rb}").unwrap();
                    registers[d] = a ^ b;
                }
                4 => {
                    writeln!(source, "slt {rd}, {ra}, {rb}").unwrap();
                    registers[d] = u32::from((a as i32) < (b as i32));
                }
                5 => {
                    writeln!(source, "sllv {rd}, {ra}, {rb}").unwrap();
                    registers[d] = a << (b % 32);
                }
                6 => {
                    writeln!(source, "sw {ra}, {addr}($zero)").unwrap();
                    data[slot] = a;
                }
                7 => {
                    writeln!(source, "lw {rd}, {addr}($zero)").unwrap();
                    registers[d] = data[slot];
                }
                8 => {
                    writeln!(source, "lb {rd}, {addr}($zero)").unwrap();
                    registers[d] = data[slot] as i8 as i32 as u32;
                }
                _ => {
                    writeln!(source, "addiu $zero, {ra}, {immediate}").unwrap();
                }
            }
            expected.push((registers, data));
        }
        source.push_str("syscall\n");
        let (mut cpu, mut memory) = initialize(&source, 288);
        let mut pipeline = Pipeline::new(cpu.clone(), memory.clone());
        let mut index = 0;
        let mut terminated = false;
        for _ in 0..(WORDS * 4 + 8) {
            if let Some(outcome) = pipeline.cycle().unwrap().writeback {
                cpu.step(&mut memory);
                compare_at_writeback(&pipeline, &cpu, &memory);
                match outcome {
                    Writeback::Retired { pc, .. } => {
                        assert_eq!(pc, index as u32 * 4, "seed {seed:#x}");
                        let (registers, data) = expected[index];
                        for (reg, value) in registers.into_iter().enumerate() {
                            assert_eq!(
                                pipeline.cpu().read_reg(reg + 8),
                                value,
                                "seed {seed:#x} PC {pc:#x}"
                            );
                        }
                        for (slot, value) in data.into_iter().enumerate() {
                            assert_eq!(pipeline.memory().read_word(DATA + 4 * slot as u32), value);
                        }
                        assert_eq!(pipeline.cpu().read_reg(0), 0);
                        index += 1;
                    }
                    Writeback::Exception {
                        exception: Exception::Syscall,
                        epc,
                        ..
                    } => {
                        assert_eq!(epc, WORDS as u32 * 4);
                        terminated = true;
                        break;
                    }
                    other => panic!("seed {seed:#x}: unexpected writeback {other:?}"),
                }
            }
        }
        assert!(terminated, "seed {seed:#x} did not terminate");
        assert_eq!(index, WORDS);
        assert_eq!(pipeline.retired(), WORDS as u64);
        assert_eq!(pipeline.cpu().cp0().read_count(), WORDS as u32 + 1);
    }
}
