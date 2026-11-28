# Changelog

All notable changes to MIPSion are documented here.

## [0.7.0] - Five-stage Pipeline Simulation

### Added

- Separate in-order IF/ID/EX/MEM/WB integer engine with explicit latches
  and in-order register and store retirement.
- Conservative RAW interlocks, load/store alignment and translation,
  integer branch and jump resolution, delay slots, and branch-likely annulment.
- Precise WB exception delivery, flushing, and explicit diagnostics for
  unsupported defined instructions.
- Typed cycle events, `--pipeline` engine selection, and debugger cycle
  stepping, state inspection and reset.
- Retirement comparisons against the functional CPU, including faulting
  delay slots and a store that replaces a fetched instruction.

### Validation

- 667 tests in debug and release (634 library, 17 CLI, 16 debugger).
- Optimized CLI pipeline and pipeline debugger smoke tests.
- Formatting, Clippy with warnings denied, and Git whitespace checks.
- Zero external Rust dependencies.

### Known limitations

- Timing coverage is limited to the documented integer subset.
- The model has no forwarding or cache timing in this release.
- Pipeline debug commands cover cycle stepping and committed state.

## [0.6.0] - Assembler

### Added

- Two-pass, dependency-free assembly with text and data sections, labels,
  basic data directives and explicit pseudo-instructions.
- Table-driven encoding for the 200 entries with functional execution paths,
  including basic FR=0 FPU and CP0/TLB forms.
- Source-located errors for invalid operands, unsupported mnemonics and
  invalid branch or jump addresses.
- Little-endian raw output via `--assemble` and direct CLI or debugger
  loading via `--source`.
- Canonical round trips for the supported ISA forms and end-to-end tests
  for byte layout, delay-slot execution and debugger loading.

### Validation

- 650 tests in debug and release (621 library, 13 CLI, 16 debugger).
- Optimized CLI assembly, execution and debugger smoke tests.
- Formatting, Clippy with warnings denied and Git whitespace checks.
- Zero external Rust dependencies. Functional ISA counts are unchanged.

### Known limitations

- Flat binary layout with no linking, relocations or macros.
- Delay slots are explicit. The debugger steps instructions and uses
  numeric breakpoints.

## [0.5.0] - Interactive Debugger

### Added

- Instruction disassembly with register and operand formatting.
- Virtual-address branch and jump target formatting.
- Stateful debugger sessions with non-mutating instruction inspection.
- Single-step execution with structured trace events.
- Bounded continuous execution with exception-aware termination.
- Full CPU and memory snapshots for session reset.
- Virtual-address execution breakpoints.
- Breakpoint-aware execution resumption.
- General-purpose register and HI/LO watchpoints.
- Physical memory watchpoints with 8-, 16-, 32- and 64-bit widths.
- Multiple simultaneous watchpoint hit reporting.
- Interactive command parser and debugger REPL.
- ELF and raw binary debugging through the --debug CLI option.
- Interactive register, CP0, FPU and memory inspection.
- Breakpoint and watchpoint configuration persistence across resets.
- Public debugger API integration tests.
- Executable CLI end-to-end tests.

### Changed

- Reorganized source files into domain-specific modules.
- Added reusable runtime execution hooks.
- Shared the execution core between standalone and interactive modes.
- Separated internal regression tests from external integration tests.
- Preserved compatibility with existing program loading and trace execution modes.

### Validation

- 638 tests passing (+147 since v0.4.0).
- 612 internal regression tests.
- 16 debugger REPL integration tests.
- 10 CLI end-to-end tests.
- All tests passing in debug and release configurations.
- Clippy validation with warnings treated as errors.
- Verified interactive ELF execution using the optimized release binary.
- Zero external Rust dependencies.
- 200 of 291 ISA entries with execution paths (68.73%).

### Known limitations

- Full MIPS32 Release 2 conformance remains incomplete.
- Watchpoints detect value changes, not individual read or write accesses.
- CP0 and FPU register watchpoints are not yet supported.
- Symbolic breakpoints and source-level debugging are not supported.
- Asynchronous debugger interruption, command history and completion are pending.
- Linux system calls and dynamic executable linking remain unsupported.
- Assembly, cycle-accurate simulation and graphical interfaces are pending.

## [0.4.0] - Program Loader and CLI

### Added

- Validated little-endian raw binary loading.
- Physical memory range validation.
- Instruction and data bus exceptions.
- Bounded execution runtime with configurable step limits.
- Exception-aware execution termination.
- Standalone command-line interface.
- Configurable load address, entry point and memory size.
- GPR, HI/LO, CP0 and FPU register inspection.
- Physical memory hexadecimal and ASCII dumps.
- Structured instruction trace events.
- Register change and exception tracing.
- Callback-based trace delivery.
- ELF32 little-endian MIPS executable parsing.
- PT_LOAD segment loading and BSS zero initialization.
- ELF entry point and stack pointer initialization.
- Automatic raw binary and ELF format detection.
- Raw binary and ELF execution examples.

### Validation

- 491 regression tests (+105 since v0.3.0).
- Raw binary and ELF runtime integration tests.
- Trace execution equivalence regression test.
- Memory bounds and exception regression tests.
- Zero external Rust dependencies.
- 200 of 291 ISA entries with execution paths (68.73%).

### Known limitations

- Full MIPS32 Release 2 conformance remains incomplete.
- ELF support is restricted to low, identity-mapped static executables.
- Full MIPS ABI validation and segment permission enforcement are pending.
- Dynamic linking and Linux system calls are unsupported.
- Trace events do not yet include memory accesses or FPU changes.
- Interactive debugging, assembly and cycle-accurate simulation are pending.

## [0.3.0] - Floating-Point Unit

### Added

- Execution paths for all 86 basic FPU instructions.
- CP1 register file and FR=0 register mapping.
- Single- and double-precision arithmetic.
- GPR/FPR transfers and floating-point memory operations.
- S/D/W format conversions.
- All four FCSR rounding modes.
- All 32 basic floating-point comparison instructions.
- Floating-point conditional branches and moves.
- Legacy NaN classification and normalization.
- Basic detection of five IEEE 754 arithmetic exceptions.
- FCSR Cause, Enable and sticky Flag handling.
- FP exception delivery and writeback suppression.
- CP0 Config1.FP and CP1 FIR feature advertisement.
- Basic FCSR.FS result flushing.
- Integer-based exactness checks for binary64.
- Fixed-size 2176-bit dyadic accumulator.

### Validation

- 386 regression tests.
- 2,048 Cause/Enable mask combinations.
- 200 of 291 ISA entries with execution paths (68.73%).
- 105 of 105 mandatory CPU execution paths.
- 86 of 86 basic FPU execution paths.

### Known limitations

- Full MIPS32 Release 2 conformance is incomplete.
- Full IEEE 754 conformance is not established.
- FS, tininess and exception priority require further audit.
- Complete Unimplemented Operation trigger conditions are pending.
- FR=1 and optional floating-point formats are unsupported.
- Independent reference-FPU differential testing is pending.
- Standalone program loading and interactive execution are pending.

## [0.2.0] - System Control and Memory Management

### Added

- CP0 system registers.
- Exception vectors and ERET.
- Kernel, supervisor and user privilege modes.
- Timer and software interrupts.
- MMU and segment permissions.
- 32-entry software-managed TLB.
- TLB translation, refill, invalid and modified exceptions.

### Validation

- 145 regression tests.
- 114 of 291 ISA entries with execution paths.
