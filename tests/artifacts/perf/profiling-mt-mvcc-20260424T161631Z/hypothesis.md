# Root-Cause Hypotheses

## H1: Full-page mutation hashing is now the top CPU hotspot

Evidence:

- MT8 flat profile: `xxh3_64_long_default` 8.09% self time.
- MT2 flat profile: same symbol 4.85% self time.
- MT8 callgraph enters the hash from `BtCursor::load_page` and `page_mutation_counter` under `execute_prepared_direct_simple_insert`.
- Source trace: `page_mutation_counter` hashes `page[..usable_size]` and folds the 64-bit hash to a 32-bit non-zero counter.

First-principle diagnosis:

The B-tree cursor needs a stable page-change token to invalidate cell-slot caches. Hashing the full usable page is a content-derived token, but it charges O(page_size) every time the token is needed. On a hot append path where page identity and pager write events are already known, a pager-maintained generation/epoch should provide the same invalidation semantics with O(1) cost.

Likely next optimization:

Replace content hashing with a pager/B-tree page generation token. Candidate designs include seqlock-style page epochs, write-epoch counters on `PageData`, or an explicit mutation generation returned by `PageReader::read_page_data`.

Proof boundary:

- Property tests for cache invalidation when a page is modified in place, reloaded, recycled, split, merged, and read from a different snapshot.
- Existing B-tree insert/delete/search tests.
- `mt-mvcc-bench` 1/2/4/8 rows-per-thread 500 and 5000.
- Roll back if `xxh3` removal saves less than 3% MT8 CPU or changes B-tree correctness behavior.

## H2: The 1-to-2 writer cliff is structural conflict coordination, not a parser/cache problem

Evidence:

- 500-row matrix: 265,987 wps at 1T, 9,559 wps at 2T.
- 5000-row probe: 584,167 wps at 1T, 79,201 wps at 2T.
- 16T probe fails after retry exhaustion with repeated page-2 snapshot conflicts.
- Syscall sketch: `futex` + `clock_nanosleep` account for about 73% of traced syscall time in a small 8T probe.
- CPU profiles show page-lock waiting less prominently than before because waiting is mostly off-CPU.

First-principle diagnosis:

Concurrent-writer benefit only appears when writers touch disjoint conflict surfaces. This benchmark inserts monotonically increasing rowids by thread stride, but all writers still share early root/interior/right-edge pages during B-tree navigation and commit validation. Once multiple writers enter the same page conflict window, retries and waits dominate.

Likely next optimization:

Treat the write path as a queueing problem, not just a lock micro-optimization. Verify whether Silo-style epoch group commit is admitting batches that still conflict on the same hot pages, and whether page-2/root/interior conflicts can be separated by deterministic writer routing, right-edge split hints, or a combined page-update critical section.

Proof boundary:

- Instrument conflict pages by count and wait time.
- Record per-transaction retry counts and conflict surface sizes.
- Add a deterministic synthetic workload with pre-partitioned leaf pages to prove whether conflict is root/interior vs leaf-only.
- Roll back if a routing/group-commit change improves MT8 but regresses single-thread insert by more than 3% or weakens conflict correctness.

## H3: Short-sample profiles still include connection lifecycle allocation

Evidence:

- MT2 flat profile: `ShardedPageCache::with_max_buffers` 3.37%, `Arc<ShardedPageCache>::drop_slow` 2.30%.
- MT8 flat profile: `ShardedPageCache::with_max_buffers` 1.27%, `Arc<ShardedPageCache>::drop_slow` 2.11%.
- Source trace: each benchmark sample creates a fresh DB path and opens fresh worker connections.
- Amortization probe improves absolute throughput materially but does not remove the concurrency cliff.

First-principle diagnosis:

Connection-open cache construction is a real product cost, but it is not the same as steady-state concurrent write throughput. The benchmark currently mixes both. Optimizing lifecycle may improve benchmark numbers, but it should not be confused with fixing the multi-writer cliff.

Likely next optimization:

Add a measurement mode that separates setup from steady-state, or add a second benchmark binary for fsqlite-only steady-state connection reuse. Engine-side lifecycle candidates include lazy flat-slot initialization and shared cold cache metadata.

Proof boundary:

- Keep the current apples-to-apples benchmark for external comparison.
- Add a new explicit steady-state scenario rather than changing existing semantics silently.
- Use both scenarios in future optimization scorecards.

## H4: Allocation and data movement are secondary but persistent

Evidence:

- MT8: `__memmove` 2.79%, `_int_malloc` 2.63%, `cfree` 0.88%, plus other allocator symbols.
- MT2: `__memmove` 3.86%, `_int_malloc` 1.61%, `__memset` 1.68%.

First-principle diagnosis:

Prepared-cache work removed the earlier parse/plan churn, but row construction, bound text payload creation, page movement, and cache lifecycle still allocate/move enough to remain visible. This is lower priority than H1/H2 because no single allocator symbol dominates and some cost belongs to benchmark payload formatting.

Likely next optimization:

Use allocation tracing or a targeted allocation counter before changing code. Candidate paths: bound parameter text conversion, cell buffer reuse, cell pointer vectors, and page cache lifecycle.
