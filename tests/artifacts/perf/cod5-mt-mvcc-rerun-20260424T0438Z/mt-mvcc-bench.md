# mt-mvcc-bench Summary

- Rows per thread: `500`
- Iterations: `10`
- Schema: `fsqlite-e2e.mt_mvcc_bench_report.v2`

- Pass-over-pass gate: `no_prior_report` (threshold `5.00%`, history `tests/artifacts/perf/cod5-mt-mvcc-rerun-20260424T0438Z/mt-mvcc-bench.history.json`)

| Threads | fsqlite p50 wps | sqlite p50 wps | Throughput ratio | fsqlite p50 ms | sqlite p50 ms | Time ratio | fsqlite failed | sqlite failed |
|---------|-----------------:|---------------:|-----------------:|---------------:|--------------:|-----------:|---------------:|--------------:|
| 1 | 290190 | 770887 | 0.38x | 1.73 | 0.65 | 2.67x | 0 | 0 |
| 2 | 9544 | 510610 | 0.02x | 104.78 | 1.96 | 53.50x | 0 | 0 |
| 4 | 9459 | 213101 | 0.04x | 211.44 | 9.39 | 22.53x | 0 | 0 |
| 8 | 12227 | 72132 | 0.17x | 327.14 | 55.45 | 5.90x | 0 | 0 |
