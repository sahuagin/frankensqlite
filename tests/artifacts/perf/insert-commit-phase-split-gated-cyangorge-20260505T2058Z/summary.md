# Insert Commit Phase Split - Gated Detailed Metrics - 2026-05-05T20:58Z

## Command

```bash
FSQLITE_BENCH_PROFILE_INSERT=1 CARGO_TARGET_DIR=.rch-target \
  cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- \
  --quick --filter insert \
  --json-out tests/artifacts/perf/insert-commit-phase-split-gated-cyangorge-20260505T2058Z/report.json
```

## Result

This is the current-code conservative/default insert profile after the detailed
commit sub-counters were gated behind `FSQLITE_BENCH_PROFILE_INSERT=1` or
`FSQLITE_WAL_DETAILED_COMMIT_METRICS=1`.

- Head: `de8f9e723695a3b75dcf570928f8cb5988526ac9`
- Mode: quick insert matrix, release-perf, 2 warmups, 3-10 iterations
- Total scenarios: 25
- FrankenSQLite faster: 0
- Comparable: 0
- C SQLite faster: 25
- Average FSQLite/C SQLite ratio: 2.51x
- Geomean ratio: 2.42x
- Primary weighted score: 1.7652

## Slowest Rows

| Row | C SQLite | FrankenSQLite | Ratio |
| --- | ---: | ---: | ---: |
| Single transaction `large_10col`, 10000 rows | 10.99 ms | 38.36 ms | 3.49x |
| Record-size comparison `large_10col`, 10000 rows | 9.49 ms | 37.52 ms | 3.95x |

## Phase-B Split

For `fs_insert_record_size_large_10col_10000`:

- `commit_phase_b_us=13332`
- `commit_prepare_us=4299`
- `commit_batch_build_us=4293`
- `commit_conflict_snapshot_us=4`
- `commit_lane_prepare_us=0`
- `commit_flush_frame_prep_us=4962`
- `commit_wal_append_us=4028`
- `commit_append_conflict_check_us=7`
- `commit_append_frames_us=4020`
- `commit_flusher_lock_wait_us=0`
- `commit_wal_sync_us=0`

For `fs_insert_single_txn_large_10col_10000`:

- `commit_phase_b_us=13817`
- `commit_prepare_us=4476`
- `commit_batch_build_us=4471`
- `commit_conflict_snapshot_us=4`
- `commit_lane_prepare_us=0`
- `commit_flush_frame_prep_us=5214`
- `commit_wal_append_us=4078`
- `commit_append_conflict_check_us=6`
- `commit_append_frames_us=4071`
- `commit_flusher_lock_wait_us=0`
- `commit_wal_sync_us=0`

## Interpretation

The default large INSERT commit gap is not lock wait or fsync. It is a
representation/copy pipeline:

1. Batch build clones about 8.3 MB of staged page images into owned
   `TransactionFrameBatch` frame payloads.
2. Flusher frame preparation builds or finalizes the WAL prepared-frame byte
   stream from those frame refs.
3. The append call writes the finalized prepared frame bytes and publishes WAL
   metadata.

The simple raw-append bypass and checksum/header micro-variants are already
rejected in the negative ledger. The next plausible target needs to reduce the
duplicate page-image ownership/prepared-frame pipeline itself while preserving
group-commit conflict, publication, and fault semantics.
