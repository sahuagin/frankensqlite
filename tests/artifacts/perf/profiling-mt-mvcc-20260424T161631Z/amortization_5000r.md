# mt-mvcc-bench Summary

- Rows per thread: `5000`
- Iterations: `3`
- Schema: `fsqlite-e2e.mt_mvcc_bench_report.v2`

- Pass-over-pass gate: `no_prior_report` (threshold `5.00%`, history `tests/artifacts/perf/profiling-mt-mvcc-20260424T161631Z/amortization_5000r_history.json`)

| Threads | fsqlite p50 wps | sqlite p50 wps | Throughput ratio | fsqlite p50 ms | sqlite p50 ms | Time ratio | fsqlite failed | sqlite failed |
|---------|-----------------:|---------------:|-----------------:|---------------:|--------------:|-----------:|---------------:|--------------:|
| 1 | 584167 | 1647930 | 0.35x | 8.56 | 3.03 | 2.82x | 0 | 0 |
| 2 | 79201 | 1247492 | 0.06x | 126.26 | 8.02 | 15.75x | 0 | 0 |
| 4 | 66562 | 497384 | 0.13x | 300.47 | 40.21 | 7.47x | 0 | 0 |
| 8 | 68950 | 293575 | 0.23x | 580.13 | 136.25 | 4.26x | 0 | 0 |
