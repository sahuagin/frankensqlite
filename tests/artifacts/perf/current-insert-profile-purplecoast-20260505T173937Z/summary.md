# Current INSERT Gap Profile - PurpleCoast - 2026-05-05

## Command

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-current-insert-profile-target \
  FSQLITE_BENCH_PROFILE_INSERT=1 cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- \
  --quick --filter insert --json-stdout \
  > tests/artifacts/perf/current-insert-profile-purplecoast-20260505T173937Z/report.json \
  2> tests/artifacts/perf/current-insert-profile-purplecoast-20260505T173937Z/run.log
```

The RCH worker wrote JSON to `run.log`; `report.json` is the extracted JSON
payload from that log. Insert profiler lines are extracted in
`insert-profiles.log`.

## Matrix Result

- Scenarios: 25
- C SQLite faster: 25
- Average ratio, FSQLite / C SQLite: 2.3356x
- Geomean ratio, FSQLite / C SQLite: 2.1860x
- Weighted insert score: 1.4720

Worst current rows:

| Scenario | Category | C SQLite ms | FSQLite ms | Ratio |
|---|---:|---:|---:|---:|
| large_10col, 10K record size | write_bulk | 10.3446 | 40.2102 | 3.8871x |
| medium_6col, 10K record size | write_bulk | 5.4614 | 10.8297 | 1.9829x |
| tiny_1col, 10K record size | write_bulk | 2.5948 | 4.9985 | 1.9264x |
| small_3col, 10K record size | write_bulk | 4.0648 | 7.0922 | 1.7448x |

## Hotspot Counters

Selected profiler rows from `insert-profiles.log`:

| Scenario | insert_us | commit_us | btree | row_build | serialize | quick | commit_round | misses |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| single_txn_tiny_1col_10000 | 12568.0 | 147.4 | 4.8ms | 0.5ms | 0.0ms | 0.0ms | 0.1ms | 21 |
| single_txn_small_3col_10000 | 13462.7 | 568.1 | 4.3ms | 1.7ms | 0.6ms | 0.0ms | 0.5ms | 68 |
| single_txn_medium_6col_10000 | 15062.2 | 3357.9 | 4.9ms | 2.8ms | 1.2ms | 0.0ms | 3.3ms | 459 |
| single_txn_large_10col_10000 | 24988.7 | 18654.3 | 9.1ms | 6.7ms | 1.9ms | 4.7ms | 18.5ms | 2013 |
| record_size_large_10col_10000 | 28028.9 | 19971.2 | 11.6ms | 7.1ms | 1.9ms | 6.8ms | 19.8ms | 2013 |

The direct INSERT profiler reports the fast path for all rows (`fast=count`,
`slow=0`, no VDBE opcodes). The dominant current large-row gap is therefore
not VDBE dispatch. It is concentrated in B-tree append/balance work plus
private `:memory:` explicit transaction dirty-page publication at COMMIT.
Tiny and small rows still have a large unaccounted per-row body cost, but many
obvious direct-insert bookkeeping skips are already in the negative ledger.

## Do Not Retry From This Profile Alone

The current profile points at several areas that have already been rejected in
the end-to-end insert matrix:

- direct DML `SharedTxnPageIo` reuse
- explicit `:memory:` retained writer reuse across `BEGIN` / `COMMIT`
- private-memory WAL or memory-journal benchmark shortcuts
- standalone MemoryVfs batch append rewrites
- retained-leaf writer append
- direct INSERT count/sum hook early-return
- direct INSERT precomputed column affinities
- WAL page-index `Arc::make_mut` hoist
- WAL checksum one-chunk header transform

Next work should first split the B-tree quick-balance and dirty-page
publication counters more finely, then keep only a candidate that moves the
same-window insert matrix. Do not optimize VDBE dispatch for this workload.
