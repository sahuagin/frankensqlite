# mt-mvcc-bench Summary

- Rows per thread: `100`
- Iterations: `2`
- Schema: `fsqlite-e2e.mt_mvcc_bench_report.v2`

- Pass-over-pass gate: `no_prior_report` (threshold `5.00%`, history `tests/artifacts/perf/profiling-mt-mvcc-20260424T161631Z/strace_probe_history.json`)

| Threads | fsqlite p50 wps | sqlite p50 wps | Throughput ratio | fsqlite p50 ms | sqlite p50 ms | Time ratio | fsqlite failed | sqlite failed |
|---------|-----------------:|---------------:|-----------------:|---------------:|--------------:|-----------:|---------------:|--------------:|
| 8 | 1284 | 18020 | 0.07x | 625.58 | 47.62 | 13.14x | 0 | 0 |
