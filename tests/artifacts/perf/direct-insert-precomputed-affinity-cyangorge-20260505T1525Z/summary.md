# Direct INSERT Precomputed Affinity Candidate

Date: 2026-05-05
Agent: CyanGorge
Base commit: `f063d54a1b9d7573f81576e8b175c6ffdff46a9e`

## Scenario

Workload:

```bash
FSQLITE_BENCH_PROFILE_INSERT=1 \
  /data/tmp/cargo-target/release-perf/comprehensive-bench \
  --quick --filter insert --no-html
```

Target: direct-simple INSERT row value handling in
`crates/fsqlite-core/src/connection.rs`, after perf attributed visible time to
`push_prepared_direct_simple_insert_value` / `SqliteValue::apply_affinity`.

Candidate: add a `column_affinities: Vec<TypeAffinity>` field to
`PreparedDirectSimpleInsert`, computed once during
`prepared_direct_simple_insert_plan`, and pass the precomputed enum into
`push_prepared_direct_simple_insert_value` instead of calling
`type_affinity_for_direct_insert(column.affinity)` for every inserted column.

Source disposition: reverted after measurement.

## Correctness Smoke

Passed:

```bash
cargo fmt --check
env CARGO_TARGET_DIR=/data/tmp/cargo-target \
  cargo test -p fsqlite-core prepared_direct_simple_insert -- --nocapture
```

The focused test filter ran 28 matching prepared-direct-insert tests, all
passing.

## Aggregate Result

Rejected by the insert matrix.

| Metric | Baseline | Candidate |
|---|---:|---:|
| primary weighted insert score | 1.5606 | 1.8360 |
| average F/C ratio | 2.3295x | 2.5739x |
| geomean F/C ratio | 2.2311x | 2.4638x |
| write_bulk geomean | 2.3883x | 2.6058x |
| write_single geomean | 1.3542x | 1.6338x |

## Selected Rows

| Row | Baseline F median | Candidate F median |
|---|---:|---:|
| small_3col 10K single txn | 7.356 ms | 7.446 ms |
| medium_6col 10K single txn | 14.186 ms | 14.213 ms |
| large_10col 10K single txn | 38.466 ms | 38.536 ms |
| small_3col 10K autocommit | 12.049 ms | 11.776 ms |
| small_3col 10K batched 1000/txn | 7.079 ms | 7.121 ms |
| small_3col 10K single txn strategy row | 6.773 ms | 6.476 ms |
| record-size large_10col 10K | 38.854 ms | 36.393 ms |

Some individual FSQLite medians improved, but the primary weighted score and
both write category geomeans regressed. The candidate is not a keep.

## Profile Counter Notes

The target large-row row-build counters did not improve reliably:

- `fs_insert_single_txn_large_10col_10000` row_build_ns:
  `6114165` baseline vs `6115810` candidate.
- `fs_insert_record_size_large_10col_10000` row_build_ns:
  `5951537` baseline vs `6813546` candidate.

## Files

- `baseline-report.json`
- `baseline-stdout.txt`
- `baseline-stderr.txt`
- `candidate-report.json`
- `candidate-stdout.txt`
- `candidate-stderr.txt`
