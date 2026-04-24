# mt-mvcc-bench Summary

- Rows per thread: `500`
- Iterations: `20`
- Schema: `fsqlite-e2e.mt_mvcc_bench_report.v2`

- Pass-over-pass gate: `no_prior_report` (threshold `5.00%`, history `/data/projects/frankensqlite/tests/artifacts/perf/profiling-mt-mvcc-20260424T161631Z/profile_mt2_history_6bd060.json`)

| Threads | fsqlite p50 wps | sqlite p50 wps | Throughput ratio | fsqlite p50 ms | sqlite p50 ms | Time ratio | fsqlite failed | sqlite failed |
|---------|-----------------:|---------------:|-----------------:|---------------:|--------------:|-----------:|---------------:|--------------:|
| 2 | 198369 | 409395 | 0.48x | 5.04 | 2.44 | 2.06x | 0 | 0 |
