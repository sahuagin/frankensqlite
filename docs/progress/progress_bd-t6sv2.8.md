# bd-t6sv2.8 progress

Summary:
- Re-read `bd-t6sv2.8` and narrowed this commit to the missing SQL exposure slice for the already-implemented cache page diagnostics path.
- Wired the existing connection-local `CachePagesVtabFactory` into live `Connection` bootstrap so `fsqlite_cache_pages()` is actually reachable from SQL on opened connections.
- Added `fsqlite_cache_pages` to the table-function column registry so planner/executor lookup can resolve the virtual table shape.
- Strengthened the cache monitor tests from a shallow availability probe to two explicit invariants:
  - `fsqlite_cache_pages()` matches `pager.cache_page_snapshots()` row-for-row.
  - querying `fsqlite_cache_pages()` is observer-only and does not perturb cache metrics.
- Added `scripts/verify_cache_monitor.sh` as the focused verification gate requested by the bead.

Scope decisions:
- I did not try to complete the full remaining ARC/advisor/MVCC-overhead surface from the bead in this commit.
- I did not touch the unrelated in-flight work already present elsewhere in the repo or in other parts of `crates/fsqlite-core/src/connection.rs`; this slice stays narrowly focused on making the cache-pages diagnostics path live and test-backed.

Constraints held:
- `concurrent_mode_default` remains `true`
- no cache-behavior or eviction-policy changes
- no extra synchronization added to the cache monitor path
- no Tokio ecosystem
- manual edits only

Verification:
- `cargo test -p fsqlite-core test_fsqlite_cache_pages_table_function_ -- --nocapture`
  - Passed: 2 tests
    - `test_fsqlite_cache_pages_table_function_available_via_registry`
    - `test_fsqlite_cache_pages_table_function_is_read_only_observer`
- `cargo test -p fsqlite-core test_pragma_cache_ -- --nocapture`
  - Passed: 4 tests, including cache-stats and cache-reset PRAGMA coverage.
- `cargo test -p fsqlite-pager test_cache_efficiency_snapshot_matches_raw_cache_metrics -- --nocapture`
  - Passed: 1 test.
- `bash scripts/verify_cache_monitor.sh`
  - Passed: focused cache monitor verification gate completed cleanly.
- `rustfmt --edition 2024 --check crates/fsqlite-core/src/connection.rs`
  - Passed.
- `cargo fmt --check`
  - Passed.
- `cargo check --workspace --all-targets`
  - Passed.
- `cargo clippy --workspace --all-targets -- -D warnings`
  - Passed.
- `ubs crates/fsqlite-core/src/connection.rs scripts/verify_cache_monitor.sh`
  - UBS exited non-zero on broad pre-existing whole-file findings in `crates/fsqlite-core/src/connection.rs`; it also reported formatting, clippy, cargo check, and tests as clean for the current tree.
