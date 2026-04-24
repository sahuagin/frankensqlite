# mt-mvcc-bench Summary

- Rows per thread: `500`
- Iterations: `10`
- Schema: `fsqlite-e2e.mt_mvcc_bench_report.v2`

- Pass-over-pass gate: `failed` (threshold `5.00%`, history `/data/projects/frankensqlite/tests/artifacts/perf/profiling-mt-mvcc-20260424T161631Z/mt_mvcc_history_current.json`)
- Regressions:
  - 1 threads: 0.38x -> 0.35x (6.79% drop)
  - 2 threads: 0.36x -> 0.02x (94.53% drop)
  - 4 threads: 0.09x -> 0.05x (45.48% drop)
  - 16 threads: 1.39x -> 1.16x (16.54% drop)

| Threads | fsqlite p50 wps | sqlite p50 wps | Throughput ratio | fsqlite p50 ms | sqlite p50 ms | Time ratio | fsqlite failed | sqlite failed |
|---------|-----------------:|---------------:|-----------------:|---------------:|--------------:|-----------:|---------------:|--------------:|
| 1 | 305920 | 874833 | 0.35x | 1.64 | 0.57 | 2.87x | 0 | 0 |
| 2 | 9648 | 490004 | 0.02x | 103.65 | 2.04 | 50.79x | 0 | 0 |
| 4 | 9578 | 205540 | 0.05x | 208.82 | 9.73 | 21.46x | 0 | 0 |
| 8 | 10837 | 49773 | 0.22x | 374.78 | 80.37 | 4.66x | 0 | 0 |
| 12 | 12454 | 29439 | 0.42x | 486.07 | 206.69 | 2.35x | 0 | 0 |
| 16 | 14642 | 12655 | 1.16x | 546.38 | 632.15 | 0.86x | 0 | 0 |
