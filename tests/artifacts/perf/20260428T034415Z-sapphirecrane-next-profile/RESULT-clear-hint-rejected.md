# Synthetic Page-One Hint Rejected

## Scenario

- Workload: `perf-update-delete 10000 100 both`
- Candidate: cache a `SharedTxnPageIo` page-one synthetic-conflict hint to skip `clear_stale_synthetic_pending_commit_surface` work.
- Immediate baseline artifact: `hyperfine-mvcc-predicate.json`
- Candidate artifact: `hyperfine-clear-hint.json`

## Result

| Run | Mean | Median | Stddev |
|-----|-----:|-------:|-------:|
| Predicate-only baseline | 1.237742438s | 1.236636755s | 0.019892274s |
| Synthetic hint candidate | 1.301156189s | 1.298983931s | 0.021522736s |

Median regressed by 5.04%, well outside the same-host noise envelope. The hint
also needed conservative initialization for preexisting synthetic page-one state,
which made the common path less clean than the original predicate-only probe.

## Decision

Reject the hint and roll back the `SharedTxnPageIo::synthetic_page_one_maybe`
state. Keep the predicate-only helper from `928148ee`: it remains the measured
win and preserves the existing stale-synthetic-page-one reconciliation behavior.

## Verification

- `cargo fmt --check`
- `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-sapphirecrane-vdbe-rollback-check cargo check -p fsqlite-vdbe --profile release-perf`
- `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-sapphirecrane-vdbe-rollback-clippy cargo clippy -p fsqlite-vdbe --all-targets -- -D warnings`
- `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-sapphirecrane-vdbe-rollback-test cargo test -p fsqlite-vdbe shared_txn_page_io --profile release-perf -- --nocapture`
