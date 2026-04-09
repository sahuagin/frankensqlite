# Conflict-History and Ownership Telemetry Inputs

**Bead:** `bd-db300.5.5.1`  
**Track:** E5.1  
**Goal:** specify the telemetry inputs needed to feed conflict-topology-aware
writer routing without adding a second hot-path telemetry stack.

## 1. Decision

Writer routing will consume **existing MVCC and VDBE signals**. The routing
layer does **not** get permission to add a parallel event stream, extra page
maps, or new synchronization on the write path.

The source-of-truth contract now lives in:

- `crates/fsqlite-mvcc/src/writer_routing_telemetry.rs`
- `WRITER_ROUTING_TELEMETRY_SOURCES`
- `WriterRoutingTelemetryInput`

The contract splits inputs into three groups:

1. `WriterTouchSurfaceTelemetry`
2. `WriterConflictHistoryTelemetry`
3. `WriterOwnershipLineageTelemetry`

This matches the routing questions we actually need to answer:

- Which pages did this transaction touch?
- How often does this workload collide or retry?
- Who appears to own the conflicting surface now or historically?

## 2. Non-Goals

This bead does **not** choose a routing policy. It only fixes the telemetry
input contract.

This bead does **not**:

- add a new observer callback on every page touch
- serialize full transaction histories in the hot path
- require a global snapshot of the registry before every write
- change `concurrent_mode_default` or any concurrency defaults

## 3. Required Inputs

The first-pass routing inputs requested in the bead comments are already
present in the codebase:

| Signal | Existing Source | Current Artifact |
|--------|------------------|------------------|
| Tier-1 vs tier-2 write counts | `fsqlite-vdbe/src/engine.rs::SharedTxnPageIo::{classify_concurrent_write_tier,write_page_data}` | `MvccWritePathMetricsSnapshot::{tier1_first_touch_writes_total,tier2_commit_surface_writes_total}` |
| Page-lock wait count/time | `fsqlite-vdbe/src/engine.rs::wait_for_page_lock_holder_change` and `fsqlite-mvcc/src/core_types.rs::InProcessPageLockTable::wait_for_holder_change` | `MvccWritePathMetricsSnapshot::{page_lock_waits_total,page_lock_wait_time_ns_total}` |
| BUSY retry count | VDBE MVCC write wait/retry loop | `MvccWritePathMetricsSnapshot::{write_busy_retries_total,write_busy_timeouts_total}` |
| Stale-snapshot rejects | VDBE stale write rejection sites plus `validate_first_committer_wins` | `MvccWritePathMetricsSnapshot::stale_snapshot_rejects_total` |
| Page-one conflict-only count/time | `fsqlite-vdbe/src/engine.rs::track_concurrent_conflict_only_page` | `MvccWritePathMetricsSnapshot::{page_one_conflict_tracks_total,page_one_conflict_track_time_ns_total}` |
| Pending-surface clear count/time | `SharedTxnPageIo::clear_stale_synthetic_pending_commit_surface` | `MvccWritePathMetricsSnapshot::{pending_commit_surface_clears_total,pending_commit_surface_clear_time_ns_total}` |

The second-pass inputs are also already present:

| Signal | Existing Source | Current Artifact |
|--------|------------------|------------------|
| Same-page conflict pages | `PreparedConcurrentCommit::conflict_pages()` and FCW conflicting-page output | Prepared commit view / `BusySnapshot` page list |
| Pages touched by the txn | `ConcurrentHandle::{read_set,write_set_pages,held_lock_pages}` | `PreparedConcurrentCommit::{read_pages,write_set_pages,held_lock_pages}` |
| Remote ownership clues | `InProcessPageLockTable::{try_acquire,holder}` | lock-holder `TxnId` returned on contention |
| Ownership lineage through serialization edges | `PreparedConcurrentCommit::{incoming_edges,outgoing_edges,conflicting_txns}` | prepared SSI edge sets |
| Structural-vs-data conflict separation | `PageTxnState::{is_conflict_only,metadata_exempt}` | handle-local page state |

## 4. Why These Inputs Are Sufficient

Routing needs three different views of contention:

### 4.1 Immediate pressure

This is the "what is blocking me right now?" view:

- page-lock wait count/time
- current holder clues
- tier-1 vs tier-2 pressure
- BUSY retries/timeouts

These tell us whether a page is actively owned by another writer, whether the
writer is stuck on first-touch handoff, and whether the collision is structural
or direct.

### 4.2 Repeated topology

This is the "do these writers keep colliding on the same surface?" view:

- same-page conflict pages
- write-set pages
- read pages that later pivot into writes
- page-one conflict-only counts
- pending-surface clear counts

These tell us whether the workload's bad behavior is driven by:

- true hot data pages
- structural page-one amplification
- stale synthetic surface expansion that should be discounted

### 4.3 Ownership lineage

This is the "who tends to own this surface over time?" view:

- active lock-holder `TxnId`
- conflicting `TxnToken`s from prepared commit
- incoming/outgoing SSI edges

This lets a later routing stage infer lane/home affinity hints without guessing
from counters alone.

## 5. Hot-Path Budget Rules

The contract has an explicit budget discipline.

Allowed capture modes:

- `ExistingCounter`
- `ExistingSet`
- `PrepareBoundaryClone`
- `DeferredFold`

Forbidden capture modes for E5 routing input:

- per-page allocation of new telemetry maps
- global scans before every write
- registry snapshots taken solely for telemetry
- lock acquisition added for observability only

Interpretation:

- If a signal is already a counter, routing reads the counter.
- If a signal is already tracked in `ConcurrentHandle`, routing clones it once
  at prepare/finalize.
- If a signal exists only in handle-local page state, routing folds it after
  the hot path rather than recording a second stream.

## 6. Struct Contract

The new MVCC telemetry types are intentionally neutral. They do not depend on
`fsqlite-vdbe`, which avoids a crate cycle while still naming the current
producers in `WRITER_ROUTING_TELEMETRY_SOURCES`.

### 6.1 Touch Surface

`WriterTouchSurfaceTelemetry` contains:

- `read_pages`
- `write_set_pages`
- `held_lock_pages`
- `conflict_only_pages`
- `metadata_exempt_pages`
- `same_page_conflict_pages`
- `tier_counts`

This is the per-attempt structural shape the routing layer will cluster on.

### 6.2 Conflict History

`WriterConflictHistoryTelemetry` contains:

- `same_page_conflict_count`
- `page_lock_wait_count`
- `page_lock_wait_nanos`
- `busy_retry_count`
- `busy_timeout_count`
- `stale_snapshot_reject_count`
- `page_one_conflict_only_count`
- `page_one_conflict_only_nanos`
- `pending_surface_clear_count`
- `pending_surface_clear_nanos`
- `retry_attributions`

`WriterRetryAttribution` reserves the second-pass breakdown requested in the
bead comments:

- `PageLockContention`
- `StructuralPageOne`
- `PendingSurfaceExpansion`
- `PublicationAdvance`
- `StaleSnapshot`
- `BusyTimeout`

### 6.3 Ownership Lineage

`WriterOwnershipLineageTelemetry` contains:

- `lock_holder_clues`
- `conflicting_txns`
- `incoming_edges`
- `outgoing_edges`

`WriterLockHolderClue` keeps the low-level active holder evidence:

- `page`
- `holder: TxnId`

The SSI edge vectors preserve the higher-level historical lineage once the
conflict survives to prepare/finalize.

## 7. Assembly Model

`WriterRoutingTelemetryInput` is the bundle a future routing adapter will build
from the current planes:

1. Start with the current writer/session identity:
   - `session_id`
   - `txn_token`
   - `begin_seq`
   - optional `planned_commit_seq`
2. Fill `touch_surface` from `ConcurrentHandle` / `PreparedConcurrentCommit`
3. Fill `conflict_history` from existing VDBE counters and retry attribution
4. Fill `ownership_lineage` from lock-holder clues plus SSI edges

This split keeps the assembly deterministic and testable:

- touch surface is per-attempt
- conflict history is rolling/aggregate
- lineage spans active and committed conflict state

## 8. How Routing Should Read the Inputs

The intent is not to blindly chase every high counter.

Interpretation rules:

- High `tier2_commit_surface_rare` with high `page_one_conflict_only_count`
  means structural amplification, not necessarily a true data-page hotspot.
- High `pending_surface_clear_count` means the routing layer should discount
  stale synthetic page-one ownership before pinning a writer home.
- High `page_lock_wait_count` with stable `lock_holder_clues` means active
  ownership concentration and is a strong lane/home signal.
- High `stale_snapshot_reject_count` with low wait time means age/publication
  issues dominate over direct ownership.
- High `same_page_conflict_pages` recurrence is the main same-page topology
  signal and should dominate over generic retry counters.

## 9. Result

Track E5.1 now has a stable, code-level contract for conflict-history and
ownership telemetry inputs.

The important property is not the struct names; it is the discipline:

- reuse the existing MVCC/VDBE evidence
- keep capture costs bounded
- preserve page sets, counts, and lineage separately
- make later routing beads consume one shared telemetry vocabulary
