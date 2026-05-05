# Full Quick Retarget Profile - CyanGorge - 2026-05-05 15:45 UTC

Fresh full quick matrix after rejected direct-INSERT affinity and WAL checksum
micro-optimizations. Source commit:
`e3421e89b96faf15ad995fba58c24f34c48a533b`.

Command:

```bash
/data/tmp/frankensqlite-cyangorge-walchk-target/release-perf/comprehensive-bench \
  --quick \
  --json-out tests/artifacts/perf/full-quick-retarget-cyangorge-20260505T1545Z/report.json \
  --no-html
```

Environment:

- Host: Ubuntu 25.10, Linux 6.17.0-19-generic, x86_64
- CPU: AMD Ryzen Threadripper PRO 5995WX 64-Cores, 128 logical cores
- RAM: 499.3 GiB
- Rust: `rustc 1.97.0-nightly (cb40c25f6 2026-05-04)`
- Cargo: `cargo 1.97.0-nightly (4f9b52075 2026-05-01)`
- Profile: `release-perf`

Overall:

- Total scenarios: 93
- FrankenSQLite faster: 58
- C SQLite faster: 35
- Primary weighted score: `0.572883`
- Average ratio: `1.065630x`
- Geomean ratio: `0.451501x`
- Median ratio: `0.395148x`
- p90 ratio: `2.706073x`
- p99 ratio: `3.883443x`

Per-category ratios:

| Category | n | Avg | Geomean | Median | p90 | p99 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| concurrent_writers | 3 | 1.2832x | 1.1628x | 1.5458x | 1.7082x | 1.7082x |
| mixed | 1 | 0.2095x | 0.2095x | 0.2095x | 0.2095x | 0.2095x |
| read_aggregate | 25 | 0.3780x | 0.1165x | 0.2279x | 0.9271x | 1.5755x |
| read_single | 33 | 0.2805x | 0.2480x | 0.2301x | 0.4210x | 0.7772x |
| write_bulk | 22 | 2.5386x | 2.4717x | 2.5187x | 3.8097x | 3.8834x |
| write_single | 9 | 2.2764x | 2.1780x | 2.2145x | 3.2630x | 3.2630x |

Worst remaining rows:

| Ratio | Category | Scenario | C SQLite median | FrankenSQLite median |
| ---: | --- | --- | ---: | ---: |
| 3.8834x | write_bulk | record-size large_10col 10K single txn | 9.634383 ms | 37.414577 ms |
| 3.8328x | write_bulk | tiny_1col 100 rows single txn | 0.066314 ms | 0.254166 ms |
| 3.8097x | write_bulk | large_10col 10K single txn | 9.293224 ms | 35.404393 ms |
| 3.2630x | write_single | 100 rows / update 10 rows | 0.083587 ms | 0.272741 ms |
| 3.2279x | write_single | 100 rows / delete 5 rows | 0.079839 ms | 0.257712 ms |
| 2.8602x | write_bulk | medium_6col 100 rows single txn | 0.101480 ms | 0.290254 ms |
| 2.7958x | write_bulk | small_3col 100 rows single txn | 0.074029 ms | 0.206968 ms |
| 2.7860x | write_bulk | small_3col 100 rows batched 100/txn | 0.073928 ms | 0.205966 ms |
| 2.7096x | write_bulk | small_3col 100 rows single txn | 0.075412 ms | 0.204333 ms |
| 2.7061x | write_bulk | medium_6col 10K single txn | 5.004092 ms | 13.541439 ms |

Retargeting conclusion:

- Do not spend more time on read-path rows until write categories move; reads are
  already faster than C SQLite in aggregate.
- The remaining matrix gap is concentrated in write_bulk and write_single.
- Recent single-lever insert/WAL guesses were rejected by this same benchmark
  family. The next candidate should be chosen from a fresh focused DML/write-path
  profile, especially the small UPDATE/DELETE rows and shared direct DML
  overhead that may also affect inserts.
