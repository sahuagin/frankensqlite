# Performance Negative Results Ledger

This ledger records performance ideas that were measured and rejected. Check it
before starting a new optimization pass, and add an entry whenever a candidate is
abandoned, reverted, or kept out of the tree because the benchmark matrix did not
move in the intended direction.

Each entry should include:
- Target workload rows or benchmark section.
- Files or subsystem touched.
- Baseline and candidate evidence.
- Result and reason for rejection.
- Conditions under which the idea is worth retrying.

## 2026-05-04 - CASS archaeology guardrails

Scope: `cass` searches restricted to FrankenSQLite content since `2026-03-04`,
covering terms such as `rejected`, `reverted`, `abandoned`, `slower`,
`regressed`, `did not help`, `no improvement`, `within noise`, `rollback`,
`candidate`, `benchmark`, and `matrix`.

- `SqliteValue` `Arc` wrapping (`Arc<str>`, `Arc<[u8]>`, `Arc<String>`,
  `Arc<Vec<u8>>`) showed up repeatedly as a clone-reduction idea, but March
  fresh-eyes sessions report that the attempt broke serde/type constraints and
  left cross-crate type mismatches. Do not retry without a designed serde story
  and a compile/test proof before measuring.
- Broad `SmallVec` register/op sweeps caused dependency, initialization, and
  borrow-check failures around `VdbeProgram`, `VdbeEngine::registers`, and
  `Opcode::MakeRecord`; the safe recovery was to restore owned clones before
  mutably borrowing storage cursors. Do not repeat as a broad mechanical sweep.
- A broad "alien" batch combining multi-tiered SSI witness indexing, B-tree
  stack elision, Adaptive Sharded ARC, and CAMP produced correctness hazards:
  custom/global witnesses were dropped, dirty write-set pages could be hidden by
  stack elision, `ArcCache::get` could deep-clone page data, witness bridge
  methods were lost during edits, and the CAMP path initially used `unsafe`.
  Revisit only as narrow, separately measured patches with SSI/witness and
  dirty-page correctness tests.
- `with_pager_write_txn` bypassing active transactions was a CASS false lead:
  the same session re-read the helper and corrected itself that the function is
  centralized and handles active transactions. Do not spend another pass on that
  theory without new evidence.
- Audit-only CASS leads such as `OP_Count` full-table scans, `cursor_column`
  payload comparison cost, parse-cache full flushes, index-ordered OFFSET after
  column reads, and Bloom one-hash false positives should remain optimization
  backlog, not negative results, until someone has a measured rejected patch.

Primary CASS evidence:
- `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-09T05-08-84f3c374.json -n 44 -C 6`
- `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-09T22-55-5b9da3d6.json -n 153 -C 18`
- `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-09T05-09-1bf54aa9.json -n 267 -C 10`

## 2026-05-05 - Prepared indexed-equality schema microbatch carry

- Target: `Read-After-Write Query Performance`, especially repeated prepared
  secondary indexed equality probes.
- Touched: `crates/fsqlite-core/src/connection.rs`.
- Candidate shape: apply the existing prepared-statement microbatch
  schema-identity carry to `PreparedStatement::try_query_clean_memory_indexed_equality_fast`,
  mirroring the rowid query-row no-refresh path.
- Evidence:
  - Baseline/read context:
    `tests/artifacts/perf/read-point-pathtrace-cyangorge-20260505T0112Z/report.json`.
  - Candidate:
    `tests/artifacts/perf/read-indexed-equality-microbatch-candidate-cyangorge-20260505T0131Z/report.json`.
  - Candidate repeat:
    `tests/artifacts/perf/read-indexed-equality-microbatch-candidate-repeat-cyangorge-20260505T0135Z/report.json`.
- Result: rejected before commit and reverted. A focused correctness proof
  showed the no-refresh indexed path could renew then carry the schema epoch,
  but the e2e read matrix did not produce a clean primary win. The first
  candidate run worsened the primary weighted score from `2.685x` to `2.995x`.
  The repeat improved the slowest 100K secondary-index ratio (`48.28x` to
  `33.06x`) and p90/p99, but still worsened the primary weighted score to
  `2.779x`; absolute FrankenSQLite secondary medians also regressed at 1K and
  10K rows.
- Do not retry the same schema-carry placement inside
  `try_query_clean_memory_indexed_equality_fast`. Reconsider only if a profile
  proves schema identity validation dominates repeated secondary probes and a
  close A/B read-section run improves the primary weighted score and
  FrankenSQLite absolute medians.

## 2026-05-05 - File-backed prepared indexed-equality last-result cache

- Target: prepared secondary indexed equality probes in the read benchmark.
- Touched during rejected candidate: `crates/fsqlite-core/src/connection.rs`.
- Candidate shape: reuse `prepared_indexed_equality_last_result` in the
  file-backed `SimpleIndexedEqualityLookup` collection and `query_row` arms,
  with file-backed proof coverage for repeat-probe reuse and invalidation after
  external writes.
- Evidence:
  - Focused proof: `cargo test -p fsqlite-core test_file_backed_clean_prepared_indexed_equality_reuses_last_probe_until_external_write -- --nocapture`.
  - Baseline: `/data/tmp/frankensqlite-purplecoast-indexeq-base-read-20260505T0100Z.json`.
  - Candidate: `/data/tmp/frankensqlite-purplecoast-indexeq-candidate-read-20260505T005522Z.json`.
- Result: rejected and reverted before commit. The proof test passed, but the
  e2e read benchmark's secondary-index row uses `:memory:` and exits through
  `PreparedStatement::try_query_clean_memory_indexed_equality_fast`, so the
  candidate did not target the matrix path. Same-HEAD A/B artifacts were too
  noisy to defend as a real matrix win.
- Do not retry the file-backed last-result cache for the current read-section
  gap. Reconsider only for a workload that actually exercises file-backed
  prepared indexed equality, or after the benchmark target is proven to enter
  the file-backed branch.

## 2026-05-05 - GROUP_CONCAT integer itoa append

- Target: string workload `GROUP_CONCAT` rows, especially
  `SELECT tag, GROUP_CONCAT(id, ',') FROM docs GROUP BY tag`.
- Touched during rejected candidate:
  `crates/fsqlite-func/src/agg_builtins.rs`,
  `crates/fsqlite-func/Cargo.toml`.
- Candidate shape: add `itoa` to `fsqlite-func` and format
  `SqliteValue::Integer` directly into the aggregate accumulator string instead
  of allocating through `to_text()` / `i64::to_string()`.
- Evidence:
  - Candidate: `/data/tmp/frankensqlite-purplecoast-groupconcat-candidate-string-20260505T0118Z.json`.
  - Same-window clean baseline: `/data/tmp/frankensqlite-purplecoast-groupconcat-base-string-20260505T0120Z.json`.
- Result: rejected before commit and manually reverted. Same-window
  FrankenSQLite medians worsened: 100 rows `77.1 us` to `242.8 us`, 1000 rows
  `701.7 us` to `725.1 us`, and 10000 rows `6.06 ms` to `8.85 ms`. The average
  string ratio stayed about `3.38x` and did not improve.
- Do not retry direct per-step `itoa::Buffer` formatting inside
  `GroupConcatFunc::step`. Reconsider only with a design that avoids per-row
  formatter setup and proves real string-section wins.

## 2026-05-05 - Positive-start ASCII-prefix SUBSTR fast path

- Target: `String & Pattern Matching Performance`, specifically
  `string functions (LENGTH + UPPER + SUBSTR)` rows.
- Touched: `crates/fsqlite-func/src/builtins.rs`.
- Candidate shape: for `SUBSTR(text, positive_start, positive_length)`, prove
  only the requested prefix is ASCII and slice by byte offset before the
  existing full-string `is_ascii()` / Unicode-count path.
- Candidate commit: `ee1649d5 perf(substr): ascii-prefix fast path for positive (start, length) substr`.
- Revert commit: `426590d5 perf(substr): revert rejected ascii-prefix fast path`.
- Evidence:
  - Baseline: `/data/tmp/frankensqlite-purplecoast-substr-prefix-base-string-20260505T0142Z.json`.
  - Candidate: `/data/tmp/frankensqlite-purplecoast-substr-prefix-candidate-string-20260505T0148Z.json`.
- Result: rejected and reverted. The candidate improved only the largest
  string-functions row slightly (`10000 rows` FrankenSQLite median `12.06 ms`
  to `11.84 ms`), while worsening smaller rows (`100 rows` `107.1 us` to
  `133.7 us`, `1000 rows` `1.23 ms` to `1.38 ms`) and worsening the string
  section average ratio from `3.17x` to `3.66x`.
- Do not retry as a per-call prefix probe in `SubstrFunc`. Reconsider only if a
  profile isolates `SUBSTR` body scanning as the dominant cost and a close A/B
  string-section run improves every affected string-functions row or the section
  score without small-row regression.

## 2026-05-05 - SmallText direct-byte Eq/Ord/Hash traits

- Target: `Read-After-Write Query Performance`, especially secondary indexed
  equality probes whose cache path compares/hashes short TEXT values.
- Touched: `crates/fsqlite-types/src/value.rs`.
- Candidate shape: make `SmallText` `PartialEq`, `Ord`, and `Hash` use
  `as_bytes_direct()` instead of `as_str()` so inline strings avoid repeated
  UTF-8 validation; preserve `str` hash compatibility by writing bytes plus the
  `0xff` separator used by `Hasher::write_str`.
- Evidence:
  - Baseline: `tests/artifacts/perf/read-indexed-baseline-cyangorge-20260504T2355Z/report.json`.
  - Noisy candidate: `tests/artifacts/perf/read-smalltext-byte-traits-cyangorge-20260505T0001Z/report.json`.
  - Candidate repeat after the competing build finished:
    `tests/artifacts/perf/read-smalltext-byte-traits-cyangorge-20260505T0010Z/report.json`.
- Result: rejected before commit and reverted. The candidate repeat did not move
  the read-section average (`3.09x` versus `3.08x` baseline). Secondary indexed
  lookup remained mixed: the 100-row fsqlite median was essentially unchanged,
  the 1000-row median worsened, and the 10000-row improvement was within noise
  while the row still had high variance.
- Do not retry as a broad `SmallText` trait cleanup. Reconsider only if a CPU or
  allocation profile shows UTF-8 validation inside `SmallText` traits dominating
  a specific workload and a clean A/B run improves FrankenSQLite absolute
  medians, not just C/FrankenSQLite ratios.

## 2026-05-05 - Direct REAL accumulator for rowid-bucket SUM GROUP BY

- Target: `Read-After-Write Query Performance`, especially
  `SUM + GROUP BY (~10 groups)` rows.
- Touched: `crates/fsqlite-vdbe/src/codegen.rs`.
- Candidate shape: for `SUM(<REAL NOT NULL column>)` grouped by a rowid bucket,
  replace generic `AggStep`/`AggFinal` dispatch with a direct `REAL 0.0`
  accumulator and `Add` opcode in the rowid-bucket sorter-bypass plan.
- Candidate commits: `7ec9d6b1 perf(codegen): direct REAL accumulator for GROUP BY rowid-bucket SUM`
  and `a0f674c6 test(codegen): swap rowid-bucket SUM test divisors back`.
- Evidence:
  - Baseline: `tests/artifacts/perf/read-indexed-baseline-cyangorge-20260504T2355Z/report.json`.
  - Candidate: `tests/artifacts/perf/read-groupby-direct-real-sum-cyangorge-20260505T0019Z/report.json`.
- Result: rejected and reverted. The 10000-row group row improved
  (`4.436 ms` to `3.888 ms`, ratio `3.44x` to `2.77x`), but the 1000-row
  group row regressed badly (`0.350 ms` to `0.800 ms`, ratio `2.77x` to
  `5.47x`), the 100-row group row slightly worsened, and the read-section
  average ratio worsened from `3.08x` to `3.56x`.
- Do not retry the direct accumulator as a narrow opcode substitution. Revisit
  only if a profile proves generic aggregate dispatch dominates all target group
  sizes and a close A/B read-section run improves the section score or every
  affected group row.

## 2026-05-05 - Direct single-rowid DELETE lowering

- Target: `UPDATE/DELETEThroughput`, especially prepared
  `DELETE FROM bench WHERE id = ?1`.
- Touched: `crates/fsqlite-vdbe/src/codegen.rs`.
- Candidate shape: when DELETE has a simple rowid equality predicate, skip the
  one-row `RowSetAdd`/`RowSetRead` two-pass plan and emit direct
  `SeekRowid`/`Delete` code, leaving non-rowid predicates on the two-pass path.
- Evidence:
  - Baseline: `tests/artifacts/perf/update-delete-current-cyangorge-20260505T0058Z/report.json`.
  - Candidate: `tests/artifacts/perf/update-delete-direct-delete-candidate-cyangorge-20260505T0100Z/report.json`.
- Result: rejected before commit and reverted. The average section ratio moved
  from `4.36x` to `4.03x`, but the targeted DELETE medians regressed at the
  smaller, high-signal sizes: `100 rows / delete 5 rows` worsened from
  `617.6 us` to `765.2 us`, and `1000 rows / delete 50 rows` worsened from
  `1.34 ms` to `1.58 ms`. The 10000-row DELETE improvement was only a small
  `10.30 ms` to `10.06 ms` move and does not justify the small-row loss.
- Do not retry as a simple RowSet skip. Reconsider only with an opcode-level
  profile proving RowSet overhead dominates DELETE and with a close A/B where
  FrankenSQLite DELETE medians improve at all three row counts.

## 2026-05-04 - Single-value insert serialization specialization

- Target: insert throughput, especially tiny/small single-column and small-record rows.
- Touched: `crates/fsqlite-types/src/record.rs`, `crates/fsqlite-vdbe/src/engine.rs`.
- Candidate commit: `7fa3f4d0 perf(record): specialize single-value insert serialization`.
- Revert commit: `5e9445ac Revert "perf(record): specialize single-value insert serialization"`.
- Evidence:
  - Baseline: `/data/tmp/frankensqlite-purplecoast-postcommit-parent-20260504T220353Z-report.json`.
  - Candidate: `/data/tmp/frankensqlite-purplecoast-postcommit-head-20260504T220353Z-report.json`.
- Result: rejected and reverted. Overall fsqlite geomean time changed by `1.0247x`
  slower, average time was `+3.89%`, with 11 improved rows and 14 regressed rows.
- Do not retry unless the exact insert section is benchmarked first and the
  implementation avoids adding overhead to multi-column insert rows.

## 2026-05-04 - Two-byte precomputed record header support

- Target: insert serialization for records whose serial types need two-byte varints.
- Touched: `crates/fsqlite-types/src/record.rs`, `crates/fsqlite-vdbe/src/engine.rs`.
- Candidate shape: add `PrecomputedSerialTypeKind::AnyTwoByteVarint` and patch
  precomputed record headers at runtime.
- Evidence:
  - Candidate: `/data/tmp/frankensqlite-purplecoast-two-byte-record-candidate-20260504T2218Z-report.json`.
  - Baseline: `/data/tmp/frankensqlite-purplecoast-postcommit-parent-20260504T220353Z-report.json`.
- Result: rejected before commit. Overall fsqlite geomean time changed by
  `1.1139x` slower, average time was `+13.97%`, with 6 improved rows and
  19 regressed rows.
- Do not retry as a general record-header optimization. Only reconsider if a
  profile proves two-byte serial type patching is isolated to a workload where
  the end-to-end matrix improves.

## 2026-05-04 - Prepared PK rowid last-result cache

- Target: `Read-After-Write Query Performance`, especially `point lookup (PK)`.
- Touched: `crates/fsqlite-core/src/connection.rs`.
- Candidate shape: one-entry version-scoped cache for repeated prepared primary
  key rowid lookups, sharing invalidation keys with existing prepared MemDB
  caches.
- Evidence:
  - Full matrix that motivated the target: `/data/tmp/frankensqlite-purplecoast-current-full-20260504T2230Z-report.json`.
  - Candidate read section: `/data/tmp/frankensqlite-purplecoast-rowid-cache-candidate-read-20260504T2245Z-report.json`.
  - Close baseline read section: `/data/tmp/frankensqlite-purplecoast-rowid-cache-baseline-read-20260504T2252Z-report.json`.
  - Saved rejected patch: `/data/tmp/frankensqlite-purplecoast-rowid-cache-20260504T2252Z.patch`.
- Result: rejected before commit. The targeted correctness test passed, but the
  close A/B read geomean regressed from `2.41x` to `3.15x` versus C SQLite.
  PK fsqlite-time rows also regressed: `100 rows` by `1.15x`, `1000 rows` by
  `1.43x`, and `10000 rows` by `2.26x`.
- Do not retry the same one-entry rowid result cache. Reconsider only if the
  query-row dispatch path is redesigned so the cache removes more work than it
  adds, and prove it with a close A/B read-section run.

## 2026-05-04 - Unbounded grouped join rowid-count helper

- Target: join read rows, especially `JOIN + GROUP BY` and `JOIN + HAVING`.
- Touched: `crates/fsqlite-core/src/connection.rs`.
- Candidate shape: remove the small-right-table limit around the prepared inner
  join grouped rowid-count helper so larger right tables use the direct helper.
- Evidence:
  - Candidate: `tests/artifacts/perf/join-rowid-count-peer-candidate-cyangorge-20260504T1955Z/report.json`.
  - Baseline context from clean quick matrix at `a05d1e02`: `JOIN + GROUP BY`
    fsqlite median about `14.08 ms`; `JOIN + HAVING` about `13.97 ms`.
- Result: rejected before commit. Candidate focused join rows measured
  `17.42 ms` for `JOIN + GROUP BY` and `19.22 ms` for `JOIN + HAVING`, worse
  than the clean context despite the direct helper test shape.
- Do not retry by simply removing the row limit. Reconsider only if the helper
  is fed through the real prepared-query refresh path and a close A/B join run
  improves the actual matrix rows.

## 2026-05-04 - Standard-library ASCII LIKE byte comparison

- Target: string workload rows, especially LIKE prefix/contains/wildcard scans.
- Touched: `crates/fsqlite-types/src/value.rs`.
- Candidate shape: replace the local ASCII-case byte comparison helper with
  `[u8]::eq_ignore_ascii_case`.
- Evidence:
  - Baseline: `tests/artifacts/perf/string-clean-head-cyangorge-20260504T2240Z/report.json`.
  - Candidate: `tests/artifacts/perf/string-std-ascii-ci-cyangorge-20260504T2246Z/report.json`.
- Result: rejected before commit. Average string-section ratio worsened from
  about `3.03x` to `3.73x`; 100-row and 10K-row prefix/wildcard rows regressed,
  with only the 1K-row prefix case improving.
- Do not retry as a general LIKE matcher cleanup. Reconsider only with an
  end-to-end string-section A/B that shows row-level wins beyond noise.

## 2026-05-04 - Exact-sized record body writes

- Target: record-size insert section, especially `large_10col`.
- Touched: `crates/fsqlite-types/src/record.rs`.
- Candidate shape: pre-size the serialized record buffer to the full record size
  and write payload bytes into exact slices instead of appending payload bytes.
- Evidence:
  - Baseline: `tests/artifacts/perf/record-current-clean-cyangorge-20260504T2300Z/report.json`.
  - Candidate: `tests/artifacts/perf/record-exact-body-write-cyangorge-20260504T2300Z/report.json`.
- Result: rejected before commit. Tiny rows improved, but small/medium/large
  FrankenSQLite medians regressed; the section only appeared better because the
  C SQLite large-row sample slowed down.
- Do not retry the same exact-body `Vec::resize` strategy unless a profile proves
  payload append/copy dominates and a close A/B record-section run improves the
  actual FrankenSQLite medians.

## 2026-05-04 - Two-byte runtime precomputed record headers, repeat

- Target: record-size insert section, especially medium/large rows with long
  TEXT serial types.
- Touched: `crates/fsqlite-types/src/record.rs`, `crates/fsqlite-vdbe/src/engine.rs`.
- Candidate shape: add a two-byte runtime precomputed-header slot for direct
  inserts whose first row has long TEXT/BLOB serial types.
- Evidence:
  - Baseline: `tests/artifacts/perf/record-current-clean-cyangorge-20260504T2300Z/report.json`.
  - Candidate: `tests/artifacts/perf/record-two-byte-runtime-header-cyangorge-20260504T2315Z/report.json`.
  - Candidate repeat: `tests/artifacts/perf/record-two-byte-runtime-header-repeat-cyangorge-20260504T2320Z/report.json`.
- Result: rejected before commit. The repeat showed tiny/medium improvements but
  large-row FrankenSQLite time regressed from the clean baseline, and the ratio
  improvement was mostly from a slower C SQLite large-row sample.
- Do not retry as a broad runtime-header extension. Only revisit if two-byte
  patching is isolated to a proven row shape and judged on FrankenSQLite absolute
  time as well as C/FrankenSQLite ratio.
