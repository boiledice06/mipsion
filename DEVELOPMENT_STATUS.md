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
  modeled timing.

Instruction reference is MIPS Technologies, *MIPS32 Architecture for
Programmers, Volume II*, revision 2.62.
https://personal.denison.edu/~bressoud/cs281-s10/Supplements/ISA_Vol_2.pdf

## Validated batches

- Batch 64 ([#1](https://github.com/boiledice06/mipsion/issues/1)) added source parsing and located diagnostics. All 641 tests passed (615 internal, 10 CLI and 16 debugger). Formatting, Clippy and Git whitespace checks passed. Instruction encoding and public assembly are part of Batch 65.
- Batch 65 ([#2](https://github.com/boiledice06/mipsion/issues/2)) added two-pass layout, text and data, directives and table-driven encoding for all 200 definitions with functional execution paths. All 647 tests passed, and formatting, Clippy and whitespace checks passed. The six new tests cover encoding vectors, boundaries, symbol resolution and canonical round trips. The raw image carries no relocations or object metadata.
- Batch 66 ([#3](https://github.com/boiledice06/mipsion/issues/3)) added raw output and direct source loading for CLI execution and debugging. All 650 tests passed with formatting, Clippy and whitespace checks. The CLI pads raw images to the loader word boundary. Source-level stepping and symbolic debugger commands remain outside this release.

## v0.6.0 batch commits

- Batch 64: `1ec755572f84aa11063b63ffbc90fef348b7d014`
- Batch 65: `bf3066f6a7be66b9c215e5109d95b468709762c7`
- Batch 66: `cea0af5b1ef7c3a1612cb906f931141241de4909`

## v0.7.0 development

The v0.6.0 annotated tag and published release were verified at `ca836f1548688459c5a81fedc747ebd8bb60f39e`. [Milestone 1](https://github.com/boiledice06/mipsion/milestone/1) closed with three completed issues.

- Batch 67 ([#4](https://github.com/boiledice06/mipsion/issues/4)) implemented separate stage latches, in-order integer ALU and memory operations, conservative RAW stalls, and retirement. All 655 tests and the batch gate passed. Control transfers are part of Batch 68.
- Batch 68 ([#5](https://github.com/boiledice06/mipsion/issues/5)) resolved integer branches and jumps, including branch-likely annulment, at EX. Delay-slot faults, architectural retirement and self-modifying code were checked against the functional CPU. All 663 tests and the batch gate passed. Unsupported instructions stop at WB with a diagnostic.
- Batch 69 ([#6](https://github.com/boiledice06/mipsion/issues/6)) added CLI engine selection, typed cycle events with terminal formatting, and debugger cycle stepping and reset. All 667 tests and the batch gate passed. The pipeline debugger has its own command set.

### v0.7.0 batch commits

- Batch 67 `0d556e0b6b3b1162a1560e7875f54f5903c78af6`
- Batch 68 `321431dc12fe22676a65863cfb2bc74a37768fcb`
- Batch 69 `4abbf609dcc9a8073f1c1267f6b1bbb36797d5ed`

## v0.8.0 development

The v0.7.0 annotated tag and published release were verified at `9421765d8893a2752f6a4e791c8abc0936a0c8d6`. [Milestone 2](https://github.com/boiledice06/mipsion/milestone/2) closed with three completed issues.

- Batch 70 ([#7](https://github.com/boiledice06/mipsion/issues/7)) added validated cache geometry, deterministic LRU, cache line state, physical address decomposition and write-through statistics. All 672 tests and the batch gate passed. The write-through model keeps lines clean.
- Batch 71 ([#8](https://github.com/boiledice06/mipsion/issues/8)) integrated I/D wait states, write-through coherence across physical aliases, register forwarding and the load-use interlock. All 679 tests and the batch gate passed. Differential tests include cache misses, simultaneous producer age selection, branch annulment and precise delay-slot faults.
- Batch 72 ([#9](https://github.com/boiledice06/mipsion/issues/9)) added cache geometry and latency flags, optional forwarding, cache statistics and CPI/IPC, typed event formatting and cold-cache debugger reset. All 683 tests and the batch gate passed. Rates describe the selected simulation and depend on its configuration.

### v0.8.0 batch commits

- Batch 70 `8fa5257601aa731d9ad99f089d2f033025da2fb7`
- Batch 71 `1b1a93f8976d4805f11009475ebef9f3277f4e8f`
- Batch 72 `c25044374b02f1632e81a191e0edae309a525e7a`

## Release validation

| Version | Library | CLI | Debugger | Debug total | Release total |
| --- | ---: | ---: | ---: | ---: | ---: |
| v0.6.0 | 621 | 13 | 16 | 650 | 650 |
| v0.7.0 | 634 | 17 | 16 | 667 | 667 |
| v0.8.0 | 646 | 21 | 16 | 683 | 683 |

Each release gate ran formatting checks, both test configurations, Clippy
with warnings denied, the optimized build, CLI smoke tests, `cargo tree`
and Git whitespace checks. The test groups had zero failures and zero
ignored tests. ISA execution counts still describe the functional engine.

For v0.9.0 work, start from the annotated `v0.8.0` tag, run both test
configurations, and build against `pipeline::CycleEvent`,
`pipeline::PipelineStatistics` and the read-only cache inspection API.
The README documents model limits and the trace semantics.

## v0.8.1 stabilization

The stabilization baseline is the published v0.8.0 merge. Batches 73–78 cover repository and architecture review, assembler encodings, retirement, cache and hazard timing, read-only visualization data, and the audited release. See [AUDIT_REPORT.md](AUDIT_REPORT.md) for severity, reproduction and validation evidence.

- Batch 73 ([#10](https://github.com/boiledice06/mipsion/issues/10)) removed tracked Windows metadata, added cross-platform CI, bounded CLI input, and replaced the functional executor host panic with a guest exception.
- Batch 74 ([#11](https://github.com/boiledice06/mipsion/issues/11)) verified 200/200 independent GNU encoding vectors, diagnosed restricted GPR and FPU selectors, and accepted a UTF-8 BOM. All 691 tests passed.
