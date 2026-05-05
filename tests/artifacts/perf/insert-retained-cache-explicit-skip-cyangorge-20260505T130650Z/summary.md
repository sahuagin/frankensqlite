# Retained Autocommit Cache Explicit-Txn Skip

Date: 2026-05-05
Agent: CyanGorge
Git base shown by benchmark: `f4252792bcd3e21f0993371c9021c7ed84b5eb31`
Command:

```bash
env FSQLITE_BENCH_PROFILE_INSERT=1 /data/tmp/frankensqlite-cyangorge-check-target/release-perf/comprehensive-bench --quick --filter insert --json-out tests/artifacts/perf/insert-retained-cache-explicit-skip-cyangorge-20260505T130650Z/report.json --no-html
```

Candidate:

- In `crates/fsqlite-core/src/connection.rs`, returned early from
  `retained_autocommit_count_sum_cache_note_insert` when
  `self.in_transaction.get()` was true.
- Rationale: explicit `BEGIN..COMMIT` insert workloads cannot seed the retained
  autocommit count/sum cache because the seed helper already returns while a
  transaction is active. The candidate attempted to avoid one per-row cache path
  in direct-simple INSERT.

Correctness/build gates before the A/B:

- `cargo fmt --check`
- `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-cyangorge-check-target cargo test -p fsqlite-core retained_autocommit_count_sum_cache -- --nocapture`
- `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-cyangorge-check-target cargo test -p fsqlite-core test_prepared_direct_simple_insert_autocommit_profile_breakdown -- --nocapture`
- `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-cyangorge-check-target cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench`

Result:

- Rejected and reverted after A/B measurement.
- Same-window baseline:
  `tests/artifacts/perf/insert-concat-owned-text-baseline-cyangorge-20260505T124529Z/`.
- Insert geomean regressed from `2.2471x` to `2.4574x`.
- Weighted insert score regressed from `1.6366` to `1.7698`.
- P99 ratio regressed from `3.7572x` to `4.0913x`.
- `write_bulk` geomean regressed from `2.3870x` to `2.6160x`.
- `write_single` geomean regressed from `1.4431x` to `1.5537x`.
- `large_10col` single-transaction 10K FrankenSQLite median regressed from
  `35.292 ms` to `36.626 ms`.
- Record-size `large_10col` 10K FrankenSQLite median regressed from
  `36.379 ms` to `36.733 ms`.

Conclusion:

Do not retry explicit-transaction skipping of
`retained_autocommit_count_sum_cache_note_insert` as a standalone direct INSERT
optimization. The cache lookup was logically redundant for this workload, but
the branch/codegen perturbation was not free and the matrix moved the wrong way.
