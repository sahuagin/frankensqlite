# Post-Pager Mixed Update/Delete Profile

## Scope

- Base commit: `1bdd7ea83cef9b102d8006b5b2d0cc98531e4e78` (`perf(pager): skip debug-only group commit telemetry work`)
- Scenario: `perf-update-delete 10000 50 both`
- Build profile: `release-perf` with frame pointers and line-table debug symbols
- Artifact owner: `AzurePine`

This pass reprofiled the mixed update/delete workload after the pager telemetry
fast path landed. No source files are part of this artifact commit.

## Timing

| Run | Total | Populate | Update | Delete | Per-row update | Per-row delete |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Baseline 1 | 1132 ms | 477 ms | 348 ms | 230 ms | 6979 ns | 9217 ns |
| Baseline 2 | 1225 ms | 515 ms | 377 ms | 251 ms | 7546 ns | 10046 ns |
| Baseline 3 | 1117 ms | 471 ms | 343 ms | 230 ms | 6867 ns | 9212 ns |
| Profile run | 1145 ms | 483 ms | 349 ms | 237 ms | 6999 ns | 9482 ns |

Baseline median total time: `1132 ms`. The second baseline run was noisy high,
but the perf-recorded run stayed close to the median.

## Current Hotspots

Flat `perf report --no-children --sort=symbol` on `perf-current.data`:

| Overhead | Symbol / interpretation |
| ---: | --- |
| 7.68% | `__memmove_avx_unaligned_erms`, mostly realloc/grow traffic |
| 5.44% | `_int_malloc`, including `Connection::capture_time_travel_snapshot` cloning |
| 4.51% | `Connection::execute_prepared_direct_simple_insert` |
| 4.19% | `fsqlite_btree::cell::cell_on_page_size_fast`, under delete defrag sorting |
| 4.11% | `BtCursor<SharedTxnPageIo>::delete`, including `remove_table_cell_from_leaf_deferred` |
| 3.28% | `sort_cells_desc_by_ptr` quicksort path |
| 2.70% | `sort_cells_desc_by_ptr` smallsort path |
| 2.59% | `fsqlite_types::record::parse_record_into` |
| 2.05% | `WalChecksumTransform::for_wal_frame` |
| 1.97% | `SharedTxnPageIo::write_page_internal` |

The previous pager group-commit candidate is no longer a top target:
`build_group_commit_batch` is now `0.09%` flat in this profile.

## Decision

Do not continue optimizing the pager group-commit copy path from the prior
artifact. The current evidence points to B-tree delete defrag/sort/cell-size
work and snapshot-clone allocation as the next meaningful lanes.

The shared worktree currently has an unstaged `crates/fsqlite-btree/src/cursor.rs`
change that adds an already-sorted check to `sort_cells_desc_by_ptr`, but Agent
Mail reports that file is exclusively reserved by `MagentaBeaver`. I left that
source diff untouched and unstaged. That in-flight patch should be measured
against this artifact before any further B-tree delete-defrag changes are made.

## Commands

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-azurepine-20260427-post-pager CARGO_PROFILE_RELEASE_PERF_DEBUG=line-tables-only CARGO_PROFILE_RELEASE_PERF_STRIP=false RUSTFLAGS='-C force-frame-pointers=yes' cargo build --profile release-perf -p fsqlite-e2e --bin perf-update-delete
```

```bash
/data/tmp/cargo-target-azurepine-20260427-post-pager/release-perf/perf-update-delete 10000 50 both
```

```bash
perf record -F 997 -g --call-graph dwarf -o tests/artifacts/perf/20260427T0300Z-azurepine-post-pager-profile/perf-current.data -- /data/tmp/cargo-target-azurepine-20260427-post-pager/release-perf/perf-update-delete 10000 50 both
```

```bash
perf report --stdio --no-children --sort=symbol -i tests/artifacts/perf/20260427T0300Z-azurepine-post-pager-profile/perf-current.data
```
