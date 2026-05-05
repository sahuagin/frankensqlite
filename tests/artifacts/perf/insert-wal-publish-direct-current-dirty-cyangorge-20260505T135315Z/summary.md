# Dirty WAL prepared-frame publication direct snapshot check

Run ID: `insert-wal-publish-direct-current-dirty-cyangorge-20260505T135315Z`

## Scenario

This run measured a peer-owned dirty change in
`crates/fsqlite-core/src/wal_adapter.rs`. The diff bypasses the
`pending_publication_frames` recording path for prepared frame batches that
already know their commit frame, publishing the visibility snapshot directly
from the prepared frame metadata.

The source diff was not edited by this agent. A copy is preserved in
`source.diff`.

## Correctness checks

- `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-cyangorge-wal-publish-target cargo test -p fsqlite-core --lib append -- --nocapture`
  passed: `17 passed; 0 failed`.
- A broader exploratory filter,
  `cargo test -p fsqlite-core append -- --nocapture`, also passed the
  WAL adapter append tests but failed
  `test_v2_plain_execute_sequential_inserts_keep_append_path_hot_across_statements`.
  That failure is preserved in `append_tests.stdout` / `append_tests.stderr`
  and should be accounted for before any landing decision.
- `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-cyangorge-wal-publish-target cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench`
  passed.

## Benchmark command

```bash
env FSQLITE_BENCH_PROFILE_INSERT=1 \
  /data/tmp/frankensqlite-cyangorge-wal-publish-target/release-perf/comprehensive-bench \
  --quick \
  --filter insert \
  --json-out tests/artifacts/perf/insert-wal-publish-direct-current-dirty-cyangorge-20260505T135315Z/report.json \
  --no-html
```

Baseline for comparison:
`tests/artifacts/perf/insert-external-qb-hint-owned-cyangorge-baseline-20260505T1318Z/report.json`.

## Result

Mixed, not a keep as-is. Some large-row FSQLite medians improved, but the
primary weighted insert score worsened:

| Metric | Baseline | Dirty candidate |
| --- | ---: | ---: |
| Average F/C ratio | 2.5011x | 2.4813x |
| Geomean F/C ratio | 2.3832x | 2.3890x |
| Median F/C ratio | 2.2317x | 2.2006x |
| Weighted score | 1.6578 | 1.7359 |
| Write-bulk geomean | 2.5538x | 2.5388x |
| Write-single geomean | 1.4354x | 1.5293x |

Selected row medians:

| Row | Baseline F median | Dirty candidate F median |
| --- | ---: | ---: |
| single txn medium_6col 10K | 14.579 ms | 14.707 ms |
| single txn large_10col 10K | 37.587 ms | 35.188 ms |
| record-size medium_6col 10K | 10.597 ms | 9.759 ms |
| record-size large_10col 10K | 39.468 ms | 34.709 ms |

The large-row profile still shows substantial commit cost:

- `fs_insert_single_txn_large_10col_10000`: `commit_roundtrip_ns=15529416`
- `fs_insert_record_size_large_10col_10000`: `commit_roundtrip_ns=15359648`

## Disposition

Do not land this exact dirty diff from this evidence alone. The direct
publication idea may be worth refining because it helped large-row medians, but
the full insert matrix did not clear the keep gate and write-single regressed.
Next retry should use an interleaved clean/candidate A/B, explain the broader
append-filter failure, and preserve the large-row improvement while restoring
the weighted score.
