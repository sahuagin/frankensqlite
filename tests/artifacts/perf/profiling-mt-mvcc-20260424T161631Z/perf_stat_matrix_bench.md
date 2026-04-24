# mt-mvcc-bench Summary

- Rows per thread: `500`
- Iterations: `5`
- Schema: `fsqlite-e2e.mt_mvcc_bench_report.v2`

- Pass-over-pass gate: `no_prior_report` (threshold `5.00%`, history `tests/artifacts/perf/profiling-mt-mvcc-20260424T161631Z/perf_stat_matrix_history.json`)

| Threads | fsqlite p50 wps | sqlite p50 wps | Throughput ratio | fsqlite p50 ms | sqlite p50 ms | Time ratio | fsqlite failed | sqlite failed |
|---------|-----------------:|---------------:|-----------------:|---------------:|--------------:|-----------:|---------------:|--------------:|
| 1 | 239279 | 757938 | 0.32x | 2.09 | 0.66 | 3.17x | 0 | 0 |
| 2 | 175114 | 458747 | 0.38x | 5.71 | 2.18 | 2.62x | 0 | 0 |
| 4 | 9429 | 205043 | 0.05x | 212.11 | 9.75 | 21.75x | 0 | 0 |
| 8 | 12275 | 49552 | 0.25x | 325.88 | 80.72 | 4.04x | 0 | 0 |
| 12 | 13456 | 45041 | 0.30x | 445.89 | 133.21 | 3.35x | 0 | 0 |
