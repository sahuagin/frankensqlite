# Compact table-leaf delete descending fast path

Date: 2026-04-27
Agent: AzurePine
Base commit: `5f7cb317b62fba209066dd3ec0debac803e84b19`
Candidate patch: `crates/fsqlite-btree/src/cursor.rs`

## Hotspot

`perf-update-delete 10000 100 both` showed delete-side cell relocation sorting as a repeat CPU cost before this pass:

- `quicksort::<... sort_cells_desc_by_ptr ...>`: 6.23% self
- `small_sort_general::<... sort_cells_desc_by_ptr ...>`: 4.07% self
- `BtCursor::delete`: 4.91% self

The optimization keeps the existing generic move-list/sort path for fragmented pages and compact pages whose pointer array is not already in descending physical cell-offset order. For the common compact table-leaf shape created by append-heavy rowid workloads, it bypasses the temporary `cells_to_move` vector and sort, deriving each cell size from the previous physical boundary while rewriting cells.

## Candidate profile

Binary:

- Base: `/data/tmp/cargo-target-azurepine-20260427-compact-desc-base/release-perf/perf-update-delete`
- Candidate: `/data/tmp/cargo-target-azurepine-20260427-compact-desc-cand2/release-perf/perf-update-delete`

Build shape:

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-azurepine-20260427-compact-desc-cand2 \
  CARGO_PROFILE_RELEASE_PERF_DEBUG=line-tables-only \
  CARGO_PROFILE_RELEASE_PERF_STRIP=false \
  RUSTFLAGS='-C force-frame-pointers=yes' \
  cargo build --profile release-perf -p fsqlite-e2e --bin perf-update-delete
```

Post-rebuild candidate sample:

```bash
perf record -F 997 -g --call-graph dwarf -o /data/tmp/azurepine-compact-desc-cand2.data -- \
  /data/tmp/cargo-target-azurepine-20260427-compact-desc-cand2/release-perf/perf-update-delete 10000 100 both
```

Result:

- Run time: `total=1666ms populate=868ms update=491ms delete=233ms`
- Lost samples: 0
- Remaining `quicksort::<... sort_cells_desc_by_ptr ...>`: 5.04% self
- Remaining `small_sort_general::<... sort_cells_desc_by_ptr ...>`: 3.29% self
- `BtCursor::delete`: 5.25% self

`perf report` emitted `addr2line` sentinel warnings while resolving inline debug info, but still produced symbol percentages and reported zero lost samples.

## Post-rebuild A/B

All rows use `perf-update-delete 10000 <batch> <mode>`. Times are milliseconds.

| mode | batch | base total avg | candidate total avg | total delta | base delete avg | candidate delete avg | delete delta |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| delete | 100 | 1104.5 | 1110.0 | -0.5% | 207.0 | 194.5 | +6.0% |
| delete | 200 | 2259.0 | 2149.5 | +4.8% | 424.0 | 363.5 | +14.3% |
| delete | 500 | 5876.0 | 5629.0 | +4.2% | 1078.0 | 960.0 | +10.9% |
| both | 100 | 1675.0 | 1677.5 | -0.1% | 257.5 | 262.5 | -1.9% |
| both | 200 | 3283.0 | 3317.0 | -1.0% | 481.0 | 492.5 | -2.4% |
| both | 500 | 8342.0 | 8255.5 | +1.0% | 1238.0 | 1177.5 | +4.9% |

Raw post-rebuild timing log:

```text
base mode=delete batch=100 run=1 total=1119ms populate=843ms update=0ms delete=213ms
cand mode=delete batch=100 run=1 total=1114ms populate=848ms update=0ms delete=197ms
base mode=delete batch=100 run=2 total=1090ms populate=830ms update=0ms delete=201ms
cand mode=delete batch=100 run=2 total=1106ms populate=844ms update=0ms delete=192ms
base mode=delete batch=200 run=1 total=2278ms populate=1726ms update=0ms delete=421ms
cand mode=delete batch=200 run=1 total=2150ms populate=1671ms update=0ms delete=362ms
base mode=delete batch=200 run=2 total=2240ms populate=1683ms update=0ms delete=427ms
cand mode=delete batch=200 run=2 total=2149ms populate=1659ms update=0ms delete=365ms
base mode=delete batch=500 run=1 total=5767ms populate=4374ms update=0ms delete=1065ms
cand mode=delete batch=500 run=1 total=5737ms populate=4436ms update=0ms delete=968ms
base mode=delete batch=500 run=2 total=5985ms populate=4531ms update=0ms delete=1091ms
cand mode=delete batch=500 run=2 total=5521ms populate=4230ms update=0ms delete=952ms
base mode=both batch=100 run=1 total=1682ms populate=847ms update=488ms delete=261ms
cand mode=both batch=100 run=1 total=1674ms populate=841ms update=485ms delete=260ms
base mode=both batch=100 run=2 total=1668ms populate=834ms update=498ms delete=254ms
cand mode=both batch=100 run=2 total=1681ms populate=841ms update=489ms delete=265ms
base mode=both batch=200 run=1 total=3300ms populate=1685ms update=957ms delete=497ms
cand mode=both batch=200 run=1 total=3359ms populate=1695ms update=973ms delete=522ms
base mode=both batch=200 run=2 total=3266ms populate=1667ms update=984ms delete=465ms
cand mode=both batch=200 run=2 total=3275ms populate=1676ms update=984ms delete=463ms
base mode=both batch=500 run=1 total=8323ms populate=4208ms update=2408ms delete=1310ms
cand mode=both batch=500 run=1 total=8113ms populate=4138ms update=2428ms delete=1171ms
base mode=both batch=500 run=2 total=8361ms populate=4292ms update=2504ms delete=1166ms
cand mode=both batch=500 run=2 total=8398ms populate=4312ms update=2516ms delete=1184ms
```

## Verification

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-azurepine-20260427-compact-desc-test \
  cargo test -p fsqlite-btree remove_cell_from_leaf_specialized_sort_matches_std -- --nocapture

rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-azurepine-20260427-compact-desc-test \
  cargo test -p fsqlite-btree test_table_delete_leaves -- --nocapture

rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-azurepine-20260427-compact-desc-test \
  cargo test -p fsqlite-btree test_btree_insert_delete_5k -- --nocapture

rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-azurepine-20260427-compact-desc-check \
  cargo check -p fsqlite-btree --all-targets

rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-azurepine-20260427-compact-desc-clippy \
  cargo clippy -p fsqlite-btree --all-targets -- -D warnings

rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-azurepine-20260427-compact-desc-workspace-check \
  cargo check --workspace --all-targets

rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-azurepine-20260427-compact-desc-workspace-check \
  cargo clippy --workspace --all-targets -- -D warnings

cargo fmt --check
```

All commands passed.

## Decision

Keep. The delete-only path clears the 3% perf bar on the measured phase for every post-rebuild batch, clears it on total runtime at 200 and 500 iterations, and the mixed workload is effectively neutral at smaller batches with a positive 500-iteration result. The fallback path preserves behavior for fragmented and non-descending pages.
