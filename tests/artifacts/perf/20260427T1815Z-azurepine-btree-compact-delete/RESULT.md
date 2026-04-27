# Compact Table Leaf Delete Defrag

## Scope

- Initial baseline commit: `8ffdf9e0`
  (`fix(btree): reject out-of-range table cell pointers`)
- Parent-baseline control: `18c0778e`
  (`fix(wal): harden parallel segment record decoding`)
- Candidate source: `crates/fsqlite-btree/src/cursor.rs`
- Workload: `perf-update-delete 10000 50 both` and `perf-update-delete 10000 200 both`
- Build profile: `release-perf` with frame pointers and line-table debug symbols

The current-head profile for `perf-update-delete 10000 100 both` put the next
tractable cluster in table-leaf DELETE defragmentation:

| Overhead | Symbol |
| ---: | --- |
| 5.23% | `sort_cells_desc_by_ptr` via `sort_unstable_by` |
| 5.22% | `BtCursor<SharedTxnPageIo>::delete` |
| 4.76% | `fsqlite_btree::cell::cell_on_page_size_fast` |

The source change adds a guarded fast path in
`remove_table_cell_from_leaf_deferred`: when a table leaf page is already
compact (`first_freeblock == 0`, `fragmented_free_bytes == 0`, and the minimum
cell pointer matches the header content offset), cell byte extents are derived
from sorted adjacent cell offsets instead of reparsing every remaining cell with
`cell_on_page_size_fast`.

The deleted cell remains in the temporary offset list as a sentinel while sizes
are derived, then it is skipped during the copy phase. That preserves the size
of the next lower cell; otherwise the deleted cell's byte range would be
incorrectly merged into its neighbor.

## Isomorphism Proof

- Ordering preserved: yes. The existing `sort_cells_desc_by_ptr` still defines
  copy order, and the pointer array is rewritten with the same post-delete cell
  indexes.
- Tie-breaking unchanged: yes for valid pages. Cell offsets are unique; corrupt
  duplicate/non-monotone offsets now return `DatabaseCorrupt`.
- Floating-point: N/A.
- RNG seeds: unchanged.
- Fallback behavior: fragmented/freeblock pages keep the existing
  `cell_on_page_size_fast` path.
- Byte movement: same packed final layout. The compact fast path computes each
  source cell's extent from its original adjacent boundary, including the
  deleted-cell boundary, then copies only non-deleted cells into the compacted
  destination area.

## Timing: 10000 Rows, 50 Iterations

| Run | Baseline total | Candidate total | Baseline populate | Candidate populate | Baseline update | Candidate update | Baseline delete | Candidate delete |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 852 ms | 838 ms | 415 ms | 418 ms | 275 ms | 253 ms | 126 ms | 125 ms |
| 2 | 859 ms | 812 ms | 411 ms | 423 ms | 271 ms | 234 ms | 139 ms | 118 ms |
| 3 | 869 ms | 841 ms | 411 ms | 426 ms | 278 ms | 250 ms | 139 ms | 125 ms |
| 4 | 841 ms | 809 ms | 410 ms | 412 ms | 268 ms | 234 ms | 126 ms | 125 ms |
| 5 | 885 ms | 822 ms | 420 ms | 414 ms | 285 ms | 248 ms | 139 ms | 123 ms |
| 6 | 873 ms | 821 ms | 417 ms | 416 ms | 276 ms | 242 ms | 140 ms | 125 ms |
| 7 | 856 ms | 823 ms | 411 ms | 419 ms | 270 ms | 242 ms | 136 ms | 124 ms |

Median total: `859 ms -> 822 ms` (`4.31%` faster).

Median update: `275 ms -> 242 ms` (`12.00%` faster).

Median delete: `139 ms -> 125 ms` (`10.07%` faster).

## Timing: 10000 Rows, 200 Iterations

| Run | Baseline total | Candidate total | Baseline populate | Candidate populate | Baseline update | Candidate update | Baseline delete | Candidate delete |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 3382 ms | 3254 ms | 1644 ms | 1661 ms | 1064 ms | 958 ms | 535 ms | 490 ms |
| 2 | 3405 ms | 3282 ms | 1669 ms | 1676 ms | 1071 ms | 966 ms | 527 ms | 495 ms |
| 3 | 3429 ms | 3155 ms | 1656 ms | 1631 ms | 1065 ms | 944 ms | 562 ms | 449 ms |
| 4 | 3396 ms | 3184 ms | 1626 ms | 1635 ms | 1083 ms | 961 ms | 543 ms | 450 ms |
| 5 | 3394 ms | 3282 ms | 1684 ms | 1671 ms | 1060 ms | 959 ms | 515 ms | 503 ms |

Median total: `3396 ms -> 3254 ms` (`4.18%` faster).

Median update: `1065 ms -> 959 ms` (`9.95%` faster).

Median delete: `535 ms -> 490 ms` (`8.41%` faster).

After a clippy-only rewrite from an `if` chain to `cmp`/`match`, the exact final
candidate binary was rebuilt and rerun on a short sanity pass:

| Run | Baseline total | Candidate total | Baseline update | Candidate update | Baseline delete | Candidate delete |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 851 ms | 817 ms | 268 ms | 244 ms | 139 ms | 127 ms |
| 2 | 862 ms | 809 ms | 267 ms | 234 ms | 137 ms | 124 ms |
| 3 | 843 ms | 807 ms | 271 ms | 235 ms | 135 ms | 124 ms |

Final-source sanity medians: total `851 ms -> 809 ms`, update `268 ms -> 235 ms`,
delete `137 ms -> 124 ms`.

While this slice was in progress, the unrelated WAL hardening commit
`18c0778e` landed underneath the final B-tree commit. To isolate this source
change from that parent movement, I built a detached `18c0778e` baseline and
reran a short comparison against the exact final candidate binary:

```bash
git worktree add --detach /data/tmp/frankensqlite-azurepine-btree-baseline-18c-20260427 18c0778e
```

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-azurepine-20260427-btree-baseline-18c CARGO_PROFILE_RELEASE_PERF_DEBUG=line-tables-only CARGO_PROFILE_RELEASE_PERF_STRIP=false RUSTFLAGS='-C force-frame-pointers=yes' cargo build --profile release-perf -p fsqlite-e2e --bin perf-update-delete
```

| Run | Parent baseline total | Candidate total | Parent baseline update | Candidate update | Parent baseline delete | Candidate delete |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 843 ms | 810 ms | 268 ms | 238 ms | 121 ms | 119 ms |
| 2 | 865 ms | 816 ms | 278 ms | 241 ms | 137 ms | 127 ms |
| 3 | 848 ms | 812 ms | 265 ms | 241 ms | 142 ms | 122 ms |

Parent-baseline medians: total `848 ms -> 812 ms`, update `268 ms -> 241 ms`,
delete `137 ms -> 122 ms`.

## Profile Check

Baseline profile command:

```bash
perf record -F 997 -g --call-graph dwarf -o /data/tmp/azurepine-next-head.data -- /data/tmp/cargo-target-azurepine-20260427-next-profile/release-perf/perf-update-delete 10000 100 both
```

Final candidate profile command:

```bash
perf record -F 997 -g --call-graph dwarf -o /data/tmp/azurepine-btree-compact-final.data -- /data/tmp/cargo-target-azurepine-20260427-btree-compact-candidate/release-perf/perf-update-delete 10000 100 both
```

Final candidate profile output:

```text
total=1600ms populate=816ms update=480ms delete=237ms
```

Filtered final profile:

| Overhead | Symbol |
| ---: | --- |
| 8.70% | `__memmove_avx_unaligned_erms` |
| 6.11% | `BtCursor<SharedTxnPageIo>::delete` |
| 5.37% | `Connection::execute_prepared_direct_simple_insert` |
| 4.07% | `sort_cells_desc_by_ptr` via `sort_unstable_by` |
| 3.78% | `small_sort_general` |

`cell_on_page_size_fast` was not present in the filtered final profile output
for the target path. Sort/copy work remains visible and is the next likely
B-tree delete target.

## Verification

```bash
cargo fmt --check
```

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-azurepine-20260427-btree-compact cargo test -p fsqlite-btree delete -- --nocapture
```

Result: `29 passed; 0 failed`.

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-azurepine-20260427-btree-compact cargo check -p fsqlite-btree --all-targets
```

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-azurepine-20260427-btree-compact cargo clippy -p fsqlite-btree --all-targets -- -D warnings
```

```bash
ubs crates/fsqlite-btree/src/cursor.rs tests/artifacts/perf/20260427T1815Z-azurepine-btree-compact-delete/RESULT.md
```

UBS exited 0. It reported existing broad warning inventory in the large
`cursor.rs` test/implementation file, with no critical findings.

## Decision

Keep the source change. It is a guarded hot-path optimization with a clear
compact-page invariant, preserves the existing parser-based path for fragmented
pages, and repeatedly improves the mixed update/delete workload by about 4% in
total wall time with larger update/delete phase wins.
