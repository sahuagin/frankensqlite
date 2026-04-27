## bd-2wt.2 Progress

Date: 2026-04-10

Scope landed in this slice:
- Added window-function executor observability in `crates/fsqlite-core/src/connection.rs`.
- Emitted a `window_eval` DEBUG span with `func_name`, `partition_count`, and `frame_type`.
- Added the `fsqlite_window_func_partitions_total` hot-path counter surface via `HotPathProfileSnapshot`.
- Added regression tests covering the new partition counter and the `window_eval` span fields.

Date: 2026-04-11

Scope landed in this slice:
- Added a deterministic file-backed E2E parity scenario in `crates/fsqlite-e2e/tests/bd_2wt_2_aggregate_window_engine.rs`.
- Covered grouped aggregates with `FILTER`, `DISTINCT`, ordered `GROUP_CONCAT`, and `STRING_AGG`.
- Covered window ranking/navigation functions plus `ROWS`, `RANGE`, `GROUPS`, `EXCLUDE CURRENT ROW`, `FILTER`, and `NTH_VALUE` semantics.
- Added replay/artifact support with `run_id`, `trace_id`, `scenario_id`, and `replay_command` so the scenario can archive operator-friendly evidence for CI or manual reruns.
- Added hot-path metric validation asserting that `window_func_partitions_total` increments during the end-to-end scenario.

Notes:
- This slice stays within the concurrent-writer constraints; no concurrency defaults or file-lock behavior were changed.
- Verification also required finishing an in-progress `fsqlite-vdbe` range-scan refactor so the workspace would compile and the bd-2wt.2 tests could run.

Notes:
- No concurrency defaults were changed. `concurrent_mode_default` remains untouched.
- No `unsafe` code was introduced.
- No new async runtime usage was added.

Verification planned for this slice:
- `cargo test -p fsqlite-e2e --test bd_2wt_2_aggregate_window_engine -- --nocapture`
- Focused window tests in `fsqlite-core`
- `cargo fmt --check`
- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`

Date: 2026-04-11

Additional scope landed in this slice:
- Fixed explicit `RANGE` frame boundary evaluation for descending `ORDER BY` windows in the connection-level window executor.
- Fixed non-numeric `RANGE <expr> PRECEDING/FOLLOWING` behavior so peer-group boundaries follow SQLite semantics instead of collapsing text/blob values into numeric zero.
- Added SQLite-oracle regression coverage for descending numeric `RANGE` offsets and text-ordered `RANGE` frames in `crates/fsqlite-core/src/connection.rs`.
