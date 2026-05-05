# DELETE stack-entry reuse A/B - 2026-05-05

Agent: ProudAnchor
Worktree: `/data/tmp/frankensqlite-proudanchor-stackdelete-20260505`
Baseline worktree: `/data/tmp/frankensqlite-proudanchor-baseline-20260505`
Baseline commit: `4dcf22bb`
Candidate file: `crates/fsqlite-btree/src/cursor.rs`
Patch SHA-256: `e62ab5ca63572f3530765b226f43a7f61d68c15ab516b9a09b65a7fdc63aa9e6`

## Profile basis

The current isolated DML profile put DELETE at roughly `5.23x` C SQLite and
showed repeated cost in:

- `TransactionKind::write_page_data`: `20.06%` self.
- `table_seek_for_insert` before DELETE: about `12.9%` children.
- `read_cell_pointers_into`: `6.45%` self.
- `__memmove_avx_unaligned_erms`: `7.13%` self.

Prior freeblock, sort-threshold, staged-page publication split, and top-stack
clone candidates are already rejected in the negative-results ledger, so this
candidate deliberately does not change the accepted eager-defrag layout or
freeblock policy.

## Candidate

`remove_table_cell_from_leaf_deferred` already has the target leaf loaded in the
cursor stack after `table_move_to`. The candidate reuses that stack entry's page
image, parsed header, and cell-pointer vector instead of rereading the same page
and reparsing the pointer array. After writing the mutated page, it rebuilds the
top stack entry from the just-mutated image instead of calling
`reload_page_fresh`.

This keeps the final table-leaf layout identical: eager compact defrag, no
freeblock chain, no fragmented bytes.

## Correctness smoke

```bash
cargo fmt --check
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-proudanchor-stackdelete-target \
  cargo test -p fsqlite-btree cursor_delete -- --nocapture
```

Result: `7` focused cursor delete tests passed.

## Measurement

Builds:

```bash
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-proudanchor-baseline-target \
  cargo build --profile release-perf -p fsqlite-e2e --bin perf-update-delete

env CARGO_TARGET_DIR=/data/tmp/frankensqlite-proudanchor-stackdelete-target \
  cargo build --profile release-perf -p fsqlite-e2e --bin perf-update-delete
```

Adjacent hyperfine:

```bash
hyperfine --warmup 2 --runs 12 \
  --export-json tests/artifacts/perf/delete-stack-entry-reuse-proudanchor-20260505T2005Z/hyperfine-v2.json \
  --command-name baseline-delete '/data/tmp/frankensqlite-proudanchor-baseline-target/release-perf/perf-update-delete 10000 1000 delete fsqlite isolated' \
  --command-name candidate-v2-delete '/data/tmp/frankensqlite-proudanchor-stackdelete-target/release-perf/perf-update-delete 10000 1000 delete fsqlite isolated' \
  --command-name baseline-both '/data/tmp/frankensqlite-proudanchor-baseline-target/release-perf/perf-update-delete 10000 250 both fsqlite isolated' \
  --command-name candidate-v2-both '/data/tmp/frankensqlite-proudanchor-stackdelete-target/release-perf/perf-update-delete 10000 250 both fsqlite isolated'
```

| Scenario | Baseline mean | Candidate mean | Result |
| --- | ---: | ---: | ---: |
| DELETE-only isolated | `1.6020s +/- 0.0177s` | `1.3124s +/- 0.0160s` | `1.22x` faster |
| UPDATE+DELETE isolated | `600.8ms +/- 4.7ms` | `580.4ms +/- 6.7ms` | `1.04x` faster |

One-shot compare logs:

| Metric | Baseline | Candidate |
| --- | ---: | ---: |
| FSQLite total | `586ms` | `578ms` |
| FSQLite UPDATE | `257ms` | `263ms` |
| FSQLite DELETE | `214ms` | `195ms` |
| DELETE ratio vs C SQLite | `5.84x` | `4.99x` |

## Decision

Keep. The target DELETE-only row moves materially and the mixed isolated row
also improves. The candidate is a one-lever B-tree change and does not revisit
the ledger-fenced freeblock/sort-threshold ideas.
