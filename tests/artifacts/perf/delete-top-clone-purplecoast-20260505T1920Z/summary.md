# Rejected DELETE top-stack clone candidate - 2026-05-05

Agent: PurpleCoast
Clean worktree: `/data/tmp/frankensqlite-purplecoast-delete-topclone` at `a50dc8ac`

## Candidate

`BtCursor::delete` cloned the top `StackEntry` before every delete. The candidate replaced that full clone with copied scalar metadata:

- `top.header.page_type.is_leaf()`
- `top.header.cell_count`
- `separator_repair_for_deleted_leaf_max(top)?`

The goal was to avoid a hot-path `PageData`/cell-pointer clone before direct table-leaf DELETE.

## Correctness

Focused B-tree delete tests passed in the clean worktree:

```bash
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-purplecoast-delete-topclone-target \
  cargo test -p fsqlite-btree cursor_delete -- --nocapture
```

Result: 7 passed, 0 failed.

## Measurement

Baseline from the committed exact profile:
`tests/artifacts/perf/dml-mutation-profile-purplecoast-20260505T1830Z/exact-isolated-compare.log`

| Metric | Baseline | Candidate |
| --- | ---: | ---: |
| FSQLite total | 580ms | 566ms |
| FSQLite UPDATE | 263ms | 252ms |
| FSQLite DELETE | 201ms | 202ms |
| Total ratio vs C SQLite | 3.20x | 3.16x |
| UPDATE ratio vs C SQLite | 2.75x | 2.71x |
| DELETE ratio vs C SQLite | 5.23x | 5.26x |

Candidate command:

```bash
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-purplecoast-delete-topclone-target \
  /data/tmp/frankensqlite-purplecoast-delete-topclone-target/release-perf/perf-update-delete \
  10000 250 both compare isolated
```

Delete-only confirmation:

```bash
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-purplecoast-delete-topclone-target \
  /data/tmp/frankensqlite-purplecoast-delete-topclone-target/release-perf/perf-update-delete \
  10000 1000 delete fsqlite isolated
```

Result: candidate DELETE was `1016ms` / `2033ns` per row, versus the committed exact-profile DELETE baseline of `1011ms` / `2024ns` per row.

## Decision

Rejected. Although total and UPDATE moved slightly in one isolated compare, the targeted DELETE row was flat/slightly worse in both the compare run and delete-only confirmation. Do not retry this top-stack clone removal as a standalone DELETE optimization.
