## bd-8q9po progress

Summary:
- Focused slice landed in `crates/fsqlite-btree/src/cursor.rs`: the interior table/index descent hot path now reads child pointers directly from raw page bytes via `read_stack_entry_cell_pointer_inline()` and `read_interior_child_inline()` instead of bouncing through the heap `cell_pointers` cache for seek/descent decisions.
- Updated the hot call sites only: `move_to_leftmost_leaf`, `table_seek_for_insert`, `binary_search_table_interior`, `index_seek`, and `binary_search_index_interior`.
- Added two focused tests that clear `entry.cell_pointers` and still verify correct table/index interior descent, proving the new path stays on the page image.

Verification:
- `cargo fmt --check` after formatting `crates/fsqlite-btree/src/cursor.rs`
  - still fails, but only on pre-existing unrelated files:
    - `crates/fsqlite-core/src/wal_adapter.rs`
    - `crates/fsqlite-e2e/tests/correctness_transactions.rs`
    - `crates/fsqlite-pager/src/pager.rs`
    - `crates/fsqlite-types/src/value.rs`
    - `crates/fsqlite-wal/src/lib.rs`
- `cargo check --workspace --all-targets`
  - fails on pre-existing unrelated `fsqlite-pager` errors:
    - missing `TransactionFrameBatchContext` import at `crates/fsqlite-pager/src/pager.rs:5741`
    - missing `TransactionFrameBatchContext` import at `crates/fsqlite-pager/src/pager.rs:5759`
- `cargo clippy --workspace --all-targets -- -D warnings`
  - fails on pre-existing unrelated `clippy::useless_conversion` in `crates/fsqlite-types/src/record.rs:1430`
- targeted `cargo test -p fsqlite-btree ...`
  - blocked by the same pre-existing `fsqlite-pager` compile failure before `fsqlite-btree` tests can run

Constraints held:
- `concurrent_mode_default` untouched
- no `unsafe_code`
- no Tokio / asupersync changes
- manual edits only
