# Insert Commit Phase Split - Auto Lane Candidate - 2026-05-05T20:50Z

## Command

```bash
FSQLITE_BENCH_PROFILE_INSERT=1 FSQLITE_PARALLEL_WAL_MODE=auto CARGO_TARGET_DIR=.rch-target \
  cargo run --profile release-perf -p fsqlite-e2e --bin comprehensive-bench -- \
  --quick --filter insert \
  --json-out tests/artifacts/perf/insert-commit-phase-split-auto-cyangorge-20260505T2050Z/report.json
```

## Result

This run measured opt-in lane-local WAL staging as a candidate for changing the
default conservative WAL mode.

- Total scenarios: 25
- FrankenSQLite faster: 0
- Comparable: 0
- C SQLite faster: 25
- Average FSQLite/C SQLite ratio: 2.86x
- Geomean ratio: 2.65x
- Primary weighted score: 2.0219

Compared with the same current-code conservative/default run
`insert-commit-phase-split-gated-cyangorge-20260505T2058Z`, auto mode regressed:

- Average ratio: 2.51x -> 2.86x
- Geomean ratio: 2.42x -> 2.65x
- Primary weighted score: 1.7652 -> 2.0219
- P99 ratio: 3.95x -> 6.15x

Large-row record-size ratio improved in this noisy quick run
(`3.95x -> 3.74x`), but the whole insert matrix and write-single section
rejected the candidate. Do not change the default WAL mode to auto based on
this evidence.
