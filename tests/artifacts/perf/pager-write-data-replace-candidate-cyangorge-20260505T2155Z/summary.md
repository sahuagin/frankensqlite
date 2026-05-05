# Pager write_page_data existing-entry replacement - 2026-05-05

Agent: CyanGorge
Source commit before candidate: `5f55a98d63134a5490fabc2c4eda8d3613527e51`
Landing base: `ba4a47fa` (docs-only perf-ledger work landed after the profile
baseline; the code diff still applies only to `crates/fsqlite-pager/src/pager.rs`)
Landed code commit: `c0987eba4b92c48355126ce8104f840cb937c609`
Candidate file: `crates/fsqlite-pager/src/pager.rs`

## Profile basis

Fresh isolated DELETE profiling on current `HEAD` showed the post-stack-reuse
mutation path still dominated by pager write-set publication:

- Current compare artifact:
  `tests/artifacts/perf/dml-mutation-current-cyangorge-20260505T2140Z/exact-isolated-compare.log`
- Current DELETE-only perf artifact:
  `tests/artifacts/perf/dml-mutation-current-cyangorge-20260505T2140Z/delete-only.perf.data`
- Current DELETE-only result: `1210ms` DELETE, `2421ns` per row.
- Current perf top self: `<TransactionKind as TransactionHandle>::write_page_data`
  at about `32.18%`.

## Candidate

When `SimpleTransaction::write_page_data` writes a page that is already present
in the transaction write-set, the old path tried the same-page overwrite fast
path and then fell through to `insert_staged_page`. For an already-present key,
that rehashed and inserted the same page number even though
`write_pages_sorted` was already correct.

The candidate keeps the successful in-place overwrite path unchanged, but when
the existing staged image cannot be overwritten in place, it replaces that map
entry directly with the new `StagedPage`.

## Correctness

Passed:

```bash
cargo fmt --check
env CARGO_TARGET_DIR=.rch-target cargo check --workspace --all-targets
env CARGO_TARGET_DIR=.rch-target cargo clippy --workspace --all-targets -- -D warnings
env CARGO_TARGET_DIR=.rch-target cargo test -p fsqlite-pager write_page_data -- --nocapture
env CARGO_TARGET_DIR=.rch-target cargo test -p fsqlite-btree cursor_delete -- --nocapture
env CARGO_TARGET_DIR=.rch-target cargo test -p fsqlite-core --lib prepared_delete -- --nocapture
```

The attempted `staged_page_overwrite` test filter matched zero tests, so it is
not counted as proof.

`ubs crates/fsqlite-pager/src/pager.rs` returned non-zero from existing
file-wide heuristic inventory, but its formatter, clippy, cargo-check,
test-build, cargo-audit, and cargo-deny sections were clean; none of the
reported locations was on the changed `write_page_data` branch.

## Focused result

Candidate artifacts:

- `exact-isolated-compare.log`
- `delete-only-run.log`
- `update-only-run.log`
- `delete-only-candidate.perf.data`
- `perf-report-nochildren-head.txt`
- `candidate.diff`

Current baseline from
`tests/artifacts/perf/dml-mutation-current-cyangorge-20260505T2140Z/`:

| Shape | Baseline | Candidate | Result |
| --- | ---: | ---: | ---: |
| isolated compare FSQLite total | `595ms` | `565ms` | `1.05x` faster |
| isolated compare FSQLite UPDATE | `270ms` | `257ms` | `1.05x` faster |
| isolated compare FSQLite DELETE | `205ms` | `190ms` | `1.08x` faster |
| isolated DELETE-only | `1210ms` | `959ms` | `1.26x` faster |
| isolated UPDATE-only | `904ms` from older exact profile / `~899ms` current candidate | `899ms` | effectively flat to slightly better |

Candidate DELETE-only perf recheck measured
`<TransactionKind as TransactionHandle>::write_page_data` at about `22.62%`
self, down from the baseline `32.18%` sample, with DELETE at `970ms` in the
sampled run.

## Quick matrix result

Candidate quick matrix:
`tests/artifacts/perf/pager-write-data-replace-quick-cyangorge-20260505T2203Z/report.json`

Baseline quick matrix:
`tests/artifacts/perf/current-quick-suite-cyangorge-20260505T2130Z/report.json`

| Metric | Baseline | Candidate |
| --- | ---: | ---: |
| primary weighted score | `0.5895` | `0.5423` |
| average ratio | `1.1405x` | `1.0209x` |
| geomean ratio | `0.4667x` | `0.4273x` |
| p90 ratio | `2.9042x` | `2.5489x` |
| p99 ratio | `4.5815x` | `4.8932x` |
| write_bulk geomean | `2.7388x` | `2.4061x` |
| write_single geomean | `2.2259x` | `1.9909x` |
| mixed OLTP ratio | `0.2548x` | `0.2072x` |

The p99 regression came from a small absolute INSERT row
(`medium_6col`, 100 rows). A focused INSERT rerun preserved the broader INSERT
gain but still showed high variance on small rows:

- Insert rerun artifact:
  `tests/artifacts/perf/pager-write-data-replace-insert-rerun-cyangorge-20260505T2208Z/report.json`
- Insert avg ratio improved relative to the earlier gated insert baseline
  (`~2.51x` to `2.43x`), and geomean improved (`~2.42x` to `2.30x`).
- The largest target rows improved in FSQLite median:
  `large_10col` 10K single transaction `39.58ms -> 36.87ms`;
  record-size large 10K `39.68ms -> 37.07ms`.
- Small-row INSERT medians were mixed and high-CV; this patch should be
  rechecked if future work targets p99 small INSERT specifically.

## Decision

Keep. The patch directly targets the current DELETE profiler's largest symbol,
improves isolated DELETE materially, improves the full quick matrix primary
score, and improves both write-heavy category geomeans. The small-row p99
regression is recorded as a caveat rather than treated as a no-ship blocker
because it is high variance, small absolute latency, and outweighed by broad
write/mixed-suite improvement.
