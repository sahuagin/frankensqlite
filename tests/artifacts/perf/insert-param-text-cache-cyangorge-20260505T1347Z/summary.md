# Direct INSERT parameter text cache A/B

Run ID: `insert-param-text-cache-cyangorge-20260505T1347Z`

## Scenario

This run tested a narrow direct INSERT row-builder candidate in an isolated
worktree based on `974370f8`: cache the decimal `itoa` text for integer bind
placeholders during one row build, so repeated concat segments such as `?1`
would reuse the same stack-backed text bytes instead of formatting each time.

The candidate was intentionally not applied to the shared main worktree. The
active main worktree had peer source work in progress, so this A/B used
`/data/tmp/frankensqlite-cyangorge-paramtext-cache-20260505T1340` and wrote the
report back to this artifact bundle.

## Correctness checks

- `cargo fmt --check`
- `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-cyangorge-paramtext-cache-target cargo test -p fsqlite-core test_prepared_direct_simple_insert_autocommit_profile_breakdown -- --nocapture`
- `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-cyangorge-paramtext-cache-target cargo test -p fsqlite-core prepared_direct_simple_insert_concat_chain -- --nocapture`
- `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-cyangorge-paramtext-cache-target cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench`

## Benchmark command

```bash
env FSQLITE_BENCH_PROFILE_INSERT=1 \
  /data/tmp/frankensqlite-cyangorge-paramtext-cache-target/release-perf/comprehensive-bench \
  --quick \
  --filter insert \
  --json-out /data/projects/frankensqlite/tests/artifacts/perf/insert-param-text-cache-cyangorge-20260505T1347Z/report.json \
  --no-html
```

Baseline for comparison:
`tests/artifacts/perf/insert-external-qb-hint-owned-cyangorge-baseline-20260505T1318Z/report.json`.

## Result

Rejected. The cache preserved focused correctness checks, but the insert matrix
got worse rather than faster:

| Metric | Baseline | Candidate |
| --- | ---: | ---: |
| Average F/C ratio | 2.5011x | 2.6225x |
| Geomean F/C ratio | 2.3832x | 2.5280x |
| Median F/C ratio | 2.2317x | 2.4743x |
| Weighted score | 1.6578 | 1.7978 |
| Write-bulk geomean | 2.5538x | 2.6975x |
| Write-single geomean | 1.4354x | 1.5703x |

Selected large-row medians also regressed:

| Row | Baseline F median | Candidate F median |
| --- | ---: | ---: |
| single txn small_3col 10K | 7.342 ms | 7.440 ms |
| single txn medium_6col 10K | 14.579 ms | 15.125 ms |
| single txn large_10col 10K | 37.587 ms | 37.762 ms |
| record-size medium_6col 10K | 10.597 ms | 10.744 ms |
| record-size large_10col 10K | 39.468 ms | 41.398 ms |

The profiled large record-size row still spent about `5.95 ms` in row build,
`7.57 ms` in B-tree insert, and `16.97 ms` in commit roundtrip. Caching repeated
integer placeholder text did not move the row-build hotspot enough to offset the
extra cache/search/codegen overhead.

## Disposition

Do not retry per-row integer placeholder text caching as a standalone
direct INSERT row-build optimization. Reconsider this area only with a direct
serialization design that avoids transient text materialization entirely and
proves a full insert-matrix win.
