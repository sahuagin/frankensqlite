# WAL Checksum Header Transform Candidate

Date: 2026-05-05
Agent: CyanGorge
Base commit: `45d3508f`

## Scenario

Workload:

```bash
FSQLITE_BENCH_PROFILE_INSERT=1 \
  /data/tmp/frankensqlite-cyangorge-walchk-target/release-perf/comprehensive-bench \
  --quick --filter insert --no-html
```

Target: `WalChecksumTransform::for_wal_frame` self-time under large INSERT WAL
frame preparation.

Candidate: replace the generic
`WalChecksumTransform::from_aligned_bytes(&frame[..8], ...)` call for the
8-byte WAL frame header prefix with the closed-form affine transform for exactly
one checksum chunk. The page payload transform stayed on the generic path.

Source disposition: reverted after measurement.

## Correctness Smoke

Passed:

```bash
cargo fmt --check
env CARGO_TARGET_DIR=/data/tmp/cargo-target \
  cargo test -p fsqlite-wal checksum_transform -- --nocapture
```

The focused test filter ran the two checksum-transform equivalence tests, both
passing.

Note: the first release-perf build attempt in the shared
`/data/tmp/cargo-target` failed with a missing bytecode file, consistent with
target-dir interference. The candidate was rebuilt in the unique target dir
`/data/tmp/frankensqlite-cyangorge-walchk-target`.

## Aggregate Result

Rejected by the insert matrix. Baseline is the same-source current-head insert
baseline from
`tests/artifacts/perf/direct-insert-precomputed-affinity-cyangorge-20260505T1525Z/baseline-report.json`.

| Metric | Baseline | Candidate |
|---|---:|---:|
| primary weighted insert score | 1.5606 | 1.7049 |
| average F/C ratio | 2.3295x | 2.4746x |
| geomean F/C ratio | 2.2311x | 2.3800x |
| write_bulk geomean | 2.3883x | 2.5361x |
| write_single geomean | 1.3542x | 1.4935x |

## Selected Rows

| Row | Baseline F median | Candidate F median |
|---|---:|---:|
| small_3col 10K single txn | 7.356 ms | 7.208 ms |
| medium_6col 10K single txn | 14.186 ms | 14.089 ms |
| large_10col 10K single txn | 38.466 ms | 38.242 ms |
| small_3col 10K autocommit | 12.049 ms | 11.525 ms |
| small_3col 10K batched 1000/txn | 7.079 ms | 6.720 ms |
| small_3col 10K single txn strategy row | 6.773 ms | 6.541 ms |
| record-size large_10col 10K | 38.854 ms | 36.671 ms |

Despite several absolute FSQLite median improvements, the primary ratio-based
matrix and both write category geomeans regressed. This is not a keep.

## Profile Counter Notes

Large-row row-build counters were not the intended target and do not explain a
keep:

- `fs_insert_single_txn_large_10col_10000` row_build_ns:
  `6114165` baseline vs `6312580` candidate.
- `fs_insert_record_size_large_10col_10000` row_build_ns:
  `5951537` baseline vs `5956654` candidate.

## Files

- `candidate-report.json`
- `candidate-stdout.txt`
- `candidate-stderr.txt`
