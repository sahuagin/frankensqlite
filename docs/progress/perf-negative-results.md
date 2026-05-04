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
