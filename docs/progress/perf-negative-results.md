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

## 2026-05-05 - Additional CASS-derived rejected candidates

Scope: last-two-month FrankenSQLite session history searched for negative
signals such as `rejected`, `reverted`, `slower`, `regressed`, `didn't help`,
`did not help`, `within noise`, `abandoned`, and nearby misspellings.

- `concurrent_page_state` structural rewrite / empty-map short-circuit:
  rejected after micro results only moved `1.6 ns` to `1.5 ns` on the empty
  case while populated lookup barely moved (`+0.1%`); the patch was reverted.
  Do not retry without a real matrix row showing state lookup dominates.
- WAL checksum transform hand-folding: rejected after the hand-folded checksum
  path measured roughly `30%` slower than the existing implementation. Do not
  retry scalar checksum reshuffling unless a CPU profile isolates checksum math
  and the candidate is checked against WAL benchmark rows.
- PAX-style `Column` decode cache: deprioritized because the important decode
  cache had already landed and later traces showed different hotspots. Do not
  reopen this as a generic "cache decoded column" idea without proving the
  current row shape is missing the existing cache.
- Same-page `PageBuf` steal allocator: a proof test passed, but wall-clock
  movement was within noise. Do not retry as allocator surgery unless a fresh
  profile shows page-buffer allocation, not pager/VDBE work, dominates.
- Statement-renewal micro-batcher: abandoned after small-N benchmark movement
  stayed within noise; a naive deadline check using `Instant::now()` regressed.
  Do not retry per-call time checks in the hot path.
- `PageData` `Arc<Vec<u8>>` to `Arc<[u8]>`: deferred as high-risk and low
  isolated expected value. Do not attempt as a broad type rewrite without a
  migration plan covering all pager/WAL/MVCC consumers and a matrix target.
- Rust PGO plus full LTO for INSERT: rejected after INSERT benchmarking showed
  roughly `20-25%` slower results. Do not repeat toolchain/profile flag
  exploration for insert throughput unless the profile setup itself changes.

## 2026-05-05 - Direct UPDATE fixed-width REAL one-byte header offset

Scope: `perf-update-delete 10000 40 update`, targeting the prepared
`UPDATE bench SET value = ?2 WHERE id = ?1` direct-simple fixed-width REAL
path in `crates/fsqlite-core/src/connection.rs`.

- Candidate shape: after `BtCursor::payload_into`, bypass
  `parse_record_projected_column_offsets` for records whose header is exactly a
  one-byte header-size varint plus one-byte serial types, validate the target
  serial type is REAL (`7`), compute the column payload offset by summing the
  preceding one-byte serial lengths, and fall back to the generic parser for
  every other record shape.
- Behavior proof: added a direct helper test comparing the computed offset to
  the generic projected-column parser, plus the existing direct-simple REAL
  update proof still passed under `rch exec -- env
  CARGO_TARGET_DIR=/data/tmp/frankensqlite-cyangorge-connection-target cargo
  test -p fsqlite-core real_column -- --nocapture` (2 matching tests passed).
- Evidence: paired release-perf hyperfine artifact
  `tests/artifacts/perf/direct-update-real-offset-candidate-cyangorge-20260505T0838Z/hyperfine-update.json`.
- Result: rejected and reverted. Baseline averaged `344.2 ms +/- 6.9 ms`;
  candidate averaged `347.2 ms +/- 5.4 ms`, so the unpatched binary was
  `1.01x +/- 0.03` faster. Do not retry header-offset microparsing for this
  direct UPDATE path unless a fresh profile shows projected record-header parse
  dominating wall time rather than page write, payload copy, or insert setup.

## 2026-05-05 - Additional CASS/artifact-backed rejects to avoid repeating

Scope: follow-up sweep of the last-two-month CASS hits, recent commits, and
artifact result files for ideas that were measured, rolled back, or explicitly
kept out of the tree but did not yet have a ledger entry.

- `MemDatabase` row-value `Arc<[SqliteValue]>` container swap: rolled back
  after the target `perf-update-delete 10000 10 both` run regressed from
  `264.6 ms +/- 3.9 ms` to `271.5 ms +/- 4.5 ms`, despite passing
  `rch exec -- cargo check -p fsqlite-vdbe -p fsqlite-core --all-targets`.
  Evidence: `docs/perf-a1-memdb-row-values-conclusion.md` and commit
  `0319ea00`. Do not retry shared row-value ownership without an independent
  snapshot-design reason; the narrower `parse_record_into` destination-slot
  idea is the only documented fallback, and only if the clone band grows above
  the ship threshold.
- Direct INSERT rowid-alias borrow: rejected after a behavior proof passed but
  alternating A/B runs on `perf-update-delete 10000 50 both` moved median total
  from `858 ms` to `872 ms` and populate from `412 ms` to `418 ms`. Evidence:
  `tests/artifacts/perf/20260427T1700Z-azurepine-direct-insert-rowid/RESULT.md`.
  Do not retry rowid-alias borrowing as the direct INSERT lever.
- Direct INSERT stateless append hint: rejected after both isolated and
  current-HEAD comparisons made populate slower by roughly `1-2%`. Evidence:
  `tests/artifacts/perf/20260427T2005Z-azurepine-direct-insert-stateless-hint/RESULT.md`.
  Do not retry by dropping retained append-hint page images from explicit
  transactions unless the B-tree hint contract changes materially.
- Synthetic page-one hint cache: rejected after `perf-update-delete 10000 100
  both` median regressed by `5.04%` (`1.2366 s` to `1.2990 s`). Evidence:
  `tests/artifacts/perf/20260428T034415Z-sapphirecrane-next-profile/RESULT-clear-hint-rejected.md`
  and commit `f113fe8c`. Keep the predicate-only stale synthetic page-one
  helper unless a profile proves page-one cleanup dominates a current workload.
- Prepared direct INSERT expression fast path: rejected after targeted concat
  and `?N op literal` handling made the same DML workload mean `3.55%` slower
  while median stayed noise-level. Evidence:
  `tests/artifacts/perf/20260428T1908Z-sapphirecrane-expr-fast/RESULT-expr-fast-rejected.md`.
  Do not add expression-shape special cases without an insert-section A/B win.
- Direct leaf payload writer for prepared INSERT: rejected after the writer
  callback/exact-size route regressed mean by `2.27%` and median by `1.07%`.
  Evidence:
  `tests/artifacts/perf/20260428T1925Z-sapphirecrane-direct-page/RESULT-direct-page-rejected.md`
  and commit `0743bc17`. This is distinct from the later retained-leaf writer
  append entry below; both measured the same basic idea as a loss.
- Direct DML cursor scratch reuse: rejected after interleaved hyperfine showed
  clean parent `1.262 s` versus scratch-routing patch `1.270 s`. Evidence:
  `tests/artifacts/perf/20260428T2135Z-sapphirecrane-direct-dml-cursor-scratch/RESULT-direct-dml-cursor-scratch.md`
  and commit `80777b6b`. Do not retry cursor scratch swaps without a broader
  cursor-owned mutation scratch API and an update/delete-isolated benchmark.
- Direct-simple UPDATE/DELETE schema-proof microbatch carry: committed as
  `4b8151fc` and forward-reverted by `df032429` after measured DML rows and
  the narrow update/delete profiler regressed. Do not reapply schema-proof carry
  to direct UPDATE/DELETE unless the validation cost is proven to dominate and
  the exact DML matrix rows improve.
- Unguarded grouped join aggregate indexed-cache carry: rejected because it
  improved only the 100-row grouped case while dense joins regressed badly
  (`JOIN + GROUP BY` 10K `11.8966 ms` to `14.1428 ms`; `JOIN + HAVING` 10K
  `10.6338 ms` to `15.4707 ms`). Evidence:
  `tests/artifacts/perf/join-grouped-index-cache-candidate-purplecoast-20260504T2040Z/summary.md`.
  Keep the guarded path shape; do not remove the density/table-size guard based
  on small-row wins alone.

## 2026-05-05 - Direct INSERT transient heap TEXT pooling

- Target: `INSERTThroughput` quick insert matrix, especially 10K single-txn
  medium/large record rows where `row_build_ns` spends milliseconds building
  concat-derived TEXT values.
- Touched during rejected candidate:
  `crates/fsqlite-core/src/connection.rs` and
  `crates/fsqlite-types/src/value.rs`.
- Candidate shape: expose the `SmallText` inline capacity, acquire a reusable
  heap `SqliteValue::Text` from the existing thread-local value pool for
  direct-simple INSERT concat chains, and return discarded transient row values
  to the same pool on write-only lazy MemDB paths.
- Evidence:
  - Baseline:
    `tests/artifacts/perf/insert-profile-current-purplecoast-20260505T060835Z/report.json`.
  - Candidate:
    `tests/artifacts/perf/direct-insert-text-pool-purplecoast-20260505T063845Z/report.json`.
  - Focused proof passed:
    `cargo test -p fsqlite-core test_prepared_direct_simple_insert_returns_transient_heap_text_to_pool --profile release-perf -- --nocapture`.
- Result: rejected and manually reverted before commit. The proof showed the
  write-only direct INSERT path could return a heap TEXT slot to the pool, but
  the real insert matrix moved the wrong way: average ratio worsened from
  `3.127x` to `3.226x`, geomean worsened from `2.894x` to `3.018x`, and the
  record-size `large_10col` 10K row regressed from `35.902 ms` to `42.537 ms`
  (`3.652x` to `4.068x` vs C SQLite). Do not retry this value-pool handoff
  unless a later design can prove lower per-row overhead and an insert-section
  A/B improves the primary ratios, not just a unit proof.

## 2026-05-05 - WAL prepared-frame no-memset serializer

- Target: insert commit hot path where WAL frame preparation appeared to pay a
  payload-sized zero-fill before overwriting the full frame bytes.
- Touched during rejected candidate:
  `crates/fsqlite-wal/src/wal.rs::prepare_frame_bytes_with_transforms_into`.
- Candidate shape: replace `Vec::resize(total_bytes, 0)` plus frame overwrite
  with direct frame-byte appends via `push_wal_frame_bytes`, preserving checksum
  transform calculation while avoiding memset-style initialization.
- Evidence:
  - Baseline:
    `tests/artifacts/perf/wal-no-memset-clean-baseline-cyangorge-20260505T063541Z/report.json`.
  - Candidate:
    `tests/artifacts/perf/wal-no-memset-clean-candidate-cyangorge-20260505T063541Z/report.json`.
  - Focused proof passed:
    `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-cyangorge-target cargo test -p fsqlite-wal test_prepared_batch -- --nocapture`.
- Result: rejected and reverted by CyanGorge before commit. A clean-worktree A/B
  on `HEAD` (`5b5212f5`) improved insert average ratio from `3.184x` to
  `2.955x` and geomean from `2.915x` to `2.750x`, but the primary weighted
  score was effectively unchanged (`2.08110` to `2.07856`) and important rows
  regressed: write-single average ratio moved from `1.821x` to `1.868x`,
  `large_10col` 10K single-transaction F median moved from `36.58 ms` to
  `38.43 ms`, and 1000-row autocommit F median moved from `1.54 ms` to
  `1.83 ms`. Do not retry this serializer shape unless a fresh profile shows
  zero-fill dominates a current workload and a full section A/B improves the
  primary/weighted score without write-single regression.

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

## 2026-05-04 - Prepared COUNT(*) LIKE snapshot cache

- Target: `String & Pattern Matching Performance`, especially prepared
  `SELECT COUNT(*) FROM docs WHERE title/body LIKE <literal pattern>` rows.
- Touched during rejected candidate: `crates/fsqlite-core/src/connection.rs`;
  adjacent byte-compare cleanup in `crates/fsqlite-types/src/value.rs` landed
  separately.
- Candidate shape: add a one-entry `PreparedCountLikePatternLastResult` cache
  for clean-memory prepared `COUNT(*) WHERE col LIKE literal` query-row calls,
  keyed by root page, column, rowid alias, LIKE fast-path kind/literal, visible
  commit sequence, and MemDB undo version.
- Candidate commit:
  `b9cc83a7 perf(core): cache prepared COUNT(*) ... LIKE pattern results across clean-memory snapshots`.
- Revert commit: `a05d1e02 perf(core): revert regressed count-like cache`.
- Evidence:
  - Candidate/revert string artifacts:
    `tests/artifacts/perf/string-like-cache-candidate-cyangorge-20260504T2055Z/report.json`
    and
    `tests/artifacts/perf/string-like-cache-revert-cyangorge-20260504T2130Z/report.json`.
  - Earlier local candidate artifacts:
    `tests/artifacts/perf/string-like-count-cache-candidate-local-20260503T031439Z/report.json`
    and repeat
    `tests/artifacts/perf/string-like-count-cache-candidate-repeat-local-20260503T031459Z/report.json`.
- Result: rejected and reverted. The cache proof was plausible, but the real
  string-section benchmark did not produce a defensible matrix win and the
  landed cache was explicitly reverted as regressed. Do not retry the same
  one-entry prepared count-like result cache. Reconsider only if a fresh profile
  proves repeated `COUNT LIKE` result caching removes more work than
  schema/snapshot validation adds, and a close A/B string-section run improves
  FrankenSQLite absolute medians for prefix and wildcard rows without moving
  regressions into other string rows.

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

## 2026-05-05 - Manual ASCII alpha bit-test in LIKE byte comparison

- Target: string workload rows, especially prepared `COUNT(*) ... LIKE`
  prefix/wildcard scans.
- Touched during rejected scratch candidate:
  `crates/fsqlite-types/src/value.rs`.
- Candidate shape: replace `u8::is_ascii_alphabetic()` in
  `fsqlite_types::ascii_ci_eq_byte` with a branchless-style
  `(byte | 0x20).wrapping_sub(b'a') <= b'z' - b'a'` helper. This was narrower
  than the previously rejected standard-library `eq_ignore_ascii_case`
  substitution.
- Evidence:
  - Correctness: `cargo test -p fsqlite-types like --release` passed in the
    clean detached worktree.
  - Baseline:
    `/data/tmp/frankensqlite-purplecoast-clean-20260505T032950Z/tests/artifacts/perf/string-clean-purplecoast-20260505T0330Z/report.json`.
  - Candidate:
    `/data/tmp/frankensqlite-purplecoast-clean-20260505T032950Z/tests/artifacts/perf/string-ascii-alpha-bit-candidate-purplecoast-20260505T0340Z/report.json`.
- Result: rejected before commit and reverted in scratch. The focused string
  matrix worsened from `3.37x` average ratio to `3.63x`; key FrankenSQLite
  medians regressed: 10K prefix LIKE `2.32 ms` to `2.78 ms`, 10K wildcard LIKE
  `3.42 ms` to `3.70 ms`, and 10K GROUP_CONCAT `6.64 ms` to `8.29 ms`.
- Do not retry bit-test microcleanup unless a future compiler/codegen profile
  proves this exact helper dominates LIKE matching.

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

## 2026-05-05 - MemoryVfs contiguous batch append

- Target: insert throughput rows, especially explicit single-transaction
  `large_10col` and record-size insert rows where profiling showed commit
  roundtrip dominated by many dirty memory pages.
- Touched during rejected candidate: `crates/fsqlite-vfs/src/memory.rs`.
- Candidate shape: keep existing `MemoryFile::write_page_batch` reservation and
  accounting, but process normalized writes in order so contiguous append
  suffixes use `Vec::extend_from_slice` instead of resizing the whole final
  file length to zero and then copying dirty pages over it.
- Evidence:
  - Baseline:
    `tests/artifacts/perf/insert-profile-cyangorge-20260505T044600Z/report.json`.
  - Candidate:
    `tests/artifacts/perf/insert-memoryvfs-batch-append-candidate-cyangorge-20260505T050100Z/report.json`.
  - Correctness: `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-cyangorge-target cargo test -p fsqlite-vfs write_page_batch -- --nocapture`
    passed the three focused `write_page_batch` tests.
- Result: rejected before commit and reverted. Insert-only average ratio
  worsened from `2.77x` to `3.12x`; `large_10col` 10K single-transaction
  FrankenSQLite median regressed from `37.81 ms` to `44.58 ms`, and the
  profile hook showed `commit_roundtrip_ns` for record-size `large_10col`
  remained essentially unchanged/slightly worse (`15.98 ms` to `16.42 ms`).
- Do not retry this as a MemoryVfs microcopy cleanup. Reconsider only if a
  lower-level profile proves `Vec::resize` zero-fill is still a top self-time
  frame and a close insert-section A/B improves FrankenSQLite absolute medians,
  not just ratio noise.

## 2026-05-05 - Prepared direct insert retained-leaf writer append

- Target: insert throughput rows, especially explicit single-transaction
  `large_10col` and record-size comparison rows where the profile showed
  serialization plus B-tree cell assembly still visible under the direct insert
  path.
- Touched during rejected candidate: `crates/fsqlite-core/src/connection.rs`,
  `crates/fsqlite-btree/src/cursor.rs`.
- Candidate shape: route prepared monotonic direct inserts through writer
  callbacks (`table_append_after_last_position_with_writer` plus a retained
  `TableAppendHint` writer analogue) and exact-size record slice serializers so
  the record bytes are written directly into the reserved leaf cell instead of
  first materializing `record_scratch`.
- Evidence:
  - Baseline:
    `tests/artifacts/perf/insert-profile-cyangorge-20260505T044600Z/report.json`.
  - Candidate:
    `tests/artifacts/perf/insert-writer-candidate-cyangorge-20260505T0545Z/report.json`.
  - Correctness: `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-cyangorge-target cargo check -p fsqlite-core -p fsqlite-btree`
    passed before measurement.
  - Correctness: `cargo test -p fsqlite-btree test_cached_rightmost_leaf_hint_with_writer_updates_retained_hint -- --nocapture`
    passed; the RCH wrapper later had to be killed while retrieving artifacts.
  - Correctness: `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-cyangorge-target cargo test -p fsqlite-core test_prepared_direct_simple_insert_large_profile_breakdown -- --nocapture`
    passed.
- Result: rejected after commit and reverted by follow-up commit. Insert-only
  average ratio worsened from `2.77x` to `3.10x`. The 10K single-transaction
  `large_10col` FrankenSQLite median regressed from `37.81 ms` to `42.26 ms`;
  the record-size `large_10col` FrankenSQLite median regressed from `40.37 ms`
  to `42.89 ms`. The profile showed the root cause: record serialization did
  shrink on the record-size `large_10col` path (`serialize_ns` about `1.74 ms`
  to `1.40 ms`), but B-tree insert time grew much more (`btree_insert_ns` about
  `7.91 ms` to `12.52 ms`) because the writer route added extra append
  preflight/callback overhead on the hot leaf path.
- Do not retry the retained-leaf writer callback as a general direct insert
  optimization. Reconsider only if the B-tree writer path can preflight room
  without duplicate layout work on full leaves and a close insert-section A/B
  improves FrankenSQLite absolute medians, not just serialization counters.

## 2026-05-05 - Explicit :memory: concurrent transaction retained writer

- Target: explicit single-transaction INSERT and UPDATE/DELETE rows where
  benchmark-shaped private `:memory:` workloads pay fixed BEGIN/COMMIT ceremony
  between logical transactions.
- Touched during rejected candidate: `crates/fsqlite-core/src/connection.rs`.
- Candidate shape: reuse the existing committed cached writer machinery across
  explicit private-memory concurrent transactions. `COMMIT` would call
  `commit_and_retain()` and park the committed writer; the next default
  explicit `BEGIN` would take that cached writer while still registering a fresh
  MVCC concurrent session.
- Evidence:
  - Correctness proof attempted:
    `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-purplecoast-local-target cargo test -p fsqlite-core test_memory_explicit_concurrent_commit_parks_and_reuses_writer -- --nocapture`
  - The focused proof failed on the second `COMMIT` with
    `BusySnapshot { conflicting_pages: "2" }` after the second transaction
    wrote the same table root page. The first retained commit appeared to park,
    and the second `BEGIN` appeared to register a distinct concurrent session,
    but FCW still treated page 2 as too new for the second logical transaction.
- Result: rejected before any benchmark. The code was reverted because it
  violated the explicit concurrent transaction visibility model. The failure is
  a correctness blocker, not a tuning tradeoff.
- Do not retry by simply allowing explicit `BEGIN` to reuse `cached_write_txn`.
  A viable version would first need a proof that the retained pager handle's
  published snapshot, the new `ConcurrentRegistry` session snapshot, and the
  `concurrent_commit_index` frontier are all advanced together before any page
  write is tracked.

## 2026-05-05 - Precomputed record-header append serializer

- Target: quick INSERT matrix, especially cached-header direct INSERT rows where
  record serialization and allocation/copy cost still show up in the profile.
- Touched during rejected candidate: `crates/fsqlite-types/src/record.rs`.
- Candidate shape: for stack-sized `PrecomputedRecordHeader` serializers, stop
  pre-sizing the whole output record with zeroes. Instead, append the cached
  header template and then append serialized payload bytes with
  `append_serialized_value`. The first draft accidentally used
  `Vec::reserve(total_size - capacity)` after `clear()`, which can under-reserve
  because `reserve` is relative to length; the final measured candidate fixed
  that to reserve against the cleared vector length before benchmarking.
- Evidence:
  - Correctness:
    `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-cyangorge-target cargo test -p fsqlite-types precomputed_header -- --nocapture`
    passed.
  - Candidate build:
    `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-wal-measure-target cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench`
    passed in the detached measurement worktree.
  - Same-window clean baseline:
    `tests/artifacts/perf/record-precomputed-append-samewindow-baseline-cyangorge-20260505T0732Z/report.json`.
  - Final corrected candidate:
    `tests/artifacts/perf/record-precomputed-append-reserve-fixed-quick-candidate-cyangorge-20260505T0723Z/report.json`.
- Result: rejected and reverted. The final candidate lost to the same-window
  clean baseline on the insert quick matrix: primary weighted score worsened
  from `1.9105` to `1.9905`, average ratio worsened from `2.9409x` to
  `3.0146x`, and the row-level comparison had 13 FrankenSQLite medians
  regressing by more than 3% versus only one improving. The largest observed
  FrankenSQLite median regressions were medium_6col 100 rows (`0.432 ms` to
  `0.578 ms`), medium_6col 1000 rows (`1.606 ms` to `1.836 ms`), and
  medium_6col record-size 10K (`9.671 ms` to `10.628 ms`).
- Do not retry this zero-fill avoidance shape for cached precomputed record
  headers. Reconsider only if a lower-level profile proves `Vec::resize`
  zero-fill is a dominant self-time frame and a same-window A/B improves
  FrankenSQLite absolute medians, not just ratio noise against C SQLite.

## 2026-05-05 - VDBE concurrent-context borrow in stale page-one clear

- Target: update/delete write rows where `clear_stale_synthetic_pending_commit_surface`
  appeared as visible self-time under `SharedTxnPageIo::write_page_internal`.
- Touched during rejected candidate: `crates/fsqlite-vdbe/src/engine.rs`.
- Candidate shape: inside `clear_stale_synthetic_pending_commit_surface`, borrow
  `self.concurrent` once and use `as_ref()` instead of calling
  `self.concurrent_context()`, avoiding a `ConcurrentContext` clone on every
  stale synthetic page-one cleanup.
- Evidence:
  - Baseline update/delete profiles:
    `tests/artifacts/perf/update-delete-update-profile-cyangorge-20260505T0824Z/`
    and
    `tests/artifacts/perf/update-delete-delete-profile-cyangorge-20260505T0819Z/`.
  - Candidate profile:
    `tests/artifacts/perf/update-clear-context-borrow-candidate-cyangorge-20260505T0835Z/`.
  - Focused A/B:
    `tests/artifacts/perf/update-clear-context-borrow-ab-cyangorge-20260505T0843Z/hyperfine-update.json`.
  - Quick update baseline/candidate:
    `tests/artifacts/perf/update-clear-context-borrow-comprehensive-baseline-cyangorge-20260505T0848Z/report.json`
    and
    `tests/artifacts/perf/update-clear-context-borrow-comprehensive-candidate-cyangorge-20260505T0853Z/report.json`.
  - Quick insert candidate:
    `tests/artifacts/perf/update-clear-context-borrow-insert-candidate-cyangorge-20260505T0858Z/report.json`,
    compared against same-code clean insert baseline
    `tests/artifacts/perf/record-precomputed-append-samewindow-baseline-cyangorge-20260505T0732Z/report.json`.
  - Correctness:
    `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-current-clean-cyangorge-target-20260505T0815Z RUSTFLAGS='-C force-frame-pointers=yes' cargo test -p fsqlite-vdbe shared_txn_page_io --profile release-perf -- --nocapture`
    passed in the detached measurement worktree.
- Result: rejected and reverted. The focused update/delete probe looked
  promising: `perf-update-delete 10000 40 update` improved from `1969 ns` to
  `1851 ns` per updated row, the focused hyperfine mean improved about `2.1%`,
  and the quick update section geomean ratio improved from `3.8912x` to
  `3.3689x`. The broader insert quick section failed the keep bar: the
  candidate's insert average ratio worsened from `2.9409x` to `2.9584x`, the
  geomean worsened from `2.6920x` to `2.7167x`, and FrankenSQLite absolute
  medians regressed across nearly every insert row, including medium_6col
  100 rows (`0.432 ms` to `0.572 ms`), small_3col 1000 rows (`1.013 ms` to
  `1.151 ms`), and record-size large_10col 10K (`34.98 ms` to `37.87 ms`).
- Do not retry this clone-avoidance borrow change as a standalone hot-path
  cleanup. Reconsider only if a same-window insert and update A/B both improve
  FrankenSQLite absolute medians, or if the stale page-one cleanup is isolated
  away from insert-heavy write paths.

## 2026-05-05 - B-tree staged-page mutation for same-size UPDATE overwrite

- Target: direct simple UPDATE rows where
  `BtCursor::table_overwrite_current_payload_same_size_no_overflow` appeared
  under the update profile and wrote an already-staged leaf page back through
  `write_page_data`.
- Touched during rejected candidate: `crates/fsqlite-btree/src/cursor.rs`.
- Candidate shape: after validating the current leaf-table cell and patching
  the cursor stack page image, call `PageWriter::try_mutate_staged_page_data`
  to patch the transaction-owned staged page payload in place. This avoided the
  full-page `write_page_data` path when the same page had already been staged
  by an earlier update in the transaction.
- Evidence:
  - Correctness:
    `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-cyangorge-btree-target cargo test -p fsqlite-btree table_overwrite_current_payload_same_size_no_overflow -- --nocapture`
    passed both focused overwrite tests, including the added staged-page proof.
    RCH then hung retrieving target artifacts and was interrupted after the
    successful test result was printed.
  - Build:
    `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-cyangorge-btree-local-target RUSTFLAGS='-C force-frame-pointers=yes' cargo build -p fsqlite-e2e --bin perf-update-delete --profile release-perf`
    passed.
  - A/B artifact:
    `tests/artifacts/perf/btree-same-size-overwrite-cyangorge-20260505T0755Z/hyperfine-update.json`.
  - Corrected same-code A/B artifact after a concurrent peer commit landed:
    `tests/artifacts/perf/btree-same-size-overwrite-current-head-cyangorge-20260505T0804Z/hyperfine-update.json`.
- Result: rejected and reverted. The preliminary A/B showed the baseline ahead,
  but it used a clean binary from before a concurrent peer commit. The corrected
  current-code A/B on the exact update workload,
  `perf-update-delete 10000 40 update`, was still a no-win: clean baseline mean
  `357.9 ms +/- 6.1 ms`, candidate mean `359.4 ms +/- 7.4 ms`, with hyperfine
  reporting the baseline as `1.00 +/- 0.03` times faster. The extra staged-page
  mutation hook and second payload copy did not clear the keep bar against the
  existing full-page overwrite-steal path.
- Do not retry staged-page mutation for same-size UPDATE as a standalone B-tree
  change. Reconsider only if the direct UPDATE caller can supply a payload-slice
  patch that avoids rebuilding the full record first, or if a profile shows
  `write_page_data` copying itself dominates after connection-level payload
  construction is removed.
