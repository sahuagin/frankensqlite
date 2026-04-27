# Direct INSERT Rowid-Alias Borrow Probe

## Scope

- Baseline commit: `b42e3236` (`perf: publish post snapshot opt-out update delete profile`)
- Scenario: `perf-update-delete 10000 50 both`
- Build profile: `release-perf` with frame pointers and line-table debug symbols
- Baseline worktree: `/data/tmp/frankensqlite-azurepine-direct-rowid-baseline-20260427`
- Candidate worktree: `/data/tmp/frankensqlite-azurepine-direct-rowid-candidate-20260427`

## Candidate

The candidate changed the compiled direct INSERT row builder so rowid-alias
literal/placeholder expressions borrowed the input value instead of first
materializing an owned `SqliteValue`.

The intended target was the benchmark-shaped prepared INSERT:

```sql
INSERT INTO bench VALUES (?1, ('user_' || ?1), (?1 * 0.137))
```

This was a single-lever test against the `Connection::execute_prepared_direct_simple_insert`
profile lane. The change preserved behavior in the focused direct INSERT test,
but the A/B benchmark rejected it.

## Behavior Proof

- Ordering preserved: yes. The row-building loop still visits columns in table
  order and only changes the rowid-alias placeholder/literal evaluation path.
- Tie-breaking unchanged: N/A.
- Floating-point unchanged: yes. The `(?1 * 0.137)` value path still uses the
  existing owned expression evaluator.
- Rowid semantics unchanged: yes. The candidate still called
  `coerce_explicit_rowid_value` and stored `SqliteValue::Null` for the IPK
  payload column.
- Focused test: `cargo test -p fsqlite-core test_prepared_direct_simple_insert_executes_inside_explicit_transaction -- --nocapture` passed.

## A/B Timing

Alternating run order was used to reduce scheduler and thermal drift.

| Run | Baseline total | Baseline populate | Baseline update | Baseline delete | Candidate total | Candidate populate | Candidate update | Candidate delete |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 855 ms | 414 ms | 272 ms | 129 ms | 872 ms | 412 ms | 277 ms | 141 ms |
| 2 | 877 ms | 413 ms | 281 ms | 141 ms | 855 ms | 412 ms | 266 ms | 135 ms |
| 3 | 854 ms | 406 ms | 268 ms | 138 ms | 872 ms | 418 ms | 274 ms | 138 ms |
| 4 | 856 ms | 406 ms | 270 ms | 139 ms | 884 ms | 429 ms | 273 ms | 138 ms |
| 5 | 858 ms | 412 ms | 265 ms | 139 ms | 853 ms | 409 ms | 263 ms | 138 ms |
| 6 | 865 ms | 417 ms | 269 ms | 138 ms | 878 ms | 419 ms | 276 ms | 139 ms |
| 7 | 859 ms | 410 ms | 270 ms | 137 ms | 857 ms | 421 ms | 265 ms | 131 ms |

Median total:

- Baseline: `858 ms`
- Candidate: `872 ms`
- Delta: `+14 ms`, about `1.6%` slower

Median populate:

- Baseline: `412 ms`
- Candidate: `418 ms`
- Delta: `+6 ms`, about `1.5%` slower

## Decision

Reject and roll back the candidate. The borrowed rowid-alias path was not a
measurable win for this workload, and the full benchmark median moved in the
wrong direction. The source change was not committed.

Next direct INSERT work should not retest this exact lever. Higher-confidence
lanes remain:

- B-tree append allocation/copy path under `try_append_table_leaf_payload_in_place_no_overflow`
- direct record construction into the B-tree destination when the cursor API can own the write
- delete defrag/cell-size work once the current B-tree reservation clears

## Commands

```bash
git worktree add --detach /data/tmp/frankensqlite-azurepine-direct-rowid-candidate-20260427 b42e3236
```

```bash
git worktree add --detach /data/tmp/frankensqlite-azurepine-direct-rowid-baseline-20260427 b42e3236
```

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-azurepine-20260427-direct-rowid cargo test -p fsqlite-core test_prepared_direct_simple_insert_executes_inside_explicit_transaction -- --nocapture
```

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-azurepine-20260427-direct-rowid-baseline CARGO_PROFILE_RELEASE_PERF_DEBUG=line-tables-only CARGO_PROFILE_RELEASE_PERF_STRIP=false RUSTFLAGS='-C force-frame-pointers=yes' cargo build --profile release-perf -p fsqlite-e2e --bin perf-update-delete
```

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-azurepine-20260427-direct-rowid-candidate CARGO_PROFILE_RELEASE_PERF_DEBUG=line-tables-only CARGO_PROFILE_RELEASE_PERF_STRIP=false RUSTFLAGS='-C force-frame-pointers=yes' cargo build --profile release-perf -p fsqlite-e2e --bin perf-update-delete
```

```bash
for run in 1 2 3 4 5 6 7; do printf 'baseline run=%s\n' "$run"; /data/tmp/cargo-target-azurepine-20260427-direct-rowid-baseline/release-perf/perf-update-delete 10000 50 both 2>&1; printf 'candidate run=%s\n' "$run"; /data/tmp/cargo-target-azurepine-20260427-direct-rowid-candidate/release-perf/perf-update-delete 10000 50 both 2>&1; done
```
