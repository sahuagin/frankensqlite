# Current-Head Mixed OLTP Optimization Pass

Run date: 2026-04-25
Agent: AzurePine
Baseline HEAD: `13ec846c` (`perf(wal): avoid consolidation snapshot in group commit scheduler`)
Verified current HEAD: `5dc7c13f` (`perf(btree): fast-path local leaf table row payload reads`)
Workload: `comprehensive-bench --quick --filter mixed --no-html`
Build: `release-perf`, debug symbols enabled, frame pointers forced

## Baseline

| Artifact | C median | F median | F/C ratio | F CV |
|---|---:|---:|---:|---:|
| `baseline_head_13ec846c.json` | 235.16 ms | 204.18 ms | 0.8683 | 2.15% |
| `current_head_13ec846c_profiled.json` | 250.22 ms | 199.97 ms | 0.7992 | 2.26% |

Top current-head profile entries from `perf_current_head_13ec846c_no_children.txt`:

| Rank | Symbol | Self time |
|---:|---|---:|
| 1 | `sqlite3VdbeExec` | 33.44% |
| 2 | `Connection::retained_autocommit_count_star_sum_row_in_txn` | 7.29% |
| 3 | `BtCursor<SharedTxnPageIo>::parse_cell_at` | 4.28% |
| 4 | `BtCursor<TransactionPageIo>::rowid_and_payload_cow` | 2.53% |
| 5 | `SimpleTransaction::commit_wal_group_commit_with_snapshot::{closure#3}` | 2.12% |

## Rejected Candidate 1: Omitted Rowid-Alias Projection

Hypothesis: `SUM(score)` on `bench(id INTEGER PRIMARY KEY, name TEXT, score INTEGER)` falls back to full row decode because the stored record omits the rowid alias column. Mapping logical column index to stored column index should avoid decoding `name`.

Decision: rejected on current HEAD.

| Artifact | C median | F median | F/C ratio | F CV |
|---|---:|---:|---:|---:|
| `after_alias_projection_fastpath_head_13ec846c.json` | 235.28 ms | 205.20 ms | 0.8722 | 3.13% |
| `after_alias_projection_fastpath_head_13ec846c_repeat2.json` | 237.68 ms | 200.70 ms | 0.8444 | 2.04% |
| `after_one_pass_alias_projection_head_13ec846c.json` | 251.49 ms | 214.46 ms | 0.8528 | 1.89% |
| `after_one_pass_alias_projection_head_13ec846c_repeat2.json` | 258.90 ms | 228.52 ms | 0.8826 | 3.26% |

Reason: the double-parse version averaged only about a 0.6% absolute F improvement and one-pass rewrite regressed repeat measurements. Both stayed under the keep threshold, so the code was rolled back.

Isomorphism proof: the experiment only remapped projected column offsets around an omitted INTEGER PRIMARY KEY alias and preserved full-record alias validation; focused retained COUNT/SUM tests passed before rollback.

## Rejected Candidate 2: Manual Integer Decode Assembly

Hypothesis: replacing `try_into().unwrap()` and temporary sign-extension buffers in `decode_big_endian_signed` with fixed byte assembly would reduce scalar decode cost.

Decision: rejected on current HEAD.

| Artifact | C median | F median | F/C ratio | F CV |
|---|---:|---:|---:|---:|
| `after_manual_integer_decode_head_13ec846c.json` | 223.65 ms | 203.18 ms | 0.9085 | 1.24% |
| `after_manual_integer_decode_head_13ec846c_repeat2.json` | 222.00 ms | 201.89 ms | 0.9094 | 1.81% |

Reason: absolute F movement was under 1% and normalized F/C ratio worsened. The code was rolled back.

Isomorphism proof: direct sign-extension boundary tests and `integer_size_boundaries` passed before rollback.

## Handoff Target

While this pass was running, `5dc7c13f` landed a B-tree change that adds a
local leaf-table fast path for `rowid_and_payload_cow`. I verified it rather
than producing a redundant patch.

| Artifact | C median | F median | F/C ratio | F CV |
|---|---:|---:|---:|---:|
| `current_head_5dc7c13f.json` | 215.91 ms | 193.30 ms | 0.8953 | 1.17% |
| `current_head_5dc7c13f_repeat2.json` | 216.90 ms | 195.46 ms | 0.9011 | 1.21% |

Compared with `baseline_head_13ec846c.json`, FrankenSQLite median improved
from 204.18 ms to 193.30/195.46 ms, a roughly 4.3-5.3% absolute F-time win.

Verification for `5dc7c13f`:

- `cargo fmt --check`
- `env CARGO_TARGET_DIR=/data/tmp/cargo-target-azurepine-record cargo test -p fsqlite-btree test_rowid_and_payload_cow_reads_local_leaf_table_payload -- --nocapture`
- `env CARGO_TARGET_DIR=/data/tmp/cargo-target-azurepine-record cargo check -p fsqlite-btree --all-targets`
- `env CARGO_TARGET_DIR=/data/tmp/cargo-target-azurepine-record cargo clippy -p fsqlite-btree --all-targets -- -D warnings`

## Rejected Candidate 3: Rowid-Only Local Leaf Fast Path

Hypothesis: retained dirty-overlay range counting calls `cursor.rowid(cx)?`;
reading only the rowid varint should avoid full `parse_cell_at` work for local
leaf-table cells.

Decision: rejected on current HEAD.

| Artifact | C median | F median | F/C ratio | F CV |
|---|---:|---:|---:|---:|
| `after_rowid_only_fastpath_head_5dc7c13f.json` | 224.98 ms | 190.20 ms | 0.8454 | 2.28% |
| `after_rowid_only_fastpath_head_5dc7c13f_repeat2.json` | 233.62 ms | 190.03 ms | 0.8134 | 2.74% |

Reason: F median improved, but the two-run average was only about 2.2% faster
than the `5dc7c13f` baseline and stayed below the keep threshold. The code was
rolled back.

## Accepted Candidate: Local Leaf Payload Prefix Fast Path

Hypothesis: `ensure_storage_cursor_row_layout` calls
`payload_prefix_into` for each new storage row. On local leaf-table cells,
the existing `rowid_and_payload_cow` shortcut already proves we can read the
payload span directly, so a prefix-specific fast path can avoid
`parse_cell_at` while preserving overflow/index/interior fallback behavior.

Decision: accepted.

| Artifact | C median | F median | F/C ratio | F CV |
|---|---:|---:|---:|---:|
| `after_prefix_fastpath_clean_worktree_head_5dc7c13f.json` | 231.71 ms | 186.02 ms | 0.8028 | 2.12% |
| `after_prefix_fastpath_clean_worktree_head_5dc7c13f_repeat2.json` | 232.15 ms | 188.54 ms | 0.8121 | 1.96% |

Compared with the clean `5dc7c13f` baseline average (194.38 ms), the prefix
fast path average (187.28 ms) is roughly 3.65% faster.

Verification note: the shared worktree had unrelated dirty parser/core files
that failed the release-perf build (`KwReal`, `KwText`, `KwInteger` missing in
`TokenKind`). The accepted candidate was therefore measured and checked in a
detached clean worktree at `/data/tmp/frankensqlite-azurepine-prefix-fastpath`
based on `5dc7c13f` with only `crates/fsqlite-btree/src/cursor.rs` modified.
