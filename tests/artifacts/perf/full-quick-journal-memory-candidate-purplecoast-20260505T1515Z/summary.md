# Full Quick Journal-Memory Benchmark Candidate

- Candidate: changed the comprehensive benchmark pragma setup from
  `PRAGMA journal_mode = WAL` to `PRAGMA journal_mode = MEMORY` for both
  C SQLite and FrankenSQLite.
- Baseline:
  `tests/artifacts/perf/full-quick-current-head-cyangorge-20260505T122449Z/report.json`.
- Candidate:
  `tests/artifacts/perf/full-quick-journal-memory-candidate-purplecoast-20260505T1515Z/report.json`.
- Source diff: reverted after measurement.

## Full-Matrix Result

Rejected. The insert-only matrix improved, but the 93-scenario quick matrix
got worse on the primary weighted score and on the write/concurrency categories.

| Metric | Baseline | Candidate |
| --- | ---: | ---: |
| Franken faster / comparable / C faster | `58 / 0 / 35` | `56 / 2 / 35` |
| Avg ratio | `1.0270x` | `1.0691x` |
| Geomean ratio | `0.4467x` | `0.4596x` |
| Weighted score | `0.5658` | `0.5808` |
| write_bulk geomean | `2.3562x` | `2.4735x` |
| write_single geomean | `2.0563x` | `2.1667x` |
| concurrent_writers geomean | `1.1514x` | `1.1830x` |
| read_single geomean | `0.2596x` | `0.2525x` |

## Disposition

Do not retry the benchmark-only `journal_mode=MEMORY` switch as a standalone
fairness/performance correction. It can make the insert-only profile look
better, including absolute large-row FrankenSQLite medians, but it loses on the
end-to-end quick score that governs this performance campaign.
