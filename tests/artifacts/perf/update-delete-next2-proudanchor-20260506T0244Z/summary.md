# UPDATE/DELETE tier0 already-staged MVCC marker candidate

Date: 2026-05-06
Agent: ProudAnchor
Base commit: 75b1e6a0
Clean candidate worktree: /data/tmp/frankensqlite-proudanchor-clean-20260506T0239Z
Pristine baseline worktree: /data/tmp/frankensqlite-proudanchor-baseline-20260506T0254Z

## Target

Current worst quick-matrix row after `7d6117e1`:
`UPDATE/DELETEThroughput` / `100 rows / delete 5 rows`, where the latest full
quick artifact reported FrankenSQLite `0.425427 ms` vs C SQLite `0.092583 ms`
(`4.595x`).

## Profile

Fresh delayed perf on the isolated delete loop:

- Command: `perf-update-delete 100 20000 delete fsqlite isolated`
- FSQLite: `total=234ms`, `delete=180ms`, `per-row-delete=1808ns`
- Top flat symbols:
  - `TransactionKind::get_page`: `15.90%`
  - `TransactionKind::write_page_data`: `12.49%`
  - `BtCursor<SharedTxnPageIo>::delete`: `10.17%`
  - `__memmove_avx_unaligned_erms`: `6.36%`
  - `BtCursor<SharedTxnPageIo>::table_seek_for_insert`: `6.05%`

## Candidate

Add a `Tier0AlreadyStaged` write tier in `SharedTxnPageIo` so writes to an
active concurrent page that already has a staged-write marker skip redundant
MVCC marker restaging and go straight to the pager write-set update.

This did not alter default concurrent-writer mode and did not touch B-tree
cursor code.

## Result

Rejected.

Interleaved clean-worktree A/B:

- Isolated delete command:
  `perf-update-delete 100 20000 delete fsqlite isolated`
  - Baseline: `227.7 ms +/- 2.6 ms`
  - Candidate: `229.7 ms +/- 3.1 ms`
  - Hyperfine: baseline `1.01 +/- 0.02` times faster.
- Standard-row proxy:
  `perf-update-delete 100 80 delete fsqlite standard`
  - Baseline: `29.6 ms +/- 0.6 ms`
  - Candidate: `30.7 ms +/- 0.9 ms`
  - Hyperfine: baseline `1.04 +/- 0.04` times faster.

Do not retry this exact tier0 already-staged marker skip as a standalone
UPDATE/DELETE optimization. The repeated marker-stage work is not large enough
to offset the extra tier classification branch/probe on this workload.

## Files

- `fingerprint.txt`
- `baseline-delete100-isolated.log`
- `baseline-delete100-standard.log`
- `delete100-fsqlite-isolated-delay-perf-report.txt`
- `candidate-tier0-staged.diff`
- `candidate-tier0-delete100-isolated.log`
- `candidate-tier0-delete100-standard.log`
- `hyperfine-tier0-staged-isolated-fsqlite.json`
- `hyperfine-tier0-staged-standard-fsqlite.json`

Raw `.perf.data` is intentionally ignored.
