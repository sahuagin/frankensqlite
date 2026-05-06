# Rejected UPDATE projected-byte page patch retry - 2026-05-05

Agent: ProudAnchor

## Candidate

Retried the direct fixed-width REAL UPDATE payload-range optimization after a
current isolated profile again showed time in payload copy/parsing and
`table_overwrite_current_payload_same_size_no_overflow`.

The candidate added a B-tree helper that parses the current no-overflow table
payload directly from the leaf page and copies only the projected replacement
bytes for the updated REAL column. The direct fixed-width REAL UPDATE path used
that helper before borrowing generic row and payload scratch buffers.

## Correctness

Focused checks passed:

```bash
env CARGO_TARGET_DIR=.rch-target cargo check -p fsqlite-core -p fsqlite-btree
env CARGO_TARGET_DIR=.rch-target cargo test -p fsqlite-btree test_table_patch_current_payload_projected_bytes_no_overflow_updates_column_only -- --nocapture
env CARGO_TARGET_DIR=.rch-target cargo test -p fsqlite-btree table_overwrite_current_payload_same_size_no_overflow -- --nocapture
env CARGO_TARGET_DIR=.rch-target cargo test -p fsqlite-core direct_simple_update -- --nocapture
```

## Measurement

Focused same-window reverse/restore runs improved the isolated UPDATE harness:

| Run | Per-row update |
| --- | ---: |
| Baseline reverse, fsqlite isolated | 880 ns |
| Candidate repeat, fsqlite isolated | 838 ns |
| Baseline reverse, compare isolated | 886 ns |
| Candidate repeat, compare isolated | 839 ns |

The broader quick `UPDATE/DELETEThroughput` section was mixed:

| Scenario | Baseline FSQLite | Candidate FSQLite | Candidate/base |
| --- | ---: | ---: | ---: |
| 100 rows / update 10 rows | 451.7 us | 468.5 us | 1.037x |
| 100 rows / delete 5 rows | 501.4 us | 491.0 us | 0.979x |
| 1000 rows / update 100 rows | 1.26 ms | 1.32 ms | 1.052x |
| 1000 rows / delete 50 rows | 1.19 ms | 1.22 ms | 1.020x |
| 10000 rows / update 1000 rows | 10.34 ms | 9.70 ms | 0.939x |
| 10000 rows / delete 500 rows | 9.21 ms | 8.63 ms | 0.937x |

FSQLite geomean across the section was only `0.993x` candidate/base.

## Decision

Rejected and reverted. The targeted 10K row improved, but the section did not
clear the keep bar because the small and mid update/delete rows regressed and
the aggregate FSQLite movement was below 1%.
