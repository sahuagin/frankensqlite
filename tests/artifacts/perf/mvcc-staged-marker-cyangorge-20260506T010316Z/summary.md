# MVCC Staged Marker Write Tracking

Agent: CyanGorge
Date: 2026-05-06T01:03:16Z
Build: `cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench`
Candidate: VDBE write paths mark MVCC staged writes without copying page payload bytes; pager remains the authoritative read-your-writes payload store.

## Artifacts

- `report.json` / `run.log`: `FSQLITE_BENCH_PROFILE_INSERT=1 ./.rch-target/release-perf/comprehensive-bench --quick --filter insert --json-out ... --no-html`
- `full-report.json` / `full-run.log`: `./.rch-target/release-perf/comprehensive-bench --quick --json-out ... --no-html`

## Baselines

- Insert baseline: `tests/artifacts/perf/insert-profile-head-20260506T004227Z-proudanchor/report.json`
- Full quick baseline: `tests/artifacts/perf/full-quick-head-20260506T003556Z-proudanchor/report.json`

## Insert-Only Result

| Metric | Baseline | Candidate |
| --- | ---: | ---: |
| average ratio | 2.151690790362746 | 2.0179518168867356 |
| geomean ratio | 1.935253424715502 | 1.8957118009468483 |
| p99 ratio | 5.361496373896971 | 3.8326365518580237 |
| weighted score | 1.7290413425198825 | 1.601047043604295 |
| write_bulk geomean | 1.9772351141343807 | 1.9577048321024224 |
| write_single geomean | 1.6534373051263505 | 1.4972341343961386 |

Row comparison against the insert baseline: 25/25 rows improved in FrankenSQLite median time by more than 1%; no row worsened by more than 1%.

## Full Quick Result

| Metric | Baseline | Candidate |
| --- | ---: | ---: |
| average ratio | 1.0073748490966477 | 0.960175655368824 |
| geomean ratio | 0.4480329227676782 | 0.417271829134275 |
| p99 ratio | 4.531495637964699 | 4.357727945002135 |
| weighted score | 0.5663061172267168 | 0.527782853930401 |
| write_bulk geomean | 2.0991297501684465 | 2.1288457639771763 |
| write_single geomean | 1.9422652293972142 | 1.939826238310555 |

Category comparison by FrankenSQLite median time against the full quick baseline:

| Category | Rows | Average F delta | Improved >1% | Worsened >1% |
| --- | ---: | ---: | ---: | ---: |
| concurrent_writers | 3 | -7.350503565779697% | 2 | 1 |
| mixed | 1 | -13.002562841957815% | 1 | 0 |
| read_aggregate | 25 | -19.143626303627855% | 19 | 3 |
| read_single | 33 | -28.591380265648617% | 25 | 6 |
| write_bulk | 22 | -11.474982818944898% | 18 | 1 |
| write_single | 9 | -20.904073122471345% | 8 | 1 |

The full quick matrix keeps the candidate: top-line average/geomean/p99/weighted score improve, and every benchmark category improves by average FrankenSQLite median time. The small write_bulk geomean-ratio regression is ratio noise from C-side timing movement; FrankenSQLite write_bulk median time improved on 18 rows and worsened on 1 row by more than 1%.
