# Current Full-Matrix Gap Refresh - PurpleCoast - 2026-05-05

## Command

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-current-gap-refresh-target \
  cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- \
  --quick --json-out tests/artifacts/perf/current-gap-refresh-purplecoast-20260505T173118Z/report.json --no-html \
  2>&1 | tee tests/artifacts/perf/current-gap-refresh-purplecoast-20260505T173118Z/run.log
```

The benchmark completed successfully on RCH worker `ts2` from
`main @ 0092a2bfe74f8f2ececc186cf1f0a7bc937182e0`. RCH did not retrieve the
remote JSON report; `run.log` is the durable local evidence for this run.

## Matrix Summary

- Total scenarios: 93
- FrankenSQLite faster: 57
- Comparable: 1
- C SQLite faster: 35
- Average time ratio, FrankenSQLite / C SQLite: 1.09x

The headline gap is still write-heavy `:memory:` INSERT. UPDATE and DELETE
rows remain slower too, but this quick matrix prepopulates those tables through
the same INSERT path, so mutation-specific profiling should isolate setup cost
before treating update/delete as independent hotspots.

## Largest Remaining Gap Families

### INSERT, Single Transaction

| Scenario | C SQLite | FSQLite | Ratio |
|---|---:|---:|---:|
| tiny_1col, 100 rows | 83.2 us | 311.6 us | 3.75x slower |
| tiny_1col, 1K rows | 265.5 us | 729.1 us | 2.75x slower |
| tiny_1col, 10K rows | 2.55 ms | 4.81 ms | 1.89x slower |
| small_3col, 100 rows | 74.2 us | 289.6 us | 3.90x slower |
| small_3col, 1K rows | 359.9 us | 1.27 ms | 3.54x slower |
| small_3col, 10K rows | 3.38 ms | 7.76 ms | 2.30x slower |
| medium_6col, 100 rows | 227.4 us | 835.8 us | 3.68x slower |
| medium_6col, 1K rows | 738.3 us | 2.04 ms | 2.76x slower |
| medium_6col, 10K rows | 6.51 ms | 15.93 ms | 2.45x slower |
| large_10col, 100 rows | 324.9 us | 657.0 us | 2.02x slower |
| large_10col, 1K rows | 1.33 ms | 3.03 ms | 2.28x slower |
| large_10col, 10K rows | 10.62 ms | 40.79 ms | 3.84x slower |

### INSERT, Transaction Strategy

| Scenario | C SQLite | FSQLite | Ratio |
|---|---:|---:|---:|
| 100 rows, autocommit | 267.6 us | 359.1 us | 1.34x slower |
| 100 rows, batched | 140.0 us | 614.9 us | 4.39x slower |
| 100 rows, single txn | 192.2 us | 365.1 us | 1.90x slower |
| 1K rows, autocommit | 1.13 ms | 1.82 ms | 1.61x slower |
| 1K rows, batched | 512.7 us | 1.02 ms | 1.99x slower |
| 1K rows, single txn | 522.7 us | 983.4 us | 1.88x slower |
| 10K rows, autocommit | 8.97 ms | 14.00 ms | 1.56x slower |
| 10K rows, batched | 3.75 ms | 7.53 ms | 2.01x slower |
| 10K rows, single txn | 3.47 ms | 7.47 ms | 2.15x slower |

### INSERT, Record Size

| Scenario | C SQLite | FSQLite | Ratio |
|---|---:|---:|---:|
| tiny_1col, 10K rows | 2.86 ms | 4.70 ms | 1.64x slower |
| small_3col, 10K rows | 3.36 ms | 6.76 ms | 2.01x slower |
| medium_6col, 10K rows | 5.59 ms | 11.87 ms | 2.12x slower |
| large_10col, 10K rows | 10.45 ms | 41.54 ms | 3.98x slower |

### Secondary Gaps

- Recursive CTE: 230.2 us C SQLite vs 350.5 us FSQLite, 1.52x slower.
- String workloads mostly favor FSQLite, but 1K wildcard LIKE is 1.10x slower
  and 1K string functions are 1.06x slower.

## Next Target

Use the focused insert profile in
`tests/artifacts/perf/current-insert-profile-purplecoast-20260505T173937Z/`.
That profile shows all tested INSERT rows are already on the direct fast path,
so the next viable lever is narrower profiling of B-tree append/balance and
private-memory dirty-page publication, not VDBE dispatch.
