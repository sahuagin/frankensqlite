# DML mutation profile refresh - 2026-05-05

Agent: PurpleCoast
Host: AMD Ryzen Threadripper PRO 5995WX, 128 logical CPUs (`lscpu.txt`)
Build: `cargo build --profile release-perf -p fsqlite-e2e --bin perf-update-delete`
Flags: `RUSTFLAGS='-C force-frame-pointers=yes'`

## Why this artifact exists

The current quick matrix says the largest remaining gaps are write-heavy DML rows, especially `:memory:` INSERT/UPDATE/DELETE. The first `perf-update-delete` recordings in this directory used the standard Section 6 shape and were still dominated by repeated populate cost:

- `update-run.log`: 10k rows, 250 iterations, update-only FSQLite. Total 2156ms, populate 1606ms, update 435ms.
- `delete-run.log`: 10k rows, 250 iterations, delete-only FSQLite. Total 2111ms, populate 1634ms, delete 365ms.

Those reports are useful context, but not a clean mutation profile.

## Harness change

`perf-update-delete` now accepts a fifth argument:

```text
perf-update-delete [rows] [iters] [update|delete|both] [fsqlite|sqlite|compare] [standard|isolated]
```

`standard` preserves the old benchmark shape. `isolated` prepopulates once and times only the mutation loops. UPDATE reuses one transaction and rolls back once after the timed section. DELETE deletes unique contiguous rowids in one transaction, avoiding the earlier profiler mistake where `perf record` captured per-iteration rollback/reload work that the printed DELETE timer excluded.

This is a profiler mode, not a matrix replacement.

## Isolated compare

Command:

```bash
env CARGO_TARGET_DIR=/data/tmp/frankensqlite-local-profile-purplecoast \
  /data/tmp/frankensqlite-local-profile-purplecoast/release-perf/perf-update-delete \
  10000 250 both compare isolated
```

Output (`exact-isolated-compare.log`):

| Engine | Total | Populate | UPDATE | DELETE | Per-row UPDATE | Per-row DELETE |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| FSQLite | 580ms | 85ms | 263ms | 201ms | 1052ns | 1614ns |
| C SQLite | 181ms | 46ms | 95ms | 38ms | 382ns | 309ns |
| Ratio | 3.20x | 1.82x | 2.75x | 5.23x | 2.75x | 5.23x |

## Perf recordings

UPDATE command:

```bash
perf record -F 999 -g \
  -o tests/artifacts/perf/dml-mutation-profile-purplecoast-20260505T1830Z/exact-isolated-update.perf.data \
  -- /data/tmp/frankensqlite-local-profile-purplecoast/release-perf/perf-update-delete \
  10000 1000 update fsqlite isolated
```

UPDATE result (`exact-isolated-update-run.log`): total 915ms, populate 7ms, update 904ms, 904ns per row, 960 samples.

Top UPDATE signals:

- `table_seek_for_insert` subtree: about 31.1% children, with `load_page`, `read_page_data`, and `read_cell_pointers_into` prominent.
- `__memmove_avx_unaligned_erms`: 7.19% self, including fixed-width REAL overwrite and cell-slot cache growth.
- `read_cell_pointers_into`: 6.77% self.
- `table_overwrite_current_payload_same_size_no_overflow`: appears under fixed-width REAL UPDATE; likely still does too much cursor/page reconstruction, but prior same-size UPDATE overwrite attempts are in the negative-results ledger and should not be repeated blindly.

DELETE command:

```bash
perf record -F 999 -g \
  -o tests/artifacts/perf/dml-mutation-profile-purplecoast-20260505T1830Z/exact-isolated-delete.perf.data \
  -- /data/tmp/frankensqlite-local-profile-purplecoast/release-perf/perf-update-delete \
  10000 1000 delete fsqlite isolated
```

DELETE result (`exact-isolated-delete-run.log`): total 1366ms, populate 347ms, delete 1011ms, 2024ns per row, 1428 samples.

Top DELETE signals:

- `TransactionKind::write_page_data`: 20.06% self; under direct DELETE it still remains the largest individual symbol.
- `table_seek_for_insert` before delete: about 12.9% children.
- `read_cell_pointers_into`: 6.45% self.
- `__memmove_avx_unaligned_erms`: 7.13% self.

## Next target

The strongest current target is the direct DELETE page-write path, especially reducing the work in `BtCursor::delete -> PageWriter::write_page_data -> TransactionKind::write_page_data`. The profile suggests this has more leverage than another UPDATE fixed-width payload tweak. Before editing, check the negative-results ledger: previous direct UPDATE overwrite and direct DELETE lowering ideas have already lost on real section scores.
