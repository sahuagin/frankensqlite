# WAL Generation and Authoritative Lookup Contract (`bd-1dp9.6.7.8.1`)

## Purpose

This document is the reference contract for the authoritative WAL page-index
work in T6.7.8.1. It explains the correctness envelope already expressed in:

- `crates/fsqlite-wal/src/wal.rs`
- `crates/fsqlite-core/src/wal_adapter.rs`
- `scripts/verify_t6_7_wal_index.sh`
- `scripts/verify_t6_7_wal_publication_plane.sh`

The goal is to make steady-state lookup semantics explicit before later T6.7.8
work removes more refresh and reverse-scan cost from the hot path. This contract
must let future implementers reason about reset, truncate, ABA, torn-tail / torn
tail, and corruption behavior without re-deriving WAL semantics from code
archaeology.

## Core Runtime Objects

### `WalGenerationIdentity`

`WalGenerationIdentity` in `crates/fsqlite-wal/src/wal.rs` is the identity for
one visible WAL generation.

Its fields are:

- `checkpoint_seq`
- `salts.salt1`
- `salts.salt2`

The generation identity is not "the salts". The generation identity is
`(checkpoint_seq, salt1, salt2)`.

That distinction is mandatory because a reset may intentionally reuse the same
salt pair. Same-salt reset with a new `checkpoint_seq` is still a new
generation and must invalidate cached lookup state to avoid an ABA bug.

### `WalPublishedSnapshot`

`WalPublishedSnapshot` in `crates/fsqlite-core/src/wal_adapter.rs` is the
adapter-visible lookup surface for one published WAL generation. Its key fields
are:

- `publication_seq`
- `generation`
- `last_commit_frame`
- `commit_count`
- `page_index`
- `index_is_partial`

The snapshot is generation-scoped. `page_index` entries are never allowed to
silently span multiple generations.

### `WalPageLookupResolution`

`WalPageLookupResolution` defines the lookup contract exposed by the adapter:

- `AuthoritativeHit`
- `AuthoritativeMiss`
- `PartialIndexFallbackHit`
- `PartialIndexFallbackMiss`

These are not cosmetic labels. They define whether the steady-state index was
complete enough for a miss to be trusted, or whether an explicit fallback path
was required.

## Generation Invariants

`INV-WAL-781-1` Generation identity is `(checkpoint_seq, salt1, salt2)`, not
salts alone.

`INV-WAL-781-2` Reset, truncate, or any header generation change invalidates
cached publication and read snapshots before the new generation is served.

`INV-WAL-781-3` `last_commit_frame` and `page_index` describe only the currently
published generation.

`INV-WAL-781-4` A same-salt reset with a new `checkpoint_seq` is a generation
rollover, not an in-place extension.

`INV-WAL-781-5` If refresh observes file shrink, torn tail, or header
generation change, it rebuilds state from the on-disk WAL instead of trusting
incremental append assumptions.

## Authoritative Lookup Contract

### Steady-state path

The steady-state path is `authoritative_index`.

In that mode:

- the per-generation `page_index` is complete for the visible WAL prefix,
- `AuthoritativeHit` means the index points directly to the newest visible
  frame for the page,
- `AuthoritativeMiss` means the page is absent from the current WAL generation,
- a miss is trusted directly and does not trigger a reverse scan.

This is the fast path later T6.7.8 beads are allowed to optimize.

### Partial-index exception

The exceptional fallback path is `partial_index_fallback`.

It exists only when `index_is_partial = true`, which occurs when a lowered
`page_index_cap` makes the in-memory index incomplete.

In that mode:

- a HashMap miss is not authoritative,
- `scan_backwards_for_page` is allowed,
- the adapter may return `PartialIndexFallbackHit` or
  `PartialIndexFallbackMiss`,
- the fallback reason is explicit: `partial_index_cap`.

This reverse scan is an exception surface, not the normal lookup model.

### Integrity guard

The adapter must not silently trust an index entry that resolves to the wrong
page. If a frame read at the resolved `frame_index` does not contain the
expected `page_number`, the contract is to raise `WalCorrupt`, not to silently
fall back or reinterpret the result.

## Exceptional Recovery Boundaries

The authoritative lookup contract must stay separate from exceptional recovery
handling.

### Reset and truncate

- WAL reset starts a new generation.
- Checkpoint reset/truncate clears visible frames for the prior generation.
- Lookup state from the old generation must be discarded before serving new
  reads.

### ABA protection

- Same-salt reset is intentionally supported.
- `checkpoint_seq` is the mandatory anti-ABA discriminator.
- Reusing `(salt1, salt2)` with a higher `checkpoint_seq` must still force
  invalidation and rebuild.

### Torn-tail and short-read handling

Incremental refresh may stop at a truncated tail. That is a recovery boundary,
not an authoritative miss. The contract is to preserve the last valid committed
prefix rather than reinterpret the damaged tail as a steady-state lookup result.

### Corruption

Header checksum mismatch, frame salt mismatch, or index-integrity mismatch are
corruption/rebuild surfaces. They must never be blurred into the normal
`authoritative_index` path.

## Structured Diagnostics Contract

The runtime diagnostics for this bead already exist and must stay explicit.

### Publication-side diagnostics

`fsqlite.wal_publication` traces must expose:

- `wal_generation`
- `wal_salt1`
- `wal_salt2`
- `publication_seq`
- `frame_delta_count`
- `latest_frame_entries`
- `snapshot_age`
- `lookup_mode`
- `fallback_reason`

### Reader-side diagnostics

Lookup/read traces in `crates/fsqlite-core/src/wal_adapter.rs` must expose:

- `wal_checkpoint_seq`
- `wal_salt1`
- `wal_salt2`
- `publication_seq`
- `snapshot_age`
- `lookup_mode`
- `fallback_reason`

These fields are the operator-facing evidence that a read came from
`authoritative_index` versus `partial_index_fallback`.

## Deterministic Verification Surface

The design is bound to deterministic tests already present in-tree.

### Adapter/runtime tests

In `crates/fsqlite-core/src/wal_adapter.rs`:

- `test_page_index_invalidated_on_wal_reset`
- `test_page_index_invalidated_on_same_salt_generation_change`
- `test_partial_index_falls_back_to_linear_scan`
- `test_lookup_contract_distinguishes_authoritative_and_fallback_paths`
- `test_commit_append_publishes_visibility_snapshot`
- `test_prepared_append_publishes_visibility_snapshot`

### WAL-generation tests

In `crates/fsqlite-wal/src/wal.rs`:

- `test_refresh_after_reset_detects_new_generation`
- `test_refresh_after_reset_with_same_salts_detects_new_generation`
- `test_crash_matrix_truncate_at_every_frame_boundary`
- `test_crash_matrix_bit_flip_at_every_frame`
- `test_crash_matrix_reset_then_crash`

### Replayable verifier entrypoints

- `scripts/verify_t6_7_wal_index.sh`
- `scripts/verify_t6_7_wal_publication_plane.sh`

Those scripts are the canonical replay layer for:

- generation invalidation,
- authoritative versus fallback lookup behavior,
- publication-plane metadata,
- truncate/torn-tail/corruption boundaries.

## Future T6.7.8 Rule

Future T6.7.8 implementation work may:

- reduce refresh cost,
- make the per-generation index more authoritative,
- remove exceptional reverse scans from more workloads,
- publish richer generation-stamped visibility metadata.

Future T6.7.8 work may not:

- treat `partial_index_fallback` as the normal lookup contract,
- collapse same-salt reset into the previous generation,
- reinterpret corruption or torn-tail handling as an `AuthoritativeMiss`,
- drop the structured distinction between steady-state lookup and exceptional
  recovery behavior.
