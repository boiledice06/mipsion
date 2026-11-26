# Mipsium

A MIPS32 emulator written in Rust.

Mipsium is a personal project exploring instruction set architecture, CPU emulation, and microarchitecture through a modular implementation of MIPS32 Release 2.

The long-term goal is to provide an interactive graphical environment for executing, debugging, and visualizing MIPS programs.

## Status

Early development.

- [x] CPU state and register file
- [x] Byte-addressable, little-endian memory
- [x] Instruction fetch and field extraction
- [x] Table-driven instruction decoder
- [x] Basic arithmetic execution
- [x] Fetch–decode–execute cycle
- [ ] Integer instruction set
- [ ] Load and store instructions
- [ ] Branch and jump instructions
- [ ] Exception handling
- [ ] Program loader and assembler
- [ ] Interactive debugger
- [ ] Five-stage pipeline simulation
- [ ] Cache and hazard simulation
- [ ] Graphical user interface

## Architecture

The emulation core is designed independently of the user interface.

- `cpu` — CPU state and instruction execution cycle
- `memory` — Memory representation and access
- `instruction` — Raw instruction and bit-field extraction
- `isa` — Instruction set definitions
- `decoder` — Table-driven instruction identification
- `executor` — Instruction semantics

## Getting Started

Requires a Rust toolchain with Cargo.

```sh
cargo run
```

The current implementation runs a basic instruction execution test.

## Goals

- Accurate MIPS32 instruction emulation
- Modular and extensible architecture
- Interactive inspection of CPU and memory state
- Visualization of instruction execution and pipeline behavior
- Minimal dependencies in the emulation core