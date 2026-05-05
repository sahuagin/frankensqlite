# External Quick-Balance Cache Single-Authority Probe - ProudAnchor - 2026-05-05T21:18Z

## Scope

Target: prepared direct INSERT external rightmost-leaf append path in
`crates/fsqlite-btree/src/cursor.rs`, specifically
`try_quick_balance_on_external_rightmost_leaf_hint`.

The profile motivation was the current insert matrix, where large 10-column
`:memory:` INSERT rows still spend measurable time in B-tree append and
quick-balance work. The candidate tried to remove one duplicate 4 KiB
`PageData` retention after an external-hint quick-balance split.

## Candidate

After `balance_quick_known_divider_rowid` succeeds, move
`result.new_page_data` into the caller-owned `TableAppendHint` and clear the
cursor's internal `rightmost_leaf_cache` instead of also storing a duplicate
internal `RightmostLeafCacheEntry`.

Rationale: adjacent external-hint append paths already make the retained hint
authoritative and clear the internal cache when no full cursor cache refresh is
performed.

Candidate diff: `candidate.diff`.

## Correctness

Focused candidate test passed in the candidate worktree:

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-proudanchor-qbcache-candidate-test-target \
  cargo test -p fsqlite-btree table_try_append_cached_rightmost_leaf_hint -- --nocapture
```

`rch` failed open because the worktree was under `/data/tmp`, so the command ran
locally. Result: `4` tests passed.

## Benchmark

Both binaries were built locally with separate target dirs:

```bash
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-proudanchor-qbcache-baseline-target \
  CARGO_INCREMENTAL=0 cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench

env CARGO_TARGET_DIR=/data/tmp/frankensqlite-proudanchor-qbcache-candidate-target \
  CARGO_INCREMENTAL=0 cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench
```

Benchmark commands:

```bash
env FSQLITE_BENCH_PROFILE_INSERT=1 \
  /data/tmp/frankensqlite-proudanchor-qbcache-baseline-target/release-perf/comprehensive-bench \
  --quick --filter insert --json-out baseline-report.json --no-html

env FSQLITE_BENCH_PROFILE_INSERT=1 \
  /data/tmp/frankensqlite-proudanchor-qbcache-candidate-target/release-perf/comprehensive-bench \
  --quick --filter insert --json-out candidate-report.json --no-html
```

## Result

Rejected. The candidate worsened the aggregate insert matrix:

| Metric | Baseline | Candidate |
| --- | ---: | ---: |
| Total scenarios | 25 | 25 |
| C SQLite faster | 25 | 25 |
| Average FSQLite/C ratio | 2.4990x | 2.5713x |
| Geomean FSQLite/C ratio | 2.3954x | 2.4847x |
| Primary weighted score | 1.7007 | 1.7335 |
| Write-bulk geomean | 2.5568x | 2.6611x |
| Write-single geomean | 1.4846x | 1.5027x |

The large 10-column record-size row did improve by the relative ratio
(`4.07x -> 3.79x`) but the absolute FSQLite median still regressed
(`36.54 ms -> 38.74 ms`) and the C SQLite reference also moved. The whole
matrix rejected the change.

## No-Retry Rule

Do not retry "move quick-balance `new_page_data` solely into external
`TableAppendHint` and clear the cursor cache" as a standalone prepared direct
INSERT optimization. The avoided duplicate retained-cache state is too small
and/or loses useful downstream locality. Revisit only if a future profile proves
the internal post-split cache entry itself is a dominant cost and a same-window
insert matrix improves on absolute FrankenSQLite medians.
