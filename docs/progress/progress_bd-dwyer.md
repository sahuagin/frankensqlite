Implemented a focused Track J test slice for lazy MemDB dirty tracking.

What changed:
- Added a VDBE unit test in `crates/fsqlite-vdbe/src/engine.rs` that writes to one table, scans a clean table, then reads the dirty table in the same statement. It verifies clean-table visibility is preserved, dirty rows remain visible through the storage-backed B-tree fallback, and only the written root page is marked dirty.
- Added a passing multi-table e2e test in `crates/fsqlite-e2e/tests/correctness_transactions.rs` that exercises explicit-transaction visibility across a dirty table and a clean table after a read boundary.
- Added an ignored e2e regression test documenting the currently failing path: after a clean-table read boundary, a later insert into the dirty table is not yet visible to `COUNT(*)` through the lazy MemDB compatibility path.

Validation:
- `rch exec -- cargo test -p fsqlite-vdbe test_lazy_dirty_clean_table_scan_then_dirty_table_fallback -- --nocapture` ✅
- `rch exec -- cargo test -p fsqlite-e2e --test correctness_transactions test_lazy_memdb_multi_table_clean_read_then_dirty_visibility -- --nocapture` ✅
- `cargo check --workspace --all-targets` ✅
- `cargo fmt --check` ✅
- `cargo clippy --workspace --all-targets -- -D warnings` ❌ unrelated existing failures:
  - `crates/fsqlite-pager/src/pager.rs:1980` `clippy::cloned_instead_of_copied`
  - `crates/fsqlite-pager/src/pager.rs:444` `clippy::significant_drop_in_scrutinee`
  - `crates/fsqlite-types/src/record.rs:1430` `clippy::useless_conversion`

Constraints held:
- `concurrent_mode_default` untouched
- no `unsafe`
- no Tokio/asupersync violations
- manual edits only
