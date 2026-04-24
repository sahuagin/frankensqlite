# Hotspot Table

Profiles were captured with `perf record -F 1999 --call-graph dwarf -g` and reduced with both callgraph and flat reports against frozen hash `6bd060fb4eb7dccef4cd49a6dc3b8f7583249a8c`. The important change versus the earlier stale bundle is that `xxh3` is gone from the top 20. The `page image token` change appears to have removed the old hashing hotspot, but it did not remove the 2-thread throughput cliff in the main matrix.

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

## Callgraph Evidence

The MT8 callgraph now highlights:

```text
PagerBackend::open_with_requested_page_size_and_page_buffer_max
  -> ShardedPageCache::with_max_buffers

Connection::execute_prepared_direct_simple_insert
  -> BtCursor<SharedTxnPageIo>::table_seek_for_insert
  -> SharedTxnPageIo::read_page_data

Connection::execute_commit_with_cx
  -> SimpleTransaction::commit_wal_group_commit_with_snapshot
```

The MT2 lifecycle hotspot is still teardown work:

```text
core::ptr::drop_in_place::<Connection>
  -> Arc<ShardedPageCache>::drop_slow
```

## Reliability Note

The sequential 1/2/4/8/12/16 matrix still completes through 16T at this frozen hash, but a cold dedicated 16-thread probe still fails before the synchronized start with `database is busy (recovery in progress)`. Startup recovery sensitivity remains separate from the CPU hotspots above.
