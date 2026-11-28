# Pipeline inspection API

`Pipeline::snapshot()` reads the current state. `Pipeline::cycle_observed()`
advances exactly one cycle and returns a `CycleObservation`. Its `event`
is the same `CycleEvent` returned by `cycle()` on an equivalent machine.
A halted pipeline or one stopped by an unsupported instruction returns
`None` when stepped. The owned views contain plain Rust values and are
separate from terminal formatting. A caller can retain them after stepping.

```rust
use mipsion::pipeline::{CycleObservation, Pipeline};

fn collect_cycles(pipeline: &mut Pipeline, limit: usize) -> Vec<CycleObservation> {
    (0..limit).map_while(|_| pipeline.cycle_observed()).collect()
}
```

## Reading a cycle

| Data | Interface and meaning |
| --- | --- |
| Cycle and execution counters | `snapshot.statistics` contains the same values as `statistics()`. Simulated cycles and architectural CP0 Count measure different boundaries. |
| Latch validity, words and dynamic identity | Each post-cycle latch is an `Option`. An occupied entry has a fetch sequence, virtual PC, optional translated address, and either a fetched instruction or its fetch fault. A refetch of the same PC gets a new sequence. Recreating a pipeline for reset starts a new sequence at one. |
| Issued operands and intermediate effects | `id_ex.inputs` contains the two captured register values, nonzero source dependencies and optional destination. A zero or unused source has no dependency. `ex_mem.effect` and `mem_wb.effect` show pending effects, including whether a store address is virtual or physical. |
| ALU and control decisions | `executed` exists only when EX consumed an entry this cycle. It includes actual integer ALU helper operands after immediate extension or shift masking, and a result when the operation succeeded. A branch or jump exposes its actual redirect, retained or annulled slot, and link value. |
| Memory request and mutation | `memory_request` describes a translated data request on its start cycle, even with caching disabled. Load bytes are shown before sign extension. `committed_write` records the actual before and after of a nonzero GPR or a store, using the store's width. A store request does not change memory. A fault or an instruction targeting GPR zero has no committed GPR or memory write. |
| Hazards, forwarding and exceptions | `event` contains stalls, forwarding operands and sources, redirects, flushes and WB outcomes. `writeback_instruction` identifies the affected dynamic entry, even when it faults or is interrupted. `snapshot.cp0` exposes Status, Cause, EPC, BadVAddr, and Count after the cycle. |
| Cache placement and pending requests | The two cache events can coexist. Their locations show the physical tag, set, offset and resident way. Invalidation separately reports the old line's location. `pending_fetch` and `pending_memory` expose remaining wait cycles. A pending fetch with zero remaining cycles can be ready while older MEM work holds the front end. |
| Read-only inspection | `Pipeline::cpu()` and `memory()` expose committed state by shared reference. `Cache::resident_line(address)` and `Cache::line(set, way)` inspect metadata without issuing accesses, touching LRU or changing counters. |

The observation is taken after the cycle. WB commits older work before MEM
uses its data. One cycle can therefore report both a store retirement and
a younger load request reading that update. At most one request per cache
port is started in a cycle. A canceled IF request may already have filled a
line and contributed to cache statistics. Before a pending IF entry is
accepted, the eventual branch delay-slot decision is represented by the
front-end target and the redirect event.

## Model boundaries

The simulator exposes values it computes and decisions it makes. Dedicated
hardware control nets, physical ALU signals for branch comparison or
address calculation, byte-level bus transactions, cache line contents,
and a complete simultaneous-stall vector require further modeling. Use the
pending-request fields to show concurrent I/D work when the primary stall
is a data wait. The GUI and any file format or wire protocol are outside
this inspection API.
