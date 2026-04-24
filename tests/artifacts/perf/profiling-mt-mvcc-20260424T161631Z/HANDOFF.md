# Optimization Handoff

This is the handoff from `profiling-software-performance` to `extreme-software-optimization`. It uses alien-artifact-coding discipline: each candidate has a mechanism, proof boundary, rollback threshold, and an idea card drawn from the alien-graveyard catalog where useful.

## Scorecard

| Priority | Candidate | Evidence strength | Expected impact | Risk | Recommended first action |
|---:|---|---:|---:|---:|---|
| 1 | O(1) page mutation generation instead of full-page `xxh3` | high | high CPU win | medium correctness risk | Design + property tests around B-tree cache invalidation. |
| 2 | Instrument and fix page-2/root conflict cliff | high | very high throughput/reliability win | high | Add conflict-page/wait/retry telemetry, then validate Silo/group-commit admission and writer routing. |
| 3 | Split benchmark into comparison vs steady-state fsqlite-only modes | high | high measurement quality | low | Add a mode or sibling binary; do not silently change current comparison semantics. |
| 4 | Lazy/shared page-cache lifecycle initialization | medium | medium short-sample win | medium | Prototype lazy `FlatPageSlots`/published state allocation behind a microbench. |
| 5 | Allocation/data movement audit in direct insert | medium | medium | medium | Capture allocation profile before touching row/cell buffers. |

## Candidate 1: Page mutation generation token

Mechanism:

`BtCursor::page_mutation_counter` currently hashes the full usable page with XXH3 and folds it to a 32-bit token. Replace that content hash with a mutation token sourced from the pager or from `PageData` metadata. A seqlock-style generation counter is the cleanest conceptual model: every write that can change page bytes increments a generation; readers cache `(page_no, generation)` instead of `(page_no, pointer, hash)`.

Alien-graveyard concepts:

- Seqlocks / versioned reads: use a cheap monotone version token to detect mutation.
- Hekaton/Silo-style epochs: keep transaction/page generation explicit rather than rediscovering it from bytes.
- Proof-carrying optimized kernels: the optimized cache token must come with tests proving invalidation across splits, merges, dirty-page reloads, page recycling, and snapshot boundaries.

Rollback threshold:

Roll back if MT8 `xxh3_64_long_default` does not fall by at least 5 percentage points or if any B-tree correctness/conformance tests fail.

## Candidate 2: Page-2/root conflict cliff

Mechanism:

The benchmark still collapses when a second writer appears and fails at 16 threads with snapshot conflicts on page 2. CPU samples understate this because parked/waiting threads are off-CPU. Add telemetry first: conflict page histogram, conflict surface size, retry count, wait duration, and group-commit batch composition.

Potential fixes after telemetry:

- Silo-style group commit certification that rejects or reorders conflicting batches earlier.
- Flat-combining for same-page append/update critical sections so one combiner applies compatible operations while other writers avoid retry storms.
- Deterministic writer routing or right-edge split hints to make append workloads land on disjoint leaves once the tree grows.
- Conflict-surface localization so root/interior metadata pages do not poison otherwise disjoint row inserts unless their logical change really conflicts.

Alien-graveyard concepts:

- Parallel WAL / Silo epoch group commit.
- Flat combining for hot lock/page update sections.
- Deterministic multithreading as a debugging harness: replay the same writer interleaving to prove the fix changes causality, not luck.
- Tail decomposition: separate service time, synchronization wait, retry work, and I/O wait.

Rollback threshold:

Roll back if the fix improves 8T but worsens 1T insert by more than 3%, increases failed rows, or weakens conflict detection tests.

## Candidate 3: Measurement split

Mechanism:

The current `mt-mvcc-bench` is valuable as an apples-to-apples comparison, but it profiles setup + fsqlite + SQLite in one binary. Add an explicit fsqlite-only steady-state mode or sibling binary that reuses prepared connections across samples. Keep the current report schema and current comparison path intact.

Alien-artifact angle:

Create a measurement artifact with a falsifiable claim: `steady_state_writer_cost != connection_lifecycle_cost`. The new mode should emit both numbers so optimization work cannot accidentally optimize the harness instead of the engine.

Rollback threshold:

No engine behavior changes allowed for this candidate. Reject the benchmark change if it makes the existing comparison output ambiguous.

## Candidate 4: Lifecycle allocation

Mechanism:

`ShardedPageCache::with_max_buffers` and `Arc<ShardedPageCache>::drop_slow` remain visible in MT2/MT8 profiles. Recent work capped flat slot allocation, but connection-open cost still matters. Consider lazily allocating flat slots/published pages only when the first page is admitted, or sharing immutable cold metadata across caches.

Alien-graveyard concepts:

- S3-FIFO/segmented cache thinking: make hot-set allocation proportional to observed working set, not configured maximum.
- Arena/region discipline: connection lifecycle should release large structures as a region rather than triggering many independent drops.

Rollback threshold:

Roll back if connection-open improves but steady-state insert regresses by more than 1%, or if cache eviction correctness tests become flaky.

## Candidate 5: Allocation/data movement

Mechanism:

`malloc`, `free`, `memmove`, and `memset` remain secondary hotspots. Do not guess. Capture an allocation profile or add scoped allocation counters around direct insert, row serialization, cell pointer rebuilds, and page cache lifecycle.

Alien-artifact angle:

Treat each allocation as an accounting event with provenance. The desired artifact is not just fewer allocations; it is a map from allocation site to row/page operation so future agents can optimize the right owner.

Rollback threshold:

Roll back any allocation optimization below 3% wall-time improvement unless it also removes meaningful code complexity.

## Suggested Next Commands

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/rch_target_<agent> RUSTFLAGS='-C force-frame-pointers=yes' cargo build --profile release-perf -p fsqlite-e2e --bin mt-mvcc-bench
perf report -i tests/artifacts/perf/profiling-mt-mvcc-20260424T161631Z/perf_mt8.data --stdio --no-children --sort=overhead,symbol,dso --call-graph none
rch exec -- cargo test -p fsqlite-btree -p fsqlite-pager -p fsqlite-vdbe --profile release-perf
```
