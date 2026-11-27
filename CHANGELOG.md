# Changelog

All notable changes to MIPSion are documented here.

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

