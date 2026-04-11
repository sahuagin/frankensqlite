# bd-t6sv2.10 progress

Summary:
- Finished the remaining core-side slice that the earlier observability-only work left open:
  - added shared connection lifecycle telemetry in `crates/fsqlite-core/src/connection.rs`
  - exposed `PRAGMA fsqlite.connection_stats` / `PRAGMA fsqlite_connection_stats`
  - kept the tracker lightweight by using per-connection atomics for hot-path updates instead of a global statement-time mutex
- Retained the existing validator / simulator / docs surface in `fsqlite-observability` and connected the docs to the new runtime PRAGMA workflow.
- Extended the bead verifier so it now checks the core PRAGMA tests in addition to the observability validator/simulator coverage.

Scope decisions:
- I left unrelated dirty worktree files alone (`crates/fsqlite-ext-fts5/src/lib.rs`, planner artifacts, heaptrack archives, other progress notes).
- I did not change MVCC defaults or introduce connection-level write serialization. `BEGIN` promotion remains driven by `concurrent_mode_default = true`.

Constraints held:
- `concurrent_mode_default` remains `true`
- no `unsafe_code`
- no Tokio ecosystem
- manual edits only

Verification:
- `cargo test -p fsqlite-core connection_stats -- --nocapture`
  - Passed: 2 new core PRAGMA tests covering shared pool lifecycle and disconnect handling.
- `rch exec -- cargo clippy -p fsqlite-core --all-targets -- -D warnings`
  - Passed.
- `cargo test -p fsqlite-observability connection_pool::tests:: -- --nocapture`
  - Passed: 14 tests.
- `cargo test -p fsqlite-observability --doc -- --nocapture`
  - Passed.
- `cargo clippy -p fsqlite-observability --all-targets --no-deps -- -D warnings`
  - Passed.
- `bash scripts/verify_pool_advisor.sh --json`
  - Passed with `verdict=pass` and `recommendation_accuracy_pct=100`.
  - Report written to `test-results/bd-t6sv2.10-pool-advisor-verify.json`.
- `rch exec -- cargo check --workspace --all-targets`
  - Passed.
- `rch exec -- cargo clippy --workspace --all-targets -- -D warnings`
  - Blocked by an unrelated pre-existing workspace lint in `crates/fsqlite-vdbe/src/codegen.rs:25214`:
    - `clippy::replace_box` on an existing `Box::new(Expr::Collate { ... })`
- `cargo fmt --check`
  - Passed.
