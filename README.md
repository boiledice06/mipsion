
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

v0.2.0 — System Control Core

| Metric | Status |
| --- | ---: |
| ISA entries with execution logic | 114 / 291 (39.18%) |
| Mandatory CPU instruction execution paths | 105 / 105 |
| Implemented (normal execution semantics) | 68 |
| Partially implemented | 46 |
| Pending | 177 |
| Regression tests | 145 / 145 passing |

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
- [x] CP0 system register access
- [x] Exception vector delivery and ERET
- [x] Kernel, supervisor and user privilege modes
- [x] Timer and software interrupts
- [x] MMU and segment permission checks
- [x] 32-entry software-managed TLB
- [x] TLB translation and variable page sizes
- [x] TLB refill, invalid and modified exceptions
- [x] CPU configuration registers
- [ ] Full system-control conformance
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
- `cp0` — system registers, privilege, exceptions and interrupts
- `mmu` — address translation and segment permissions
- `tlb` — software-managed translation lookaside buffer
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

- Functional instruction emulation, not cycle-accurate simulation.
- Bare translation mode remains the default for compatibility.
- TLB-backed translation is available through the TLB execution mode.
- Physical memory bounds and bus errors remain incomplete.
- CP0, TLB and exception behavior cover a limited architecture profile.
- Cache behavior, execution hazards and full synchronization semantics are incomplete.
- Floating-point instructions and optional coprocessors are not implemented.
- Standalone program loading and graphical interfaces are not yet available.

## Goals

- Accurate MIPS32 instruction emulation
- Modular and extensible architecture
- Interactive inspection of CPU and memory state
- Visualization of instruction execution and pipeline behavior
- Minimal dependencies in the emulation core
