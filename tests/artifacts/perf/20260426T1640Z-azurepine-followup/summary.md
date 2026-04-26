# Mixed OLTP follow-up profile

Run ID: `20260426T1640Z-azurepine-followup`
Head: `25c9398f`

## Scenario

- Continuity benchmark: `comprehensive-bench --quick --filter mixed --no-html`
- FSQLite-only probe: `perf-update-delete 10000 50 both`
- Build: `release-perf`, `debug=line-tables-only`, `strip=false`, `RUSTFLAGS=-C force-frame-pointers=yes`
- Target dir: `/data/tmp/cargo-target-azurepine-followup`

## Current-head timing

| Artifact | C SQLite | FrankenSQLite |
| --- | ---: | ---: |
| `head_25c9398f_mixed_quick_1.json` | `224.09 ms` | `85.44 ms` |
| `head_25c9398f_mixed_quick_2.json` | `217.46 ms` | `85.18 ms` |
| `head_25c9398f_profiled.json` | `219.67 ms` | `82.92 ms` |

The previous mixed OLTP gains held after the later `b0f0d063` and `25c9398f`
commits. The combined profile is now dominated by the C SQLite reference half,
so direct FrankenSQLite target selection needs either an FSQLite-only harness or
a focused helper.

## FSQLite-only probe

`perf-update-delete 10000 50 both`:

- total: `1469 ms`
- populate: `591 ms`
- update: `457 ms`
- delete: `334 ms`
- per-row update: `9143 ns`
- per-row delete: `13375 ns`

Top actionable rows from `perf_update_delete_head_25c9398f_no_children.txt`:

| Rank | Location | Overhead | Evidence | Interpretation |
| --- | --- | ---: | --- | --- |
| 1 | `BtCursor<TransactionPageIo<dyn TransactionHandle>>::parse_cell_at` | `11.17%` | under `rowid` / `payload` from `reload_memdb_from_txn_with_mode` | `connection.rs` still has split rowid+payload scan loops in this reload path |
| 2 | `__memmove_avx_unaligned_erms` | `8.00%` | profile row | secondary copy cost, probably follows record/payload materialization |
| 3 | `_int_malloc` | `4.21%` | profile row | allocation cost in reload record parsing |
| 4 | `BtCursor<SharedTxnPageIo>::delete` | `3.23%` | profile row | delete leaf defrag/rebalance work |
| 5 | `execute_prepared_direct_simple_insert` | `2.99%` | profile row | populate side of helper |

## Opportunity matrix

| Target | Impact | Confidence | Effort | Score | Status |
| --- | ---: | ---: | ---: | ---: | --- |
| Replace split `cursor.rowid(cx)?` + `cursor.payload(cx)?` loops in `reload_memdb_from_txn_with_mode` with `cursor.rowid_and_payload_cow(cx)?` | 4 | 5 | 2 | 10.0 | Blocked by `AzureGorge` reservation on `crates/fsqlite-core/src/connection.rs` |
| Investigate delete defrag sort/cell-size hot path in `BtCursor::delete` | 2 | 3 | 3 | 2.0 | Blocked by `MagentaBeaver` reservation on `crates/fsqlite-btree/src/cursor.rs` |

## Handoff

The best next code slice is in `crates/fsqlite-core/src/connection.rs`:

- `reload_memdb_from_txn_with_mode` has two scan loops that still read rowid and
  payload separately.
- `reload_memdb_rows_from_txn_preserving_schema` already uses the desired fused
  shape: `let (rowid, payload) = cursor.rowid_and_payload_cow(cx)?;`.
- The expected behavior proof is isomorphic: one current-cell parse instead of
  two accessors; same rowid, same payload bytes, same parse and schema logic.

No source files were edited in this pass because both code surfaces with
score-qualified opportunities were exclusively reserved by other agents.
