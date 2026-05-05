# Insert Journal-Memory Benchmark Candidate

- Candidate: changed the comprehensive benchmark pragma setup from
  `PRAGMA journal_mode = WAL` to `PRAGMA journal_mode = MEMORY` for both
  C SQLite and FrankenSQLite.
- Rationale: C SQLite on `:memory:` reports and keeps `journal_mode=memory`
  even after `PRAGMA journal_mode=WAL`, while FrankenSQLite honors WAL for
  private in-memory benchmark databases. This tested whether the insert gap was
  partly a benchmark-mode mismatch.
- Baseline:
  `tests/artifacts/perf/insert-profile-current-head-cyangorge-20260505T122449Z/report.json`.
- Candidate:
  `tests/artifacts/perf/insert-journal-memory-candidate-purplecoast-20260505T1450Z/report.json`.

## Insert-Only Result

The insert-only matrix looked positive:

| Metric | Baseline | Candidate |
| --- | ---: | ---: |
| Avg ratio | `2.4610x` | `2.3692x` |
| Geomean ratio | `2.3623x` | `2.2924x` |
| Weighted score | `1.6991` | `1.6703` |
| write_bulk geomean | `2.5153x` | `2.4349x` |
| write_single geomean | `1.4908x` | `1.4731x` |

Large-row absolute FrankenSQLite medians also improved:

| Row | Baseline F median | Candidate F median |
| --- | ---: | ---: |
| single transaction `large_10col` 10K | `36.165 ms` | `33.412 ms` |
| record-size `large_10col` 10K | `37.056 ms` | `34.171 ms` |

The candidate was not kept from this narrow result alone. The full quick
matrix rejected it in
`tests/artifacts/perf/full-quick-journal-memory-candidate-purplecoast-20260505T1515Z/summary.md`.
