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

## Assembler audit — Batch 74

| ID | Severity | Evidence and fix |
| --- | --- | --- |
| A05 | Medium | v0.8.0 accepted architecturally unpredictable `jalr $t0, $t0` and `bgezal $ra, 4`. The encoder now diagnoses identical JALR/JALR.HB link and target registers and register 31 as the source of the four conditional branch-and-link instructions. |
| A06 | Medium | v0.8.0 assembled `ctc1 $t0, $0` and reserved basic FPU control selectors even though the functional CPU rejected those accesses. The assembler now permits FIR only for CFC1 and validates the other basic control selectors. CP0 register/selector fields retain their architectural bit ranges because actual availability is CPU-profile dependent. |
| A07 | Low | A leading UTF-8 BOM in an otherwise valid Windows assembly file was parsed as part of the first mnemonic. It is now accepted. Diagnostics retain byte columns in the source text after the optional BOM. |

The baseline matched GNU binutils 2.44 on all **200 of 200** representative
forms. The fixed assembler also matches all 200, once each, with every
supported mnemonic covered exactly once in the independent corpus.
The corpus includes 105 mandatory CPU and 86 basic FPU forms. The nine
remaining forms are functional CP0/TLB instructions. This is encoding
coverage for representative operands and does not establish instruction
execution conformance. Pseudoinstructions, section layout, range
boundaries, malformed operands and source locations have separate tests.

The golden file is `tests/fixtures/gnu-mips32r2.tsv`. To reproduce it, take
the assembly column, remove its comment header, prepend `.set noreorder`,
`.set nomacro`, `.set noat` and `.text`, and append `target:`. GNU uses
`div $zero, rs, rt` and `divu $zero, rs, rt` to request the native
encodings where MIPSion uses two operands. Run:

```sh
mipsel-linux-gnu-as -EL -mips32r2 -mabi=32 -mfp32 -mno-fix-loongson3-llsc --fatal-warnings -o gnu.o gnu.s
mipsel-linux-gnu-ld -Ttext=0 -e 0 -o gnu.elf gnu.o
mipsel-linux-gnu-objcopy -j .text -O binary gnu.elf gnu.bin
```

The `target` symbol was at `0x320`. The extracted 800-byte image had
SHA-256 `774f97f7bf7e14213f7911064425ce87db9052376c7222e0860f252b583f717a`.
No GNU executable is needed for the Rust regression suite.

Batch 74 passed 691 tests on Windows, including four new library tests.

## Pipeline retirement audit — Batch 75

| ID | Severity | Evidence and fix |
| --- | --- | --- |
| A08 | Medium | The v0.8.0 pipeline reported an unsupported instruction at writeback before testing for an eligible interrupt. With Status=0x8001 and Compare=2, a branch at PC 0 followed by MFC0 in its delay slot stopped at PC 4 instead of delivering the timer interrupt. The fixed pipeline checks the architectural boundary first. It reports Interrupt with EPC 0 and Cause.BD set, and flushes the younger work. |

Run `cargo test -q timer_interrupt_preempts_a_delay_slot_unsupported_by_the_pipeline`.
The test initializes Count to zero, writes Status and Compare, and checks
the boundary against the functional CPU. A genuine unsupported diagnostic
with no competing interrupt remains non-retiring and leaves architectural
state untouched.

A directed control-flow program checked that a taken branch redirects at
cycle 3, the branch, its slot, and its first target instruction retire at
cycles 5, 6 and 7, and SYSCALL takes effect at cycle 8. The test checks the
skipped store and separately expects architectural CP0 Count to equal 4.
A functional-to-pipeline handoff after JAL independently checks link
address 8 and the delay-slot fetch order.

The seeded differential used seeds `1`, `0x243f6a88`, `0xb7e15163`, and
`0xdeadbeef`. Four 128-instruction programs used defined wrapping integer
arithmetic, comparisons, variable shifts, aligned stores, loads, sign
extension and writes to GPR zero. There were 512 successful retirements
and four terminal exceptions. In addition to comparing the two engines,
the test computes the expected registers and data words independently
and checks each retired PC for loss or duplication. Simulated cycle count
is counted separately from architectural instruction boundaries.

Pipeline audit work in Batch 75 covers the documented uncached, conservative
pipeline baseline. The cache and forwarding configuration matrix belongs to
Batch 76.

The new interrupt regression fails on the pre-fix Batch 74 tree. The
pipeline remains at PC 4 while the functional CPU vectors to
`0x80000180`. With the fix, all 695 tests passed in both debug and release
on Windows (Rust 1.91.0) and Debian WSL2 (Rust 1.100.0). Strict Clippy
passed on both toolchains, along with formatting and Git whitespace
checks.
