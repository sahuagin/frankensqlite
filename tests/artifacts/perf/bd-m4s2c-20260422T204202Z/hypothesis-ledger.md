# Hypothesis Ledger

| Hypothesis | Verdict | Evidence |
|---|---|---|
| Scenario 1 INSERT 1000 small_3col can be profiled on current HEAD | blocked | `profile_build_plain.stderr` shows `comprehensive-bench` fails to compile with a `serde_json::json!` recursion-limit error. |
| Scenario 2 UPDATE 100/1000 can be profiled with `perf-update-delete` | blocked | `cpu_update_script.stderr` shows a current `fsqlite-core` compile failure: missing `Clone` for `JoinTableSource`. |
| RCH workers have required profiler tools | supports | `remote_which_samply.stdout`, `remote_which_heaptrack.stdout`, and `remote_which_hyperfine.stdout`. |
| MT-MVCC 8-thread path remains a serious scaling hotspot | supports | `mt-mvcc-scenarios-3-4.stderr` reports 8-thread FSQLite p50 5377.74 ms vs SQLite p50 90.92 ms, time ratio 59.15x. |
| 120-scenario avg_ratio can be refreshed on current HEAD | blocked | `profile_build_plain.stderr` shows `comprehensive-bench` cannot compile, so bd-0winn cannot run. |
