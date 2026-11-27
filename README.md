
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

v0.3.0 (in development) — Floating-Point Unit

| Metric | Status |
| --- | ---: |
| ISA entries with execution logic | 200 / 291 (68.73%) |
| Mandatory CPU instruction execution paths | 105 / 105 |
| Basic FPU instruction execution paths | 86 / 86 |
| Implemented (normal execution semantics) | 68 |
| Partially implemented | 132 |
| Pending | 91 |
| Regression tests | 343 / 343 passing |

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
- [x] CP1 register file and access control
- [x] FPU control registers and basic exception entry
- [x] FR=0 doubleword register mapping
- [x] GPR/FPR and word/doubleword memory transfers
- [x] Basic single-precision arithmetic
- [x] Basic double-precision arithmetic
- [x] Basic S/D/W format conversions
- [x] FCSR rounding modes for CVT.W instructions
- [x] Invalid/Inexact exception handling for CVT.W instructions
- [x] Explicit word rounding instructions (ROUND/TRUNC/CEIL/FLOOR)
- [x] All 32 single/double comparison instructions
- [x] FCC0–7 condition code access
- [x] Legacy NaN classification for comparisons
- [x] Floating-point conditional branches
- [x] Floating-point conditional moves
- [x] Execution paths for all 86 basic FPU instructions
- [x] Legacy NaN normalization for arithmetic
- [x] Basic Invalid and Divide-by-Zero exception handling
- [x] Basic Overflow, Underflow and Inexact detection for S/D arithmetic
- [x] Integer-based exactness checks for double-precision arithmetic
- [x] Arithmetic exception traps and destination writeback suppression
- [ ] Full floating-point conformance
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
- `fpu` — floating-point arithmetic, format conversion, rounding, comparison and exception results
- `executor` — instruction semantics and control flow
- `cp0` — system registers, privilege, exceptions and interrupts
- `cp1` — floating-point registers, control registers and data transfers
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
- FPU support includes FR=0 register mapping, control registers, word/doubleword data transfers, basic single- and double-precision arithmetic, and S/D/W format conversions.
- FCSR rounding and Invalid/Inexact exception handling are implemented for CVT.W and explicit word rounding instructions.
- Comparisons support legacy NaN classification and Invalid Operation handling.
- All 86 basic FPU instructions have execution paths.
- Basic detection of all five IEEE 754 arithmetic exceptions is available for single- and double-precision arithmetic.
- Arithmetic currently uses native nearest-even results with simplified tininess detection.
- Full FCSR rounding, FS behavior, exception priority and IEEE 754 conformance remain incomplete.
- CP1 feature advertisement and full exception conformance remain incomplete.
- Optional coprocessors other than CP1 are not implemented.
- Standalone program loading and graphical interfaces are not yet available.

## Goals

- Accurate MIPS32 instruction emulation
- Modular and extensible architecture
- Interactive inspection of CPU and memory state
- Visualization of instruction execution and pipeline behavior
- Minimal dependencies in the emulation core

## License

MIPSion is dual-licensed under either of:

- [MIT License](LICENSE-MIT)
- [Apache License, Version 2.0](LICENSE-APACHE)

at your option.