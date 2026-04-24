# cod_5 mt-mvcc rerun after `0f04cb25`

Date: `2026-04-24`
HEAD: `5392c91e`
Artifact dir: `tests/artifacts/perf/cod5-mt-mvcc-rerun-20260424T0438Z`

## Requested command

```bash
timeout 120s rch exec -- env CARGO_TARGET_DIR=/data/tmp/rch_target_cod5_rerun \
  cargo run --profile=release-perf -p fsqlite-e2e --bin mt-mvcc-bench -- \
  --rows-per-thread=500 --iters=10 --threads=1,2,4,8 --apples-to-apples
```

`mt-mvcc-bench` did not accept `--apples-to-apples` before this pass, so this
session added CLI compatibility for that flag. A cold `cargo run` also spent the
full 120s budget rebuilding `release-perf`, so the measured artifact below comes
from the warmed binary with the same workload parameters plus explicit artifact
output flags:

```bash
/data/tmp/rch_target_cod5_rerun/release-perf/mt-mvcc-bench \
  --rows-per-thread=500 \
  --iters=10 \
  --threads=1,2,4,8 \
  --apples-to-apples \
  --json-output tests/artifacts/perf/cod5-mt-mvcc-rerun-20260424T0438Z/mt-mvcc-bench.json \
  --summary-md tests/artifacts/perf/cod5-mt-mvcc-rerun-20260424T0438Z/mt-mvcc-bench.md \
  --history-json tests/artifacts/perf/cod5-mt-mvcc-rerun-20260424T0438Z/mt-mvcc-bench.history.json
```

## Current fs_wps

| Threads | fs_wps p50 | sqlite_wps p50 | Throughput ratio |
|--------:|-----------:|---------------:|-----------------:|
| 1 | 290,190 | 770,887 | 0.38x |
| 2 | 9,544 | 510,610 | 0.02x |
| 4 | 9,459 | 213,101 | 0.04x |
| 8 | 12,227 | 72,132 | 0.17x |

## Comparison vs cliff-tracking baseline

Baseline source: `tests/artifacts/perf/bd-cnk5d-2t-cliff-verify-20260424/summary.md`

| Threads | Baseline fs_wps | Current fs_wps | Delta |
|--------:|----------------:|---------------:|------:|
| 1 | 88,837 / 304,986 typical | 290,190 | within prior typical band |
| 2 | 8,918 | 9,544 | +7.0% |
| 4 | 5,963 | 9,459 | +58.6% |
| 8 | 5,458 | 12,227 | +124.0% |

## Readout

- The recovery-fence change did **not** close the 1→2 cliff.
- 2-thread `fs_wps` is still only `9,544`, which is effectively the same as the
  earlier post-fence-fix verification (`9,537`) and still far below the 1-thread
  result.
- 4-thread and 8-thread throughput remain materially better than the campaign
  baseline.
- This rerun is consistent with the earlier conclusion that the startup recovery
  fence was a real open-time blocker, but not the main driver of steady-state
  2-thread scaling.

## Files

- `mt-mvcc-bench.json`
- `mt-mvcc-bench.md`
- `mt-mvcc-bench.history.json`
