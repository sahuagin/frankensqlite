# Stage-Only External Quick-Balance Hint A/B

Date: 2026-05-05

Clean base: `f7ea3cdd docs(perf): publish insert commit profile artifacts`

## Candidate

Targeted `crates/fsqlite-btree/src/cursor.rs` in the external rightmost append
hint quick-balance path.

Patch shape:

- After `balance_quick_known_divider_rowid`, keep only `TableAppendHint`
  metadata when the pager can mutate staged `PageData` directly.
- Clear `hint.page_data` and the internal rightmost cache for staged-capable
  writers to avoid retaining/cloning the newly split leaf image twice.
- Preserve the existing retained-page behavior for PageWriters that cannot
  mutate staged page data.
- When a staged external hint fills, try quick-balance from the staged-page
  branch instead of returning `false` immediately.

The first stage-only attempt failed
`test_table_try_append_cached_rightmost_leaf_hint_reuses_retained_leaf_image`
with row-order corruption (`59` expected, `95` observed). The measured candidate
included the staged-capability guard and staged-page quick-balance fallback; the
same B-tree proof then passed.

## Verification

Shared worktree verification was blocked by an unrelated dirty
`crates/fsqlite-pager/src/pager.rs` compile error during the first proof run, so
the candidate was isolated in a clean detached worktree under `/data/tmp` and
applied against `f7ea3cdd`.

Passed in the clean worktree:

```bash
cargo fmt --check
rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-stage-only-qb-clean-target cargo test -p fsqlite-btree table_try_append_cached_rightmost_leaf_hint --profile release-perf -- --nocapture
rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-stage-only-qb-clean-target cargo test -p fsqlite-core prepared_direct_simple_insert_implicit_rowid --profile release-perf -- --nocapture
```

Notes:

- `rch` fell back to local execution for the `/data/tmp` detached worktree
  because that path is outside RCH's `/data/projects` canonical root.
- B-tree proof: `4` matching tests passed.
- Core proof: `3` matching tests passed.

## Benchmark

Commands:

```bash
/data/tmp/frankensqlite-stage-only-qb-clean-target/release-perf/comprehensive-bench --quick --filter insert --json-out tests/artifacts/perf/stage-only-qb-hint-purplecoast-20260505T1716Z/baseline-insert-report.json --no-html
/data/tmp/comprehensive-bench-stage-only-qb-candidate-purplecoast-20260505T1712 --quick --filter insert --json-out tests/artifacts/perf/stage-only-qb-hint-purplecoast-20260505T1716Z/candidate-insert-report.json --no-html
```

Reports:

- `baseline-insert-report.json`
- `candidate-insert-report.json`
- `ab-summary.json`
- `baseline-run.log`
- `candidate-run.log`

## Result

Rejected.

Same-window insert quick matrix:

- Rows: `25`
- FSQLite median wins/regressions: `10` improved, `15` regressed
- FSQLite median geomean: `1.0254x` candidate/baseline, so `2.54%` slower
- C-relative ratio geomean: `0.9590x`, but this was driven by C-side timing
  movement and did not reflect an absolute FSQLite win

Largest FSQLite regressions:

- `small_3col` 1000 single-txn: `0.802 ms -> 0.947 ms` (`+18.0%`)
- `large_10col` 1000 single-txn: `1.916 ms -> 2.149 ms` (`+12.1%`)
- small transaction-strategy 10K single txn: `6.367 ms -> 7.084 ms`
  (`+11.3%`)
- `small_3col` 10K single-txn: `6.863 ms -> 7.567 ms` (`+10.3%`)

Target large rows were mixed, not enough to keep:

- `large_10col` 10K single-txn improved `37.483 ms -> 36.182 ms`
  (`3.47%` faster)
- record-size `large_10col` 10K regressed `35.613 ms -> 36.716 ms`
  (`3.10%` slower)

Do not retry this stage-only external quick-balance retained-hint clone
avoidance as a standalone optimization. The retained page image acts as an
important fallback/rollback shape, and skipping it did not move the insert
matrix even after preserving correctness for staged-capable writers.
