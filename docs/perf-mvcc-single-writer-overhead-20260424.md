# MVCC Single-Writer Overhead — Findings (cc_4, 2026-04-24)

**Context.** At 1t, `sq_wps ≈ 725k` vs fsqlite in the same shape is
materially behind. The mvcc layer, designed for concurrent-writer MVCC
(INV-7, §5.2–5.3), pays setup/teardown cost on every op even when the
process is provably single-threaded. This doc enumerates which costs
are **safely** skippable with today's protocol, which costs are
structurally required by the existing SSI / FCW / version-chain
contract, and what protocol changes would be required to eliminate the
remaining overhead.

## What already fires on every 1t write + read

Per-read (`SharedTxnPageIo::read_page` / `::read_page_data` in
`crates/fsqlite-vdbe/src/engine.rs:2684 / :2733`):

1. `ctx.handle.lock()` — `Arc<Mutex<ConcurrentHandle>>` mutex acquire.
   Uncontended at 1t, but still ~5–10 ns per op.
2. `concurrent_page_read_state(&handle, page_no)` — compound
   `page_states` probe (already landed in 78350138; saved one probe).
3. `handle.record_read(page_no)` — `HashSet<PageNumber>::insert` +
   (on first sight) `HashMap::entry(page).or_default().push(…)`.
4. `handle.lock()` drop.

Per-write (`VersionStore::publish` in `crates/fsqlite-mvcc/src/invariants.rs:611`):

1. `shard.ensure_slot(pgno)` — page-to-slot directory lookup.
2. `self.arena.write()` — `RwLock<VersionArena>::write()` exclusive
   lock. Held for alloc + prev-link.
3. `arena.alloc(version)` — mutable append to `Vec<Vec<ArenaSlot>>`.
4. CAS loop on the per-page `AtomicU64` chain head.
5. `record_cas_attempt(cas_attempts)` — gated under
   `MVCC_CAS_METRICS_ENABLED` since d2156302; no-op when off.
6. Visibility-ranges write — gated under
   `MVCC_VISIBILITY_RANGES_TRACKING_ENABLED` since f2707d1a; no-op when
   off.
7. `record_write_witness` / `record_write` called from upstream on
   every write (populates per-handle `write_index`).

Per-commit (`prepare_concurrent_commit_with_ssi` in
`crates/fsqlite-mvcc/src/begin_concurrent.rs`):

1. If `can_use_uncontended_prepare_fast_path` returns true (alone +
   committed history is all behind our `begin_seq`) — commit skips
   `ActiveEdgeDiscoveryIndex::build` + incoming/outgoing candidate
   discovery entirely (line 2421). This already shortcuts SSI work at
   1t.
2. Otherwise: build `ActiveEdgeDiscoveryIndex`, probe committed
   reader/writer indexes, discover edges.

## Overhead classification

### A. Safely skippable TODAY at 1t — no new invariants required

1. **The page-lock `try_acquire` + `release` pair per page write.**
   `InProcessPageLockTable` protects concurrent writers from each
   other. When `registry.active.len() == 1` and holds us, no peer can
   race. **BUT** a new writer may register concurrently; they'd see
   our uncommitted pages without holding a page lock.

   **Not safe as a pure write-side check.** Needs a two-phase
   protocol: on any `registry.active` transition from 1→2, the
   incoming writer must *acquire all page locks our still-active
   handle holds virtual claim on*. Equivalent to lock-promotion on
   contention. Implementation cost: add an atomic `active_writers: u32`
   on the registry, fast-path `try_acquire` on `== 1`, and have
   `begin_concurrent` drain pending uncommitted writes before
   returning when `active_writers.fetch_add(1)` went 1→2. **Design
   work, not a single commit.**

2. **`record_read` / `record_read_witness` when `active_writers == 1`.**
   SSI incoming-edge detection scans our read_set only when another
   writer commits. If no other writer ever exists, no scan ever runs,
   and the tracking is dead weight.

   **Not safe as a simple skip.** Writers scan *currently-active*
   handles at their commit time. If we skipped record_read while alone
   at T0, a writer W starting at T1 will see our empty read_set at
   T_w_commit and miss the edge — silent data corruption under SSI.

   **Safe version.** Two options, both requiring protocol work:
   - (i) Abort-on-upgrade: tag the handle "reads-untracked". When a
     second writer registers, flag all existing handles as
     "must-abort-on-commit." Correct under SSI (any serializable
     history is retained by aborting), but penalizes the rare 1→2
     transition heavily.
   - (ii) Deferred re-tracking: keep a chain-walk replay buffer on the
     handle. When 2nd writer joins, replay the buffer into read_set.
     Paid only at the transition, not per-read.

3. **`mvcc_snapshot_established` / `_released`.** Already gated under
   `MVCC_SNAPSHOT_METRICS_ENABLED` in 03c49886; zero cost at 1t (one
   relaxed bool load).

### B. Structurally required by current protocol

1. **`handle.lock()` mutex on every read.** Needed because GC may
   scan active handles concurrently; SSI edge discovery locks peer
   handles; process-wide `notify_all_waiters` may poke into the
   active set. Eliding requires a per-handle single-owner fast path
   (biased locking), which is a large protocol change.

2. **`VersionStore::publish` `arena.write()` exclusive lock.**
   `VersionArena` is a growable `Vec<Vec<ArenaSlot>>`; `alloc` needs
   `&mut self` because the outer Vec can reallocate. Per-thread
   arena chunks would eliminate cross-thread serialization. Not a
   single-commit change.

3. **CAS loop on chain head.** `compare_exchange_weak` fails
   spuriously; loop is needed for correctness. At 1t the expected
   number of iterations is 1 — `compare_exchange` (strong) instead
   of `_weak` would eliminate the loop body entirely. **This one is
   a single-commit lever**; weak vs strong CAS is a measurable
   micro-optimization. TODO: bench.

4. **Version chain publication.** Required for MVCC snapshot reads
   even at 1t (own-txn read-your-writes uses staged_data but committed
   reads use the chain). Even for pure writers with no reader
   contention, the chain head must be updated so later readers (in
   new connections) see the write.

### C. Already eliminated this session

`visibility_ranges` HashMap writes, CAS histogram atomics,
`active_snapshots` gauge, snapshot-read histogram — all gated off by
default in bc4fa6b5 / f2707d1a / d2156302 / 03c49886.

`ActiveEdgeDiscoveryIndex` `RandomState` hasher replaced with identity
hasher in 40c64b53; same change for `committed_readers/writers_by_page*`
in 9521594b.

`SireadTable`-empty short-circuit in
`{incoming,outgoing}_candidate_refs` landed in 9521594b — saves the
per-call clone + sort/dedup when the active view-set contains no
reader/writer.

Compound page-state probe (78350138) eliminated one of two back-to-back
HashMap lookups per read.

Waiter-shard mutex skip on post-park wake path (06e13def) saves one
per-wake acquire.

## Concrete single-commit next levers within this area

1. **`compare_exchange_weak` → `compare_exchange` (strong) in the
   chain-head install loop** at `invariants.rs:638`. Under
   `arena.write()` exclusive hold, spurious failures are the only
   reason for the loop; strong CAS eliminates the loop entirely in the
   common case.

   *Risk.* None — strong CAS has the same semantics modulo not
   spuriously failing. Cost is a slightly more expensive instruction
   on some architectures (x86 has no difference).

   *Expected delta.* Small — probably <3 % on `publish`. Measurable
   via `bench_publish_visibility_ranges_gate` run with both gates off.

2. **`registry.active_writers: AtomicUsize` fast-path counter** for
   the "am I alone?" check. Today `can_use_uncontended_prepare_fast_path`
   reads `self.active.len()` which requires the ConcurrentRegistry's
   `active: HashMap<u64, _>` already to be under the registry's outer
   lock. Exposing a plain atomic read would let the 1t fast path avoid
   the registry lock on commit prep. Non-trivial interactions with the
   existing registry API.

3. **Skip `record_read` when `registry.active_writers == 1` AND
   `committed_readers.is_empty()`.** Combines the "alone now" check
   with "no reader ever finished." Safe iff the 1→2 transition
   re-tracks in-flight reads (needs the replay buffer from
   classification A.2). **Not a single commit.**

## Recommended landing order

1. Strong CAS in chain-head install (single commit, measurable).
2. Registry `active_writers: AtomicUsize` fast-path counter, plumb
   through `can_use_uncontended_prepare_fast_path` (single commit, no
   new protocol).
3. "Alone at commit" bypass for read-index / write-index rollup —
   skip the hashmap walks when we know no peer could care. (Scope
   depends on audit.)
4. Single-writer protocol with replay-buffer read-tracking and
   page-lock fast path. (Large; multiple commits, requires careful
   test matrix.)

## References

- `crates/fsqlite-mvcc/src/begin_concurrent.rs:818` — existing uncontended
  prepare fast path.
- `crates/fsqlite-mvcc/src/invariants.rs:611` — `VersionStore::publish`.
- `crates/fsqlite-mvcc/src/invariants.rs:638` — weak CAS loop targeted by
  lever (1).
- `crates/fsqlite-vdbe/src/engine.rs:2684` / `:2733` — per-read handle
  lock + `record_read` sites.
- `crates/fsqlite-mvcc/src/begin_concurrent.rs:374` — `record_read`
  definition.
- `crates/fsqlite-mvcc/src/begin_concurrent.rs:403` —
  `record_read_witness`.
