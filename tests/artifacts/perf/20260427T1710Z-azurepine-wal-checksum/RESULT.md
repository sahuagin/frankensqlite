# WAL Checksum Transform Recurrence

## Scope

- Baseline commit: `b42e3236` (`perf: publish post snapshot opt-out update delete profile`)
- Candidate source: `crates/fsqlite-wal/src/checksum.rs`
- Scenario A: `perf-update-delete 10000 50 both`
- Scenario B: `perf-update-delete 10000 200 both`
- Build profile: `release-perf` with frame pointers and line-table debug symbols

The profile after the earlier B-tree and benchmark fixes still showed
`WalChecksumTransform::for_wal_frame` in the flat profile. The source change
keeps the affine transform representation but composes each 8-byte SQLite WAL
checksum step with the equivalent closed-form recurrence instead of
materializing a one-step transform and calling the general matrix composition.

For a WAL word pair `(x0, x1)`, SQLite updates a running checksum as:

```text
s1' = s1 + s2 + x0
s2' = s1 + 2*s2 + x0 + x1
```

Applying that recurrence to the accumulated affine coefficients is algebraically
equivalent to `transform.then(from_checksum_words(x0, x1))`, but avoids the
general affine multiply/add path for every word pair in a frame.

## Timing: 10000 Rows, 50 Iterations

| Run | Baseline total | Candidate total | Baseline populate | Candidate populate |
| --- | ---: | ---: | ---: | ---: |
| 1 | 873 ms | 855 ms | 418 ms | 414 ms |
| 2 | 871 ms | 878 ms | 413 ms | 419 ms |
| 3 | 867 ms | 895 ms | 420 ms | 425 ms |
| 4 | 882 ms | 867 ms | 418 ms | 413 ms |
| 5 | 868 ms | 869 ms | 419 ms | 412 ms |
| 6 | 889 ms | 861 ms | 420 ms | 408 ms |
| 7 | 870 ms | 846 ms | 412 ms | 408 ms |

Median total: `871 ms -> 867 ms` (`0.46%` faster).

Median populate: `418 ms -> 413 ms` (`1.20%` faster).

This short scenario was a weak positive signal, so I reran a longer mixed
scenario before deciding whether to keep the source change.

## Timing: 10000 Rows, 200 Iterations

| Run | Baseline total | Candidate total | Baseline populate | Candidate populate |
| --- | ---: | ---: | ---: | ---: |
| 1 | 3544 ms | 3399 ms | 1739 ms | 1625 ms |
| 2 | 3455 ms | 3433 ms | 1665 ms | 1659 ms |
| 3 | 3432 ms | 3440 ms | 1704 ms | 1655 ms |
| 4 | 3360 ms | 3322 ms | 1641 ms | 1631 ms |
| 5 | 3342 ms | 3348 ms | 1643 ms | 1635 ms |

Median total: `3432 ms -> 3399 ms` (`0.96%` faster).

Median populate: `1665 ms -> 1635 ms` (`1.80%` faster).

Additional phase medians from the run output:
update `1079 ms -> 1070 ms` (`0.83%` faster); delete
`500 ms -> 535 ms` (`7.00%` slower, noisy/non-target path).

The longer scenario supports keeping the candidate as a small hot-path
improvement, not as a broad benchmark-level breakthrough.

## Profile Check

Candidate perf sample:

```bash
perf record -F 997 -g --call-graph dwarf -o /data/tmp/azurepine-wal-checksum-candidate.data -- /data/tmp/cargo-target-azurepine-20260427-wal-checksum-candidate/release-perf/perf-update-delete 10000 100 both
```

Benchmark output for that sample:

```text
total=1745ms populate=829ms update=556ms delete=279ms
```

Flat profile grep found:

```text
1.47% [.] <fsqlite_wal::checksum::WalChecksumTransform>::for_wal_frame
```

The prior post-monotone artifact recorded the same symbol at `2.66%` in the
current profile. The samples are not a perfect matched-pair profile, but the
targeted symbol moved in the expected direction and the A/B benchmark stayed
positive on the longer scenario.

## Verification

Passed on the candidate:

```bash
cargo fmt --check
```

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-azurepine-20260427-wal-checksum cargo test -p fsqlite-wal test_wal_checksum_transform_matches_frame_checksum -- --nocapture
```

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-azurepine-20260427-wal-checksum cargo check -p fsqlite-wal --all-targets
```

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-azurepine-20260427-wal-checksum cargo clippy -p fsqlite-wal --all-targets -- -D warnings
```

```bash
ubs crates/fsqlite-wal/src/checksum.rs tests/artifacts/perf/20260427T1710Z-azurepine-wal-checksum/RESULT.md
```

UBS exited 0. It reported existing warning inventory inside `checksum.rs`; the
new changed slices are guarded by `chunks_exact(8)`.

The broader `checksum` test filter exposed an existing failing torn-write test:

```text
checksum::tests::test_classify_torn_write_mid_frame panicked:
assertion `left == right` failed: only 3 complete frames before truncation
left: 2
right: 3
```

The same torn-write test failed from a clean detached baseline worktree at
`b42e3236`, so it is not caused by this checksum-transform recurrence change.

## Decision

Keep the source change. It is behavior-preserving by direct algebraic
equivalence, has focused checksum-transform coverage, and shows a modest
repeatable improvement in the benchmark path that was still visible in the
profile.
