# Connection Pooling for FrankenSQLite

FrankenSQLite is not stock SQLite. Its MVCC model can use multiple writer
connections productively, so a single shared writer connection is not the safe
default. If your workload has real concurrent write demand, collapsing
everything onto one connection recreates the SQLite bottleneck this project is
built to remove.

## Start Here

Use this sizing rule first:

`pool_size ~= min(cpu_cores, write_concurrency_needed)`

Then validate with the observability helpers:

- `validate_connection_pool()` for anti-pattern detection
- `simulate_connection_pool()` for a deterministic candidate sweep across a few
  pool sizes before you change production settings

The validator looks for:

- single-connection writer serialization
- over-pooling that mostly adds snapshots and idle churn
- stale idle connections that retain snapshots or transactions
- rapid connect/disconnect thrashing
- hot loops that still prepare statements per query

## Practical Defaults

For FrankenSQLite pools:

- prefer multiple writer connections when the workload has concurrent writes
- start with `min(cpu_cores, concurrent_writers)`
- cap read-heavy pools more aggressively than write-heavy pools
- recycle idle connections before they become long-lived snapshot holders
- prepare hot statements per connection instead of repreparing in tight loops

For stock SQLite you might often centralize writes onto one connection. Do not
carry that pattern over unchanged here.

## Runtime Snapshot

Use `PRAGMA fsqlite.connection_stats;` from any connection attached to the same
database path to inspect the live shared pool state that FrankenSQLite can see
inside this process:

```sql
PRAGMA fsqlite.connection_stats;
PRAGMA fsqlite_connection_stats;
```

The PRAGMA reports:

- `pool_size_estimate` and `open_connections` for the currently tracked pool
- `idle_connections` and `active_transactions` so you can spot stuck snapshot
  holders quickly
- `connection_age_max_ms` and `idle_ms_max` to distinguish healthy reuse from
  long-idle leak patterns
- `queries_executed_total`, `prepare_calls_total`, and
  `transactions_started_total` for the shared workload
- `current_connection_*` fields for the connection issuing the PRAGMA, which
  makes it easier to correlate a local handle with the shared aggregate view

This is intentionally a lightweight live diagnostic surface, not a replacement
for `validate_connection_pool()` or `simulate_connection_pool()`. Use the
PRAGMA to capture raw pool behavior, then feed representative samples into the
observability helpers when you want recommendations.

## Validator Example

This example matches the compile-checked API shape used by the observability
crate:

```rust
use fsqlite_observability::{
    ConnectionLifecycleSnapshot, ConnectionPoolTelemetrySample,
    ConnectionPoolWorkloadProfile, validate_connection_pool,
};

let sample = ConnectionPoolTelemetrySample {
    workload_profile: ConnectionPoolWorkloadProfile::WriteHeavy,
    cpu_cores: 4,
    configured_pool_size: 4,
    observed_active_connections: 4,
    peak_concurrent_checkout_requests: 4,
    concurrent_writers: 4,
    connect_events: 4,
    disconnect_events: 4,
    measurement_window_ms: 60_000,
    connections: vec![
        ConnectionLifecycleSnapshot {
            connection_id: 1,
            age_ms: 20_000,
            idle_ms: 200,
            open_transactions: 0,
            active_snapshot_age_ms: None,
            queries_executed: 240,
            prepare_calls: 8,
        },
        ConnectionLifecycleSnapshot {
            connection_id: 2,
            age_ms: 20_000,
            idle_ms: 250,
            open_transactions: 0,
            active_snapshot_age_ms: None,
            queries_executed: 230,
            prepare_calls: 8,
        },
    ],
};

let report = validate_connection_pool(&sample);
assert_eq!(report.recommendation.recommended_pool_size, 4);
assert!(report.findings.is_empty());
```

## Simulator Example

Use the simulator to compare a few candidate pool sizes before changing
production settings. The simulator is deterministic and heuristic-driven, so it
is useful for guidance and regression tests, not as a replacement for real
benchmarking:

```rust
use fsqlite_observability::{
    ConnectionLifecycleSnapshot, ConnectionPoolTelemetrySample,
    ConnectionPoolWorkloadProfile, simulate_connection_pool,
};

let sample = ConnectionPoolTelemetrySample {
    workload_profile: ConnectionPoolWorkloadProfile::WriteHeavy,
    cpu_cores: 4,
    configured_pool_size: 1,
    observed_active_connections: 1,
    peak_concurrent_checkout_requests: 4,
    concurrent_writers: 4,
    connect_events: 4,
    disconnect_events: 4,
    measurement_window_ms: 60_000,
    connections: vec![ConnectionLifecycleSnapshot {
        connection_id: 1,
        age_ms: 10_000,
        idle_ms: 100,
        open_transactions: 0,
        active_snapshot_age_ms: None,
        queries_executed: 400,
        prepare_calls: 8,
    }],
};

let simulation = simulate_connection_pool(&sample, &[1, 2, 4, 8]);
assert_eq!(simulation.recommended_pool_size, 4);
assert!(
    simulation.point_for_pool_size(4).unwrap().throughput_score
        > simulation.point_for_pool_size(1).unwrap().throughput_score
);
```

## Library Integration Notes

These are the policy settings to carry into common Rust pool wrappers:

| Pool wrapper | What to set |
|---|---|
| `sqlx::Pool` | Start `max_connections` near the validator recommendation. Reap idle connections aggressively enough that old snapshots do not linger. |
| `r2d2` | Keep `max_size` aligned with write parallelism, not request fan-out. Use connection customizers to pre-prepare hot statements when possible. |
| `deadpool` | Favor a moderate `max_size` plus explicit statement caching per connection. Avoid very large idle pools for read-heavy services. |
| `bb8` | Size for useful MVCC write concurrency, then tune `max_lifetime` / idle timeouts to recycle stale snapshot holders. |

The important part is the policy, not the wrapper: do not default to a
single-connection writer funnel unless the workload is genuinely single-threaded.

## Common Findings

| Finding | Interpretation | Fix |
|---|---|---|
| `SingleConnectionSerializedWriters` | Multiple writers want concurrency but the pool exposes one connection. | Increase pool size above `1`. |
| `OverPooling` | The pool is much larger than useful observed parallelism. | Shrink toward the recommendation and cut idle churn. |
| `StaleIdleSnapshot` | Idle connections are holding transactions or old snapshots. | Reduce idle timeout / max age and close leaked handles. |
| `ConnectionThrashing` | Connections are created and destroyed too often. | Reuse pooled connections instead of recreating per request. |
| `UnpreparedHotLoop` | Statement preparation is still happening inside hot loops. | Prepare once per connection or enable statement caching. |

## Verification

Use the bead-specific verifier:

```bash
scripts/verify_pool_advisor.sh
scripts/verify_pool_advisor.sh --json
```

It runs the connection-pool validator tests, simulator tests, doc tests, and a
content check that this guide still covers the required MVCC guidance, the
`PRAGMA fsqlite.connection_stats` workflow, and the common Rust pool wrappers.
