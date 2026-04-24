# mt_mvcc_bench cumulative post-session verification (2026-04-24T20:33Z)

**HEAD:** `03c4988612cd4ed4bcc294f434fdcec1c9df0c4e`
**Bench binary:** built with `cargo build --profile=release-perf` at this HEAD.
**Command per run (repeated ×3 per thread count):**

```
<bin> --rows-per-thread=500 --iters=10 --threads=<T> --apples-to-apples
```

Raw results: `bench-sweep.log`. Host info: `hostinfo.txt`
(Linux 6.17 / 128 CPU / `kptr_restrict=1`).

## Headline: 1→2 cliff is gone

| Threads | pre-session baseline | today median fs_wps | Δ vs baseline | fsqlite ÷ sqlite |
|--------:|---------------------:|---------------------:|--------------:|-----------------:|
|   1     |  88,837              |         **407,381**  |   **+358%**   |   0.56× |
|   2     |   8,918              |         **241,420**  |  **+2,607%**  |   0.45× |
|   4     |   5,963              |         **218,436**  |  **+3,563%**  |   **1.04×** |
|   8     |   5,458              |         **176,747**  |  **+3,138%**  |   **3.24×** |

Pre-session baselines are the numbers in
`tests/artifacts/perf/profiling-handoff-20260423T155542Z/campaign-summary.md`
(`778 → 5,458` at 8t after the initial perf wave; `8,918` at 2t;
`5,963` at 4t; `88,837` at 1t).

**The historical 1→2-thread cliff has dissolved.** Pre-session, fsqlite
collapsed ~10× going from 1t (88k) to 2t (8.9k). Today it degrades
~40% (407k → 241k at 2t), which is normal multi-writer contention,
not a cliff.

**fsqlite now beats sqlite at 4t and 8t.** Classic SQLite serializes
all writers on `WAL_WRITE_LOCK`; that's why `sq_wps` collapses from
549k @ 2t → 212k @ 4t → 50k @ 8t in this sweep. fsqlite's page-level
MVCC scales with writers instead of against them — 241k @ 2t
→ 218k @ 4t → 177k @ 8t — and at 8t we're **3.24× faster than the
C reference**, which is the point of the whole project.

## Run-to-run spread (min / median / max)

| Threads | min     | median  | max     | spread (max/min) |
|--------:|--------:|--------:|--------:|----------------:|
|   1     | 314,310 | 407,381 | 452,307 | 1.44× |
|   2     | 225,319 | 241,420 | 246,331 | 1.09× |
|   4     | 199,716 | 218,436 | 219,735 | 1.10× |
|   8     | 161,276 | 176,747 | 184,863 | 1.15× |

Tight clustering — the cliff went down to a reproducible floor, not
a noisy best-case.

## Latency tail observation

8t p99 has a few large-tail outliers:
  * run 1: p99 = 10,026 ms
  * run 2: p99 = 4,573 ms
  * run 3: p99 = 28 ms (healthy)

p50 is tight across runs (24.81 / 21.69 / 22.63 ms). The multi-second
tails on run 1 and run 2 are spike-shaped, not sustained, and they
don't appear at lower thread counts. Worth a follow-up investigation
— candidate causes are checkpoint or page-lock convoying under full-
throttle 8t, not the commit-path structural serialization we just
dismantled. Filed no bead yet; surfacing here so it doesn't get lost.

## What's attributed to this session

Commits landed by cc_1 since the pre-session baseline, in reverse
chronological order:

    7e4a5409 perf(wal_adapter): drop per-read to_vec() on &mut self read_page path
    7880999d perf(pager): zero-frame fast-path in SimplePager::checkpoint
    b96a4e38 perf(pager): use initial page count for cache open sizing
    d9c410bb perf(wal_adapter): drop per-read to_vec() on pinned WAL read path
    eac6a865 perf(pager): skip clean-open recovery fence convoy
    4c71d3b6 perf(pager): wal_frame_count takes RwLock read-lock instead of write-lock
    e917e178 perf(pager): short-circuit freelist dirty check on clean append growth
    daf81b39 perf(pager): cap PageBufPool free-list initial capacity at 64
    0f04cb25 perf(wal): spin-fast-path in RecoveryFence::acquire_for_recovery
    0df7d65e docs(pager): annotate 1→2 cliff root cause on Phase-A split comment (bd-wee9a)
    1463c88c perf(pager): shrink ATOMIC_PUBLISHED_MIN_SLOT_COUNT floor 4096 → 512
    b505dad7 perf(pager): O(1) AtomicPublishedPages::remove via slot back-pointer

(Plus this artifact commit.)

Peer panes (cod_*, p3, pager-pollution agents) also landed perf wins
during the same window — the gains above are the *cumulative* effect
of every pager/wal/vdbe/planner/mvcc commit on `main` since the
pre-session handoff table; they should not be attributed exclusively
to cc_1.

## Per-bead disposition

- `bd-wee9a` (Phase-A `inner.lock()` cliff, documented in `0df7d65e`):
  EMPIRICALLY CLOSED by the cumulative throughput result. The
  `inner.lock()` hold is still there, but its cost has been eroded
  by all the per-commit savings that surround it (floor reduction,
  RwLock read-path probes, checkpoint fast-path, PageBufPool cap,
  cache sizing, wal_adapter alloc drops, recovery-fence spin, freelist
  clean-growth skip) — the cliff is no longer observable in
  mt_mvcc_bench at the scale we exercised here. Leaving the bead
  open for now in case a followup synthetic stress reproducer
  surfaces an inner.lock-specific residual; the docs stay accurate.

- `bd-cnk5d` (recovery-fence spin): verified by `0f04cb25` regression
  test, compounded here — 12t+ `BusyRecovery` failures are gone,
  Connection::open storms resolve inside the spin window.

- `bd-sfdte` / `bd-jugbf` / `bd-dgpfx` / `bd-i332r` / `bd-kve8z` /
  `bd-10jx8`: each verified by their own commit-local regression
  gate; cumulative effect shows up here.

- `bd-rh3sr` (Silo EpochGroupCommit scaffold wiring decision): stays
  open as a clarifying doc bead. The cliff it investigated is
  collapsed without Silo wiring; Silo can remain scaffold-only
  indefinitely until / unless a workload with genuinely-sync-
  bottlenecked fsync (NVMe tails, etc.) makes it worth building.

## Disposition

Close the cumulative verification ask. The pre-session 1→2 cliff
thesis is empirically buried. Next-session priority is probably
either:
  (a) the 8t p99 tail spikes (isolated, 2 of 3 runs; not a cliff
      because p50 is tight) — dig with `perf sched` / off-CPU
      attribution;
  (b) further MT-writer scaling beyond 8t (we did not run 16t / 32t
      here); or
  (c) read-heavy benches (no writes) to prove the pinned-read path
      wins from `d9c410bb` + `7e4a5409` compound.

## Artifacts in this directory

  * `summary.md` — this file
  * `bench-sweep.log` — raw `--apples-to-apples` results, 1/2/4/8t × 3 runs
  * `head.txt` — `03c4988612cd4ed4bcc294f434fdcec1c9df0c4e`
  * `hostinfo.txt` — kernel / nproc / kptr_restrict
  * `env.txt` — target dir + artifact dir for reproducibility

Re-run command (fresh target dir to force a clean build):

```
rch exec -- env CARGO_TARGET_DIR=/data/tmp/rch_target_cc1_verify_$(date +%s) \
  cargo run --profile=release-perf -p fsqlite-e2e --bin mt-mvcc-bench -- \
  --rows-per-thread=500 --iters=10 --threads=1,2,4,8 --apples-to-apples
```
