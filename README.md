
<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)"
            srcset="assets/logo-dark.png">
    <source media="(prefers-color-scheme: light)"
            srcset="assets/logo.png">
    <img alt="MIPSion" src="assets/logo.png" width="480">
  </picture>
</p>

<h3 align="center">
  A MIPS32 Release 2 Emulator Written in Rust
</h3>


A MIPS32 Release 2 emulator written in Rust.

MIPSion is a personal project exploring instruction set architecture, CPU emulation, and microarchitecture through a modular implementation of MIPS32 Release 2.

The long-term goal is to provide an interactive graphical environment for executing, debugging, and visualizing MIPS programs.

## Status

Early development — baseline integer core milestone.

| Metric | Status |
| --- | ---: |
| ISA entries with execution logic | 105 / 291 (36.08%) |
| Mandatory CPU instruction execution paths | 105 / 105 |
| Implemented (normal execution semantics) | 68 |
| Partially implemented | 37 |
| Pending | 186 |
| Regression tests | 54 / 54 passing |

The ISA count excludes three assembly idioms and includes optional instruction families. Regression results reflect the latest reported local run, not exhaustive ISA conformance or code coverage.

- [x] CPU state, GPRs and HI/LO registers
- [x] Byte-addressable, little-endian memory
- [x] Instruction fetch and field extraction
- [x] Table-driven instruction decoder
- [x] Integer arithmetic, logic, shifts and bit manipulation (normal paths)
- [x] Multiplication, division and multiply-accumulate
- [x] Conditional move
- [x] Basic and unaligned load/store (normal paths)
- [x] Integer branches and jumps, delay slots and branch-likely annul
- [x] Baseline execution paths for all mandatory CPU instructions
- [x] Basic CP0 exception state, signed overflow and alignment checks
- [x] Single-CPU LL/SC and simplified synchronization / RDHWR
- [ ] Complete exception delivery, CP0 privilege model, MMU and TLB
- [ ] Floating-point unit
- [ ] Program loader and assembler
- [ ] Interactive debugger
- [ ] Five-stage pipeline simulation
- [ ] Cache and hazard simulation
- [ ] Graphical user interface

See [ISA_CHECKLIST.md](ISA_CHECKLIST.md) for instruction-level implementation and regression status.

## Architecture

The functional emulation core is designed independently of the user interface.

- `cpu` — CPU state, register file, PC/next-PC and execution cycle
- `memory` — byte-addressable memory and aligned/unaligned access operations
- `instruction` — raw instruction and bit-field extraction
- `isa` — instruction definitions and operation types generated at build time
- `decoder` — table-driven instruction identification
- `alu` — arithmetic, logical, bit-field and HI/LO computations
- `executor` — instruction semantics and control flow
- `cp0` — initial system register and exception state
- `exception` — architectural exception codes
- `test_helpers` / `regression` — instruction encoding and regression tests
- `build.rs` — ISA table generation from the checklist

## Getting Started

Requires a Rust toolchain with Cargo.

```sh
cargo build
cargo test -q
```

The current core is exercised through regression tests. A standalone program loader and interactive runtime are not yet available.

## Current Limitations

Exceptions currently record CP0 state and halt rather than transfer to an exception vector. Memory bounds and virtual address translation, privilege checks, full synchronization semantics and floating-point instructions remain incomplete or unimplemented. The core is currently a functional instruction emulator, not a cycle-accurate pipeline simulator.

## Goals

- Accurate MIPS32 instruction emulation
- Modular and extensible architecture
- Interactive inspection of CPU and memory state
- Visualization of instruction execution and pipeline behavior
- Minimal dependencies in the emulation core
