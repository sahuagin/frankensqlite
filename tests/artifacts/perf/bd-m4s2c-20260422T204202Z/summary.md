# bd-m4s2c profile handoff attempt

Run id: `bd-m4s2c-20260422T204202Z`
Date: 2026-04-22
Agent: `cod4`
HEAD: `b11cd2323cf5d653d7ba1a5edddb11cc8a6a002b`

## Claim

`br update bd-m4s2c --status in_progress --assignee cod4 --force`

## Completed Evidence

`fingerprint.json` was captured with the profiling skill's `env_fingerprint.sh`.

The prior MT-MVCC run was copied into this artifact directory as the evidence for scenarios 3 and 4, per the assignment note:

- `mt-mvcc-scenarios-3-4.command`
- `mt-mvcc-scenarios-3-4.exit`
- `mt-mvcc-scenarios-3-4.stderr`

Extracted MT-MVCC rows from that run:

| threads | fsqlite_wps | sqlite_wps | throughput_ratio | fsqlite_ms_p50 | fsqlite_ms_p95 | fsqlite_ms_p99 | sqlite_ms_p50 | sqlite_ms_p95 | sqlite_ms_p99 | time_ratio |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 1 | 16533 | 47144 | 0.35x | 30.24 | 33.65 | 33.95 | 10.61 | 13.55 | 13.81 | 2.85x |
| 2 | 24347 | 55388 | 0.44x | 41.07 | 41.36 | 41.39 | 18.05 | 18.74 | 18.80 | 2.27x |
| 4 | 16610 | 69353 | 0.24x | 120.41 | 122.71 | 122.92 | 28.84 | 32.46 | 32.79 | 4.18x |
| 8 | 744 | 43995 | 0.02x | 5377.74 | 9721.16 | 10107.24 | 90.92 | 106.56 | 107.95 | 59.15x |

## Blockers

Scenario 1 profile and the 120-scenario `bd-0winn` avg_ratio refresh are blocked by `comprehensive-bench` compile failure:

```text
error: recursion limit reached while expanding `$crate::json_internal!`
    --> crates/fsqlite-e2e/src/bin/comprehensive_bench.rs:869:5
```

Evidence:

- `profile_build_plain.command`
- `profile_build_plain.exit` = `101`
- `profile_build_plain.stderr`

Scenario 2 `samply`/`heaptrack` profile collection is blocked on the current `fsqlite-core` compile failure when building `perf-update-delete` on a fresh RCH worker:

```text
error[E0599]: no method named `clone` found for struct `JoinTableSource` in the current scope
     --> crates/fsqlite-core/src/connection.rs:18208:54
```

Evidence:

- `cpu_update_script.command`
- `cpu_update_script.exit` = `101`
- `cpu_update_script.stderr`

Auxiliary checks:

- `btree_check.exit` = `0`
- `perf_update_delete_build.exit` = `0` on one RCH worker, but the profile-script build recompiled on a different worker and failed on the current `fsqlite-core` error above.
- `remote_which_samply.exit` = `0`
- `remote_which_heaptrack.exit` = `0`
- `remote_which_hyperfine.exit` = `0`

## Missing Required Outputs

The following required bd-m4s2c outputs were not produced because the current profiled binaries could not be built consistently across RCH workers:

- `baseline.json`
- `cpu-{scenario}.svg`
- `alloc-{scenario}.json`
- `offcpu-{scenario}.svg`
- current self-time rows for the 2026-04-19 top-7 hotspot comparison

No optimization changes were made.
