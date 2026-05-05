# Insert Sorted Page Append Candidate

Date: 2026-05-05
Agent: CyanGorge
Target: `INSERTThroughput` quick insert matrix
Candidate file: `crates/fsqlite-pager/src/pager.rs`

## Candidate

`insert_page_sorted` first checked `pages.last()` and appended page numbers that
arrived after the current tail, returned for duplicate-tail inserts, and kept
the existing binary-search insertion for out-of-order page numbers.

The intent was to optimize the common sorted append case for
`write_pages_sorted` before WAL commit publication.

## Verification

- `cargo fmt --check` passed before benchmarking.
- `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-cyangorge-sorted-page-target cargo test -p fsqlite-pager sorted -- --nocapture` passed the focused pager sorted-order tests.
- `env CARGO_TARGET_DIR=/data/tmp/cargo-target cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench` passed.
- `env FSQLITE_BENCH_PROFILE_INSERT=1 /data/tmp/cargo-target/release-perf/comprehensive-bench --quick --filter insert --json-out tests/artifacts/perf/insert-sorted-page-append-cyangorge-20260505T1450Z/report.json --no-html` completed and wrote `report.json`.

## Result

Rejected and reverted before commit.

Baseline artifact:
`tests/artifacts/perf/insert-profile-current-head-cyangorge-20260505T122449Z/report.json`

Candidate artifact:
`tests/artifacts/perf/insert-sorted-page-append-cyangorge-20260505T1450Z/report.json`

Summary ratios:

| Metric | Baseline | Candidate |
| --- | ---: | ---: |
| Primary weighted score | 1.6991 | 1.7591 |
| Average ratio | 2.4610x | 2.5153x |
| Geomean ratio | 2.3623x | 2.4081x |
| write_bulk geomean | 2.5153x | 2.5565x |
| write_single geomean | 1.4908x | 1.5530x |

Key row medians:

| Row | Baseline F median | Candidate F median | Baseline ratio | Candidate ratio |
| --- | ---: | ---: | ---: | ---: |
| small_3col 10K single txn | 6.8949 ms | 7.1046 ms | 2.1630x | 2.2842x |
| medium_6col 10K single txn | 13.6661 ms | 12.9439 ms | 2.6769x | 2.4660x |
| large_10col 10K single txn | 36.1651 ms | 36.9092 ms | 3.7657x | 4.0238x |
| record-size large_10col 10K | 37.0559 ms | 36.8537 ms | 3.8273x | 3.9575x |

The candidate helped one medium-row median but worsened the primary score,
write-single score, average/geomean ratios, and the main large-row insert gap.
Do not retry this as a standalone sorted-page append/equal-last fast path unless
a fresh profile shows sorted-page maintenance dominates and a full insert matrix
improves the primary score and large-row medians.
