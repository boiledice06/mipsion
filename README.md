
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

v0.7.0 — Five-stage Pipeline Simulation

| Metric | Status |
| --- | ---: |
| ISA entries with execution logic | 200 / 291 (68.73%) |
| Mandatory CPU instruction execution paths | 105 / 105 |
| Basic FPU instruction execution paths | 86 / 86 |
| Implemented (normal execution semantics) | 68 |
| Partially implemented | 132 |
| Pending | 91 |
| Internal regression tests | 634 / 634 passing |
| REPL integration tests | 16 / 16 passing |
| CLI end-to-end tests | 17 / 17 passing |
| Total tests | 667 / 667 passing |
| External Rust dependencies | 0 |

The ISA count excludes three assembly idioms and includes optional instruction families. Test results reflect verified local debug and release builds, not exhaustive ISA conformance or code coverage.

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
- [x] All four FCSR rounding modes for S/D arithmetic
- [x] Directed rounding for narrowing format conversions
- [x] Legacy NaN and exception handling for format conversions
- [x] Exact dyadic comparison using fixed-size integer accumulators
- [x] CP0 Config1.FP and CP1 FIR feature advertisement
- [x] Basic FCSR.FS result flushing for S/D arithmetic
- [x] FCSR Cause/Enable exhaustive mask testing
- [x] Unimplemented Operation exception request handling
- [x] FP exception delivery in branch delay slots
- [x] Sticky Flag and architectural state preservation
- [ ] Full floating-point conformance
- [x] Validated raw binary loader
- [x] Physical memory bounds and bus exceptions
- [x] Bounded program execution runtime
- [x] Standalone command-line interface
- [x] GPR, CP0 and FPU register inspection
- [x] Physical memory inspection
- [x] Structured instruction tracing
- [x] ELF32 little-endian MIPS executable loading
- [x] PT_LOAD segment loading and BSS initialization
- [x] ELF entry point and stack pointer initialization
- [x] Instruction disassembler
- [x] Stateful debugger session
- [x] Single-step and bounded continuous execution
- [x] Full CPU and memory reset
- [x] Virtual-address execution breakpoints
- [x] Register and physical memory watchpoints
- [x] Interactive command-line debugger
- [x] REPL integration and CLI end-to-end tests
- [x] Assembler
- [x] Five-stage pipeline simulation
- [ ] Cache and hazard simulation
- [ ] Graphical user interface

See [ISA_CHECKLIST.md](ISA_CHECKLIST.md) for instruction-level implementation and regression status.

## Architecture

MIPSion separates the reusable emulation library from the command-line interface.

| Module | Responsibility |
| --- | --- |
| `cpu/` | CPU state, ALU, instruction execution, CP0, CP1 and FPU |
| `memory/` | Physical memory, MMU and TLB |
| `isa/` | Instruction definitions, decoding, disassembly and field extraction |
| `assembler/` | Source parsing, two-pass symbol resolution and instruction encoding |
| `pipeline/` | Five-stage latches, timing, retirement and cycle events |
| `loader/` | Raw binary and ELF32 executable loading |
| `runtime/` | Execution control, tracing, hooks and state inspection |
| `debugger/` | Debugger sessions, breakpoints, watchpoints, commands and REPL |
| `cli/` | Command-line argument parsing |
| `src/tests/` | Internal regression tests and shared helpers |
| `tests/` | Public API integration and CLI end-to-end tests |
| `lib.rs` | Reusable emulator library |
| `main.rs` | Command-line application entry point |
| `build.rs` | Build-time ISA table generation |

The emulation library is independent of the application entry point.

The interactive debugger reuses the same execution core through
stateful sessions and runtime execution hooks.

Tests are organized by architectural domain, with separate
integration tests for the public debugger API and executable CLI.

## Getting Started

Requires a Rust toolchain with Cargo.

```sh
cargo build
cargo test -q
```

Run the included examples:

    cargo run -- examples/hello.bin
    cargo run -- examples/hello.elf

Inspect execution and CPU state:

    cargo run -- examples/hello.elf --trace --dump-registers
    cargo run -- examples/hello.bin --dump-memory 0 12
    cargo run -- examples/hello.bin --dump-cp0 --dump-fpu

For all available options:

    cargo run -- --help

The emulator has no external Rust dependencies.

## Assembler

```sh
cargo run -- --assemble examples/delay-slot.s -o delay-slot.bin --load-address 0x1000
cargo run -- delay-slot.bin --load-address 0x1000 --dump-registers
cargo run -- --source examples/delay-slot.s --debug
```

The library entry point is `assembler::assemble(source, load_address)`.
It returns little-endian bytes, text size, data offset, and absolute symbol
addresses. Diagnostics carry a one-based line and UTF-8 byte column. CLI
output requires a new file and pads the image to a multiple of four bytes
for the raw loader. When running a raw image, use the same load address at
which it was assembled.

The assembler supports the checklist's 200 entries with functional execution
paths, including integer, CP0/TLB and basic FR=0 FPU forms. The supported
syntax includes `$t0` and `$8` GPRs, `$f0` FPRs, `$fcc0` or numeric
condition codes, and decimal, hexadecimal (`0x`), and binary (`0b`) literals.
Labels are case-sensitive. Branch and jump operands take absolute addresses
or labels. Load/store operands use `offset(base)` and an omitted offset
means zero. Signed immediates use the mathematical signed value.

Text starts at the word-aligned load address. Data follows at the next word
boundary. `.text` and `.data` switch sections. Data directives are `.byte`,
`.half`, `.word`, `.space`, `.align` (power-of-two exponent 0–16),
`.ascii` and `.asciiz`. The assembler enforces a 256 MiB image limit and
checks alignment and address ranges. Data directives do not implicitly align
subsequent instructions. Write branch delay slots explicitly.

| Pseudo-instruction | Expansion |
| --- | --- |
| `nop` | `sll $zero, $zero, 0` |
| `move rd, rs` | `addu rd, rs, $zero` |
| `b target` | `beq $zero, $zero, target` |
| `bal target` | `bgezal $zero, target` |
| `li rt, constant` | `lui` plus `ori`, always two words |
| `la rt, label` | `lui` plus `ori`, always two words |

The assembler emits flat images. Object files, linking and relocations,
macros, expression arithmetic, automatic delay-slot scheduling, and
source-level debugger commands are outside this release.

## Interactive Debugger

Launch the debugger with a raw binary or ELF executable:

```sh
cargo run -- --debug examples/hello.elf
cargo run -- --debug examples/hello.bin
```

### Example Session

```text
MIPSion Interactive Debugger
Type 'help' for available commands.

00001000  2408002A  addiu $t0, $zero, 42
(mipsion) break 0x1004
Breakpoint added: 0x00001004
(mipsion) watch t0
Watchpoint added: $t0
(mipsion) continue
Watchpoint hit: $t0
  0x0 -> 0x2A
Steps: 1
00001004  25090001  addiu $t1, $t0, 1
(mipsion) continue
Breakpoint hit: 0x00001004
Steps: 0
00001004  25090001  addiu $t1, $t0, 1
(mipsion) step
[000002] PC=00001004  25090001  addiu
         $t1: 00000000 -> 0000002B
         PC -> 00001008
00001008  0000000C  syscall
```

### Debugger Commands

| Command | Alias | Description |
| --- | --- | --- |
| `help` | `h` | Show available commands |
| `step` | `s` | Execute one CPU step |
| `continue [count]` | `c` | Resume bounded execution |
| `reset` | `r` | Restore the initial machine state |
| `regs` | | Inspect general-purpose registers |
| `cp0` | | Inspect system control registers |
| `fpu` | | Inspect floating-point registers |
| `pc` | | Inspect PC and next PC |
| `disasm [addr] [count]` | `d` | Disassemble instructions |
| `mem <addr> <size>` | `m` | Dump physical memory |
| `break <addr>` | `b` | Add an execution breakpoint |
| `delete <addr>` | | Remove a breakpoint |
| `breakpoints` | `bl` | List breakpoints |
| `clear-breakpoints` | | Remove all breakpoints |
| `watch <reg>` | `w` | Watch register value changes |
| `unwatch <reg>` | | Remove a register watchpoint |
| `watchmem <addr> <width>` | `wm` | Watch physical memory changes |
| `unwatchmem <addr> <width>` | | Remove a memory watchpoint |
| `watchpoints` | `wl` | List watchpoints |
| `clear-watchpoints` | | Remove all watchpoints |
| `status` | | Display debugger session status |
| `quit` | `q` | Exit the debugger |

Breakpoints use virtual addresses and stop before instruction execution.

Watchpoints detect value changes after execution. General-purpose
registers, HI/LO and physical memory are supported. Memory
watchpoints support widths of 1, 2, 4 and 8 bytes.

Reset restores the initial CPU and memory state while preserving
breakpoint and watchpoint configurations.

The debugger is implemented entirely using the Rust standard library.

## Five-stage pipeline

```sh
cargo run -- --pipeline --source examples/delay-slot.s --trace --dump-registers
cargo run -- --pipeline --debug examples/hello.elf
```

The model is a single-issue, in-order IF/ID/EX/MEM/WB pipeline. EX computes
results, branch targets and effective addresses. MEM checks and prepares
loads and stores, and WB commits registers and stores in program order.
The four interstage registers carry explicit valid entries or bubbles.
RAW hazards stall ID and IF until the producer reaches WB.

The supported subset covers ADD/ADDI/ADDIU/ADDU, SUB/SUBU, MUL, integer
logic and comparisons, immediate and variable shifts/rotates, LUI,
LB/LBU/LH/LHU/LW and SB/SH/SW, integer conditional and likely branches,
J/JAL/JR/JALR (including the hazard-barrier encoding), and SYSCALL/BREAK.
NOP is the zero SLL encoding. CP0/CP1 instructions, HI/LO-specific
instructions, LL/SC and unaligned merge loads/stores are outside this model.
The functional engine remains the default for those instruction families.

Taken control transfers retain the delay slot. Untaken branch-likely
instructions annul it. Faults and interrupts commit through CP0 at WB and
flush younger work. An unsupported defined instruction stops at WB with
its address and word. Branches or jumps in delay slots are reported as
unsupported. CP0 Count advances at architectural instruction boundaries.

`Pipeline::cycle` returns a typed `CycleEvent` with post-cycle latch PCs,
fetch, stall, redirect, flush, and retirement or fault information. Calls
to `pause_fetch` let occupied latches drain before `resume_fetch` restarts
fetch. Trace formatting is separate from architectural effects.

In pipeline CLI mode, `--max-cycles` bounds simulation and `--max-steps`
bounds successful retirements. Exceptions are reported and stop execution
by default. The pipeline debugger supports `cycle [count]`, `regs`, `cp0`,
`pc`, `mem <address> <size>`, `reset` and `quit`. The standard debugger
retains its original instruction stepping, breakpoints and watchpoints.

## Current Limitations

- The pipeline is an educational timing model for a documented integer subset.
- Bare translation mode remains the default for compatibility.
- TLB-backed translation is available through the TLB execution mode.
- CPU fetch and data accesses include physical memory bounds checks and bus exceptions.
- ELF loading supports a restricted ELF32 little-endian static MIPS executable profile.
- ELF loading uses low, identity-mapped addresses.
- Full ELF ABI validation, segment permissions and dynamic linking are unsupported.
- Linux system calls and process initialization are not implemented.
- Instruction tracing currently includes fetched instructions, GPR changes, HI/LO changes and exceptions.
- CP0, TLB and exception behavior cover a limited architecture profile.
- Cache behavior, execution hazards and full synchronization semantics are incomplete.
- FPU support includes FR=0 register mapping, control registers, word/doubleword data transfers, basic single- and double-precision arithmetic, and S/D/W format conversions.
- FCSR rounding and Invalid/Inexact exception handling are implemented for CVT.W and explicit word rounding instructions.
- Comparisons support legacy NaN classification and Invalid Operation handling.
- All 86 basic FPU instructions have execution paths.
- Basic detection of all five IEEE 754 arithmetic exceptions is available for single- and double-precision arithmetic.
- Arithmetic supports all four rounding modes through native nearest-even results and exact-result comparisons.
- Narrowing format conversions support directed rounding and basic range exception detection.
- Basic FCSR.FS result flushing is implemented.
- Full FS semantics, tininess boundary cases, exception priority and IEEE 754 conformance remain incomplete.
- Independent reference-FPU differential testing remains pending.
- Config1.FP and FIR advertise the implemented S/D/W profile.
- FR=1, optional formats and full exception conformance remain incomplete.
- Unimplemented Operation request handling is supported, but complete architectural trigger conditions remain pending.
- Optional coprocessors other than CP1 are not implemented.
- Watchpoints detect value changes rather than individual memory access events.
- CP0 and FPU register watchpoints are not yet supported.
- Symbolic breakpoints and source-level debugging are not yet supported.
- Command history, tab completion and asynchronous execution interruption are not yet supported.
- Cache simulation and graphical interfaces are pending.

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
