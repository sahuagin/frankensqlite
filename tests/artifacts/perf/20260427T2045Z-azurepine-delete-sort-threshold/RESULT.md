# Delete sort threshold rejection

Agent: AzurePine
Date: 2026-04-27
Workload: `perf-update-delete 10000 <iters> <delete|both>`
Decision: rejected and rolled back

## Profile basis

Current `HEAD` before the candidate:

```text
4d5347ad perf: publish direct insert stateless hint rejection
```

Profile command:

```bash
perf record -F 997 -g --call-graph dwarf \
  -o /data/tmp/azurepine-proceed-head-62c3ecc.data -- \
  /data/tmp/cargo-target-azurepine-20260427-head-62c3ecc/release-perf/perf-update-delete 10000 100 both
```

Profiled run:

```text
total=1695ms populate=856ms update=492ms delete=267ms
```

Flat profile showed delete-side sorting as a plausible target:

| symbol | self |
| --- | ---: |
| `__memmove_avx_unaligned_erms` | 8.13% |
| `sort_unstable_by<sort_cells_desc_by_ptr>` quicksort | 6.23% |
| `Connection::execute_prepared_direct_simple_insert` | 5.37% |
| `BtCursor<SharedTxnPageIo> as BtreeCursorOps>::delete` | 4.91% |
| `small_sort_general<sort_cells_desc_by_ptr>` | 4.07% |

Children profile also put `sort_cells_desc_by_ptr` under delete:

| symbol | children | self |
| --- | ---: | ---: |
| `remove_table_cell_from_leaf_deferred` | 12.92% | 0.00% |
| `sort_cells_desc_by_ptr` | 11.11% | 0.00% |
| `sort_unstable_by<(usize, usize, usize)>` | 11.11% | 0.00% |
| quicksort comparator closure | 10.88% | 6.17% |

## Candidate

The candidate changed only `sort_cells_desc_by_ptr`:

```rust
const INSERTION_SORT_THRESHOLD: usize = 20;
```

to:

```rust
const INSERTION_SORT_THRESHOLD: usize = 64;
```

The sort key and order stayed unchanged: largest cell pointer first. The candidate was intended to avoid the generic pdqsort path for 40-60 cell delete-defrag inputs.

Correctness check while the candidate was present:

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-azurepine-20260427-delete-sort64-test \
  cargo test -p fsqlite-btree remove_cell_from_leaf_specialized_sort_matches_std -- --nocapture
```

Result: passed.

Candidate binary:

```text
/data/tmp/cargo-target-azurepine-20260427-delete-sort64/release-perf/perf-update-delete
```

Baseline binary:

```text
/data/tmp/cargo-target-azurepine-20260427-head-62c3ecc/release-perf/perf-update-delete
```

## A/B summary

Short matrix:

| workload | iters | metric | baseline avg | candidate avg | result |
| --- | ---: | --- | ---: | ---: | ---: |
| delete | 50 | total | 563.3ms | 562.3ms | neutral |
| delete | 50 | delete | 107.0ms | 106.0ms | 0.9% faster |
| delete | 100 | total | 1107.0ms | 1121.7ms | 1.3% slower |
| delete | 100 | delete | 210.0ms | 208.7ms | 0.6% faster |
| delete | 200 | total | 2214.0ms | 2211.7ms | neutral |
| delete | 200 | delete | 418.3ms | 417.7ms | neutral |
| both | 50 | total | 817.0ms | 829.0ms | 1.5% slower |
| both | 100 | total | 1641.7ms | 1626.7ms | 0.9% faster |
| both | 200 | total | 3268.0ms | 3277.7ms | neutral |

Longer 500-iteration confirmation:

| workload | metric | baseline avg | candidate avg | result |
| --- | --- | ---: | ---: | ---: |
| delete | total | 5470.7ms | 5579.3ms | 2.0% slower |
| delete | delete | 1030.7ms | 1035.7ms | 0.5% slower |
| both | total | 8147.0ms | 8145.3ms | neutral |
| both | delete | 1205.3ms | 1217.7ms | 1.0% slower |

The longer run rejected the candidate. The lower-threshold historical crossover in the code comment was still valid for the full workload shape; moving insertion sort to N <= 64 did not convert the profile hotspot into a wall-time win.

## Raw runs

```text
base which=delete batch=50 run=1 total=563ms populate=419ms update=0ms delete=108ms
cand which=delete batch=50 run=1 total=550ms populate=412ms update=0ms delete=104ms
base which=delete batch=50 run=2 total=569ms populate=427ms update=0ms delete=106ms
cand which=delete batch=50 run=2 total=567ms populate=424ms update=0ms delete=106ms
base which=delete batch=50 run=3 total=558ms populate=416ms update=0ms delete=107ms
cand which=delete batch=50 run=3 total=570ms populate=424ms update=0ms delete=108ms

base which=delete batch=100 run=1 total=1123ms populate=841ms update=0ms delete=214ms
cand which=delete batch=100 run=1 total=1102ms populate=832ms update=0ms delete=207ms
base which=delete batch=100 run=2 total=1082ms populate=816ms update=0ms delete=203ms
cand which=delete batch=100 run=2 total=1156ms populate=874ms update=0ms delete=212ms
base which=delete batch=100 run=3 total=1116ms populate=835ms update=0ms delete=213ms
cand which=delete batch=100 run=3 total=1107ms populate=835ms update=0ms delete=207ms

base which=delete batch=200 run=1 total=2222ms populate=1654ms update=0ms delete=429ms
cand which=delete batch=200 run=1 total=2217ms populate=1654ms update=0ms delete=423ms
base which=delete batch=200 run=2 total=2179ms populate=1650ms update=0ms delete=405ms
cand which=delete batch=200 run=2 total=2214ms populate=1680ms update=0ms delete=410ms
base which=delete batch=200 run=3 total=2241ms populate=1676ms update=0ms delete=421ms
cand which=delete batch=200 run=3 total=2204ms populate=1664ms update=0ms delete=420ms

base which=both batch=50 run=1 total=826ms populate=406ms update=252ms delete=125ms
cand which=both batch=50 run=1 total=821ms populate=415ms update=237ms delete=127ms
base which=both batch=50 run=2 total=812ms populate=412ms update=242ms delete=119ms
cand which=both batch=50 run=2 total=814ms populate=408ms update=239ms delete=127ms
base which=both batch=50 run=3 total=813ms populate=407ms update=237ms delete=129ms
cand which=both batch=50 run=3 total=852ms populate=462ms update=245ms delete=109ms

base which=both batch=100 run=1 total=1619ms populate=836ms update=482ms delete=226ms
cand which=both batch=100 run=1 total=1634ms populate=819ms update=479ms delete=256ms
base which=both batch=100 run=2 total=1650ms populate=824ms update=486ms delete=259ms
cand which=both batch=100 run=2 total=1624ms populate=817ms update=469ms delete=259ms
base which=both batch=100 run=3 total=1656ms populate=838ms update=476ms delete=257ms
cand which=both batch=100 run=3 total=1622ms populate=847ms update=484ms delete=215ms

base which=both batch=200 run=1 total=3256ms populate=1627ms update=948ms delete=525ms
cand which=both batch=200 run=1 total=3268ms populate=1632ms update=981ms delete=498ms
base which=both batch=200 run=2 total=3303ms populate=1661ms update=971ms delete=513ms
cand which=both batch=200 run=2 total=3309ms populate=1673ms update=983ms delete=497ms
base which=both batch=200 run=3 total=3245ms populate=1628ms update=947ms delete=518ms
cand which=both batch=200 run=3 total=3256ms populate=1672ms update=957ms delete=483ms

base-long which=delete batch=500 run=1 total=5550ms populate=4189ms update=0ms delete=1050ms
cand-long which=delete batch=500 run=1 total=5729ms populate=4317ms update=0ms delete=1057ms
base-long which=delete batch=500 run=2 total=5388ms populate=4088ms update=0ms delete=1013ms
cand-long which=delete batch=500 run=2 total=5407ms populate=4116ms update=0ms delete=1002ms
base-long which=delete batch=500 run=3 total=5474ms populate=4139ms update=0ms delete=1029ms
cand-long which=delete batch=500 run=3 total=5602ms populate=4201ms update=0ms delete=1048ms

base-long which=both batch=500 run=1 total=8214ms populate=4121ms update=2476ms delete=1241ms
cand-long which=both batch=500 run=1 total=8262ms populate=4185ms update=2438ms delete=1268ms
base-long which=both batch=500 run=2 total=8052ms populate=4136ms update=2430ms delete=1131ms
cand-long which=both batch=500 run=2 total=8022ms populate=4153ms update=2401ms delete=1111ms
base-long which=both batch=500 run=3 total=8175ms populate=4139ms update=2403ms delete=1244ms
cand-long which=both batch=500 run=3 total=8152ms populate=4124ms update=2373ms delete=1274ms
```

## Conclusion

The source edit was rolled back. Keep `INSERTION_SORT_THRESHOLD = 20`.

The next delete-side lever should avoid building or sorting the move list for monotone pages, rather than extending insertion sort deeper into the mixed-order case.
