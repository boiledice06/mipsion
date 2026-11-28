# MIPSion v0.8.1 architecture audit

## Baseline and method

Audit date 2026-11-28. The clean `main` checkout, `origin/main` and the
annotated `v0.8.0` tag resolved to
`b38c24672f81c1f6b97c5008fceedcecdbb48857`. Both debug and release ran 683
tests on Windows x86-64 with Rust 1.91.0 and Debian x86-64 under WSL2
with Rust 1.100.0. Each configuration had 646 library, 21 CLI and 16
debugger tests. Windows Clippy passed. Linux Clippy flagged a fixed-size
chunking pattern in the assembler tests under its newer lint set.

The review inventory covers the Cargo metadata, build script and generated ISA
tables, CPU/CP0/CP1/ALU and exception paths, memory and loaders, assembler,
pipeline and caches, functional and pipeline debuggers, runtime and CLI, tests,
and repository configuration. The `v0.5.0..v0.8.0` delta changed 41 files.
Dedicated follow-up batches examine operands, retirement, timing and inspection
with separate tests.

Reference manuals include [MIPS32 Instruction Set, MD00086 revision 2.62](https://www.cse.iitb.ac.in/~br/courses/cs305-autumn2011/labs/lab05/MD00086-2B-MIPS32BIS-AFP-02_62.pdf)
and [MIPS32 Privileged Resource Architecture, MD00090 revision 2.50](https://cse.sc.edu/~jbakos/611/mips/MIPS32_Volume_III.pdf).

## Confirmed findings

| ID | Severity | Affected files and evidence | Resolution |
| --- | --- | --- | --- |
| A01 | High | `src/cpu/executor.rs` reached `todo!` for decoded pending operations. The v0.8.0 optimized CLI exited 101 on `.word 0x42000020` (WAIT). | Batch 73 routes the unsupported fallback to the guest Reserved Instruction exception. Tests cover CLI behavior, guest EPC and delay-slot Cause.BD. [Issue #16](https://github.com/boiledice06/mipsion/issues/16) |
| A02 | High | `assets/logo-dark.png:Zone.Identifier` prevented a normal Windows checkout. Git rejected that pathname while the ordinary PNG was valid. | Batch 73 removes the tracked metadata, preserves the PNG and adds an asset sidecar ignore rule. A normal Windows branch checkout is checked after commit. |
| A03 | Medium | `src/main.rs` read files before checking image size. Both raw/ELF and assembly paths could allocate based on arbitrarily large input. | Batch 73 checks filesystem size and bounds streaming reads to 256 MiB and 16 MiB respectively. A CLI test rejects a 16 MiB + 1 byte source. |
| A04 | Low | `src/assembler/tests.rs` failed strict Clippy on Rust 1.100.0 with `chunks_exact_to_as_chunks`. No cross-platform CI was configured. | Batch 73 uses typed four-byte slices with a remainder assertion and adds Windows and Ubuntu CI on Rust 1.91.0. |

### Reproduce A01 on the baseline

Create `wait.s` with `.word 0x42000020` and run
`cargo run --release -- --source wait.s --max-steps 1`. The baseline
panics in `executor.rs`. The stabilized branch reports guest exception
code 10 at EPC 0 and returns from the CLI normally.

### Reproduce A02 on Windows

A normal checkout of the exact v0.8.0 tree fails with the invalid
`assets/logo-dark.png:Zone.Identifier` path. Check out the stabilization
branch to verify its index contains the ordinary PNG and neither a colon
metadata path nor sparse-checkout entries.

## Reviewed risks and model limits

- Low-level `Memory` and register accessors expose indexing preconditions
  to library callers. The loader, execution engines and debugger validate
  architectural/file-derived addresses before access. Direct misuse of those
  public accessors can panic.
- The ISA generator is intentionally fail-fast for inconsistent source tables.
  Optional instruction encodings can be recognized even when the emulator
  does not provide an execution path.
- ELF32 loading uses a restricted static executable profile. The standalone
  loader recognizes ELF by magic. The pipeline uses functional architectural
  helpers and a separate stage engine, so comparison must include independent
  expected values as well as differential results.
- Latency and replacement describe the educational single-core timing model.
  Hardware cacheability, device timing, multicore coherence and write-back
  are outside this profile.
