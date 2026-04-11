# bd-t6sv2.10 progress

Summary:
- Re-read `bd-t6sv2.10` and narrowed this commit to the pool validator / simulator / guidance slice that is already centered in `fsqlite-observability`.
- Added the missing deterministic `simulate_connection_pool()` API plus serializable simulation report types so the documented simulator surface now exists in the crate instead of only in `docs/connection-pooling.md`.
- Added focused tests for:
  - single-connection underprovisioning vs. multi-writer sizing
  - read-heavy pool capping
  - simulator determinism across repeated runs
  - docs-example API shapes for both validator and simulator flows
- Kept the docs aligned with the exported API and retained MVCC-specific guidance that multiple writer connections are the default recommendation for FrankenSQLite.

Scope decisions:
- I intentionally did not touch the currently dirty `crates/fsqlite-core/src/connection.rs` in this commit.
- The bead's core-side `PRAGMA fsqlite_connection_stats` / shared lifecycle tracking follow-up remains open work; this commit focuses on the best-practices and validator/simulator surface without mixing in unrelated pre-existing `connection.rs` changes.

Constraints held:
- `concurrent_mode_default` remains `true`
- no `unsafe_code`
- no Tokio ecosystem
- manual edits only

Verification:
- `cargo test -p fsqlite-observability connection_pool::tests:: -- --nocapture`
  - Passed: 14 tests, including the new simulator and docs-shape tests.
- `cargo test -p fsqlite-observability test_simulator -- --nocapture`
  - Passed: 3 simulator-focused tests.
- `cargo test -p fsqlite-observability test_docs_ -- --nocapture`
  - Passed: 2 docs-shape tests.
- `cargo test -p fsqlite-observability --doc -- --nocapture`
  - Passed: crate doc-test harness completed cleanly (0 doc tests in this crate).
- `cargo clippy -p fsqlite-observability --all-targets --no-deps -- -D warnings`
  - Passed after fixing one local `clippy::unnecessary_lazy_evaluations` finding in the simulator projection helper.
- `rustfmt --edition 2024 --check crates/fsqlite-observability/src/connection_pool.rs crates/fsqlite-observability/src/lib.rs`
  - Passed.
- `bash scripts/verify_pool_advisor.sh --json --no-rch`
  - Passed with `verdict=pass` and `recommendation_accuracy_pct=100`.
  - Report written to `test-results/bd-t6sv2.10-pool-advisor-verify.json`.
- `rch exec -- cargo check --workspace --all-targets`
  - Blocked by unrelated existing `fsqlite-pager` errors in `crates/fsqlite-pager/src/page_cache.rs`:
    - missing `record_access` on `PageBuf` at line 594
    - missing `mark_dirty` on `PageBuf` at line 601
    - missing `mark_clean` on `PageBuf` at line 608
- `rch exec -- cargo clippy --workspace --all-targets -- -D warnings`
  - Blocked by the same unrelated existing `fsqlite-pager` errors in `crates/fsqlite-pager/src/page_cache.rs`.
- `cargo fmt --check`
  - Blocked by unrelated formatting drift in:
    - `crates/fsqlite-harness/tests/bd_1sf8n_phase9_time_travel_gate.rs`
    - `crates/fsqlite-pager/src/page_cache.rs`
- `ubs crates/fsqlite-observability/src/connection_pool.rs crates/fsqlite-observability/src/lib.rs`
  - UBS exited non-zero on broad pre-existing findings in `crates/fsqlite-observability/src/lib.rs` (existing unwrap/panic/unsafe-pattern inventory and similar whole-file heuristics), not on the new `bd-t6sv2.10` simulator code.
