## bd-udl9m progress

Summary:
- Read `/data/projects/frankensqlite/AGENTS.md` and kept the hard constraints in force: `concurrent_mode_default` stays `true`, no `unsafe`, no Tokio ecosystem, manual edits only, no file deletion.
- Re-read `br show bd-udl9m` and traced the sequential insert path through `fsqlite-core`, `fsqlite-vdbe`, and the prepared/direct-insert dispatch.
- Reapplied the missing generic DML fix in `crates/fsqlite-core/src/connection.rs`: `execute_table_program_with_cx` now lets DML executions retain storage cursors by threading `allow_retained_cursor_reuse = invalidate_memdb_count_shortcuts_on_success` into `execute_table_program_with_db(...)` instead of hard-coding `false`.
- Added a regression test in `crates/fsqlite-core/tests/v2_superinstruction_tests.rs` that targets repeated sequential ad-hoc `INSERT` statements on the reusable table-program lane rather than the direct compiled insert lane.

Verification:
- `cargo check --workspace --all-targets`
  - Passed on 2026-04-10.
- `cargo clippy --workspace --all-targets -- -D warnings`
  - Passed on 2026-04-10.
- `cargo fmt --check`
  - Blocked by pre-existing unrelated formatting drift in other worktree files:
    - `crates/fsqlite-core/src/wal_adapter.rs`
    - `crates/fsqlite-e2e/tests/correctness_transactions.rs`
    - `crates/fsqlite-mvcc/src/core_types.rs`
    - `crates/fsqlite-mvcc/src/write_coordinator.rs`
    - `crates/fsqlite-pager/src/pager.rs`
    - `crates/fsqlite-types/src/value.rs`
    - `crates/fsqlite-wal/src/lib.rs`
- `timeout 180s cargo test -p fsqlite-core --test v2_superinstruction_tests test_v2_plain_execute_sequential_inserts_keep_append_path_hot_across_statements -- --nocapture`
  - Blocked before reaching the bd-udl9m test body by a pre-existing unrelated `fsqlite-pager` test-build failure:
    - `crates/fsqlite-pager/src/pager.rs`: unused import `TransactionFrameBatchContext`
    - `crates/fsqlite-pager/src/pager.rs`: missing `ParallelWalFallbackReason::ControllerCalibrationStale`

Constraints held:
- `concurrent_mode_default` remains `true`
- no `unsafe_code`
- no Tokio ecosystem
- manual edits only
