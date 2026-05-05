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

## 2026-05-05 - CASS synonym sweep coverage note

Scope: user-requested CASS search restricted to FrankenSQLite session history
since `2026-03-05`, using direct `/data/projects/frankensqlite` workspace
filters first and then the archived Gemini workspace alias
`/home/ubuntu/.gemini/tmp/frankensqlite` when the direct workspace filter was
sparse. Searched terms included `rejected`, `reverted`, `slower`,
`regressed`, `didn't help`, `did not help`, `within noise`, `abandoned`,
`abandones`, `no improvement`, `rollback`, `worse`, `failed to improve`,
`no measurable`, `revert it for now`, `not worth`, and `failed the keep`.

- No new benchmark-rejected performance ideas were found beyond the existing
  CASS/artifact sections in this ledger. Useful hits were already represented
  by the Arc/SmallVec, stale raw-benchmark, prepared-DML bypass, async-rewrite
  plan-space, and recent artifact-backed no-retry entries below.
- The remaining hits were intentionally excluded because they were correctness
  fixes that landed, commit-log summaries from multi-repo sessions, accepted
  optimizations, issue-triage text, or CASS false positives where the negative
  word was unrelated to a performance candidate.
- The attempted `cass index --json` refresh timed out after staying in
  `preparing total=0`, so this note is based on the existing CASS index plus
  direct `cass view` inspection of the relevant hits. Refresh CASS before
  repeating this sweep only if newer sessions need to be included.

## 2026-05-05 - Exact-path CASS session-set follow-up

Scope: follow-up to the user request to restrict CASS mining to this project
folder and the last two months. Because direct
`--workspace /data/projects/frankensqlite` was sparse and returned at least one
cross-project false positive, the search first built a session set from CASS
sessions that explicitly mention `/data/projects/frankensqlite`, then searched
only those sessions with `--sessions-from` and `--days 60`.

- Session seed command:
  `cass search '/data/projects/frankensqlite' --days 60 --robot-format sessions --limit 500 --mode lexical`
  returned `38` session paths in the existing CASS index.
- Negative terms searched inside that seed set included `rejected`,
  `reverted`, `slower`, `regressed`, `didn't help`, `did not help`,
  `abandoned`, `abandones`, `within noise`, `no improvement`, `rollback`,
  `worse`, and `failed to improve`, plus benchmark/perf phrase combinations.
- No additional benchmark-rejected performance candidates were found that were
  not already represented elsewhere in this ledger. The high-signal perf hits
  led back to existing entries such as stale March hash/cache experiments,
  page-1 synthetic hint state, WAL/checksum/publication candidates, direct
  INSERT row-build candidates, and benchmark-policy rejects.
- Excluded hits were non-perf or non-negative: multi-repo commit grouping
  summaries, FrankenTUI accessibility sessions indexed under a broad workspace,
  SHM correctness work with pre-existing harness failures, UNIQUE/quoting bug
  fixes, and landed feature summaries.
- Practical rule for future sweeps: prefer this explicit-path session-set
  method over trusting the exact workspace filter alone, then add only
  artifact-backed perf rejects or correctness-abandoned optimization attempts.

## 2026-05-05 - CASS user-term dedupe refresh

Scope: follow-up to the explicit request to search last-two-month project history
for failure vocabulary such as `rejected`, `reverted`, `slower`,
`didn't help`, `did not help`, `abandoned`, `abandones`, `within noise`,
`no improvement`, `rollback`, `worse`, `failed to improve`, and `not worth`.
The existing CASS index was stale but usable. A direct session seed for
`/data/projects/frankensqlite` returned `38` session paths; direct
`--sessions-from` searches reported term totals without usable snippets, so the
fallback was global `frankensqlite <term>` CASS search plus targeted `cass view`
inspection of only source paths/titles clearly tied to this repo.

- No additional benchmark-rejected or correctness-abandoned optimization
  candidates were found beyond entries already represented in this ledger.
- Hits for the March `bench_insert` serializer/VFS/hash-map optimization pass
  reinforce the existing stale-benchmark rule: the raw benchmark moved only
  about `0.271 s` to `0.265 s` while thrashing parse/codegen with unique SQL
  strings. Do not use that run as a keep/retry signal for current insert work.
- Hits for `SqliteValue` `Arc<str>` / `Arc<[u8]>`, prepared-DML direct VDBE
  execution, and public `Row` `SmallVec` were already covered by the CASS
  last-60-day no-retry expansion below.
- Hits for async VFS / true-asupersync migration beads were already classified
  as architecture plan-space, not a rejected micro-optimization.
- Hits for `ConcurrentRegistry` global-lock stripping, VDBE/B-tree index-record
  parse hoists, cancellation checkpoints, and JSON/VFS correctness audits were
  excluded because CASS presented them as accepted or correctness-focused work,
  not as ideas that were tried and abandoned. Add them later only if a commit,
  artifact, or follow-up session shows a measured revert or keep-gate failure.

CASS evidence inspected in this refresh:
- `cass search '/data/projects/frankensqlite' --days 60 --robot-format sessions --limit 500 --mode lexical`
- `cass search 'frankensqlite rejected' --days 60 --json --fields summary --limit 20 --mode lexical`
- `cass search 'frankensqlite reverted' --days 60 --json --fields summary --limit 20 --mode lexical`
- `cass search 'frankensqlite slower' --days 60 --json --fields summary --limit 20 --mode lexical`
- `cass search 'frankensqlite abandoned' --days 60 --json --fields summary --limit 30 --mode lexical`
- `cass search "frankensqlite didn't help" --days 60 --json --fields summary --limit 20 --mode lexical`
- `cass search 'frankensqlite did not help' --days 60 --json --fields summary --limit 30 --mode lexical`
- `cass search 'frankensqlite within noise' --days 60 --json --fields summary --limit 30 --mode lexical`
- `cass search 'frankensqlite no improvement' --days 60 --json --fields summary --limit 30 --mode lexical`
- `cass search 'frankensqlite rollback' --days 60 --json --fields summary --limit 30 --mode lexical`
- `cass search 'frankensqlite worse' --days 60 --json --fields summary --limit 30 --mode lexical`
- `cass search 'frankensqlite failed to improve' --days 60 --json --fields summary --limit 30 --mode lexical`
- `cass search 'frankensqlite not worth' --days 60 --json --fields summary --limit 30 --mode lexical`
- `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-09T05-08-a1108e5a.json -n 104 -C 60`
- `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-09T05-09-1bf54aa9.json -n 285 -C 24`
- `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-09T22-55-f0efb944.json -n 219 -C 28`
- `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-07T20-25-52485ea5.json -n 13 -C 24`
- `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-08T22-16-466c7bcd.json -n 168 -C 30`
- `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-08T00-01-e13b2d1e.json -n 4 -C 20`

## 2026-05-05 - Direct DML `SharedTxnPageIo` wrapper reuse

Scope: prepared direct INSERT/UPDATE/DELETE in concurrent mode, after the
UPDATE/DELETE profile showed fixed setup costs around short-lived B-tree cursor
and page I/O wrapper construction.

- Touched during rejected candidate:
  `crates/fsqlite-core/src/connection.rs` and
  `crates/fsqlite-vdbe/src/engine.rs`; source was reverted after measurement.
- Candidate shape: park a reusable `SharedTxnPageIo` wrapper on `Connection`,
  refill it with the current pager transaction plus concurrent writer context
  for each direct DML statement, then drain the transaction back to
  `active_txn`. The intent was to avoid rebuilding the internal
  `Rc<RefCell<...>>` pair for every prepared direct INSERT/UPDATE/DELETE row.
- Correctness smoke for the candidate passed:
  `cargo fmt --check` and
  `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-direct-dml-pageio-target cargo test -p fsqlite-vdbe shared_txn_page_io --profile release-perf -- --nocapture`
  (`15` matching tests). A broader `fsqlite-core` filtered test attempt was
  killed after the remote command ran silently for more than ten minutes, so
  the keep/revert decision used benchmark evidence instead.
- Evidence artifacts:
  `tests/artifacts/perf/direct-dml-pageio-reuse-candidate-purplecoast-20260505T1640Z/baseline-update-report.json`,
  `tests/artifacts/perf/direct-dml-pageio-reuse-candidate-purplecoast-20260505T1640Z/update-report.json`,
  `tests/artifacts/perf/direct-dml-pageio-reuse-candidate-purplecoast-20260505T1640Z/baseline-insert-report.json`,
  and
  `tests/artifacts/perf/direct-dml-pageio-reuse-candidate-purplecoast-20260505T1640Z/candidate-insert-report.json`.
- Result: rejected. Same-machine A/B showed the INSERT FrankenSQLite median
  geomean improved only `0.9%` while the C-relative geomean regressed `2.2%`
  (`25` scenarios, `14` FSQLite medians up and `11` down). UPDATE/DELETE was
  effectively flat on FSQLite geomean (`0.36%` slower), regressed the tiny
  delete row by `21.7%`, and regressed the C-relative geomean by `13.9%`.
- Do not retry direct DML `SharedTxnPageIo` wrapper reuse as a standalone
  optimization. The allocation avoided here is too small and too noisy relative
  to row-build, B-tree, pager, WAL, and benchmark fixed costs.

## 2026-05-05 - Stage-only external quick-balance retained hint

Scope: prepared direct INSERT rightmost-leaf append path, after profiles showed
large-row time in B-tree quick-balance and `PageData` clone/retention around
`try_quick_balance_on_external_rightmost_leaf_hint`.

- Touched during rejected candidate: `crates/fsqlite-btree/src/cursor.rs`;
  source was reverted after measurement.
- Candidate shape: after `balance_quick_known_divider_rowid`, skip retaining
  the new leaf `PageData` in the caller-owned external `TableAppendHint` when
  the pager can mutate staged `PageData` directly. The measured version also
  preserved the old retained-page behavior for non-staged PageWriters and added
  a staged-page quick-balance fallback when the staged hinted leaf fills.
- Correctness note: the first stage-only attempt was rejected before
  benchmarking because
  `test_table_try_append_cached_rightmost_leaf_hint_reuses_retained_leaf_image`
  found row-order corruption (`59` expected, `95` observed). The measured
  staged-capability guarded candidate passed the focused clean-worktree proofs:
  `cargo fmt --check`,
  `cargo test -p fsqlite-btree table_try_append_cached_rightmost_leaf_hint --profile release-perf -- --nocapture`
  (`4` matching tests), and
  `cargo test -p fsqlite-core prepared_direct_simple_insert_implicit_rowid --profile release-perf -- --nocapture`
  (`3` matching tests). Shared worktree verification was blocked at the time by
  an unrelated dirty `crates/fsqlite-pager/src/pager.rs` compile error, so the
  proof and benchmark used a clean detached worktree at `f7ea3cdd`.
- Evidence artifacts:
  `tests/artifacts/perf/stage-only-qb-hint-purplecoast-20260505T1716Z/baseline-insert-report.json`,
  `tests/artifacts/perf/stage-only-qb-hint-purplecoast-20260505T1716Z/candidate-insert-report.json`,
  `tests/artifacts/perf/stage-only-qb-hint-purplecoast-20260505T1716Z/ab-summary.json`,
  and
  `tests/artifacts/perf/stage-only-qb-hint-purplecoast-20260505T1716Z/summary.md`.
- Result: rejected. Same-window insert quick matrix had `10` FSQLite median
  wins and `15` regressions, with FSQLite geomean `1.0254x`
  candidate/baseline (`2.54%` slower). C-relative ratio geomean improved to
  `0.9590x`, but this was driven by C-side timing movement rather than absolute
  FSQLite improvement. The target `large_10col` 10K single-txn row improved
  `37.483 ms -> 36.182 ms`, but record-size `large_10col` 10K regressed
  `35.613 ms -> 36.716 ms`; small/medium rows regressed materially, including
  `small_3col` 1000 `+18.0%` and small transaction-strategy 10K single txn
  `+11.3%`.
- Do not retry this stage-only retained-hint clone avoidance as a standalone
  B-tree optimization. The retained leaf image is a useful fallback/rollback
  shape, and removing it does not improve the end-to-end insert matrix even
  when correctness is preserved for staged-capable writers.

## 2026-05-05 - Large borrowed WAL commit threshold

Scope: `comprehensive-bench --quick --filter insert`, targeting the large-row
commit path after `insert-commit-profile-cyangorge-20260505T1615Z` showed
`pager::build_group_commit_batch` cloning staged pages into owned
`TransactionFrameBatch` frames.

- Touched during reverted candidate: `crates/fsqlite-pager/src/pager.rs`.
- Candidate shape: promote the borrowed `collect_wal_commit_batch` helper out
  of test-only code and, for commits with at least `512` frames, bypass the
  owned group-commit batch by appending borrowed frame refs directly while
  still checking the pinned WAL conflict snapshot, using prepared-frame
  validation, taking the DB-file `Reserved` lock, honoring sync policy, and
  updating `inner.db_size`.
- Correctness checks: `cargo test -p fsqlite-pager test_collect_wal_commit_batch
  -- --nocapture` passed (`4` tests), and `cargo test -p fsqlite-pager
  group_commit -- --nocapture --test-threads=1` passed (`22` tests). The same
  `group_commit` filter without serialized test execution showed existing
  fault-hook interference between tests, so the serialized rerun was used for
  the candidate check.
- Evidence artifact:
  `tests/artifacts/perf/group-commit-large-borrowed-cyangorge-20260505T1650Z/summary.md`.
- Result: abandoned/reverted. The benchmark run was contaminated by an
  unrelated dirty `crates/fsqlite-btree/src/cursor.rs` diff that appeared while
  measuring, but the candidate was not promising enough to justify an isolated
  repeat: weighted insert score worsened `1.699053 -> 1.787694`, geomean ratio
  worsened `2.362302x -> 2.390798x`, `write_bulk` worsened `2.515348x ->
  2.526914x`, and `write_single` worsened `1.490767x -> 1.592921x`. Target
  FSQLite medians did not improve cleanly (`large_10col` 10K
  `36.165071 ms -> 37.493052 ms`, record-size large 10K
  `37.055950 ms -> 37.160930 ms`, record-size medium 10K
  `9.888943 ms -> 11.164965 ms`).
- Do not retry this exact borrowed large-commit threshold without an isolated
  A/B and a proof that bypassing the queue still preserves the group-commit
  fault/publish semantics under concurrent writers.

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

## 2026-05-05 - Quick-balance one-cell pointer Vec pooling

Scope: insert-only comprehensive e2e matrix after
`199bd14b perf(btree/balance): gate balance_quick on the exact divider size`,
targeting the quick-balance success path in
`crates/fsqlite-btree/src/cursor.rs`.

- Candidate shape: add a helper that takes a `Vec<u16>` from the existing
  thread-local cell-pointer pool and pushes the single new-cell pointer,
  replacing the two `vec![result.new_cell_ptr]` allocations used after
  `balance_quick_known_divider_rowid` succeeds.
- Behavior proof: `env CARGO_TARGET_DIR=/data/tmp/frankensqlite-cyangorge-check-target
  cargo test -p fsqlite-btree rightmost_leaf_hint -- --nocapture` passed
  (8 tests).
- Evidence: baseline artifact
  `tests/artifacts/perf/insert-quick-balance-exact-space-cyangorge-20260505T115109Z/report.json`;
  candidate artifact
  `tests/artifacts/perf/insert-quick-balance-pointer-pool-cyangorge-20260505T120405Z/report.json`
  and `run.log`.
- Result: rejected and reverted. The summary ratios looked better, but they
  were distorted by C SQLite variance. Engine-side medians regressed on the
  split-heavy single-transaction rows (`large_10col` 10K `34.756 ms` ->
  `37.287 ms`, 100K `415.902 ms` -> `451.660 ms`) and the hot counter moved
  the wrong way (`btree_quick_balance_ns` for `large_10col` 10K `4.309 ms` ->
  `5.262 ms`). Do not retry the one-cell pooled-Vec helper unless allocator
  profiling proves those tiny `Vec` allocations dominate and the thread-local
  pool access can be made cheaper than allocation.

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

## 2026-05-05 - Direct UPDATE fixed-width REAL payload-range patch

Scope: `perf-update-delete 10000 40 update`, targeting the prepared
`UPDATE bench SET value = ?2 WHERE id = ?1` direct-simple fixed-width REAL
path after the one-byte header-offset candidate still left full-payload copy and
same-size overwrite work in the hot path.

- Touched during rejected candidate: `crates/fsqlite-btree/src/cursor.rs` and
  `crates/fsqlite-core/src/connection.rs`.
- Candidate shape: add a B-tree helper that borrows the current local
  no-overflow table payload for record-header inspection, plus a second helper
  that patches only the 8-byte REAL value range in the current leaf payload.
  The direct UPDATE path used these helpers to avoid `BtCursor::payload_into`
  and avoid copying the whole payload back through
  `table_overwrite_current_payload_same_size_no_overflow`.
- Behavior proof: focused B-tree helper test passed, and
  `test_direct_simple_update_single_real_column_patches_payload_without_decode`
  passed after adding an assertion that the fixed-width REAL path performs zero
  local-payload copy calls.
- Evidence: paired release-perf hyperfine artifact
  `tests/artifacts/perf/direct-update-real-range-patch-candidate-cyangorge-20260505T0900Z/hyperfine-update.json`.
- Result: rejected and reverted. Baseline averaged `348.6 ms +/- 5.7 ms`;
  candidate averaged `354.1 ms +/- 8.2 ms`, so the unpatched binary was
  `1.02x +/- 0.03` faster. Do not retry this two-helper payload-range patch as
  an UPDATE microcopy optimization unless a fresh profile proves payload copy is
  again dominant and the B-tree helper overhead has been removed or amortized.

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

## 2026-05-05 - CASS follow-up: stale targets and older no-retry artifacts

Scope: second CASS sweep restricted to FrankenSQLite last-two-month history,
using negative-result terms such as `rejected`, `reverted`, `slower`,
`regressed`, `abandon*`, `did not help`, `within noise`, `worse`, and
`rollback`, then cross-checking matching repo artifacts before adding entries.

- Pre-prepared-statement benchmark ratios are stale routing evidence, not
  current engine targets. March CASS records show a large artificial penalty
  where FrankenSQLite benchmark loops used dynamic `execute(format!(...))`
  while the C SQLite side used prepared statements; commit
  `473f82c3 perf(e2e): convert benchmarks to prepared statements for
  structurally fair comparisons` fixed that class. Do not reuse the old
  `read_count_star 275x` / read-heavy ratios as current target selection
  without rerunning the current benchmark matrix. Do not count benchmark-harness
  rewrites as engine wins unless the asymmetry still exists in current code.
- Tiny ASCII `lower()` / `upper()` stack-buffering in
  `crates/fsqlite-func/src/builtins.rs` was rejected after the string-function
  row failed to show a clean end-to-end win. Evidence:
  `tests/artifacts/perf/string-small-ascii-case-purplecoast-20260504T1940Z/summary.md`.
  Do not retry this exact tiny-ASCII case-conversion lever without a cleaner
  A/B harness and all affected string-function rows improving.
- JSON path array-index ASCII parsing in
  `crates/fsqlite-ext-json/src/lib.rs::resolve_path` was rejected. Forward
  A/B favored baseline (`711.238 ms` vs `731.814 ms`), reverse A/B favored the
  candidate only noisily (`726.703 ms` vs `717.422 ms`). Evidence:
  `tests/artifacts/perf/20260428T1845Z-icybluff-json-path-index/RESULT.md`.
  Do not retry local digit parser specialization for JSON paths unless a
  process-level benchmark clears the stability bar.
- WAL frame assembly v2, which built a local 24-byte frame header and appended
  header plus payload instead of the committed field-by-field helper, was
  rejected because current-head v1 was slightly faster (`327.444 ms` vs
  `330.427 ms`). Evidence:
  `tests/artifacts/perf/20260428T0920Z-icybluff-wal-frame-assembly/RESULT.md`.
  Keep the existing `push_wal_frame_bytes` shape unless a fresh WAL benchmark
  shows a real frame-assembly hotspot.
- WAL checksum `then_aligned_bytes` streaming was rejected as within noise:
  candidate `329.915 ms` versus baseline `331.209 ms`, a `0.39%` delta inside
  run sigma. Evidence:
  `tests/artifacts/perf/20260428T0900Z-icybluff-wal-checksum/RESULT.md`.
  Do not retry checksum-transform reshaping based on sub-1% microbench movement.
- B-tree delete sort-record narrowing was rejected. Replacing
  `(usize, usize, usize)` triples with a compact `CellMove` did not improve the
  target path; longest check was flat/slower overall (`7885 ms` to `7902 ms`)
  while delete regressed by about `11.3%`. Evidence:
  `tests/artifacts/perf/20260427T1855Z-azurepine-btree-sort-record/RESULT.md`.
  Do not retry by shrinking the carried sort record width alone.
- Compact table-leaf delete sub-ideas: deferred scratch reuse and unrefined
  physical-neighbor delete were both rejected while the refined accepted path
  was kept. Deferred scratch reuse showed no measured win, and applying the
  physical-neighbor path to all compact leaves regressed delete-only. Evidence:
  `tests/artifacts/perf/20260427T2348Z-snowyfortress-next-hotspot/RESULT.md`.
  Do not replace the cheaper descending fast path or reintroduce scratch reuse
  without a delete-only win.
- Profile-pass hypotheses rejected as primary causes: syscall I/O and
  lock/futex contention were explicitly ruled out as first targets. Evidence:
  `tests/artifacts/perf/20260424T212631Z-profile-pass/HYPOTHESIS_LEDGER.md`
  and `tests/artifacts/perf/20260424T212631Z-profile-pass/REPORT.md`.
  For mixed/insert OLTP, start from row materialization, decode, cursor
  traversal, commit maintenance, memdb reload, and snapshot cloning before
  spending another pass on syscall or futex tuning.

Primary CASS evidence for the stale-target and false-lead guardrails:
- `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-09T05-08-84f3c374.json -n 42 -C 12`
- `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-09T22-55-5b9da3d6.json -n 153 -C 24`
- `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-09T05-09-1bf54aa9.json -n 267 -C 28`

## 2026-05-05 - CASS/artifact follow-up: older measured rejects

Scope: additional last-two-month CASS pass over the user-suggested negative
terms, then cross-checking older April artifact bundles that the CASS hits
pointed back toward. These are not broad design opinions; each item had a
measured reject or focused-test rollback.

- Mixed-OLTP record-header length microparser: replacing the serial-type length
  branch in `parse_record_header_into` with direct `SMALL_TYPE_SIZES` table use
  was rejected. The quick mixed baseline envelope was `1.399 s` and `1.425 s`,
  while candidate repeats were `1.390 s` and `1.518 s`; the average after-run
  was slower and the patch was rolled back. Evidence:
  `tests/artifacts/perf/20260424T2334Z-optimization-pass/RESULT.md`. Do not
  retry record-header length table reshuffling as an isolated mixed-OLTP lever;
  the later two-byte-header insert rejects reinforce that header microparsing
  only matters when a full matrix row moves.
- Delete sort insertion threshold: raising
  `sort_cells_desc_by_ptr::INSERTION_SORT_THRESHOLD` from `20` to `64` passed
  the focused sort-order proof but failed the wall-clock confirmation. The
  500-iteration delete run regressed from `5470.7 ms` to `5579.3 ms`, and the
  500-iteration `both` delete phase regressed from `1205.3 ms` to `1217.7 ms`.
  Evidence:
  `tests/artifacts/perf/20260427T2045Z-azurepine-delete-sort-threshold/RESULT.md`.
  Keep the threshold at `20`; do not tune it upward from a sort microbench
  without a delete/both e2e win.
- Delete large-N monotonic pre-scan removal: removing the pre-scan in
  `sort_cells_desc_by_ptr` improved local sort microbench cases, but the e2e
  `both` workload regressed within noise (`1.566 s` to `1.578 s`) and
  delete-only was only `1.01x +/- 0.03`, below the keep bar. Evidence:
  `tests/artifacts/perf/20260427T2235Z-snowyfortress-sort-prescan/RESULT.md`.
  Do not remove the pre-scan based on local sort timings; the accepted packed
  gap-shift path was the useful part of that pass.
- Early prepared direct INSERT zero-copy writer: an attempt to serialize
  prepared direct INSERT records directly into retained rightmost-leaf page
  space was fully rolled back before benchmarking because focused
  `direct_simple_insert` tests exposed unsafe retained/autocommit validation
  behavior (`29 passed`, `2 failed`). Evidence:
  `tests/artifacts/perf/20260428T0106Z-snowyfortress-post-compact/RESULT.md`.
  This is an earlier correctness-abandoned version of the later measured
  retained-leaf writer reject; do not re-enter this route without first
  isolating the retained/autocommit validation surface.

Primary CASS evidence that led back to these older bundles:
- `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-09T05-08-a1108e5a.json -n 120 -C 35`
- `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-09T22-55-68d80f81.json -n 118 -C 24`

## 2026-05-05 - CASS follow-up: correctness-abandoned fast paths

Scope: last-60-day CASS search for the user-suggested negative terms. Direct
`--workspace /data/projects/frankensqlite` searches returned no hits for
`rejected`, `reverted`, `slower`, and `within noise`, so the follow-up searched
`frankensqlite <term>` and accepted only source paths or titles clearly tied to
this repo, especially `/home/ubuntu/.gemini/tmp/frankensqlite`.

- Prepared DML direct-VDBE execution bypass: a March optimization pass started
  changing prepared statements so DML could execute the stored `VdbeProgram`
  directly instead of re-entering `execute_statement_dispatch`, but abandoned
  the idea after reading the dispatch path. The reason is semantic, not just
  performance noise: DML dispatch owns trigger firing, FK enforcement,
  constraint handling, autocommit wrapping, and complex fallback routing. Do not
  retry by simply calling the precompiled VDBE program from
  `execute_prepared_with_params` for `INSERT`, `UPDATE`, or `DELETE`. A viable
  retry must first design a semantic-preserving prepared-DML executor that
  carries all trigger/FK/constraint/autocommit behavior, then prove it with
  DML correctness tests before any matrix benchmark.
- Whole-engine async/asupersync rewrite as an immediate perf lever: CASS
  contains conflicting March analyses, with one session arguing FrankenSQLite
  was leaving asupersync runtime benefits on the table and creating async VFS /
  pager / B-tree / VDBE migration beads, while a sibling session argued the
  synchronous `Cx` bridge is the intentional compatibility design. Treat this
  as architecture plan-space, not a rejected micro-optimization and not a
  substitute for current matrix profiling. Do not spend a performance campaign
  pass on "make the engine async" unless it is picked up as a tracked
  architecture epic with FFI/WASM compatibility, cancellation, and e2e logging
  gates.

Primary CASS evidence:
- `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-08T22-16-ee1022e3.json -n 27 -C 6`
- `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-07T20-25-52485ea5.json -n 13 -C 6`
- `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-07T20-28-be5f24f8.json -n 9 -C 6`

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

## 2026-05-05 - Pager sorted write-page append fast path

- Target: `INSERTThroughput` quick insert matrix, especially split-heavy 10K
  single-transaction rows where the pager maintains `write_pages_sorted` before
  WAL commit publication.
- Touched during rejected candidate:
  `crates/fsqlite-pager/src/pager.rs::insert_page_sorted`.
- Candidate shape: check the current last sorted page first, append when the
  new page number is greater, return on duplicate-last, and fall back to the
  existing binary-search insertion only for out-of-order page numbers.
- Evidence:
  - Baseline:
    `tests/artifacts/perf/insert-profile-current-head-cyangorge-20260505T122449Z/report.json`.
  - Candidate:
    `tests/artifacts/perf/insert-sorted-page-append-cyangorge-20260505T1450Z/report.json`.
  - Candidate summary:
    `tests/artifacts/perf/insert-sorted-page-append-cyangorge-20260505T1450Z/summary.md`.
  - Focused pager sorted-order tests passed under `rch exec -- env
    CARGO_TARGET_DIR=/data/tmp/frankensqlite-cyangorge-sorted-page-target cargo
    test -p fsqlite-pager sorted -- --nocapture`; `cargo fmt --check` also
    passed before the benchmark run.
- Result: rejected and manually reverted before commit. The primary weighted
  score worsened from `1.6991` to `1.7591`, average ratio from `2.4610x` to
  `2.5153x`, and geomean ratio from `2.3623x` to `2.4081x`. The important
  10K single-transaction rows did not produce a usable win: `small_3col`
  worsened from `6.895 ms` to `7.105 ms`, `large_10col` worsened from
  `36.165 ms` to `36.909 ms`, and only `medium_6col` improved
  (`13.666 ms` to `12.944 ms`). Do not retry this standalone
  append/equal-last `write_pages_sorted` micro-optimization unless a fresh
  profile shows sorted-page maintenance dominating and a full insert-section
  A/B improves the primary weighted score and the large-row medians.

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

## 2026-05-05 - VDBE IntDivide opcode for rowid-bucket GROUP BY

- Target: remaining read-aggregate gap, especially
  `100 rows / SUM + GROUP BY (~10 groups)`.
- Touched during rejected candidate: `crates/fsqlite-types/src/opcode.rs`,
  `crates/fsqlite-vdbe/src/lib.rs`, `crates/fsqlite-vdbe/src/engine.rs`,
  and `crates/fsqlite-vdbe/src/codegen.rs`.
- Candidate shape: add a custom `Opcode::IntDivide`, emitted only by
  `codegen_select_group_by_rowid_bucket_sum`, to fast-path already-integer
  `rowid / divisor` before falling back to ordinary `Divide` semantics.
- Evidence:
  - Correctness:
    `cargo fmt --check` passed.
    `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-purplecoast-intdivide-test-target cargo test -p fsqlite-types opcode_ -- --nocapture`
    passed.
    `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-purplecoast-intdivide-test-target cargo test -p fsqlite-vdbe rowid_bucket -- --nocapture`
    passed.
    `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-purplecoast-intdivide-test-target cargo test -p fsqlite-vdbe divide -- --nocapture`
    passed.
    `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-purplecoast-intdivide-test-target cargo test -p fsqlite-vdbe division -- --nocapture`
    passed.
  - Same-host A/B reports:
    `tests/artifacts/perf/read-groupby-intdivide-clean-current-peer-baseline-purplecoast-20260505T082235Z/report.json`
    and
    `tests/artifacts/perf/read-groupby-intdivide-candidate-current-peer-purplecoast-20260505T082725Z/report.json`.
  - Repeat remote run log:
    `tests/artifacts/perf/read-groupby-intdivide-repeat-purplecoast-20260505T0926Z/run.log`.
    RCH did not retrieve the ignored `tests/artifacts/.../report.json`, so
    treat this as corroborating log evidence only, not the primary artifact.
- Result: rejected and reverted. The same-host read weighted score improved
  from `0.25776` to `0.24784`, but the targeted FrankenSQLite medians did not
  justify a new opcode: 100-row group-by improved only `0.022081 ms` to
  `0.021861 ms`, 1000-row group-by improved only `0.119825 ms` to
  `0.119293 ms`, and 10000-row group-by regressed from `1.111733 ms` to
  `1.162087 ms`. The apparent section-score and ratio wins were mostly C
  SQLite timing noise and unrelated read-single movement, while the remaining
  100-row group-by gap stayed open.
- Do not retry this by adding a narrow arithmetic opcode or by special-casing
  `Divide` dispatch for the rowid-bucket aggregate path. Reconsider only if a
  fresh bytecode profile proves division dispatch itself dominates the current
  workload and a same-window A/B improves FrankenSQLite absolute medians at
  all row counts plus the read-section weighted score.

## 2026-05-05 - Explicit transaction retained count/sum insert hook early return

- Target: insert throughput e2e matrix, especially explicit
  single-transaction insert rows.
- Touched during rejected candidate: `crates/fsqlite-core/src/connection.rs`.
- Candidate shape: return early from
  `retained_autocommit_count_sum_cache_note_insert` when
  `self.in_transaction.get()` is true, on the theory that retained autocommit
  count/sum cache maintenance is irrelevant inside explicit transactions.
- Evidence:
  - Baseline:
    `tests/artifacts/perf/insert-countsum-explicit-baseline-cyangorge-20260505T0925Z/report.json`.
  - First candidate:
    `tests/artifacts/perf/insert-countsum-explicit-candidate-cyangorge-20260505T0931Z/report.json`.
  - Repeat baseline:
    `tests/artifacts/perf/insert-countsum-explicit-repeat-cyangorge-20260505T0932Z-baseline/report.json`.
  - Repeat candidate:
    `tests/artifacts/perf/insert-countsum-explicit-repeat-cyangorge-20260505T0933Z-candidate/report.json`.
- Result: rejected and reverted. The first pass looked mildly positive, but
  the repeat run failed the keep bar. Repeat candidate worsened primary
  weighted score from `1.9154` to `1.9516`, geomean ratio from `2.6390x` to
  `2.7181x`, FrankenSQLite absolute geomean from `2.3051 ms` to
  `2.3575 ms` (`+2.28%`), and FrankenSQLite absolute average from
  `6.3954 ms` to `6.5695 ms` (`+2.72%`). The largest repeat regression was
  record-size comparison 10K large_10col, `35.059 ms` to `37.517 ms`
  (`+7.01%`).
- Do not retry this as a standalone branch-elision micro-optimization.
  Reconsider only if retained autocommit cache maintenance is redesigned or a
  profile shows this exact hook dominating a retained-autocommit-only workload.

## 2026-05-05 - Exact transaction-control `execute` parse bypass

- Target: insert throughput e2e matrix, especially explicit
  single-transaction insert rows that call `execute("BEGIN;")` and
  `execute("COMMIT;")`.
- Touched during rejected candidate: `crates/fsqlite-core/src/connection.rs`.
- Candidate shape: add an exact-string fast path in `Connection::execute` for
  `BEGIN`, `BEGIN;`, `COMMIT`, `COMMIT;`, `ROLLBACK`, and `ROLLBACK;`, calling
  the existing direct transaction helpers after `background_status()` and
  incrementing `note_connection_statement_execution_count(1)` only after the
  operation succeeds.
- Evidence:
  - Correctness proof passed before rejection:
    `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-purplecoast-exact-txn-test-target cargo test -p fsqlite-core test_execute_exact_transaction_controls_skip_sql_parse_and_count_success -- --nocapture`
    showed zero parser calls and correct successful-execution stats. RCH then
    hung in post-test target artifact retrieval; the test body itself passed.
  - Existing guard still passed:
    `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-purplecoast-exact-txn-test-target cargo test -p fsqlite-core test_file_backed_begin_transaction_api_skips_sql_parse -- --nocapture`.
  - Same-window baseline log:
    `tests/artifacts/perf/insert-exact-txn-baseline-purplecoast-20260505T101018Z/run.log`.
  - Same-window candidate log:
    `tests/artifacts/perf/insert-exact-txn-candidate-purplecoast-20260505T103455Z/run.log`.
    RCH did not retrieve the ignored JSON reports for this run, so treat these
    logs as the measurement artifact.
- Result: rejected and reverted. The local proof was real, but the matrix did
  not move in the right direction. Average time ratio worsened from `2.36x` to
  `2.55x`. Targeted FrankenSQLite medians were mixed or worse: single-txn
  tiny_1col 100 rows regressed from `299.9 us` to `336.1 us`, 1000 rows
  improved only from `836.0 us` to `805.4 us`, and 10000 rows regressed from
  `4.65 ms` to `4.87 ms`. Transaction-strategy small_3col single-txn rows
  regressed at all measured sizes: `219.1 us` to `267.1 us`, `1.04 ms` to
  `1.08 ms`, and `6.81 ms` to `7.12 ms`.
- Do not retry exact transaction-control parse bypass as a standalone
  optimization. Reconsider only if fresh profiles show `BEGIN`/`COMMIT` SQL
  parsing itself dominates the current insert workload and a repeated
  same-window A/B improves the absolute FrankenSQLite medians plus the
  insert-section score.

## 2026-05-05 - CASS last-60-day no-retry expansion

Scope: follow-up `cass` archaeology over the last 60 days, using a session set
from direct `/data/projects/frankensqlite` hits plus archived
`/home/ubuntu/.gemini/tmp/frankensqlite` sessions, then searching negative
signals including `rejected`, `reverted`, `abandoned`, `slower`,
`didn't help`, `did not help`, `no improvement`, `within noise`,
`regressed`, `worse`, `rollback`, `failed to improve`, `no measurable`, and
`revert it for now`. The attempted `cass index --json` refresh timed out in
the preparing phase, so these are evidence from the existing CASS index.

- Do not revive the `SqliteValue` `Arc<str>` / `Arc<[u8]>` conversion as a
  prerequisite for `Opcode::SCopy`, sorter, pseudo-cursor, or row-cache work.
  CASS shows it was attempted during the sorter/column-cache optimization pass,
  caused widespread cross-crate breakage, and was explicitly reverted back to
  `String`/`Vec<u8>` to regain a compilable state. This reinforces the older
  generic `SqliteValue` `Arc` entry: retry only with a designed serde and
  cross-crate migration plan, not as a local VDBE hot-path patch.
- Do not implement prepared DML execution by simply calling the compiled VDBE
  program and bypassing `execute_statement_dispatch`. CASS records the agent
  rejecting that shape after tracing DML dispatch: triggers, foreign keys,
  constraint enforcement, autocommit wrapping, and fallback paths live there.
  The acceptable shape is a precompiled-program hook that still preserves DML
  dispatch semantics; a direct bytecode-only shortcut is a correctness trap.
- Do not change the public `Row` representation from `Vec<SqliteValue>` to
  `SmallVec` as a standalone allocation optimization. CASS shows that the
  public-row `SmallVec` idea was reverted for API stability while keeping the
  internal VDBE `SmallVec` paths. Reconsider only with an explicit public API
  migration plan and downstream compatibility proof.
- Do not use the old raw-string `bench_insert` benchmark as the keep/reject
  proof for engine-level insert changes. CASS records an optimization pass that
  attacked serializer, VFS append, and hash-map hotspots but moved the benchmark
  only from about `0.271 s` to `0.265 s` because the benchmark itself generated
  10,000 distinct SQL strings and thrashed parse/codegen caches. Use the current
  prepared-statement matrix rows, or a same-window prepared insert microbench,
  before keeping engine patches.
- Treat `Opcode::MustBeInt`, `BtCursor::last` `at_eof`, active-transaction
  checkpoint blocking, and `with_pager_write_txn` active-transaction bypass as
  CASS false leads, not optimization targets. The mined sessions re-read those
  paths and concluded the current implementations were already handling the
  suspected issue or that the target was not a performance defect.

CASS evidence:
- `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-09T05-09-1bf54aa9.json -n 204 -C 45`
- `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-09T05-09-1bf54aa9.json -n 230 -C 80`
- `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-09T05-08-a1108e5a.json -n 120 -C 45`
- `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-08T22-16-ee1022e3.json -n 30 -C 25`
- `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-09T05-08-854547a1.json -n 140 -C 45`

## 2026-05-05 - CASS/git follow-up: reverted fast paths not yet named

Scope: another `cass` pass over the last 60 days restricted to FrankenSQLite
signals (`cass search "frankensqlite <term>" --days 60 ...`) for `reverted`,
`rollback`, `slower`, `worse`, `abandon*`, and related wording. The useful
new leads were then cross-checked against recent revert commits and preserved
artifact bundles. These entries are intentionally terse because they mainly
serve as search handles for future agents.

- `ensure_storage_cursor_row_layout` early-return fast path: reverted by
  `9dd7bc53`. The premise that a non-empty row decode table plus a large enough
  payload buffer meant the layout was reusable was false: multi-row cursor
  callers relied on the slow path to reset eager-value state. Do not re-add an
  early return here unless the guard also proves prior-row eager values cannot
  leak, with correctness coverage before any read benchmark.
- Prepared indexed-equality text/null side maps: reverted by `53679a91`
  (`7d9814e5`). The idea added `SmallText` and NULL-specific rowid maps beside
  the generic `PreparedIndexedEqualityCache`, but was dropped before becoming a
  durable read win. This is distinct from the later last-result cache rejects:
  do not retry by adding parallel value-shape maps unless a profile proves
  generic lookup-key construction dominates and a read-section A/B improves
  absolute FrankenSQLite medians.
- B-tree cell-slot cache rotation experiment: reverted by `facba056`.
  Replacing remove/insert LRU promotion with slice rotation and special
  in-entry slot updates did not survive the measured/reviewed perf pass. Keep
  the simpler current promotion path; do not retry cache-order micro-rotation
  without a profile showing `CellSlotCache` promotion itself is hot and a join
  or read-index A/B win.
- VDBE index-prefix binary compare shortcut: rejected by `f7fce439`. The
  candidate bypassed the collation registry for apparently binary index
  prefixes, then was removed in favor of the single registry-backed
  `compare_index_prefix_keys` path. Do not retry a registry-free prefix compare
  unless the collation and DESC/null semantics are proven with focused tests and
  the index-boundary/read-query artifacts show a real row-level win.
- Prepared rowid-bucket `SUM` fast path family: reverted by `6d8a44f4` after
  the initial `SimpleGroupByRowidBucketSum` helper and later streaming variant
  failed the keep bar. Artifacts include
  `tests/artifacts/perf/read-after-write-group-by-rowid-bucket-sum-candidate-calm-20260503T2008Z/report.json`
  and
  `tests/artifacts/perf/read-after-write-rowid-bucket-stream-candidate-calm-20260503T2018Z/report.json`.
  Do not recreate a whole prepared fast-path variant for `rowid / divisor`
  grouped `SUM` unless all row counts and the read-section score improve, not
  just the largest row.
- UPDATE reinsert existence-probe skip: reverted by `8dd631d7`. The candidate
  skipped the existence probe when reinserting the same rowid during UPDATE,
  but the update/delete section was worse than the disabled comparison
  (`4.1226` weighted score versus `3.7545` in
  `tests/artifacts/perf/update-delete-reinsert-skip-candidate-chartreuse-20260504T0057Z/report.json`
  and
  `tests/artifacts/perf/update-delete-reinsert-skip-disabled-dirty-chartreuse-20260504T0101Z/report.json`).
  Do not retry this as a local `PendingUpdateRestore` shortcut without an
  UPDATE-only A/B win and conflict/unique-index coverage.
- Top-category CTE rowid-carry regression: reverted by `86944a1b`. The
  candidate carried rowids for top categories through the direct CTE helper and
  then had to be unwound back to the simpler rescan-by-category shape. Evidence
  lives in
  `tests/artifacts/perf/subquery-current-head-cte-rowid-carry-local-20260501T0523Z/`
  and
  `tests/artifacts/perf/subquery-current-head-cte-rowid-carry-reverted-local-20260501T0530Z/`.
  Do not retry by preserving per-category rowid vectors unless the subquery/CTE
  row improves in a same-window run and memory growth is bounded.
- Prepared ORDER BY LIMIT winner-maintenance path: rejected by `3bfd8fa1` and
  removed again by `0cb0379e`. The candidate kept the winners vector sorted via
  `partition_point` / insert on every replacement; the reverted shape returned
  to unsorted winner replacement plus one final sort. Do not retry per-row
  sorted winner insertion for the prepared ORDER BY LIMIT path unless a
  same-window read/order benchmark proves the maintenance cost is hot and
  absolute FrankenSQLite medians improve.
- Stack-layout record serializer cache: reverted by `be75bb57`. The candidate
  added fixed stack arrays for up to 16 values in `serialize_record_iter_into_impl`
  to cache value refs, serial types, and payload lengths, then was removed from
  `crates/fsqlite-types/src/record.rs`. Do not retry this stack-layout serializer
  cache as a generic record-write optimization; use the existing record
  serializer entries and require a record/insert matrix win before reintroducing
  stack layout state.
- Integer-key fast path for inner-join grouped aggregate: reverted by
  `19f0b188`. The candidate added `memdb_integer_join_key_with_source`,
  `PreparedJoinGroupState`, and an integer-key grouped-join implementation
  beside the generic hash-key path, then was dropped back to the generic grouping
  flow. Do not retry a separate integer-only join grouping path unless join
  artifacts show generic `HashableJoinKey` construction dominates and all
  affected grouped-join rows improve.
- Direct DML cursor scratch routing: reverted by `80777b6b`; artifact bundle
  `tests/artifacts/perf/20260428T1743Z-sapphirecrane-direct-dml-cursor-scratch/RESULT-direct-dml-cursor-scratch.md`
  was preserved. This reinforces the existing direct-DML scratch no-retry rule:
  do not route INSERT/UPDATE/DELETE through shared cursor scratch as a local
  hot-path cleanup without a full correctness and update/delete matrix proof.

## 2026-05-05 - Conservative WAL raw append for large INSERT commits

Scope: `comprehensive-bench --quick --filter insert`, targeting the default
conservative WAL path in
`crates/fsqlite-pager/src/pager.rs::commit_wal_group_commit_with_snapshot`
after insert profiling showed 2014-frame `large_10col` commits spending
several milliseconds in prepared-frame construction and WAL append.

- Candidate shape: when `ParallelWalFallbackReason::OperatorForced` selected
  conservative mode and no lane-prepared batch was available, skip
  `wal.prepare_append_frames` / `finalize_prepared_frames` and fall through to
  the existing fused `wal.append_frames` raw append path.
- Evidence: baseline
  `tests/artifacts/perf/insert-profile-after-wal-default-cyangorge-20260505T1022Z/report-insert-profile.json`;
  candidate
  `tests/artifacts/perf/insert-profile-after-wal-default-cyangorge-20260505T1022Z/report-insert-raw-conservative-candidate.json`;
  candidate profile
  `tests/artifacts/perf/insert-profile-after-wal-default-cyangorge-20260505T1022Z/run-insert-raw-conservative-candidate.log`.
- Result: rejected and reverted. Insert geomean worsened `2.384x -> 2.444x`;
  write-bulk geomean worsened `2.546x -> 2.623x`; p99 worsened
  `4.301x -> 4.460x`. The motivating large rows regressed badly:
  `Single Transaction large_10col 10000` F median `35.404 ms -> 43.130 ms`,
  and `Record Size Comparison large_10col 10K` F median
  `34.613 ms -> 49.192 ms`.
- Do not retry raw conservative WAL append as a standalone prepared-batch
  bypass. Revisit only if a new design preserves prelock prepared-frame
  construction while reducing its transform/buffer cost, and proves the
  large-row insert section improves in a same-window matrix.

## 2026-05-05 - Private `:memory:` WAL commit bypass

Scope: `comprehensive-bench --quick --filter insert`, targeting private
`/:memory:` pager commits in `crates/fsqlite-pager/src/pager.rs` after insert
profiles showed large-row single-transaction commits dominated by dirty-page
publication.

- Candidate shape: in a clean temporary worktree based on `71b6720f`, route
  `memory_db_bump_alloc` commits through direct private-memory page flushing
  before the WAL branch, skip WAL conflict prediction for private memory, and
  avoid synthetic page-1 rewrites for ordinary private-memory growth unless
  page 1 or the freelist was actually dirty. The candidate also made
  `commit_and_retain` defer private-memory VFS flushing when the retained
  writer could publish committed pages through its retained cache.
- Evidence:
  - Focused proof:
    `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-purplecoast-memcommit-target cargo test -p fsqlite-pager private_memory -- --nocapture`
    passed in the temporary worktree.
  - Baseline:
    `tests/artifacts/perf/private-memory-commit-base-purplecoast-20260505T1120Z/report.json`.
  - Candidate:
    `tests/artifacts/perf/private-memory-commit-candidate-purplecoast-20260505T1120Z/report.json`.
- Result: rejected and not applied to the shared worktree. The insert ratio
  summary looked better (`avg_ratio 2.283x -> 2.097x`, weighted score
  `1.6279 -> 1.4773`), but the absolute FrankenSQLite medians were worse:
  geomean time ratio `1.107x`, average time ratio `1.127x`, with `17/25`
  insert rows slower. Notable regressions included small_3col 10K autocommit
  `13.09 ms -> 21.48 ms`, small_3col 1K autocommit
  `1.52 ms -> 2.24 ms`, 100-row batched `218.9 us -> 323.6 us`, and
  large_10col 10K single transaction `42.18 ms -> 44.28 ms`.
- Do not retry private `:memory:` WAL bypass as a standalone pager shortcut.
  Revisit only with a same-window proof that improves absolute FrankenSQLite
  medians and the insert-section score; ratio-only gains are suspect because
  the C SQLite denominator can move enough to hide FrankenSQLite regressions.

## 2026-05-05 - PageData shared-pair quick-balance handoff

Scope: full `comprehensive-bench --filter insert` after the exact-divider
quick-balance win, targeting the full-page clone in
`crates/fsqlite-btree/src/balance.rs::balance_quick_known_divider_rowid`.

- Candidate shape: add `PageData::into_shared_pair()` in
  `crates/fsqlite-types/src/lib.rs` and use it to move the freshly split right
  sibling page into one `Arc<[u8]>`, handing one shared handle to the writer and
  one shared handle back to the rightmost-leaf cache.
- Evidence: first run
  `tests/artifacts/perf/insert-pagedata-shared-pair-cyangorge-20260505T121337Z/`;
  rerun
  `tests/artifacts/perf/insert-pagedata-shared-pair-rerun-cyangorge-20260505T121651Z/`;
  baseline
  `tests/artifacts/perf/insert-quick-balance-exact-space-cyangorge-20260505T115109Z/`.
- Result: rejected and reverted. The aggregate ratio moved in the right
  direction on the rerun (`geomean_ratio 2.3519x -> 2.1634x`, weighted score
  `1.7141 -> 1.6914`), but the split-heavy absolute FrankenSQLite medians
  regressed: `large_10col` single-transaction 10K
  `34.756 ms -> 38.651 ms`, and 100K `415.902 ms -> 444.772 ms`.
  The root cause is representation semantics: the existing `PageData::clone()`
  path pays one snapshot clone but keeps the cursor's new rightmost page backed
  by owned mutable bytes; the shared-pair variant made the cursor cache shared
  too, so the next append to that page pays copy-on-write.
- Do not retry by making both split-page handles shared. A future version would
  need a writer handoff that preserves an owned mutable page for the cursor, or
  a different rightmost-cache design, and must improve the large-row absolute
  medians in the same insert matrix.

## 2026-05-05 - Direct INSERT rowid-alias double-eval skip

Scope: `comprehensive-bench --quick --filter insert` after the current
`237261d2` full quick matrix showed the remaining biggest ratios clustered in
write-heavy insert rows. The candidate targeted the compiled direct-insert row
builder in `crates/fsqlite-core/src/connection.rs`.

- Candidate shape: after `eval_prepared_direct_simple_insert_explicit_rowid_only`
  had already evaluated the INTEGER PRIMARY KEY alias expression for append
  routing, skip re-evaluating the same compiled rowid/IPK expression in the
  row-build loop and push the storage `NULL` placeholder directly.
- Evidence: baseline insert profile
  `tests/artifacts/perf/insert-profile-current-head-cyangorge-20260505T122449Z/`;
  candidate
  `tests/artifacts/perf/insert-rowid-alias-skip-cyangorge-20260505T123625Z/`.
  Focused tests passed before the A/B:
  `cargo test -p fsqlite-core test_prepared_direct_simple_insert_autocommit_profile_breakdown -- --nocapture`
  and
  `cargo test -p fsqlite-core test_prepared_direct_insert_without_change_tracking_skips_tls_sync -- --nocapture`.
- Result: rejected and reverted. The insert section regressed
  (`geomean_ratio 2.3623x -> 2.4502x`, weighted score
  `1.6991 -> 1.7605`, p99 `4.1407x -> 4.3519x`). The targeted
  `large_10col` single-transaction 10K median improved only slightly
  (`36.165 ms -> 35.335 ms`) while the record-size `large_10col` 10K row
  regressed (`37.056 ms -> 37.477 ms`) and multiple smaller insert rows
  worsened.
- Do not retry rowid-alias double-eval skipping as a standalone direct-insert
  micro-optimization. The skipped expression is too cheap relative to row text
  construction, B-tree work, and commit publication, and the codegen perturbation
  did not move the matrix.

## 2026-05-05 - Direct INSERT concat owned-text move

Scope: `comprehensive-bench --quick --filter insert` with
`FSQLITE_BENCH_PROFILE_INSERT=1`, targeting the direct-simple INSERT concat
row builder in `crates/fsqlite-core/src/connection.rs` after profiles showed
large-row `row_build_ns` around 5-6 ms for 10K-row large-record inserts.

- Candidate shape: keep inline-size concat strings on the existing borrowed
  `SmallText::new` path, but for longer concat results move the reusable
  `String` scratch into `SmallText::from_string` instead of copying
  `text_scratch.as_str()` into a second heap string.
- Evidence: same-window baseline
  `tests/artifacts/perf/insert-concat-owned-text-baseline-cyangorge-20260505T124529Z/`;
  candidate
  `tests/artifacts/perf/insert-concat-owned-text-cyangorge-20260505T125310Z/`.
  Focused proof tests passed before the A/B:
  `cargo test -p fsqlite-core test_prepared_direct_simple_insert_autocommit_profile_breakdown -- --nocapture`
  and
  `cargo test -p fsqlite-core test_prepared_direct_insert_without_change_tracking_skips_tls_sync -- --nocapture`.
- Result: rejected and reverted. Insert geomean regressed
  `2.2471x -> 2.5245x`, weighted score regressed `1.6366 -> 1.7467`,
  and p99 regressed `3.7572x -> 4.4258x`. The target large rows also
  regressed in absolute FrankenSQLite medians:
  `large_10col` single-transaction 10K `35.292 ms -> 43.055 ms`, and
  record-size `large_10col` 10K `36.379 ms -> 41.902 ms`.
- Do not retry concat owned-string moving as a standalone direct INSERT row
  builder optimization. The root cause is allocator locality: moving the
  scratch avoids one copy but destroys scratch-capacity reuse, forcing the hot
  concat builder to reallocate repeatedly. Future row-build work should avoid
  materializing transient `SqliteValue::Text` for lazy `:memory:` inserts or
  serialize concat output directly into a record/page destination with a
  same-window insert matrix win.

## 2026-05-05 - Quotient-filter empty-map maintenance skip

Scope: direct INSERT per-row bookkeeping in
`crates/fsqlite-core/src/connection.rs`, after insert profiles showed
substantial execute-body time not fully covered by row-build, serialization,
B-tree, and commit counters. The candidate targeted `qf_record_insert` /
`qf_record_delete`, which are called after successful direct-simple INSERT and
DELETE maintenance.

- Candidate shape: return early when `self.quotient_filters.borrow().is_empty()`
  before taking the existing mutable borrow and attempting a root-page lookup.
  The intended fast path was benchmark-style INSERT workloads where no
  quotient filter has been seeded yet, making QF maintenance a logical no-op.
- Evidence: correctness gate failed before benchmarking:
  `cargo test -p fsqlite-core quotient_filter -- --nocapture`. Artifact note:
  `tests/artifacts/perf/insert-qf-empty-skip-cyangorge-20260505T1256Z/summary.md`.
- Result: rejected and reverted before A/B measurement. Two existing tests
  failed: `test_quotient_filter_short_circuits_absent_rowids_on_delete`
  reported `expected >= 90 QF short-circuits, got 0`, and
  `test_quotient_filter_delete_then_redelete_short_circuits` reported that the
  second delete of a removed rowid did not short-circuit.
- Do not retry an empty-map early return in QF maintenance without first
  reworking the lazy seed lifecycle. The empty-map state is not merely an
  inert "disabled" state; it can be part of the path that lets later DELETE /
  UPDATE consultation seed and maintain the filter correctly.

## 2026-05-05 - Retained autocommit count-sum explicit transaction skip

Scope: direct-simple INSERT per-row bookkeeping in
`crates/fsqlite-core/src/connection.rs`, after insert profiles showed large
unaccounted execute-body time beyond row-build, serialization, B-tree insert,
and commit counters. The candidate targeted
`retained_autocommit_count_sum_cache_note_insert`, which runs after successful
direct-simple INSERT.

- Candidate shape: return early from
  `retained_autocommit_count_sum_cache_note_insert` when
  `self.in_transaction.get()` is true. Explicit `BEGIN..COMMIT` insert
  workloads cannot seed the retained autocommit count/sum cache because
  `maybe_seed_retained_autocommit_count_sum_cache_from_clean_memdb` already
  returns inside a transaction, so the candidate tried to avoid one per-row
  cache path.
- Evidence: same-window baseline
  `tests/artifacts/perf/insert-concat-owned-text-baseline-cyangorge-20260505T124529Z/`;
  candidate
  `tests/artifacts/perf/insert-retained-cache-explicit-skip-cyangorge-20260505T130650Z/`.
  Focused correctness/build gates passed before the A/B:
  `cargo fmt --check`,
  `cargo test -p fsqlite-core retained_autocommit_count_sum_cache -- --nocapture`,
  `cargo test -p fsqlite-core test_prepared_direct_simple_insert_autocommit_profile_breakdown -- --nocapture`,
  and
  `cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench`.
- Result: rejected and reverted. Insert geomean regressed
  `2.2471x -> 2.4574x`, weighted score regressed `1.6366 -> 1.7698`,
  and p99 regressed `3.7572x -> 4.0913x`. The target large rows also
  regressed in absolute FrankenSQLite medians:
  `large_10col` single-transaction 10K `35.292 ms -> 36.626 ms`, and
  record-size `large_10col` 10K `36.379 ms -> 36.733 ms`.
- Do not retry explicit-transaction skipping of retained-autocommit count/sum
  cache maintenance as a standalone direct INSERT optimization. The cache path
  is logically redundant for this workload, but the branch/codegen perturbation
  was not free and the benchmark matrix moved the wrong way.

## 2026-05-05 - Agent Mail CASS/git addenda: remaining no-retry shapes

Scope: patch-ready peer handoff from the last-60-day CASS/git negative-result
expansion while this agent held `docs/progress/perf-negative-results.md`.
Direct `/data/projects/frankensqlite` CASS workspace searches were sparse, so
the useful leads came from `cass search "frankensqlite <term>" --days 60`,
archived Gemini FrankenSQLite sessions, preserved artifacts, and recent revert
commits. Entries already present in this ledger were not duplicated.

- Broad and parent-only structural preclaim: rejected and reverted on the
  flagship `commutative_inserts_disjoint_keys / frankensqlite / c8` row. The
  broad shape preclaimed structural pages before split/rebalance writes via
  `crates/fsqlite-btree/src/cursor.rs` plus VDBE preclaim/rollback plumbing;
  the parent-only narrowing was even worse. Evidence includes
  `artifacts/perf/20260314_direct_handle_owned_fastpath_pass3/disjoint_c8_release_perf_both.jsonl`,
  `artifacts/perf/20260314_direct_handle_owned_fastpath_v2/disjoint_c8_release_perf_both.jsonl`,
  `artifacts/perf/20260314_structural_preclaim/disjoint_c8_release_perf_both.jsonl`,
  `artifacts/perf/20260314_parent_preclaim/disjoint_c8_release_perf_both.jsonl`,
  and `docs/planning/STATE_OF_THE_CODEBASE_AND_NEXT_STEPS.md`. Do not retry
  earlier deterministic claiming of shared B-tree structure as the concurrency
  fix; it lengthened the convoy and widened the effective choke point. Future
  work must reduce shared structural work, shorten hold duration, or change
  physical layout, and rerun the full focused c1/c4/c8 family.
- Quotient-filter build-on-first-consult for direct UPDATE/DELETE: rejected as
  a severe benchmark regression. The lazy build-on-first-consult path scanned
  the full table at the first DELETE/UPDATE after `Connection::open`; commit
  `4ea55010` records `update-deletethroughput__100-rows-delete-5-rows`
  regressing by about `369x` because a roughly `30 ms` scan was added to a
  roughly `0.1 ms` delete. Do not lazily build rowid membership filters on
  first DML consult for existing tables. Retry only with an explicit activation
  policy where build cost is known-zero or amortized outside the target
  operation, and prove the UPDATE/DELETE matrix moves.
- Mechanical `SqliteValue` Arc conversion via Python/cargo-fix traversal: do
  not repeat it. March CASS shows the text/blob Arc idea was attempted as a
  broad conversion across tests, property macros, and record/value helpers; it
  caused serde/type mismatches, mangled `record.rs` / `value.rs` patterns and
  test assertions, and required reverting to `String` / `Vec<u8>`. This is the
  process-specific variant of the broader Arc no-retry rule: retry only with a
  designed serde/API migration and narrow hand-edited proof, never as a
  mechanical repo sweep.
- Read-heavy `query_row_with_params()` wrapper swap: rejected. The
  `mt_read_bench` pass changed the FSQLite side from `query_with_params()` to
  `query_row_with_params()` and made the remote matrix worse (`0.05x`,
  `0.06x`, `0.07x`, `0.25x`), then reverted it before closeout. Evidence:
  `tests/artifacts/perf/read-heavy-20260430T021702Z/RESULT.md`. Do not retry a
  query/query_row wrapper substitution; the documented next lever is
  file-backed prepared MemDB direct lookup after read-state refresh.
- `concat_ws` pre-sizing scan: rejected as slower than the accepted direct
  append path. The direct append candidate measured `24,453,767 ns`, while the
  pre-sizing pass measured `34,885,096 ns` because the extra scan outweighed
  saved growth for the 24-text-argument benchmark. Evidence:
  `tests/artifacts/perf/20260428T2100Z-icybluff-concat-ws-direct/RESULT.md`.
  Keep the direct-append implementation; do not add a pre-size scan unless a
  new workload has much larger output growth and proves the scan pays for
  itself.
- Mixed-OLTP omitted rowid-alias projection remapping: rejected. The
  double-parse version averaged only about `0.6%` absolute FrankenSQLite
  improvement and the one-pass rewrite regressed repeat measurements, so both
  were rolled back. Evidence:
  `tests/artifacts/perf/20260425T1921Z-azurepine-alias-projection-fastpath/summary.md`.
  Do not retry IPK-alias projection remapping as an isolated COUNT/SUM lever
  unless the mixed matrix moves beyond the keep threshold.
- Manual integer decode assembly in `decode_big_endian_signed`: rejected.
  Absolute FrankenSQLite movement stayed under `1%` and normalized F/C ratio
  worsened despite passing direct sign-extension and integer-boundary proofs.
  Evidence:
  `tests/artifacts/perf/20260425T1921Z-azurepine-alias-projection-fastpath/summary.md`.
  Do not replace the current integer decoder with hand assembly for scalar
  microbench reasons alone.
- Rowid-only local leaf fast path for retained dirty-overlay range counting:
  rejected. F median improved, but the two-run average was only about `2.2%`
  faster than the accepted local-leaf payload-prefix baseline and stayed below
  the keep threshold; the patch was rolled back. Evidence:
  `tests/artifacts/perf/20260425T1921Z-azurepine-alias-projection-fastpath/summary.md`.
  Do not re-add a rowid-only local-leaf branch unless the retained range-count
  row is again a top matrix gap and clears the threshold.
- `xxh3_64` to `crc32c` for `page_mutation_counter`: rejected on this host.
  The April profiling handoff records the T5a experiment as reverted because
  `crc32c` was `28%` slower for 4 KiB inputs. Evidence:
  `tests/artifacts/perf/profiling-handoff-20260423T155542Z/campaign-summary.md`
  and `tests/artifacts/perf/bd-cnk5d-2t-cliff-verify-20260424/summary.md`.
  Do not swap hash functions because CRC32C sounds hardware-accelerated;
  require a same-host checksum/profile proof and a matrix win.
- `PublishedPagerState::new` / connection-open cost as a standalone target:
  false lead for production-style workloads. The profiling handoff marks it as
  connection-open cost visible in microbenches that open fresh connections, not
  an operation-count cost for long-lived connections. Evidence:
  `tests/artifacts/perf/profiling-handoff-20260423T155542Z/hypothesis-ledger.md`.
  Do not spend a perf pass optimizing this in isolation unless a connection
  pool or open-heavy workload is the explicit benchmark target.

## 2026-05-05 - Pager EOF page-lease batch size 8 -> 32

- Target: INSERT throughput, especially large single-transaction rows that
  allocate about 2K pages and show `page_pool_misses=2013` plus multi-ms B-tree
  quick-balance/commit time.
- Touched during rejected candidate: `crates/fsqlite-pager/src/pager.rs`
  (`PAGE_LEASE_BATCH_SIZE`). Reverted to `8` after measurement.
- Candidate shape: increase `PAGE_LEASE_BATCH_SIZE` from `8` to `32` so
  concurrent transactions pre-allocate follow-on EOF pages in larger batches,
  aiming to reduce repeated `inner` mutex acquisitions during right-edge B-tree
  splits.
- Evidence artifacts:
  `tests/artifacts/perf/page-lease-8-baseline-purplecoast-20260505T1316Z/report.json`
  and
  `tests/artifacts/perf/page-lease-32-candidate-purplecoast-20260505T1322Z/report.json`.
- Result: rejected and reverted. The focused insert matrix worsened overall
  average ratio from `2.36x` to `2.56x`. Primary large-row medians did not
  improve: `large_10col` single transaction FSQLite moved `37.57 ms` to
  `38.27 ms`, and record-size `large_10col` moved `36.99 ms` to `42.37 ms`.
  Medium 10K single transaction also worsened `14.28 ms` to `15.88 ms`; small
  10K worsened `7.25 ms` to `7.96 ms`.
- Do not retry as a standalone larger EOF lease batch. Reconsider only with a
  page-allocation profile showing `TransactionHandle::allocate_page`
  inner-lock acquisition dominating and an adaptive policy that preserves or
  improves the full insert matrix, especially the large record-size row.

## 2026-05-05 - External quick-balance owned page handoff

- Target: INSERT throughput rows that split the rightmost leaf through
  `try_quick_balance_on_external_rightmost_leaf_hint`, especially 10K
  `large_10col` single-transaction and record-size rows.
- Touched during rejected candidate: `crates/fsqlite-btree/src/cursor.rs`.
  The code was reverted after measurement.
- Candidate shape: on the external retained-hint quick-balance success path,
  move `result.new_page_data` directly into `hint.page_data` and clear
  `rightmost_leaf_cache` instead of cloning the page into the hint and storing
  another owned copy in the cursor-local cache.
- Correctness smoke:
  `cargo test -p fsqlite-btree test_table_try_append_cached_rightmost_leaf_hint -- --nocapture`
  passed (`4` tests).
- Evidence artifacts:
  `tests/artifacts/perf/qb-owned-handoff-baseline-dirtyconn-purplecoast-20260505T132841Z/report.json`,
  `tests/artifacts/perf/qb-owned-handoff-candidate-purplecoast-20260505T132443Z/report.json`,
  and
  `tests/artifacts/perf/qb-owned-handoff-candidate-repeat-purplecoast-20260505T133407Z/report.json`.
  A peer dirty-tree check reached the same disposition in
  `tests/artifacts/perf/insert-external-qb-hint-current-dirty-cyangorge-20260505T1333Z/summary.md`.
- Result: rejected and reverted. The primary weighted score looked better in
  the local paired runs (`1.8386` baseline to `1.7808` / `1.7728` candidate),
  but this was not a full-workload win: geomean ratio worsened on both
  candidates (`2.5061x` to `2.5690x` / `2.6859x`), write-bulk worsened on the
  repeat (`2.8074x` to `2.9619x`), and the main 10K `large_10col`
  single-transaction FSQLite median worsened `37.61 ms` to `39.12 ms` and then
  `42.22 ms`. Record-size tiny/small/medium rows consistently regressed
  (`4.50/6.78/10.68 ms` baseline to `5.36/8.00/12.04 ms`, then
  `6.03/8.01/12.75 ms`). The only large record-size improvement was unstable
  (`44.89 ms` baseline to `37.97 ms`, then `43.03 ms`).
- Do not retry this exact "move page to external hint and clear internal
  cache" handoff. Reconsider only with a different rightmost-cache design that
  avoids the page clone while preserving the useful cache state, and require an
  interleaved A/B that improves the full insert matrix without regressing the
  small/medium record-size rows or the 10K large single-transaction row.

## 2026-05-05 - Direct INSERT integer placeholder text cache

- Target: direct-simple INSERT concat row building after the insert profile
  showed multi-ms row-build cost on 10K medium/large rows. The candidate was
  tested in the isolated worktree
  `/data/tmp/frankensqlite-cyangorge-paramtext-cache-20260505T1340` so the
  shared main worktree and peer source edits were not disturbed.
- Candidate shape: add a stack-local cache for integer bind placeholder decimal
  text during one direct INSERT row build, aiming to avoid repeated `itoa`
  formatting for repeated concat references such as `?1`.
- Correctness smoke passed in the isolated worktree:
  `cargo fmt --check`,
  `cargo test -p fsqlite-core test_prepared_direct_simple_insert_autocommit_profile_breakdown -- --nocapture`,
  `cargo test -p fsqlite-core prepared_direct_simple_insert_concat_chain -- --nocapture`,
  and `cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench`.
- Evidence artifacts:
  `tests/artifacts/perf/insert-external-qb-hint-owned-cyangorge-baseline-20260505T1318Z/report.json`
  and
  `tests/artifacts/perf/insert-param-text-cache-cyangorge-20260505T1347Z/report.json`.
  Summary:
  `tests/artifacts/perf/insert-param-text-cache-cyangorge-20260505T1347Z/summary.md`.
- Result: rejected and not applied to main. The focused insert matrix worsened:
  geomean F/C ratio `2.3832x` to `2.5280x`, weighted score `1.6578` to
  `1.7978`, write-bulk geomean `2.5538x` to `2.6975x`, and write-single
  geomean `1.4354x` to `1.5703x`. Target large rows did not improve:
  single-transaction `large_10col` 10K moved `37.5866 ms` to `37.7624 ms`,
  and record-size `large_10col` moved `39.4682 ms` to `41.3979 ms`.
- Do not retry per-row integer placeholder text caching as a standalone
  row-build optimization. Reconsider only with a direct serialization design
  that avoids transient text materialization rather than caching its decimal
  representation, and require a full insert-matrix win.

## 2026-05-05 - Dirty WAL prepared-frame direct publication snapshot

- Target: INSERT commit/publish cost on large single-transaction rows, where
  profiles still show multi-ms `commit_roundtrip_ns`. The measured source diff
  was peer-owned dirty work in `crates/fsqlite-core/src/wal_adapter.rs`; this
  entry records an independent dirty-tree A/B, not a source change landed by
  CyanGorge.
- Candidate shape: for prepared frame batches with a known commit frame, publish
  the WAL visibility snapshot directly from `prepared.frame_metas` instead of
  first recording those frame entries in `pending_publication_frames`.
- Correctness smoke:
  `cargo test -p fsqlite-core --lib append -- --nocapture` passed
  (`17` tests). A broader exploratory
  `cargo test -p fsqlite-core append -- --nocapture` run passed the WAL adapter
  append tests but failed
  `test_v2_plain_execute_sequential_inserts_keep_append_path_hot_across_statements`,
  so that integration failure must be resolved or shown unrelated before
  landing.
- Evidence artifacts:
  `tests/artifacts/perf/insert-external-qb-hint-owned-cyangorge-baseline-20260505T1318Z/report.json`
  and
  `tests/artifacts/perf/insert-wal-publish-direct-current-dirty-cyangorge-20260505T135315Z/report.json`.
  Summary:
  `tests/artifacts/perf/insert-wal-publish-direct-current-dirty-cyangorge-20260505T135315Z/summary.md`.
- Result: mixed and not a keep as-is. Large FSQLite medians improved
  (`large_10col` single transaction `37.5866 ms` to `35.1876 ms`,
  record-size `large_10col` `39.4682 ms` to `34.7089 ms`), but the insert
  matrix did not clear the keep gate: geomean F/C ratio worsened slightly
  `2.3832x` to `2.3890x`, weighted score worsened `1.6578` to `1.7359`,
  and write-single geomean worsened `1.4354x` to `1.5293x`.
- Do not land this exact direct-publish dirty diff from this evidence alone.
  Retry only with an interleaved clean/candidate A/B that preserves the
  large-row improvement, restores the weighted score/write-single rows, and
  explains or fixes the broader append-filter failure.

## 2026-05-05 - Thresholded WAL prepared-frame direct publication

- Target: same large INSERT commit/publish cost as the dirty direct-publication
  check, but with a frame-count threshold intended to keep small/write-single
  commits on the existing path.
- Touched during rejected isolated candidate:
  `crates/fsqlite-core/src/wal_adapter.rs` in temporary worktree
  `/data/tmp/frankensqlite-cyangorge-wal-threshold-20260505T1406`; the shared
  source file was reserved by PurpleCoast and was not edited.
- Candidate shape: factor WAL commit snapshot publication over generic frame
  entries, then use direct publication from `prepared.frame_metas` only when
  `prepared.frame_count() >= 128`. A new 128-frame unit test asserted the large
  direct path publishes all pages and leaves no pending publication entries.
- Correctness smoke:
  `cargo fmt`,
  `cargo test -p fsqlite-core --lib append -- --nocapture` passed
  (`18` tests), and
  `cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench`
  passed in the isolated worktree.
- Evidence artifacts:
  `tests/artifacts/perf/insert-external-qb-hint-owned-cyangorge-baseline-20260505T1318Z/report.json`,
  `tests/artifacts/perf/insert-wal-publish-direct-current-dirty-cyangorge-20260505T135315Z/report.json`,
  and
  `tests/artifacts/perf/insert-wal-publish-threshold-cyangorge-20260505T1406Z/report.json`.
  Summary:
  `tests/artifacts/perf/insert-wal-publish-threshold-cyangorge-20260505T1406Z/summary.md`.
- Result: rejected. The thresholded variant was worse than both clean baseline
  and the full dirty direct-publication variant: geomean F/C ratio `2.3832x`
  baseline / `2.3890x` full-direct / `2.5341x` threshold, weighted score
  `1.6578` / `1.7359` / `1.8890`, and write-single geomean `1.4354x` /
  `1.5293x` / `1.6811x`. It also failed to preserve the full direct large-row
  win: record-size `large_10col` F median was `39.4682 ms` baseline,
  `34.7089 ms` full-direct, and `38.2606 ms` threshold.
- Do not retry a simple frame-count threshold around WAL prepared-frame direct
  publication. First prove why the full direct path improved large rows, then
  design a narrower change that does not disturb write-single or B-tree timing.

## 2026-05-05 - CASS strict project-folder follow-up

Scope: user-requested CASS pass restricted to the last 60 days. Direct
`--workspace /data/projects/frankensqlite` searches for `rejected`,
`reverted`, `slower`, `didn't help`, `abandoned`, and the misspelling
`abandones` found only a sparse 2026-03-07 direct-workspace slice and no direct
negative-term hits. To avoid treating that as an empty history, the follow-up
used CASS workspace aliases whose source paths clearly map to this repo,
especially `/home/ubuntu/.gemini/tmp/frankensqlite`, then cross-checked leads
against preserved perf artifacts before recording them here.

- Session-shared page-1 synthetic hint flag: rejected after the target
  `SharedTxnPageIo::clear_stale_synthetic_pending_commit_surface` profile stack
  dropped but `perf-update-delete 10000 100 both` stayed inside noise. Baseline
  mean was `1.206 s +/- 0.021 s`; candidate v3 mean was
  `1.204 s +/- 0.025 s` (`1.00 +/- 0.03` faster). Evidence:
  `tests/artifacts/perf/20260428T2230Z-sapphirecrane-page1-synthetic-flag/RESULT-page1-synthetic-flag.md`.
  Do not add session-shared page-1 hint state in `Connection` /
  `SharedTxnPageIo` merely because the narrow stack disappears; require a
  measurable update/delete matrix win.
- Unguarded rowid-count helper for larger right tables: this reinforces the
  existing rowid-count guardrail with a clean local A/B. Removing
  `ROWID_COUNT_SMALL_RIGHT_ROW_LIMIT` improved only the 100-order HAVING row
  (`0.2168 ms` to `0.2113 ms`) but regressed the 1000-order HAVING row
  materially (`1.2285 ms` to `1.6221 ms`) and did not improve the 10000-order
  row (`10.6338 ms` to `10.7713 ms`). Evidence:
  `tests/artifacts/perf/join-rowid-count-large-candidate-purplecoast-20260504T2045Z/summary.md`.
  Do not remove the rowid-count right-table guard without a close join-section
  A/B that improves all affected row counts or the section score.
- March raw-`bench_insert` hash-swap/cache experiments are stale evidence, not
  a keep/retry basis. CASS shows attempts to justify `foldhash` swaps in SQL
  cache, cursor/hash maps, pager `PageCache`, and `MemPageStore` from the old
  raw-string `bench_insert` profile while repeated compile churn and background
  edits prevented a stable current-matrix proof. This reinforces the existing
  stale-benchmark rule: retry hash-function or dense-index storage changes only
  from a current prepared-statement matrix/profile, not from old raw SQL-string
  cache-thrash sessions.

CASS evidence:
- `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-09T05-08-a1108e5a.json -n 84 -C 60`
- `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-09T05-08-9581ae40.json -n 120 -C 40`
- `cass view /home/ubuntu/.gemini/tmp/frankensqlite/chats/session-2026-03-09T05-08-628c8b17.json -n 90 -C 35`

## 2026-05-05 - Direct INSERT row-value text pooling

- Target: prepared direct INSERT row-build cost on medium/large concat-heavy
  rows after the current insert profile showed `row_build_ns` around
  `5.96 ms` on both large 10K single-transaction rows.
- Touched during rejected candidate: `crates/fsqlite-core/src/connection.rs`;
  source was reverted after the benchmark.
- Candidate shape: return heap-backed `SqliteValue::Text` row-scratch values
  to the existing `fsqlite_types::value` TLS pool when lazy private-memory
  direct inserts clear `mem_row_values`, then build concat-chain text results
  from a pooled `SmallText` slot via `SmallText::overwrite`.
- Correctness/build smoke passed before the A/B:
  `cargo fmt --check`,
  `cargo test -p fsqlite-core test_prepared_direct_simple_insert_returns_concat_text_to_value_pool -- --nocapture`,
  `cargo test -p fsqlite-core test_prepared_direct_simple_insert_large_profile_breakdown -- --nocapture`,
  `cargo test -p fsqlite-core test_prepared_direct_simple_insert_autocommit_profile_breakdown -- --nocapture`,
  and `cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench`.
- Evidence artifacts:
  `tests/artifacts/perf/insert-profile-current-head-cyangorge-20260505T122449Z/report.json`
  and
  `tests/artifacts/perf/insert-row-text-pool-cyangorge-20260505T1434Z/report.json`.
  Summary:
  `tests/artifacts/perf/insert-row-text-pool-cyangorge-20260505T1434Z/summary.md`.
- Result: rejected and reverted. Insert avg/geomean ratios improved
  (`2.4610x -> 2.3595x`, `2.3623x -> 2.2890x`), but the primary weighted
  insert score regressed `1.6991 -> 1.7329` and write-single geomean regressed
  `1.4908x -> 1.5517x`. Important absolute FrankenSQLite medians worsened:
  `small_3col` 1K single transaction `0.8055 ms -> 0.9613 ms`, `small_3col`
  10K single transaction `6.8949 ms -> 7.7481 ms`, `medium_6col` 10K
  `13.6661 ms -> 14.6216 ms`, `large_10col` 10K `36.1651 ms -> 36.7869 ms`,
  and record-size `large_10col` `37.0559 ms -> 37.6541 ms`.
- Do not retry direct INSERT row-value pooling / pooled `SmallText::overwrite`
  as a standalone row-build optimization. The profile counters showed the
  root hypothesis failed on the target large rows: row-build time got worse
  (`large_10col` single transaction `5.958 ms -> 7.404 ms`, record-size
  `large_10col` `5.973 ms -> 6.722 ms`), so TLS pool traffic cost more than
  it saved.

## 2026-05-05 - Benchmark-only journal_mode=MEMORY switch

- Target: private `:memory:` benchmark write gap, especially large INSERT
  rows. The motivating observation was that C SQLite reports and keeps
  `journal_mode=memory` for `:memory:` even after `PRAGMA journal_mode=WAL`,
  while FrankenSQLite honors WAL for private in-memory databases.
- Touched during rejected candidate:
  `crates/fsqlite-e2e/src/bin/comprehensive_bench.rs`; source was reverted
  after measurement.
- Candidate shape: change the benchmark pragma setup from
  `PRAGMA journal_mode = WAL` to `PRAGMA journal_mode = MEMORY` for both
  C SQLite and FrankenSQLite.
- Evidence artifacts:
  `tests/artifacts/perf/insert-journal-memory-candidate-purplecoast-20260505T1450Z/report.json`
  and
  `tests/artifacts/perf/full-quick-journal-memory-candidate-purplecoast-20260505T1515Z/report.json`.
  Summaries:
  `tests/artifacts/perf/insert-journal-memory-candidate-purplecoast-20260505T1450Z/summary.md`
  and
  `tests/artifacts/perf/full-quick-journal-memory-candidate-purplecoast-20260505T1515Z/summary.md`.
- Insert-only result looked tempting: weighted insert score improved
  `1.6991 -> 1.6703`, geomean ratio improved `2.3623x -> 2.2924x`,
  write_bulk geomean improved `2.5153x -> 2.4349x`, and absolute large-row
  FrankenSQLite medians improved (`large_10col` 10K single transaction
  `36.165 ms -> 33.412 ms`, record-size `large_10col` 10K
  `37.056 ms -> 34.171 ms`).
- Full quick matrix rejected it: weighted score worsened
  `0.5658 -> 0.5808`, avg/geomean ratios worsened `1.0270x -> 1.0691x` and
  `0.4467x -> 0.4596x`, write_bulk geomean worsened `2.3562x -> 2.4735x`,
  write_single worsened `2.0563x -> 2.1667x`, and concurrent writers worsened
  `1.1514x -> 1.1830x`.
- Do not retry the benchmark-only `journal_mode=MEMORY` switch as a standalone
  fairness/performance correction. It is only worth revisiting as part of a
  broader benchmark policy change that improves or preserves the full
  end-to-end matrix, not merely the insert-only rows.

## 2026-05-05 - insert_page_sorted append/equal fast path

- Target: sequential INSERT write-set staging in the pager, where page numbers
  are often appended in sorted order.
- Touched during rejected isolated candidate:
  `crates/fsqlite-pager/src/pager.rs` in clean worktree
  `/data/tmp/frankensqlite-purplecoast-clean-20260505T1458`; shared source was
  not edited or staged by this measurement.
- Candidate shape: check `pages.last()` in `insert_page_sorted` and return
  immediately for monotonic append (`last < page_no`) or duplicate-last
  (`last == page_no`) before falling back to the existing binary search and
  insertion path.
- Evidence artifact:
  `tests/artifacts/perf/insert-page-sorted-append-candidate-purplecoast-20260505T1504Z/report.json`.
  Summary:
  `tests/artifacts/perf/insert-page-sorted-append-candidate-purplecoast-20260505T1504Z/summary.md`.
- Result: rejected. Avg/geomean ratios improved slightly
  (`2.4610x -> 2.4231x`, `2.3623x -> 2.3470x`) and write_bulk geomean improved
  (`2.5153x -> 2.4909x`), but the primary weighted insert score regressed
  `1.6991 -> 1.7171` and write-single geomean regressed
  `1.4908x -> 1.5168x`.
- Do not retry the simple `insert_page_sorted` last-page append/equal branch as
  a standalone optimization. The branch is cheap and plausible, but current
  end-to-end insert evidence says it is not a keep.

## 2026-05-05 - WAL publication page-index Arc::make_mut hoist

- Target: large INSERT commit publication overhead in
  `WalBackendAdapter::publish_pending_commit_snapshot`.
- Touched during rejected candidate: `crates/fsqlite-core/src/wal_adapter.rs`;
  source was reverted after measurement.
- Candidate shape: hoist `Arc::make_mut(&mut page_index)` out of the
  per-frame loop so a commit that publishes thousands of frames only performs
  the mutable Arc access once.
- Correctness/build smoke passed:
  `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-wal-makemut-target cargo test -p fsqlite-core --lib append -- --nocapture`
  (`17` tests) and
  `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-wal-makemut-target cargo build --profile release-perf -p fsqlite-e2e --bin comprehensive-bench`.
- Evidence artifact:
  `tests/artifacts/perf/insert-wal-page-index-makemut-purplecoast-20260505T1513Z/report.json`.
  Summary:
  `tests/artifacts/perf/insert-wal-page-index-makemut-purplecoast-20260505T1513Z/summary.md`.
- Result: rejected. Insert weighted score regressed `1.6991 -> 1.8022`,
  avg/geomean ratios regressed `2.4610x -> 2.5586x` and
  `2.3623x -> 2.4753x`, write_bulk regressed `2.5153x -> 2.6295x`, and
  write_single regressed `1.4908x -> 1.5889x`.
- Do not retry this simple `Arc::make_mut` hoist as a standalone WAL
  publication optimization. The branch looked mechanically cheaper, but the
  current end-to-end insert matrix rejected it.

## 2026-05-05 - Direct INSERT precomputed column affinities

- Target: direct-simple INSERT row value handling in
  `crates/fsqlite-core/src/connection.rs`, after perf showed visible time in
  `push_prepared_direct_simple_insert_value` / `SqliteValue::apply_affinity`
  on the insert matrix.
- Touched during rejected candidate: `crates/fsqlite-core/src/connection.rs`;
  source was reverted after measurement.
- Candidate shape: add `column_affinities: Vec<TypeAffinity>` to
  `PreparedDirectSimpleInsert`, compute it once during
  `prepared_direct_simple_insert_plan`, and pass the precomputed enum to
  `push_prepared_direct_simple_insert_value` instead of calling
  `type_affinity_for_direct_insert(column.affinity)` for every inserted column.
- Correctness smoke passed:
  `cargo fmt --check` and
  `env CARGO_TARGET_DIR=/data/tmp/cargo-target cargo test -p fsqlite-core prepared_direct_simple_insert -- --nocapture`
  (`28` matching tests).
- Evidence artifacts:
  `tests/artifacts/perf/direct-insert-precomputed-affinity-cyangorge-20260505T1525Z/baseline-report.json`
  and
  `tests/artifacts/perf/direct-insert-precomputed-affinity-cyangorge-20260505T1525Z/candidate-report.json`.
  Summary:
  `tests/artifacts/perf/direct-insert-precomputed-affinity-cyangorge-20260505T1525Z/summary.md`.
- Result: rejected. The primary weighted insert score regressed
  `1.5606 -> 1.8360`, avg/geomean ratios regressed
  `2.3295x -> 2.5739x` and `2.2311x -> 2.4638x`, write_bulk geomean
  regressed `2.3883x -> 2.6058x`, and write_single geomean regressed
  `1.3542x -> 1.6338x`. The target large-row row-build counters did not
  improve reliably: `large_10col` single txn row_build_ns was essentially flat
  (`6114165 -> 6115810`), while record-size `large_10col` worsened
  (`5951537 -> 6813546`).
- Do not retry precomputing direct INSERT column affinity metadata as a
  standalone micro-optimization. The per-row char-to-affinity match is not the
  bottleneck; future affinity work should remove or fuse value coercion itself
  and must improve the same-window insert matrix.

## 2026-05-05 - WAL checksum one-chunk header transform

- Target: `WalChecksumTransform::for_wal_frame` self-time under large INSERT
  WAL frame preparation.
- Touched during rejected candidate: `crates/fsqlite-wal/src/checksum.rs`;
  source was reverted after measurement.
- Candidate shape: replace the generic
  `WalChecksumTransform::from_aligned_bytes(&frame[..8], ...)` call for the
  8-byte WAL frame header prefix with the closed-form affine transform for
  exactly one checksum chunk. The page payload transform stayed on the generic
  path.
- Correctness smoke passed:
  `cargo fmt --check` and
  `env CARGO_TARGET_DIR=/data/tmp/cargo-target cargo test -p fsqlite-wal checksum_transform -- --nocapture`
  (`2` matching tests). The first release-perf build attempt in the shared
  `/data/tmp/cargo-target` failed with a missing bytecode file, so the
  candidate benchmark was built in the unique target dir
  `/data/tmp/frankensqlite-cyangorge-walchk-target`.
- Evidence artifacts:
  baseline
  `tests/artifacts/perf/direct-insert-precomputed-affinity-cyangorge-20260505T1525Z/baseline-report.json`
  and candidate
  `tests/artifacts/perf/wal-checksum-header-transform-cyangorge-20260505T1535Z/candidate-report.json`.
  Summary:
  `tests/artifacts/perf/wal-checksum-header-transform-cyangorge-20260505T1535Z/summary.md`.
- Result: rejected. The primary weighted insert score regressed
  `1.5606 -> 1.7049`, avg/geomean ratios regressed
  `2.3295x -> 2.4746x` and `2.2311x -> 2.3800x`, write_bulk geomean
  regressed `2.3883x -> 2.5361x`, and write_single geomean regressed
  `1.3542x -> 1.4935x`. Several absolute FSQLite 10K medians improved, but
  the ratio-weighted matrix and category scores failed the keep gate.
- Do not retry a special one-chunk header transform inside
  `WalChecksumTransform::for_wal_frame` as a standalone micro-optimization.
  Future WAL checksum work should reduce the payload transform or prepared-frame
  pipeline cost and must improve the full insert matrix.
