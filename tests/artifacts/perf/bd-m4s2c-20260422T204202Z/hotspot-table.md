# Hotspot Table

Current-HEAD CPU/allocation profiles were not collected.

| Rank | Location | Metric | Value | Category | Evidence |
|---:|---|---:|---:|---|---|
| 1 | build blocker: `comprehensive_bench.rs` JSON schema macro | compile exit | 101 | blocker | `profile_build_plain.stderr` |
| 2 | build blocker: `JoinTableSource::clone` missing | compile exit | 101 | blocker | `cpu_update_script.stderr` |
| 3 | MT-MVCC 8-thread degradation | time ratio | 59.15x | throughput | `mt-mvcc-scenarios-3-4.stderr` |

2026-04-19 hotspot list to compare after the build blockers land:

| Prior Rank | Prior Hotspot | Prior Self Time |
|---:|---|---:|
| 1 | `memcpy` | 6.77% |
| 2 | `CellRef::parse` | 4.80% |
| 3 | `execute_prepared_direct_simple_insert` | 4.24% |
| 4 | `_int_malloc` | 3.96% |
| 5 | `Arc::make_mut` | 1.77% |
| 6 | `Vec::finish_grow` | 1.20% |
| 7 | `WalChecksumTransform` | 0.79% |
