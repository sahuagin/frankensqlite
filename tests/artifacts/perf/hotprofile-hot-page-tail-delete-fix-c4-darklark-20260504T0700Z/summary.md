# Hot-Path Profile

- Bead: `hot-page-tail-delete-fix-c4`
- Run ID: `hot-page-tail-delete-fix-c4-hot_page_contention-frankensqlite-c4-s42-1777877939950`
- Trace ID: `hot-page-tail-delete-fix-c4.hot_page_contention:frankensqlite:c4`
- Scenario: `hot-page-tail-delete-fix-c4.hot_page_contention`
- Fixture: `frankensqlite`
- Workload: `hot_page_contention`
- Seed: `42`
- Concurrency: `4`
- Scale: `20`
- Concurrent mode: `ON`
- Integrity check: `disabled`
- Golden dir: `/data/projects/frankensqlite/sample_sqlite_db_files/golden`
- Working base: `/data/projects/frankensqlite/sample_sqlite_db_files/working`

## Engine Summary

- Wall time (ms): 48
- Ops total: 800
- Ops/sec: 16577.28
- Retries: 10
- Aborts: 10
- Integrity check: skipped
- Notes: `mode=concurrent (MVCC); parallel worker execution; backend_identity=iouring:parity_cert_strict; retry_diag=kind[busy=0,busy_snapshot=10,busy_recovery=0,other=0] phase[begin=0,body=10,commit=0,rollback=0] max_batch_attempts=4 top_conflict_pages[p2653:10] last_busy="database is busy (snapshot conflict on pages: 2653)"; conflict_stats[page_contentions=8,fcw_drifts=0,ssi_aborts=0,fcw_merge_attempts=0,fcw_merge_successes=0] top_hotspots[p2653:8]`

## Parser Reuse

- Parse cache hits/misses: 2/43
- `plan_cache_hit` / `plan_cache_miss`: 0/4
- `statement_cache_hit` / `statement_cache_miss`: 30/14
- `compile_reuse_count`: 30
- `invalidation_reason`: emitted by `fsqlite.statement_reuse` telemetry for cache invalidations such as `schema_cookie_changed` and `explicit_invalidate`.

## Record Decode

- `decode_cache_hit` / `decode_cache_miss`: 0/0
- `decode_invalidation_reason` counts: position=0 write=0 pseudo=0
- `record_decodes_per_row`: n/a
- Decode time / heap bytes / column reads: 217415/0/0

## Connection Ceremony

- Background gates: status_checks=1143 op_cx=38 dispatch=0
- Schema/publication refreshes: prepared_schema=84 lightweight=0 full_reload=0 pager_publication=268
- Cached snapshot reuse/parks: 0/0
- Prepared engine fresh/reuse: 0/0
- Prepared insert fast/instrumented lanes: 40/0
- Prepared update/delete fast/instrumented lanes: 0/0
- Prepared update/delete fallback reasons (returning/sqlite_sequence/without_rowid/live_vtab/trigger/fk): 0/0/0/0/0/0
- Prepared DML affected-only runs: 0
- sqlite_sequence fast-path/scan refresh: 0/0
- Column-default evaluation passes: 0
- Statement lookaside alloc/reset/fallback: 6128/177/10
- :memory: autocommit fast-path begins: 0

## MVCC Write Path

- Writes total: 870 (tier0=770, tier1=100, tier2=0)
- Page-touch classes: already_owned=770 first_touch=100 commit_surface=0 page_one_tracks=0 pending_surface_clears=0
- Page-lock waits: 8 (time_ns=1426749)
- BUSY retries/timeouts: 8/0
- Runtime retry taxonomy: total=10 aborts=10 kind[busy=0,busy_snapshot=0,busy_recovery=0,other=0] phase[begin=0,body=0,commit=0,rollback=0] max_batch_attempts=0
- Stale snapshot rejects: 10
- Page-one conflict tracking: 0 (time_ns=0)
- Pending surface clears: 0 (time_ns=0)

## Page Buffer Pool

- `page_buffer_pool_hit` / `page_buffer_pool_miss`: 37/20
- `page_image_slab_reuse_count`: 37
- `staged_page_copy_bytes`: 0

## PageData Motion

- Borrowed normalization calls: 0 (exact-size copies=0)
- Owned normalization calls: 870 (passthrough=870, in_place_zero_extends=0, resized_copies=0)
- Normalized bytes: 0 (payload=0, zero_fill=0)

## B-Tree Copy Kernel Targets

- rank 1 btree_owned_payload_materialization: bytes=4278 -> 372 payload materialization call(s) forced 4278 byte(s) into fresh owned buffers
- rank 2 btree_local_payload_copy: bytes=3207 -> 810 local payload copy call(s) copied 3207 byte(s) into caller scratch without overflow traversal
- rank 3 btree_table_leaf_cell_assembly: bytes=434 -> 61 table-leaf cell assembly call(s) emitted 434 byte(s) before page insert
- note: this is a bytes-first replacement target list for later kernel work; it isolates owned-buffer and reassembly surfaces rather than claiming an exclusive wall-time partition.

## Ranked Hotspots

- rank 1 mvcc_page_lock_wait: time_ns=1426749 -> E2.1/E5.1 target: measured page-lock wait time is a first-class MVCC tax, so publish shrink and disjoint-page topology should move before deeper executor tuning. [bd-db300.5.2.1, bd-db300.5.5.1]
- rank 2 parser_ast_churn: time_ns=306485 -> J2/J4 target: parser, AST, and compile churn should be reduced through prepared-artifact reuse and arena-backed scratch. [bd-db300.10.2, bd-db300.10.4]
- rank 3 record_decode: time_ns=217415 -> J2/J5 target: row decode work is expensive enough to justify scratch-space reuse, decode caching, and copy reduction. [bd-db300.10.2, bd-db300.10.5]
- rank 4 mvcc_page_one_conflict_tracking: time_ns=0 -> E5.1/E5.2 target: page-one conflict-only tracking is visible enough to justify home-lane/extent runway work that keeps disjoint inserts physically disjoint longer. [bd-db300.5.5.1, bd-db300.5.5.2]
- rank 5 mvcc_pending_commit_surface_clear: time_ns=0 -> E2.1/E2.2 target: synthetic pending-surface cleanup is now measured directly, so the next metadata-plane cut should shrink or bypass this publish-side maintenance. [bd-db300.5.2.1, bd-db300.5.2.2]
- rank 6 row_materialization: time_ns=0 -> J2/J6/J7 target: result-row materialization is still paying avoidable clone/allocation and reusable-frame cost in the mixed hot path. [bd-db300.10.2, bd-db300.10.6, bd-db300.10.7]

## Baseline Reuse Ledger

- rank 1 compiled_plan_cache: supported=true, hits=0, misses=4, hit_rate_bps=0 -> J4 target: compiled-plan misses are direct evidence for statement/plan caching work. [bd-db300.10.4]
- rank 2 cursor_frame_reuse: supported=true, hits=0, misses=0, hit_rate_bps=0 -> J7 target: prepared engine fresh-vs-reuse counters now expose setup churn directly, so the next cuts should drive the reuse rate up instead of treating frame setup as opaque. [bd-db300.10.7]
- rank 3 record_decode_cache: supported=true, hits=0, misses=0, hit_rate_bps=0 -> J5 target: decode-cache hits/misses are now surfaced directly, so the next cuts should push the hit rate up and the invalidation counts down instead of treating decode churn as opaque. [bd-db300.10.5]
- rank 4 statement_parse_cache: supported=true, hits=2, misses=43, hit_rate_bps=444 -> J4 target: repeated parse misses still show avoidable prepare churn on the low-contention path. [bd-db300.10.4]
- rank 5 page_buffer_pool_reuse: supported=true, hits=37, misses=20, hit_rate_bps=6491 -> J3 target: page-buffer pool hits/misses and staged page-copy bytes are now explicit, so the next cuts should drive miss rate and copied bytes down instead of treating staging churn as opaque. [bd-db300.10.3]
- rank 6 prepared_statement_cache: supported=true, hits=30, misses=14, hit_rate_bps=6818 -> Secondary baseline reuse surface after the named Track J cache/reuse buckets. []
- rank 7 page_data_ownership_reuse: supported=true, hits=870, misses=0, hit_rate_bps=10000 -> J6 target: PageData ownership reuse is now measured directly, so next cuts can use passthrough, in-place zero-extend, and resized-copy evidence instead of treating ownership churn as a blind spot. [bd-db300.10.6]

## Baseline Waste Ledger

- rank 1 busy_retry_queueing: class=structural_side_effect, time_ns=56838575, wall_share_bps=11841, allocator_pressure_bytes=0, activity_count=10 -> Structural spillover: retry and BUSY queueing are not baseline tax and should steer Track A work instead of J-lane fixes. [bd-db300.2.4]
- rank 2 boundary_coordination: class=mixed_or_residual, time_ns=20224957, wall_share_bps=4214, allocator_pressure_bytes=0, activity_count=800 -> Mixed lane: boundary coordination should stay visible so baseline fixes do not accidentally absorb residual commit/path coordination into the wrong bucket. [bd-db300.1.5, bd-db300.5.1]
- rank 3 mvcc_page_lock_wait: class=structural_side_effect, time_ns=1426749, wall_share_bps=297, allocator_pressure_bytes=0, activity_count=8 -> Structural spillover: measured page-lock wait is explicit MVCC contention tax and should steer tiny-publish and topology work before generic baseline cleanup. [bd-db300.5.2.1, bd-db300.5.5.1]
- rank 4 executor_body_residual: class=mixed_or_residual, time_ns=1365006, wall_share_bps=284, allocator_pressure_bytes=0, activity_count=800 -> J6/J7/J8 target: residual service time beyond decode/materialization is where VDBE setup, cursor motion, page fetch, and ownership churn likely still hide. [bd-db300.10.6, bd-db300.10.7, bd-db300.10.8]
- rank 5 parser_prepare_churn: class=baseline_tax, time_ns=306485, wall_share_bps=64, allocator_pressure_bytes=1703, activity_count=45 -> J2/J4 target: parse/rewrite/compile work is still visible enough that caching and arena-backed scratch should move first. [bd-db300.10.2, bd-db300.10.4]
- rank 6 record_decode: class=baseline_tax, time_ns=217415, wall_share_bps=45, allocator_pressure_bytes=0, activity_count=1466 -> J2/J5 target: decode time and decoded-value heap churn remain direct baseline-tax candidates. [bd-db300.10.2, bd-db300.10.5]
- rank 7 durability: class=mixed_or_residual, time_ns=0, wall_share_bps=0, allocator_pressure_bytes=0, activity_count=800 -> Mixed lane: durability must stay explicit so later baseline work does not overclaim gains that really belong to WAL/commit-path changes. [bd-db300.5.1]
- rank 8 mvcc_commit_surface_maintenance: class=mixed_or_residual, time_ns=0, wall_share_bps=0, allocator_pressure_bytes=0, activity_count=0 -> Mixed lane: page-one tracking plus pending-surface maintenance are now measurable enough to steer publish-shrink versus topology work without hiding them inside generic service time. [bd-db300.5.2.1, bd-db300.5.5.1]
- rank 9 page_data_normalization: class=baseline_tax, bytes=0, wall_share_bps=n/a, allocator_pressure_bytes=0, activity_count=870 -> J3/J6 target: page normalization bytes are now explicit baseline tax, so reusable page buffers and ownership-preserving writes can be prioritized with real evidence. [bd-db300.10.3, bd-db300.10.6]
- rank 10 row_materialization: class=baseline_tax, time_ns=0, wall_share_bps=0, allocator_pressure_bytes=0, activity_count=0 -> J2/J6/J7 target: emitted-row cloning is still paying avoidable heap and ownership cost on the common path. [bd-db300.10.2, bd-db300.10.6, bd-db300.10.7]
- note: baseline and structural spillover entries are intentionally listed together here so low-retry rows can be separated from contention-driven wall time without hiding either class of cost.

## Quantified Cost Components

- rank 1 parser_ast_churn: time_ns=306485, time_share_bps=5850, allocator_pressure_bytes=0, allocator_share_bps=0, activity_count=45 -> J2/J4 target: parser and compile reuse still dominate this component enough to justify prepared-artifact work next. [bd-db300.10.2, bd-db300.10.4]
- rank 2 record_decode: time_ns=217415, time_share_bps=4150, allocator_pressure_bytes=0, allocator_share_bps=0, activity_count=1466 -> J2/J5 target: decode cost is large enough to justify scratch buffers and decode-cache work. [bd-db300.10.2, bd-db300.10.5]
- rank 3 page_data_motion: time_ns=0, time_share_bps=0, allocator_pressure_bytes=0, allocator_share_bps=0, activity_count=870 -> J3/J6 target: page-image normalization is now visible as its own copy lane, so reusable page buffers and owned passthrough should move before more speculative executor surgery. [bd-db300.10.3, bd-db300.10.6]
- rank 4 row_materialization: time_ns=0, time_share_bps=0, allocator_pressure_bytes=0, allocator_share_bps=0, activity_count=0 -> J2/J6/J7 target: emitted-row cloning and transient value ownership remain a first-class hot-path cost. [bd-db300.10.2, bd-db300.10.6, bd-db300.10.7]

## Wall-Time Decomposition

- rank 1 retry: time_ns=52000000, wall_share_bps=10833 -> B target: sleep-based retry backoff is still showing up in the hot cell and should be replaced with a bounded handoff strategy. [bd-db300.2.4]
- rank 2 synchronization: time_ns=20224957, wall_share_bps=4214 -> A/E target: transaction-boundary coordination is still a first-class wall-time tax and should be pushed toward narrower residual serialized regions. [bd-db300.1.5, bd-db300.5.1]
- rank 3 queueing: time_ns=4838575, wall_share_bps=1008 -> A/B target: retried BUSY attempts are consuming visible wall time before useful work resumes, so the handoff policy should be tightened before scaling further. [bd-db300.2.4]
- rank 4 service: time_ns=1582421, wall_share_bps=330 -> J target: useful body execution still dominates enough wall time that parser, decode, row-path, and residual VDBE/page-motion optimizations remain the main throughput lever once contention is under control. [bd-db300.10.2, bd-db300.10.4, bd-db300.10.5, bd-db300.10.6, bd-db300.10.7, bd-db300.10.8]
- rank 5 mvcc_wait: time_ns=1426749, wall_share_bps=297 -> E2.1/E5.1 target: page-lock wait time is now explicit in the wall-time story, so tiny-publish and topological disjointness can be judged against a real synchronization lane instead of guesses. [bd-db300.5.2.1, bd-db300.5.5.1]
- rank 6 allocator_copy: time_ns=0, wall_share_bps=0 -> J target: allocator and copy work is large enough to justify scratch-space reuse, row-value ownership reduction, and reusable buffers. [bd-db300.10.2, bd-db300.10.3, bd-db300.10.6, bd-db300.10.7]
- rank 7 durability: time_ns=0, wall_share_bps=0 -> E target: commit durability is now an explicit measured lane, so future architecture work can separate durable ordering from general executor service cost. [bd-db300.5.1]
- rank 8 mvcc_commit_surface: time_ns=0, wall_share_bps=0 -> E2.1/E5.1 target: page-one tracking plus pending-surface maintenance are an explicit wall-time lane, which is the right steering signal for publish-plane shrink versus page-topology work. [bd-db300.5.2.1, bd-db300.5.5.1]
- note: component shares are evidence-backed but may overlap on multi-worker runs, so they should steer classification rather than be treated as an exclusive partition.

## Causal Classification

- Dominant bucket: retries (estimated_time_ns=52000000, wall_share_bps=10833, score_bps=6494, mixed_or_ambiguous=false)
- Runner-up: synchronization (estimated_time_ns=21651706, score_bps=2704, gap_bps=3790)
- Rationale: `retries` leads the concrete bucket ranking by 3790 score bps over `synchronization` using classified hot-path time derived from the existing wall-time, waste-ledger, and cost-component artifacts.
- rank 1 retries: dominant=true, estimated_time_ns=52000000, wall_share_bps=10833, score_bps=6494 -> Retry-dominant rows are losing wall time to configured backoff, which is distinct from in-attempt queueing and should stay explicit. [bd-db300.2.4]
  evidence actionable_ranking.json .wall_time_components[] | select(.component == "retry") | .time_ns: time_ns=52000000 -> retry bucket maps to explicit configured backoff sleep rather than in-attempt wait time
  evidence profile.json .engine_report.runtime_phase_timing.retry_backoff_time_ns: time_ns=52000000 -> runtime phase timing already exposes aggregate retry-backoff sleep directly
  evidence profile.json .engine_report.retries: count=10 -> top-level engine retries keep the wall-time lane tied to observed retry frequency
  evidence profile.json .mvcc_write.runtime_retry.total_retries: count=10 -> structured retry taxonomy refines the retry bucket without collapsing it into generic queueing
  evidence profile.json .mvcc_write.runtime_retry.total_aborts: count=10 -> abort counts remain adjacent evidence for retry-heavy rows that fail to converge cleanly
- rank 2 synchronization: dominant=false, estimated_time_ns=21651706, wall_share_bps=4511, score_bps=2704 -> Synchronization-dominant rows are paying boundary or MVCC coordination cost; keep publish-plane and page-topology work ahead of generic copy tuning. [bd-db300.1.5, bd-db300.5.1, bd-db300.5.2.1, bd-db300.5.5.1]
  evidence actionable_ranking.json .wall_time_components[] | select(.component == "synchronization") | .time_ns: time_ns=20224957 -> boundary coordination already has its own explicit wall-time component
  evidence actionable_ranking.json .wall_time_components[] | select(.component == "mvcc_wait") | .time_ns: time_ns=1426749 -> page-lock handoff wait is the strongest direct MVCC coordination counter the profile exposes today
  evidence actionable_ranking.json .wall_time_components[] | select(.component == "mvcc_commit_surface") | .time_ns: time_ns=0 -> page-one tracking plus pending-surface maintenance are already split out as a separate coordination lane
  evidence profile.json .engine_report.hot_path_profile.vfs.lock_ops: count=0 -> VFS lock activity is emitted in the hot-path profile and helps explain synchronization-heavy rows without inventing a new counter
- rank 3 queueing: dominant=false, estimated_time_ns=4838575, wall_share_bps=1008, score_bps=604 -> Queueing-dominant rows are spending time inside failed BUSY attempts; steer fixes toward contention topology rather than allocator cleanup. [bd-db300.2.4]
  evidence actionable_ranking.json .wall_time_components[] | select(.component == "queueing") | .time_ns: time_ns=4838575 -> queueing is the wall-time lane for time spent inside BUSY attempts that later retried
  evidence profile.json .engine_report.runtime_phase_timing.busy_attempt_time_ns: time_ns=4838575 -> runtime phase timing captures the raw busy-attempt wall time directly
  evidence actionable_ranking.json .baseline_waste_ledger[] | select(.component == "busy_retry_queueing") | .metric_value: time_ns=56838575 -> the spillover ledger keeps busy-attempt queueing visible alongside the separate retry-backoff lane
- rank 4 service: dominant=false, estimated_time_ns=1582421, wall_share_bps=330, score_bps=198 -> Service-dominant rows should be explained with parser/decode/materialization evidence before blaming contention or durability. [bd-db300.10.2, bd-db300.10.4, bd-db300.10.5, bd-db300.10.6, bd-db300.10.7, bd-db300.10.8]
  evidence actionable_ranking.json .wall_time_components[] | select(.component == "service") | .time_ns: time_ns=1582421 -> service starts from executor body time after the explicit allocator-copy carve-out
  evidence profile.json .engine_report.runtime_phase_timing.body_execution_time_ns: time_ns=1582421 -> raw executor body time anchors the useful-work lane before allocation is split out
  evidence actionable_ranking.json .cost_components[] | select(.component == "parser_ast_churn") | .time_ns: time_ns=306485 -> parser/prepare churn is already exposed as a named cost component inside service work
  evidence actionable_ranking.json .cost_components[] | select(.component == "record_decode") | .time_ns: time_ns=217415 -> record decode remains one of the direct measured contributors to useful in-engine service time
  evidence actionable_ranking.json .cost_components[] | select(.component == "row_materialization") | .time_ns: time_ns=0 -> row materialization is kept visible so service and allocation can be reasoned about together instead of conflated
- rank 5 allocation: dominant=false, estimated_time_ns=0, wall_share_bps=0, score_bps=0 -> Allocation-dominant rows are burning time or bytes on value/page copies; use the ownership-preserving and page-buffer lanes before broader executor work. [bd-db300.10.2, bd-db300.10.3, bd-db300.10.6, bd-db300.10.7]
  evidence actionable_ranking.json .wall_time_components[] | select(.component == "allocator_copy") | .time_ns: time_ns=0 -> allocator/copy wall time is already isolated at the VDBE boundary
  evidence actionable_ranking.json .baseline_waste_ledger[] | select(.component == "page_data_normalization") | .metric_value: bytes=0 -> page normalization bytes are the clearest existing artifact for write-path allocation and copy pressure
  evidence actionable_ranking.json .cost_components[] | select(.component == "page_data_motion") | .allocator_pressure_bytes: bytes=0 -> page-data motion already exposes allocator pressure even when the surrounding wall time lives inside service work
  evidence profile.json .allocator_pressure.result_value_heap_bytes_total: bytes=0 -> result-row heap bytes keep value materialization pressure attached to the allocation bucket
- rank 6 io: dominant=false, estimated_time_ns=0, wall_share_bps=0, score_bps=0 -> I/O-dominant rows are durability-bound, so WAL/VFS evidence should steer the next cut instead of retry or parser hypotheses. [bd-db300.5.1]
  evidence actionable_ranking.json .wall_time_components[] | select(.component == "durability") | .time_ns: time_ns=0 -> durability is the explicit wall-time lane for commit-path I/O pressure
  evidence profile.json .engine_report.hot_path_profile.wal.commit_path.wal_service_us_total: time_us=0 -> commit-path WAL service time is the direct source for the durability wall-time lane when raw WAL telemetry is present
  evidence profile.json .engine_report.hot_path_profile.vfs.sync_ops: count=0 -> VFS sync operations are already exposed in the raw hot-path profile
  evidence profile.json .engine_report.hot_path_profile.vfs.write_bytes_total: bytes=0 -> VFS write-byte volume anchors file-system I/O intensity
  evidence profile.json .engine_report.hot_path_profile.wal.frames_written_total: count=0 -> WAL frames written quantify commit-path write volume without needing a new artifact
  evidence profile.json .engine_report.hot_path_profile.wal.bytes_written_total: bytes=0 -> WAL byte volume is the clearest existing counter for log-heavy I/O rows
  evidence profile.json .engine_report.hot_path_profile.wal.group_commit_latency_us_total: time_us=0 -> group-commit latency keeps the durability bucket connected to real commit-path timing
- rank 7 mixed: dominant=false, estimated_time_ns=0, wall_share_bps=0, score_bps=0 -> Mixed stays available as a deliberate fallback when the top concrete buckets are too close to call with the current evidence pack. [bd-db300.2.4, bd-db300.1.5, bd-db300.5.1, bd-db300.5.2.1, bd-db300.5.5.1]
  evidence actionable_ranking.json .wall_time_components[] | select(.component == "retry") | .time_ns: time_ns=52000000 -> retry bucket maps to explicit configured backoff sleep rather than in-attempt wait time
  evidence actionable_ranking.json .wall_time_components[] | select(.component == "synchronization") | .time_ns: time_ns=20224957 -> boundary coordination already has its own explicit wall-time component
- note: `mixed` becomes dominant when the leader concrete bucket is too small or within 1,000 score-bps of the runner-up, so near-ties stay explicit until more evidence lands.

## Microarchitectural Signatures

- rank 1 durability: primary=durability_pressure, secondary=none, confidence=high (9000bp), mixed=false -> E target: commit durability is now an explicit measured lane, so future architecture work can separate durable ordering from general executor service cost. [bd-db300.5.1] evidence=wall_time_component:durability, wal_runtime:wal_service_us_total, wal_runtime:wal_append_us_total, wal_runtime:wal_sync_us_total, wal_runtime:checkpoint_duration_us_total
- rank 2 allocator_copy: primary=llc_pressure, secondary=tlb_pressure, confidence=low (5400bp), mixed=true -> J target: allocator and copy work is large enough to justify scratch-space reuse, row-value ownership reduction, and reusable buffers. [bd-db300.10.2, bd-db300.10.3, bd-db300.10.6, bd-db300.10.7] evidence=wall_time_component:allocator_copy
- rank 3 service: primary=front_end_starvation, secondary=branch_waste, confidence=low (4800bp), mixed=true -> J target: useful body execution still dominates enough wall time that parser, decode, row-path, and residual VDBE/page-motion optimizations remain the main throughput lever once contention is under control. [bd-db300.10.2, bd-db300.10.4, bd-db300.10.5, bd-db300.10.6, bd-db300.10.7, bd-db300.10.8] evidence=wall_time_component:service, subsystem_hotspot:parser_ast_churn
- rank 4 mvcc_commit_surface: primary=mixed_or_ambiguous, secondary=none, confidence=low (4300bp), mixed=true -> E2.1/E5.1 target: page-one tracking plus pending-surface maintenance are an explicit wall-time lane, which is the right steering signal for publish-plane shrink versus page-topology work. [bd-db300.5.2.1, bd-db300.5.5.1] evidence=wall_time_component:mvcc_commit_surface, mvcc:page_one_conflict_track_time_ns_total, mvcc:pending_commit_surface_clear_time_ns_total
- rank 5 mvcc_wait: primary=mixed_or_ambiguous, secondary=none, confidence=low (4200bp), mixed=true -> E2.1/E5.1 target: page-lock wait time is now explicit in the wall-time story, so tiny-publish and topological disjointness can be judged against a real synchronization lane instead of guesses. [bd-db300.5.2.1, bd-db300.5.5.1] evidence=wall_time_component:mvcc_wait, mvcc:page_lock_wait_time_ns_total
- rank 6 synchronization: primary=mixed_or_ambiguous, secondary=none, confidence=low (4000bp), mixed=true -> A/E target: transaction-boundary coordination is still a first-class wall-time tax and should be pushed toward narrower residual serialized regions. [bd-db300.1.5, bd-db300.5.1] evidence=wall_time_component:synchronization, wal_runtime:flusher_lock_wait_us_total, wal_runtime:wal_backend_lock_wait_us_total, wal_runtime:hist_wal_backend_lock_wait, wal_runtime:wake_reasons
- rank 7 queueing: primary=mixed_or_ambiguous, secondary=none, confidence=low (3500bp), mixed=true -> A/B target: retried BUSY attempts are consuming visible wall time before useful work resumes, so the handoff policy should be tightened before scaling further. [bd-db300.2.4] evidence=wall_time_component:queueing
- rank 8 retry: primary=mixed_or_ambiguous, secondary=none, confidence=low (3500bp), mixed=true -> B target: sleep-based retry backoff is still showing up in the hot cell and should be replaced with a bounded handoff strategy. [bd-db300.2.4] evidence=wall_time_component:retry

## Allocator Pressure

- rank 1 parser_sql_bytes: bytes=1703 -> J2/J4 target: parse-volume churn is visible and should be reduced with reuse rather than repeated prepare work. [bd-db300.10.2, bd-db300.10.4]
- rank 2 page_data_normalization_bytes: bytes=0 -> J3/J6 target: full-page normalization is materializing avoidable bytes before writes, so owned passthrough and reusable page buffers are still live optimization work. [bd-db300.10.3, bd-db300.10.6]
- rank 3 record_decode_values: heap_bytes=0 -> J2/J5 target: decoded record values create enough heap churn to justify scratch buffers and decode caching. [bd-db300.10.2, bd-db300.10.5]
- rank 4 result_row_values: heap_bytes=0 -> J2/J6/J7 target: emitted result rows are carrying most of the transient heap pressure and should benefit from ownership and frame reuse. [bd-db300.10.2, bd-db300.10.6, bd-db300.10.7]

## Top Opcodes


## Replay

```sh
rch exec -- env FSQLITE_HOT_PATH_BEAD_ID=hot-page-tail-delete-fix-c4 FSQLITE_HOT_PATH_WORKSPACE_ROOT=/data/projects/frankensqlite cargo run -p fsqlite-e2e --bin realdb-e2e -- hot-profile --db frankensqlite --workload hot_page_contention --golden-dir /data/projects/frankensqlite/sample_sqlite_db_files/golden --working-base /data/projects/frankensqlite/sample_sqlite_db_files/working --concurrency 4 --seed 42 --scale 20 --output-dir /data/projects/frankensqlite/tests/artifacts/perf/hotprofile-hot-page-tail-delete-fix-c4-darklark-20260504T0700Z --mvcc
```

## Structured Artifacts

- `profile.json` — raw scenario profile
- `opcode_profile.json` — raw opcode totals for this profiled run
- `subsystem_profile.json` — raw execution-subsystem timing, heap profile, WAL commit-path split/tail metrics, and B-tree copy-kernel target list for this run
- `actionable_ranking.json` — hotspot, MVCC, reuse, and baseline-waste ledger for follow-on Track E/Track J work
- `manifest.json` — replay metadata + artifact inventory

## Mandatory Perf Checklist

- [ ] Confirm the cited claim matches this exact tuple: fixture `frankensqlite`, workload `hot_page_contention`, concurrency `c4`, concurrent mode `ON`.
- [ ] Confirm the build/profile provenance is carried with the artifact bundle and that `manifest.json` plus the replay command remain attached to any benchmark note or PR.
- [ ] Confirm `summary.md`, `profile.json`, `subsystem_profile.json`, and `actionable_ranking.json` are published together before drawing conclusions from a single wall-time number.
- [ ] Confirm the top hotspot or causal bucket is translated into a concrete next action, including the mapped bead IDs or an explicit note that no mapped follow-up exists.
- [ ] Confirm any caveat from integrity-check state, retry behavior, or fallback counters is disclosed alongside the claim instead of being left implicit.
