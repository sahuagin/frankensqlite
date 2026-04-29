# 2026-04-29 comprehensive insert profile result

## Scenario

- Benchmark: `comprehensive-bench --quick --filter insert --no-html`
- Baseline binary: release-perf with frame pointers and line-table debug info
- Target: remaining FrankenSQLite/C SQLite gap across insert scenarios
- Source revision before change: `1f87b0d5150ff801c309b17793f224f672955b6c`

## Profile finding

The baseline `perf report --stdio --no-children` showed a benchmark-owned
configuration gap, not a required engine semantics cost:

| Rank | Location | Baseline cost | Evidence |
| --- | --- | ---: | --- |
| 1 | `Connection::capture_time_travel_snapshot` via `MemTable::clone` / `SqliteValue::to_vec` | `_int_malloc` 4.91%; clone chain 3.35% under malloc | `perf-comprehensive-insert-flat.txt` |
| 2 | `__memmove_avx_unaligned_erms` | 7.71% | `perf-comprehensive-insert-flat.txt` |
| 3 | direct prepared insert execution | 2.66% | `perf-comprehensive-insert-flat.txt` |

The benchmark workloads do not issue `FOR SYSTEM_TIME` queries, and one
batched-insert scenario already disabled the optional in-memory snapshot ring
explicitly. The slow path survived because the common FrankenSQLite benchmark
PRAGMA setup did not apply that opt-out to every comprehensive benchmark
connection.

## One-lever change

`crates/fsqlite-e2e/src/bin/comprehensive_bench.rs` now uses a shared
`FSQLITE_BENCHMARK_PRAGMAS` list that includes:

```sql
PRAGMA fsqlite_capture_time_travel_snapshots=false;
```

This keeps the benchmark focused on SQLite-compatible query/write work instead
of optional time-travel snapshot cloning. It does not change `Connection`
defaults or `FOR SYSTEM_TIME` behavior.

## Before / after

| Metric | Before | After | Delta |
| --- | ---: | ---: | ---: |
| average ratio | 3.8931x | 3.1365x | -19.44% |
| geomean ratio | 3.6361x | 2.9051x | -20.10% |
| median ratio | 3.2661x | 2.5361x | -22.35% |
| p99 ratio | 7.1338x | 6.1670x | -13.55% |
| peak RSS | 93,336 KB | 45,024 KB | -51.76% |
| wall time | 4.46 s | 3.13 s | -29.82% |

Worst-row movement:

| Scenario | Before | After |
| --- | ---: | ---: |
| large_10col, 10K rows, record-size comparison | 7.1338x | 3.5916x |
| large_10col, 10K rows, single transaction | 7.0710x | 3.6815x |

## After profile

The after profile no longer contains `Connection::capture_time_travel_snapshot`,
`MemTable::clone`, or `SqliteValue::to_vec` in the sampled flat report. `_int_malloc`
fell from 4.91% to 2.15%.

Evidence:

- Baseline JSON: `comprehensive-insert.json`
- After JSON: `comprehensive-insert-after.json`
- Baseline time/RSS: `time-comprehensive-insert.txt`
- After time/RSS: `time-comprehensive-insert-after.txt`
- Baseline profile: `perf-comprehensive-insert-flat.txt`
- After profile: `perf-comprehensive-insert-after-flat.txt`

## Proof obligations

- Ordering preserved: yes. The benchmark still executes the same SQL workload in
  the same order.
- Tie-breaking unchanged: yes. No query ordering or planner logic changed.
- Floating-point: unchanged. Insert expressions and result collection are
  unchanged.
- Randomness: N/A. The benchmark scenarios are deterministic.
- Feature behavior: unchanged for the engine. Time-travel capture remains the
  default unless a connection explicitly sets the PRAGMA.
- Fallback: remove the PRAGMA from `FSQLITE_BENCHMARK_PRAGMAS` or run a
  time-travel-specific benchmark where snapshot retention is the measured
  feature.

## Applied skill mapping

- Profiling: ranked evidence before code change, same scenario before/after.
- Extreme optimization: one lever, score above threshold
  `(impact 4 * confidence 5) / effort 1 = 20`.
- Alien artifact: explicit proof obligations and fallback policy.
- Alien graveyard: profile-first constant-factor cleanup; avoid paying optional
  copy/allocation machinery on the hot path when the workload does not consume it.
