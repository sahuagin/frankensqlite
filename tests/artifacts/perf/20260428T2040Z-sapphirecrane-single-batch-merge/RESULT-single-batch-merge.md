# WAL Single-Batch Merge Fast Path

Date: 2026-04-28

## Scenario

Benchmark command:

```bash
perf-update-delete 10000 100 both
```

Baseline binary:
`/data/tmp/frankensqlite-sapphirecrane-head-5b8636c0/release-perf/perf-update-delete`

Candidate binary:
`/data/tmp/frankensqlite-sapphirecrane-single-batch-merge/release-perf/perf-update-delete`

## Hypothesis

`merge_prepared_group_commit_batches` was paying the full multi-batch merge
cost even when it received exactly one already-canonical prepared WAL batch.
That path copied frame bytes, rewrote commit headers, dropped pre-lock
finalization state, and recomputed checksum transforms over each frame.

The fast path landed as `13c90da5` and reuses a single canonical prepared
batch directly. A follow-up guard landed as `095f82be` and rejects
non-canonical single batches whose metadata and serialized frame db-size
headers disagree, preserving the old merge path's hidden-commit-marker cleanup
semantics.

## Result

Flat profile target:

| Symbol | Baseline | Candidate after guard |
|---|---:|---:|
| `WalChecksumTransform::for_wal_frame` | 2.23% | 1.07% |
| `PreparedWalFrameBatch::recompute_checksum_transforms` under merge | 0.95% | absent from flat top list |
| `merge_prepared_group_commit_batches` children | 3.23% | 2.10% |

Interleaved A/B after the guard:

| Build | Mean | Median | Stddev | Min | Max |
|---|---:|---:|---:|---:|---:|
| baseline-5b8636c0 | 1.251s | 1.248s | 0.019s | 1.217s | 1.296s |
| single-batch-merge-guard | 1.256s | 1.256s | 0.024s | 1.210s | 1.298s |

Throughput result is neutral within noise (`baseline ran 1.00 +/- 0.02x faster
than candidate`). The profile confirms the targeted checksum recompute was
removed from the hot path, but the saved work is not large enough to move this
whole workload's wall-clock time reliably.

## Decision

Keep the fast path from `13c90da5` because it removes a provably redundant
single-batch merge stage and reduces the measured checksum hotspot. Keep the
hidden-header guard from `095f82be` as correctness hardening so the optimization
cannot preserve a byte-level commit marker that the conservative merge path
would have cleared.
