## Latest follow-up

This section supersedes older verification notes below when they differ.

Current status:
- `HEAD` already contains the main `bd-3wop3.1.2` production changes, including the lane-staging e2e hardening in `6b363823` and the named verifier updates in `3bf3b5d4`.
- This follow-up commit only stabilizes the WAL lane-stager unit slice after the named verifier exposed global slot-counter leakage between tests.

This focused follow-up adds:
- a bead-local `clippy` cleanup in `crates/fsqlite-wal/src/parallel_wal.rs` by replacing `map_or(false, ...)` with `is_some_and(...)`
- deterministic test isolation for `test_lane_stager_reuses_lanes_after_worker_churn()` by serializing the test with a static lock and resetting the per-core slot counter before the assertion

Constraints held:
- `concurrent_mode_default` remains `true`
- no `unsafe_code`
- no Tokio ecosystem
- manual edits only

Verification:
- `bash -n scripts/verify_d1_parallel_wal_staging.sh` passed.
- `cargo test -p fsqlite-wal lane_stager -- --nocapture` passed.
- `scripts/verify_d1_parallel_wal_staging.sh` passed and wrote artifacts under `artifacts/bd-3wop3.1.2/bd-3wop3.1.2-20260411T001142Z/`.
- `rustfmt --edition 2024 crates/fsqlite-wal/src/parallel_wal.rs` passed.

Known blockers outside this focused change:
- `cargo fmt --check` fails on an unrelated local formatting delta in `crates/fsqlite-core/src/connection.rs:88914`.
- `cargo check --workspace --all-targets` fails in `crates/fsqlite-e2e/src/perf_runner.rs:4715` because `HotPathProfileSnapshot` is initialized without the new `window_func_partitions_total` field; the same run also emitted an unrelated `dead_code` warning for `crates/fsqlite-core/src/connection.rs:75084`.
- `cargo clippy --workspace --all-targets -- -D warnings` fails on unrelated existing `dead_code` in `crates/fsqlite-core/src/connection.rs:75084`.

## bd-3wop3.1.2 progress

Current status:
- Read `/data/projects/frankensqlite/AGENTS.md` and `br show bd-3wop3.1.2`.
- Traced the existing lane-local staging implementation in `crates/fsqlite-pager/src/pager.rs`, `crates/fsqlite-wal/src/group_commit.rs`, `crates/fsqlite-wal/src/per_core_buffer.rs`, and `crates/fsqlite-wal/src/parallel_wal.rs`.
- Confirmed the production path already stages prepared WAL batches per lane, logs `fsqlite::wal::lane_staging` events, and routes `auto`, `conservative`, and `shadow_compare` through the pager commit path.

This focused commit adds:
- bead-scoped e2e coverage for auto/conservative/shadow-compare lane staging plus forced `lane_overflow` fallback
- structured-log validation for `wal_lane_id`, backlog, staged frame count, control mode, shadow verdict, compatibility selector, fallback reason, and elapsed time
- the named verification entrypoint `scripts/verify_d1_parallel_wal_staging.sh` with artifact-bundle output

Constraints held:
- `concurrent_mode_default` remains `true`
- no `unsafe_code`
- no Tokio ecosystem
- manual edits only

Verification:
- `cargo test -p fsqlite-pager bd_3wop3_1_2 -- --nocapture` passed locally.
- `cargo test -p fsqlite-e2e --test bd_3wop3_1_2_parallel_wal_staging -- --nocapture --test-threads=1` passed and validated auto/conservative/shadow-compare plus forced `lane_overflow`.
- `cargo check --workspace --all-targets` passed.
- `scripts/verify_d1_parallel_wal_staging.sh` passed and wrote artifacts under `artifacts/bd-3wop3.1.2/bd-3wop3.1.2-20260410T205914Z/`.
- `rustfmt --check crates/fsqlite-e2e/tests/bd_3wop3_1_2_parallel_wal_staging.rs` passed.
- `bash -n scripts/verify_d1_parallel_wal_staging.sh` passed.

Known pre-existing blockers outside this focused change:
- `cargo clippy --workspace --all-targets -- -D warnings` fails on existing `clippy::useless_conversion` in `crates/fsqlite-types/src/record.rs:1430`.

## Implementation slice: WAL-owned lane staging

Current status:
- Re-ran `br show bd-3wop3.1.2` to confirm the remaining acceptance criteria still required production lane-local staging, explicit ownership, telemetry, and conservative/shadow diagnostics in the ordinary append path.
- Moved the lane-staging ownership model into `crates/fsqlite-wal/src/parallel_wal.rs` by introducing a reusable `ParallelWalLaneStager<T>` plus `ParallelWalLaneBatch<T>` and `ParallelWalShadowVerdict`.
- Re-exported the WAL-owned staging API from `crates/fsqlite-wal/src/lib.rs` so the pager consumes the shared control surface instead of carrying a duplicate implementation.
- Refactored `crates/fsqlite-pager/src/pager.rs` so the group-commit queue delegates lane identity, backlog accounting, batch recording, and same-lane flush ordering to the WAL-owned stager while keeping the pager-specific test override hook.

This focused commit adds:
- production lane-local staging state in `fsqlite-wal` with explicit lane ownership, per-lane backlog tracking, stable batch identifiers, and same-lane FIFO flush validation
- a single env-driven control-surface resolver in `fsqlite-wal` for `auto`, conservative single-lane routing, and shadow-compare diagnostics
- WAL unit tests for lane identity stability, lane reuse after worker churn, conservative-mode collapse to one lane, and refusal to drain mismatched same-lane ordering
- pager integration updates so ordinary append staging no longer needs pager-owned centralized queue metadata just to build and enqueue prepared batches

Constraints held:
- `concurrent_mode_default` remains `true`
- no `unsafe_code`
- no Tokio ecosystem
- manual edits only

Verification:
- `cargo test -p fsqlite-wal lane_stager -- --nocapture` passed.
- `cargo test -p fsqlite-pager parallel_wal -- --nocapture` passed after serializing the lane-identity-sensitive pager tests.
- `cargo test -p fsqlite-pager pager::tests::test_parallel_wal_concurrent_writers_on_disjoint_lanes_commit_successfully -- --exact --nocapture --test-threads=1` passed.
- `cargo check --workspace --all-targets` passed.
- `cargo fmt --check` passed.
- `scripts/verify_d1_parallel_wal_staging.sh` passed and wrote artifacts under `artifacts/bd-3wop3.1.2/bd-3wop3.1.2-20260410T214605Z/`.
- `rustfmt --edition 2024 crates/fsqlite-wal/src/parallel_wal.rs crates/fsqlite-wal/src/lib.rs crates/fsqlite-pager/src/pager.rs` passed.

Known pre-existing blockers outside this focused change:
- `cargo clippy --workspace --all-targets -- -D warnings` still fails on the existing `clippy::useless_conversion` in `crates/fsqlite-types/src/record.rs:1430`.
