# bd-m4s2c Hotspot Table

Capture: `bd-m4s2c-20260423T013614Z` on `thinkstation2` via RCH-built `release-perf` binaries.

Local HEAD at artifact assembly: `9f35e94166e6bc9eff3a484c4c15b42a2b3f5da3` (`capture-local-head.txt`). Remote git metadata reported `6e89c8261cbb9c4a15550738bf108f61e14e9b27` after RCH sync (`capture-context.txt`); use local HEAD plus `capture-local-git-status.txt` as the source-tree identity.

Method: `perf record -e cpu-clock:u -F 497 --call-graph fp` followed by `perf report --stdio --no-children --sort=overhead,symbol,dso`. The worker started at `perf_event_paranoid=4`, was temporarily relaxed to `1`, and was restored to `4`; see `perf-event-paranoid-before-relax.txt`, `perf-event-paranoid-during.txt`, and `perf-event-paranoid-after-restore.txt`.

Short single-thread scenarios were repeated inside one capture only to collect enough samples. The scenario input shape stayed fixed: each insert run is 1,000 rows, and each update iteration updates 100 rows in a 1,000-row table.

## Scenario 1: 1t INSERT 1000 small_3col

Command: `cmd-insert_1000_small_3col.txt`

Samples: 64 (`run-insert_1000_small_3col.stderr.log`). Wall/RSS: 0.76s, 76,040 KB (`time-insert_1000_small_3col.txt`).

| rank | self-time | symbol | evidence |
| ---: | ---: | --- | --- |
| 1 | 10.94% | `<fsqlite_pager::pager::PublishedPagerState>::new` | `perf-report-insert_1000_small_3col.txt:12` |
| 2 | 10.94% | `[vdso] clock_gettime path` | `perf-report-insert_1000_small_3col.txt:45` |
| 3 | 7.81% | `<fsqlite_core::connection::Connection>::execute_prepared_direct_simple_insert` | `perf-report-insert_1000_small_3col.txt:161` |
| 4 | 6.25% | `core::ptr::drop_in_place::<[AtomicPublishedPageSlot]>` | `perf-report-insert_1000_small_3col.txt:187` |
| 5 | 4.69% | `xxhash_rust::xxh3::xxh3_64_long_default` | `perf-report-insert_1000_small_3col.txt:215` |
| 6 | 3.12% | `BtCursor::parse_cell_at` | `perf-report-insert_1000_small_3col.txt:238` |
| 7 | 3.12% | `fsqlite_btree::cell::read_cell_pointers_into` | `perf-report-insert_1000_small_3col.txt:261` |
| 8 | 3.12% | `malloc` | `perf-report-insert_1000_small_3col.txt:290` |

Instrumentation evidence from the ignored bench confirms the 1,000-row direct-insert path: `direct_execs=1000`, `fast_execs=1000`, and repeated `btree_insert_us` timing rows in `run-insert_1000_small_3col.stderr.log:1`.

## Scenario 2: UPDATE 100 of 1000

Command: `cmd-update_100_of_1000.txt`

Samples: 34 (`run-update_100_of_1000.stderr.log`). Wall/RSS: 0.27s, 52,488 KB (`time-update_100_of_1000.txt`). The workload reports `update_count=100` and 20 built-in iterations for sampling (`run-update_100_of_1000.stderr.log:1`).

| rank | self-time | symbol | evidence |
| ---: | ---: | --- | --- |
| 1 | 11.76% | `[vdso] clock_gettime path` | `perf-report-update_100_of_1000.txt:12` |
| 2 | 5.88% | `<alloc::sync::Arc<PublishedPagerState>>::drop_slow` | `perf-report-update_100_of_1000.txt:37` |
| 3 | 2.94% | `<alloc::sync::Arc<ShardedPageCache>>::drop_slow` | `perf-report-update_100_of_1000.txt:50` |
| 4 | 2.94% | `<fsqlite_btree::cell::CellRef>::parse` | `perf-report-update_100_of_1000.txt:63` |
| 5 | 2.94% | `BtCursor::rowid` | `perf-report-update_100_of_1000.txt:77` |
| 6 | 2.94% | `BtCursor::parse_cell_at` | `perf-report-update_100_of_1000.txt:92` |
| 7 | 2.94% | `BtCursor::table_seek_for_insert` | `perf-report-update_100_of_1000.txt:105` |
| 8 | 2.94% | `WalChecksumTransform::for_wal_frame` | `perf-report-update_100_of_1000.txt:221` |

## Scenario 3: Concurrent Writers 8t x 500

Command: `cmd-mt_writers_8x500.txt`

Samples: 59 (`run-mt_writers_8x500.stderr.log`). Wall/RSS: 5.54s, 65,812 KB (`time-mt_writers_8x500.txt`). Throughput row: fsqlite 778 wps vs sqlite 32,147 wps, 0.02x throughput, 41.33x time ratio (`run-mt_writers_8x500.stdout.log:2`).

| rank | self-time | symbol | evidence |
| ---: | ---: | --- | --- |
| 1 | 13.56% | `<fsqlite_pager::pager::PublishedPages>::clear` | `perf-report-mt_writers_8x500.txt:12` |
| 2 | 5.08% | `<alloc::sync::Arc<PublishedPagerState>>::drop_slow` | `perf-report-mt_writers_8x500.txt:51` |
| 3 | 5.08% | `ShardedPageCache::metrics_snapshot` / `PageSlot::stable_snapshot` iterator | `perf-report-mt_writers_8x500.txt:63` |
| 4 | 5.08% | `cfree` | `perf-report-mt_writers_8x500.txt:88` |
| 5 | 3.39% | `<fsqlite_core::connection::Connection>::execute_prepared_direct_simple_insert` | `perf-report-mt_writers_8x500.txt:118` |
| 6 | 1.69% | `<alloc::raw_vec::RawVecInner>::finish_grow` | `perf-report-mt_writers_8x500.txt:129` |
| 7 | 1.69% | `<fsqlite_ast::InsertStatement as Clone>::clone` | `perf-report-mt_writers_8x500.txt:161` |
| 8 | 1.69% | `<fsqlite_btree::cell::CellRef>::parse` | `perf-report-mt_writers_8x500.txt:174` |

## Scenario 4: Concurrent Writers 16t x 500

Command: `cmd-mt_writers_16x500.txt`

Samples: 97 (`run-mt_writers_16x500.stderr.log`). Wall/RSS: 1.15s, 127,544 KB (`time-mt_writers_16x500.txt`). Throughput row: fsqlite 36,988 wps vs sqlite 12,364 wps, 2.99x throughput, 0.33x time ratio (`run-mt_writers_16x500.stdout.log:2`).

| rank | self-time | symbol | evidence |
| ---: | ---: | --- | --- |
| 1 | 12.37% | `<fsqlite_pager::pager::PublishedPages>::clear` | `perf-report-mt_writers_16x500.txt:12` |
| 2 | 7.22% | `ShardedPageCache::metrics_snapshot` / `PageSlot::stable_snapshot` iterator | `perf-report-mt_writers_16x500.txt:51` |
| 3 | 6.19% | `xxhash_rust::xxh3::xxh3_64_long_default` | `perf-report-mt_writers_16x500.txt:92` |
| 4 | 5.15% | `<fsqlite_pager::pager::PublishedPagerState>::new` | `perf-report-mt_writers_16x500.txt:119` |
| 5 | 4.12% | `<fsqlite_pager::page_cache::FlatPageSlots>::clear` | `perf-report-mt_writers_16x500.txt:130` |
| 6 | 3.09% | `<fsqlite_core::connection::Connection>::execute_prepared_with_params_after_background_status` | `perf-report-mt_writers_16x500.txt:157` |
| 7 | 3.09% | `malloc` | `perf-report-mt_writers_16x500.txt:167` |
| 8 | 2.06% | `<fsqlite_ast::InsertStatement as Clone>::clone` | `perf-report-mt_writers_16x500.txt:204` |

## Prior Top-7 Hotspot Status

| 2026-04-19 hotspot | prior self-time | current self-time evidence | movement |
| --- | ---: | --- | --- |
| `memcpy` | 6.77% | Not present in the top reported rows for any scenario (`perf-report-*.txt`) | appears retired or below sample visibility |
| `CellRef::parse` | 4.80% | 2.94% in update (`perf-report-update_100_of_1000.txt:63`), 1.69% in 8t (`perf-report-mt_writers_8x500.txt:174`), 1.03% in 16t (`perf-report-mt_writers_16x500.txt:439`) | lower but still present |
| `execute_prepared_direct_simple_insert` | 4.24% | 7.81% in insert (`perf-report-insert_1000_small_3col.txt:161`), 3.39% in 8t (`perf-report-mt_writers_8x500.txt:118`), 2.94% in update setup (`perf-report-update_100_of_1000.txt:157`) | still central |
| `_int_malloc` | 3.96% | `malloc` 3.12% in insert (`perf-report-insert_1000_small_3col.txt:290`), 3.09% in 16t (`perf-report-mt_writers_16x500.txt:167`); `cfree` 5.08% in 8t (`perf-report-mt_writers_8x500.txt:88`) | allocation/free still material |
| `Arc::make_mut` | 1.77% | only appears in call chains, not as a self-time row (`perf-report-update_100_of_1000.txt:400`, `perf-report-mt_writers_8x500.txt:619`) | not a current top self-time row |
| `Vec::finish_grow` | 1.20% | `RawVecInner::finish_grow` 1.69% in 8t (`perf-report-mt_writers_8x500.txt:129`) | still present in concurrent writer path |
| `WalChecksumTransform` | 0.79% | 2.94% in update (`perf-report-update_100_of_1000.txt:221`) | grew in update scenario |

## Wave-6 Readout

The ranked table points Wave 6 toward pager publication/snapshot churn first, not the old `memcpy` hotspot. `PublishedPages::clear`, `PublishedPagerState::new`, and `ShardedPageCache::metrics_snapshot` dominate both concurrent-writer profiles. The 8-thread throughput row is still pathological in this capture, while the 16-thread one was favorable in this single run; treat the 16t result as requiring repeat confirmation before using it to dismiss the 8t bottleneck.

The prepared INSERT path remains visible, especially in the single-thread insert scenario. Allocation/free and WAL checksum work are still measurable, but they are below the publication/snapshot churn in the concurrent-writer captures.
