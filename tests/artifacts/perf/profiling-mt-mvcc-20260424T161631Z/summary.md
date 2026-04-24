# Profiling Summary

Run id: `profiling-mt-mvcc-20260424T161631Z`

This refresh is frozen to base `HEAD` `6bd060fb4eb7dccef4cd49a6dc3b8f7583249a8c` (`perf(btree): key cell cache by page image token`) plus the same minimal local build-unblock overlay for the `Arc<str>` migration fallout in `fsqlite-core` and `fsqlite-vdbe`. The benchmark binary was rebuilt with `rch exec` into `/data/tmp/rch_target_tanivy_profile_current2` using `RUSTFLAGS='-C force-frame-pointers=yes'`. Kernel symbols remain partially unresolved because `perf_event_paranoid=1` and `kptr_restrict=1`.

## Matrix

Command:

```bash
timeout 240s /data/tmp/rch_target_tanivy_profile_current2/release-perf/mt-mvcc-bench \
  --rows-per-thread=500 --threads=1,2,4,8,12,16 --iters=10 \
  --json-output tests/artifacts/perf/profiling-mt-mvcc-20260424T161631Z/mt_mvcc_matrix.json \
  --summary-md tests/artifacts/perf/profiling-mt-mvcc-20260424T161631Z/mt_mvcc_matrix.md \
  --history-json tests/artifacts/perf/profiling-mt-mvcc-20260424T161631Z/mt_mvcc_history_current.json
```

| Threads | fsqlite p50 wps | sqlite p50 wps | Throughput ratio | fsqlite p50 ms | sqlite p50 ms | Time ratio |
|--------:|----------------:|---------------:|-----------------:|---------------:|--------------:|-----------:|
| 1 | 305,920 | 874,833 | 0.35x | 1.64 | 0.57 | 2.87x |
| 2 | 9,648 | 490,004 | 0.02x | 103.65 | 2.04 | 50.79x |
| 4 | 9,578 | 205,540 | 0.05x | 208.82 | 9.73 | 21.46x |
| 8 | 10,837 | 49,773 | 0.22x | 374.78 | 80.37 | 4.66x |
| 12 | 12,454 | 29,439 | 0.42x | 486.07 | 206.69 | 2.35x |
| 16 | 14,642 | 12,655 | 1.16x | 546.38 | 632.15 | 0.86x |

Key readout:

- The pass-over-pass gate fires hard on this hash: `2T 0.36x -> 0.02x`, `4T 0.09x -> 0.05x`, and `16T 1.39x -> 1.16x`.
- The 1-to-2 cliff is back. The `page image token` change removed the old CPU hashing hotspot, but it did not stabilize throughput scaling.
- The sequential matrix still reaches a valid 16-thread datapoint, but a cold dedicated 16-thread-only probe still fails during worker open with `database is busy (recovery in progress)`.

## Focused Profiles

These isolated 20-iteration runs are for hotspot attribution, not apples-to-apples comparison with the mixed thread-count matrix.

| Focus run | fsqlite p50 wps | sqlite p50 wps | Throughput ratio | fsqlite p50 ms | sqlite p50 ms |
|---|---:|---:|---:|---:|---:|
| MT2 | 198,369 | 409,395 | 0.48x | 5.04 | 2.44 |
| MT8 | 31,812 | 71,315 | 0.45x | 125.75 | 56.09 |

## Top 10 Hotspots

| Rank | Symbol / cluster | MT8 self | MT2 self | Mechanism | Evidence |
|-----:|---|---:|---:|---|---|
| 1 | `__memmove_avx_unaligned_erms` | 3.06% | 3.17% | Page/value copying is now the largest recurring user-space CPU cost. | `perf_mt8_flat.txt`, `perf_mt2_flat.txt` |
| 2 | `Arc<ShardedPageCache>::drop_slow` | 1.46% | 3.13% | Connection teardown still pollutes the short isolated samples, especially MT2. | `perf_mt2_flat.txt`, `perf_mt2_callgraph.txt`, `perf_mt8_flat.txt` |
| 3 | `_int_malloc` | 2.50% | 2.51% | Allocation churn remains a major secondary cost after the hash-path change. | `perf_mt8_flat.txt`, `perf_mt2_flat.txt` |
| 4 | `ShardedPageCache::with_max_buffers` | 2.26% | 2.22% | Connection-open/page-cache construction still matters materially in both focus runs. | `perf_mt8_flat.txt`, `perf_mt2_flat.txt`, `perf_mt8_callgraph.txt` |
| 5 | `Connection::execute_prepared_direct_simple_insert` | 2.17% | 2.21% | The direct insert body is still centrally hot. | `perf_mt8_flat.txt`, `perf_mt2_flat.txt`, `perf_mt8_callgraph.txt` |
| 6 | `SharedTxnPageIo::read_page_data` | 1.64% | n/a | Read/decode cost is now more visible than the old mutation hash. | `perf_mt8_flat.txt`, `perf_mt8_callgraph.txt` |
| 7 | `BtCursor<...>::table_seek_for_insert` | 1.20% | n/a | Insert navigation remains an MT8 cost center. | `perf_mt8_flat.txt`, `perf_mt8_callgraph.txt` |
| 8 | `commit_wal_group_commit_with_snapshot::<...>::{closure#3}` | 0.97% | n/a | Group-commit work is now visible in MT8 after the old hash hotspot fell away. | `perf_mt8_flat.txt`, `perf_mt8_callgraph.txt` |
| 9 | `BtCursor<...>::table_leaf_rowid_at` | 0.96% | n/a | Leaf-rowid lookup remains part of the insert-path read side. | `perf_mt8_flat.txt`, `perf_mt8_callgraph.txt` |
| 10 | `read_cell_pointers_into` / `FlatPageSlots::clear` | 0.92% / 0.76% | 2.06% / 1.51% | Cell-pointer rebuilds and slot clearing are now clearer residual costs in both MT2 and MT8. | `perf_mt8_flat.txt`, `perf_mt2_flat.txt` |

## Conclusions

- `6bd060fb` appears to have succeeded at one narrow goal: `xxh3`-based mutation hashing disappears from the top hotspot list.
- That CPU win did not translate into a healthier benchmark matrix. The 2-thread cliff returns, the pass-over-pass gate fails, and mid-range scaling remains poor.
- Startup reliability is still not fixed: a cold dedicated 16-thread run can fail before the synchronized barrier with recovery-in-progress opens, even though the sequential matrix survives through 16T.
