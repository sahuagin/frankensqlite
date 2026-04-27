# Current Mixed Update/Delete Profile

Date: 2026-04-27
Agent: AzurePine
Base commit: 85efd5ec
Scenario: `perf-update-delete 10000 50 both`

## Purpose

Refresh the mixed update/delete profile after these landed commits:

- `f7582663 perf(core): fuse memdb reload row payload reads`
- `46a3db18 fix(btree): report corrupt out-of-range cell offsets`
- `85efd5ec perf(core): preserve count-sum cache across no-op direct DML`

The earlier memdb reload target is no longer a top hotspot. In this run,
`reload_memdb_from_txn_with_mode` is 0.83% flat self time.

## Baseline

Build:

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-azurepine-20260427-current CARGO_PROFILE_RELEASE_PERF_DEBUG=line-tables-only CARGO_PROFILE_RELEASE_PERF_STRIP=false RUSTFLAGS='-C force-frame-pointers=yes' cargo build --profile release-perf -p fsqlite-e2e --bin perf-update-delete
```

Runs:

| Run | total | populate | update | delete | per-row update | per-row delete |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 1139ms | 485ms | 350ms | 231ms | 7013ns | 9252ns |
| 2 | 1145ms | 480ms | 352ms | 238ms | 7051ns | 9526ns |

Median total: 1142ms.

## Profile

Command:

```bash
perf record -F 997 -g --call-graph dwarf -o perf-current.data -- /data/tmp/cargo-target-azurepine-20260427-current/release-perf/perf-update-delete 10000 50 both
perf report --stdio --no-children --sort=symbol -i perf-current.data > perf-current-no-children.txt
```

Profile run total: 1188ms.
Samples: 1559.

Top flat symbols:

| Rank | Symbol | Flat |
| ---: | --- | ---: |
| 1 | `__memmove_avx_unaligned_erms` under group commit page copies | 7.97% |
| 2 | `Connection::execute_prepared_direct_simple_insert` | 4.41% |
| 3 | `_int_malloc` | 4.16% |
| 4 | `BtCursor<SharedTxnPageIo>::delete` | 3.82% |
| 5 | `sort_cells_desc_by_ptr` quicksort path | 3.69% |
| 6 | `cell_on_page_size_fast` | 3.64% |
| 7 | `sort_cells_desc_by_ptr` smallsort path | 2.88% |
| 8 | `parse_record_into` | 2.08% |
| 9 | `SharedTxnPageIo::write_page_internal` | 2.08% |

Notes:

- Kernel symbols were restricted by `kptr_restrict` / `perf_event_paranoid`.
- `addr2line` emitted sentinel warnings, but the flat user-space symbol report
  was still generated and is included.

## Rejected Lever

Hypothesis: `remove_table_cell_from_leaf_deferred` still allocated fresh
defrag pointer and move buffers. Reusing the cursor-owned `defrag_ptrs_scratch`
and `defrag_cells_scratch` buffers should reduce allocator pressure in the
table-leaf DELETE path.

Result: rejected for this pass. The isolated clean worktree at
`/data/tmp/frankensqlite-azurepine-isolated-20260427-0239` showed only a small
total-time movement:

| Tree | Run | total | populate | update | delete | per-row update | per-row delete |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 1 | 1139ms | 485ms | 350ms | 231ms | 7013ns | 9252ns |
| baseline | 2 | 1145ms | 480ms | 352ms | 238ms | 7051ns | 9526ns |
| isolated patched | 1 | 1094ms | 483ms | 328ms | 217ms | 6565ns | 8687ns |
| isolated patched | 2 | 1135ms | 493ms | 341ms | 227ms | 6824ns | 9092ns |
| isolated patched | 3 | 1135ms | 486ms | 343ms | 233ms | 6861ns | 9351ns |

Baseline median total: 1142ms.
Isolated patched median total: 1135ms.
Total-time delta: -0.6%.

The patch was manually backed out from the main worktree. No code change from
this rejected lever was committed.

## Current Blocker

`crates/fsqlite-pager/src/pager.rs` became dirty during this pass and Agent Mail
reports it is exclusively reserved by `MagentaBeaver` until
2026-04-27T04:37:09Z. That file is also the top current profile target:
`build_group_commit_batch` page copies show up under `__memmove_avx_unaligned_erms`.

Next high-confidence optimization handoff: once the pager reservation lands or
releases, inspect whether group commit can avoid one owned page copy on the
synchronous/local path, or whether staged byte accounting/logging can be kept
off the hot path without changing WAL ownership semantics.
