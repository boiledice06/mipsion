# MIPSion development status

## Baseline and reproduction

The annotated `v0.5.0` tag and remote `main` both resolve to
`6710d04b6ce144da9a69c7be4fa9c9fbb8ab13b8`. The working tree was clean before
Batch 64. On 2026-11-28, Rust 1.91.0 on Windows passed the full debug suite
(612 internal, 10 CLI and 16 debugger tests), formatting and Clippy with
warnings denied. `cargo tree` listed only MIPSion. The tracked
`assets/logo-dark.png:Zone.Identifier` path is excluded from the Windows
working tree using sparse checkout. Its Git object remains in the tree.

## Batch plan

| Version | Branch | Batch | Acceptance criteria |
| --- | --- | ---: | --- |
| v0.6.0 | `feature/assembler` | 64 | Parse source, labels, comments and operand shapes with located diagnostics. |
| v0.6.0 | `feature/assembler` | 65 | Resolve symbols and sections in two passes. Encode supported integer, CP0 and basic FPU forms with range checks, directives and fixed-size pseudo expansions. |
| v0.6.0 | `feature/assembler` | 66 | Assemble raw little-endian files from the CLI and run or debug assembled programs. Verify canonical round trips, independent encodings and delay-slot execution. |
| v0.7.0 | `feature/pipeline` | 67 | Define the integer subset and implement IF/ID/EX/MEM/WB latches, ALU and memory effects, retirement and conservative RAW stalls. |
| v0.7.0 | `feature/pipeline` | 68 | Resolve control transfers with delay slots, deliver precise exceptions, and verify architectural state against the functional CPU. |
| v0.7.0 | `feature/pipeline` | 69 | Deliver structured cycle events and CLI/debugger cycle stepping. |
| v0.8.0 | `feature/cache-hazards` | 70 | Validate geometry, deterministic replacement, write policy and cache statistics. |
| v0.8.0 | `feature/cache-hazards` | 71 | Integrate I/D timing, outstanding accesses, single-core coherence, forwarding and load-use interlocks. |
| v0.8.0 | `feature/cache-hazards` | 72 | Expose configuration, cycle/cache/hazard events, CPI/IPC and end-to-end validation. |

Each batch gets a GitHub issue in its version milestone. Each validated batch
uses a conventional commit referencing its issue, is pushed to its feature
branch, and records measured test results and a commit hash in the issue
before closing it. Each version gets its own release gate, merge, annotated
tag and verified GitHub Release before work starts on the next version.

## Architecture decisions

- The assembler uses the generated ISA definitions for fixed encoding bits.
  Its operand grammar validates and places variable bits. Branch and jump
  addresses are based on PC + 4. Raw output uses explicit little-endian
  encoding and the programmer writes delay slots.
- The pipeline is a separate in-order engine. The functional CPU stays
  available as the reference, including instruction families outside the
  timing model. Stage latches carry decoded values and pending effects.
  Unsupported instructions produce explicit diagnostics.
- The single-core cache model uses deterministic replacement and
  write-through memory at retirement. Cache events and pipeline events are
  structured library data. Architectural state and faults take priority over
  modeled timing. These pipeline and cache choices are planned.

Instruction reference is MIPS Technologies, *MIPS32 Architecture for
Programmers, Volume II*, revision 2.62.
https://personal.denison.edu/~bressoud/cs281-s10/Supplements/ISA_Vol_2.pdf

## Validated batches

- Batch 64 ([#1](https://github.com/boiledice06/mipsion/issues/1)) added source parsing and located diagnostics. All 641 tests passed (615 internal, 10 CLI and 16 debugger). Formatting, Clippy and Git whitespace checks passed. Instruction encoding and public assembly are part of Batch 65.
- Batch 65 ([#2](https://github.com/boiledice06/mipsion/issues/2)) added two-pass layout, text and data, directives and table-driven encoding for all 200 definitions with functional execution paths. All 647 tests passed, and formatting, Clippy and whitespace checks passed. The six new tests cover encoding vectors, boundaries, symbol resolution and canonical round trips. The raw image carries no relocations or object metadata.
- Batch 66 ([#3](https://github.com/boiledice06/mipsion/issues/3)) added raw output and direct source loading for CLI execution and debugging. The CLI pads raw images to the loader word boundary. Source-level stepping and symbolic debugger commands remain outside this release.
