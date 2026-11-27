# Changelog

All notable changes to MIPSion are documented here.

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

