# Current MT Read Refresh - PurpleCoast - 2026-05-05

## Command

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-current-mt-read-target \
  cargo run --profile release-perf -p fsqlite-e2e --bin mt-read-bench -- \
  --rows=5000 --reads-per-thread=20000 --threads=1,2,4,8 \
  > tests/artifacts/perf/current-mt-read-purplecoast-20260505T1810Z/run.log 2>&1
```

The benchmark completed successfully on RCH worker `ts2` from current `main`.

## Result

| Threads | FSQLite reads/s | C SQLite reads/s | Ratio |
|---:|---:|---:|---:|
| 1 | 273026 | 292633 | 0.93x |
| 2 | 444912 | 450799 | 0.99x |
| 4 | 810819 | 475393 | 1.71x |
| 8 | 1423833 | 356502 | 3.99x |

The stale `bd-eyh2e` 1-thread read gap is no longer the next target. It was
reported on 2026-04-24 as `18.9K` FSQLite reads/s versus `288K` C SQLite
reads/s (`0.07x`). Current `main` is close to C SQLite at 1-2 threads and
faster at 4-8 threads on the same benchmark shape.

Continue prioritizing the current write-side gaps, especially explicit
`:memory:` INSERT throughput and the mutation rows whose setup still depends on
INSERT prepopulation.
