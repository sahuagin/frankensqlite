# Compact Table-Leaf Delete Fast Path

Date: 2026-04-28
Agent: SnowyFortress
Baseline: `7f417c99` benchmark binary; `0d610086` only changed `crates/fsqlite-cli/src/main.rs`, outside this benchmark crate graph.

## Profile

Target command:

```bash
perf-update-delete 10000 100 both
```

Fresh `perf record -F 997 -g --call-graph dwarf` on the current benchmark binary showed the next actionable hot path inside table-row UPDATE/DELETE:

- `remove_table_cell_from_leaf_deferred` under `BtCursor::delete`
- `sort_cells_desc_by_ptr` / `sort_unstable_by` under that path: about 10% children in the UPDATE/DELETE half of the call graph
- `__memmove_avx_unaligned_erms`: about 8.5% self

The key observation was that UPDATE disturbs logical pointer order on otherwise compact table leaves. The old compact fast path only handled logically descending cell pointers; non-descending compact leaves fell through to a full sort just to recover physical byte extents.

## Accepted Candidate

Patch: preserve the existing descending compact fast path, and add a 4 KiB compact-page physical-neighbor scan for non-descending table leaves:

- scan pointers once with a small offset bitset to reject duplicates
- find the next higher physical cell pointer to compute the deleted cell extent
- shift the lower physical byte range once
- adjust only pointers below the deleted physical offset
- keep large-page compact leaves on the existing conservative sort fallback

## A/B Measurements

All runs used `hyperfine --warmup 1 --runs 12`.

### Mixed UPDATE + DELETE

```text
base-7f417c99:                         1.539 s +/- 0.019 s
candidate-compact-physical-delete:     1.323 s +/- 0.025 s
speedup:                               1.16x +/- 0.03
```

JSON: `hyperfine-compact-physical-delete-refined-10000x100-both.json`

### UPDATE-only

```text
base-7f417c99:                         1.324 s +/- 0.027 s
candidate-compact-physical-delete:     1.171 s +/- 0.027 s
speedup:                               1.13x +/- 0.04
```

JSON: `hyperfine-compact-physical-delete-refined-10000x100-update.json`

### DELETE-only

```text
base-7f417c99:                         942.8 ms +/- 18.4 ms
candidate-compact-physical-delete:     958.4 ms +/- 25.6 ms
result:                                statistically flat/slightly noisy
```

JSON: `hyperfine-compact-physical-delete-refined-10000x100-delete.json`

## Rejected Candidates

### Deferred Scratch Reuse

Reused `defrag_cells_scratch` in `remove_table_cell_from_leaf_deferred`.

```text
both:   base 1.527 s +/- 0.015 s, candidate 1.555 s +/- 0.022 s
delete: base 938.3 ms +/- 41.1 ms, candidate 941.5 ms +/- 9.8 ms
```

Rejected: no measured win.

### Unrefined Physical Delete

Applied the physical-neighbor path to all compact table leaves.

```text
both:   base 1.561 s +/- 0.042 s, candidate 1.504 s +/- 0.216 s
delete: base 949.4 ms +/- 22.0 ms, candidate 980.4 ms +/- 14.5 ms
```

Rejected: it replaced the cheaper descending fast path and regressed delete-only.

## Verification

```bash
cargo fmt --check
ubs crates/fsqlite-btree/src/cursor.rs
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-snowyfortress-20260428-main-verify cargo test -p fsqlite-btree delete -- --nocapture
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-snowyfortress-20260428-main-verify cargo check --workspace --all-targets
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-snowyfortress-20260428-main-verify cargo clippy --workspace --all-targets -- -D warnings
```

Results:

- `cargo fmt --check`: passed
- `ubs crates/fsqlite-btree/src/cursor.rs`: no critical findings; existing broad warnings only
- `cargo test -p fsqlite-btree delete`: 30 passed
- `cargo check --workspace --all-targets`: passed
- `cargo clippy --workspace --all-targets -- -D warnings`: passed
