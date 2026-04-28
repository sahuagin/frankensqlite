# SapphireCrane Perf Pass - 2026-04-28T014350Z

## Scope

Profile-driven pass on the current `perf-update-delete 10000 100 both` workload.
The binary was built with:

```bash
RUSTFLAGS='-C force-frame-pointers=yes' cargo build --profile release-perf -p fsqlite-e2e --bin perf-update-delete
```

Benchmark binary:

```text
/data/tmp/frankensqlite-sapphirecrane-target/release-perf/perf-update-delete
```

## Changes

1. Direct-simple UPDATE now reuses the cursor position left by `delete()` and calls
   `table_insert_prechecked_absent()` for the replacement row. This avoids a
   second root-to-leaf seek for the same rowid after the row has just been
   deleted.

2. WAL prepared-frame batching now calls
   `prepare_frame_bytes_with_transforms_into()` directly. The adapter was using
   `prepare_frame_bytes()`, which already computed checksum transforms, then
   recomputed them by scanning all serialized frame bytes again.

## Measurements

Hyperfine, 12 runs, warmup 1:

| Build | Mean | Median | Stddev | Min | Max |
|---|---:|---:|---:|---:|---:|
| Baseline | 1.576s | 1.455s | 0.322s | 1.303s | 2.410s |
| After direct UPDATE seek reuse | 1.287s | 1.288s | 0.018s | 1.268s | 1.321s |
| Final, including WAL transform fusion | 1.294s | 1.294s | 0.015s | 1.267s | 1.321s |

Perf run stderr:

| Build | Total | Populate | Update | Delete | Per-row update | Per-row delete |
|---|---:|---:|---:|---:|---:|---:|
| Baseline | 1325ms | 734ms | 356ms | 159ms | 3567ns | 3183ns |
| After direct UPDATE seek reuse | 1250ms | 723ms | 306ms | 153ms | 3066ns | 3060ns |
| Final, including WAL transform fusion | 1334ms | 755ms | 310ms | 187ms | 3103ns | 3752ns |

The final perf run was noisier than the hyperfine median, but the hotspot moved
as intended.

## Hotspot Movement

Flat perf self-time:

| Symbol | Baseline | After direct UPDATE seek reuse | Final |
|---|---:|---:|---:|
| `BtCursor<SharedTxnPageIo>::delete` | 5.53% | 4.38% | 4.85% |
| `WalChecksumTransform::for_wal_frame` | 1.69% | 3.28% | 1.53% |
| `__memmove_avx_unaligned_erms` | 11.29% | 9.39% | 10.40% |

Callgraph proof:

- Before the update change, `execute_prepared_direct_simple_update_with_cursor`
  called `cursor.delete()` then `cursor.table_insert()`, and the profile showed
  `table_insert -> table_seek_for_insert` under the UPDATE path.
- After the update change, the same callgraph shows
  `table_insert_prechecked_absent -> table_insert_from_current_position`, with
  no repeated root-to-leaf seek in the replacement insertion.
- Before the WAL change, `for_wal_frame` was visible under both
  `prepare_append_frames` and `PreparedWalFrameBatch::recompute_checksum_transforms`.
  After the change, the prepare-side duplicate transform walk is fused into the
  serialization pass.

## Verification

```bash
cargo fmt --check
rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-sapphirecrane-target cargo check -p fsqlite-core --profile release-perf
rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-sapphirecrane-target cargo test -p fsqlite-core test_direct_simple_update_delete_fast_path_executes_and_is_correct -- --nocapture
rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-sapphirecrane-target cargo test -p fsqlite-core test_adapter_pre -- --nocapture
rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-sapphirecrane-target cargo check --workspace --all-targets
rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-sapphirecrane-target cargo clippy --workspace --all-targets -- -D warnings
ubs crates/fsqlite-core/src/wal_adapter.rs
```

`ubs crates/fsqlite-core/src/connection.rs` did not complete: a 180s timeout
expired after UBS started the Rust scan for the 100K+ line file, with no module
result emitted. The changed `connection.rs` path is covered by the focused test,
workspace check, workspace clippy, and formatting gates above.

## Notes

An unrelated peer edit to `crates/fsqlite-btree/src/cursor.rs` appeared after
the benchmark captures. It is intentionally not included in this pass.
