# WAL Publication Page-Index make_mut Hoist Candidate

- Candidate: in `crates/fsqlite-core/src/wal_adapter.rs`, hoist
  `Arc::make_mut(&mut page_index)` out of the per-frame loop in
  `publish_pending_commit_snapshot`.
- Rationale: large INSERT commits publish thousands of WAL frame entries, so
  avoiding a repeated `Arc::make_mut` call looked like a cheap way to trim
  commit publication overhead without changing the prepared-frame publication
  path.
- Baseline:
  `tests/artifacts/perf/insert-profile-current-head-cyangorge-20260505T122449Z/report.json`.
- Candidate:
  `tests/artifacts/perf/insert-wal-page-index-makemut-purplecoast-20260505T1513Z/report.json`.
- Source diff: reverted after measurement.

## Correctness

`rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-wal-makemut-target cargo test -p fsqlite-core --lib append -- --nocapture`
passed: `17` tests.

`rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-wal-makemut-target cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench`
passed.

## Result

Rejected. The proof tests passed, but the insert matrix got substantially
worse.

| Metric | Baseline | Candidate |
| --- | ---: | ---: |
| Avg ratio | `2.4610x` | `2.5586x` |
| Geomean ratio | `2.3623x` | `2.4753x` |
| Weighted score | `1.6991` | `1.8022` |
| write_bulk geomean | `2.5153x` | `2.6295x` |
| write_single geomean | `1.4908x` | `1.5889x` |

Selected absolute medians:

| Row | Baseline F median | Candidate F median |
| --- | ---: | ---: |
| single transaction `tiny_1col` 100 | `0.267 ms` | `0.275 ms` |
| single transaction `small_3col` 100 | `0.293 ms` | `0.311 ms` |
| single transaction `large_10col` 10K | `36.165 ms` | `35.504 ms` |
| record-size `large_10col` 10K | `37.056 ms` | `37.232 ms` |

## Disposition

Do not retry this simple `Arc::make_mut` hoist as a standalone WAL publication
optimization. It slightly helped one large-row median, but the end-to-end insert
score and write-single section rejected it.
