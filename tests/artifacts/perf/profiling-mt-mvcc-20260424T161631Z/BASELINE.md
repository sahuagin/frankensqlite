# Baseline Results

Run id: `profiling-mt-mvcc-20260424T161631Z`

## Primary Matrix

Command shape:

```bash
mt-mvcc-bench --rows-per-thread=500 --threads=1,2,4,8,12 --iters=10
```

| Threads | fsqlite p50 wps | SQLite p50 wps | ratio | fsqlite p50 ms | fsqlite p95 ms | fsqlite p99 ms | failed rows |
|--------:|----------------:|---------------:|------:|---------------:|---------------:|---------------:|------------:|
| 1 | 265,987 | 855,676 | 0.31x | 1.88 | 2.55 | 2.72 | 0 |
| 2 | 9,559 | 497,306 | 0.02x | 104.61 | 104.97 | 105.01 | 0 |
| 4 | 7,940 | 210,227 | 0.04x | 261.03 | 313.75 | 314.14 | 0 |
| 8 | 9,274 | 49,885 | 0.19x | 431.37 | 5,974.71 | 9,537.42 | 0 |
| 12 | 11,111 | 25,904 | 0.43x | 540.02 | 10,462.70 | 10,606.67 | 0 |

Primary observation: the current HEAD still has a severe 1-to-2-thread anti-scaling cliff. Throughput falls from 265,987 wps at 1 thread to 9,559 wps at 2 threads, then stays in the 8k-11k wps band for 4-12 threads. Tail latency becomes extreme at 8+ threads.

## Amortization Probe

Command shape:

```bash
mt-mvcc-bench --rows-per-thread=5000 --threads=1,2,4,8 --iters=3
```

| Threads | fsqlite p50 wps | SQLite p50 wps | ratio | fsqlite p50 ms | failed rows |
|--------:|----------------:|---------------:|------:|---------------:|------------:|
| 1 | 584,167 | 1,647,930 | 0.35x | 8.56 | 0 |
| 2 | 79,201 | 1,247,492 | 0.06x | 126.26 | 0 |
| 4 | 66,562 | 497,384 | 0.13x | 300.47 | 0 |
| 8 | 68,950 | 293,575 | 0.23x | 580.13 | 0 |

This reduces connection/setup amortization and shows the same structural pattern: single-thread speed is much better, but adding a second writer still destroys throughput. The cliff is not only a connection-open artifact.

## 16-Thread Probe

Command shape:

```bash
timeout 240s mt-mvcc-bench --rows-per-thread=500 --threads=16 --iters=3
```

Exit status: `1`.

The run failed after retry exhaustion with repeated `database is busy (snapshot conflict on pages: 2)` messages and a final COMMIT failure. This should be treated as a correctness/reliability target for the benchmarked concurrent-writer path, not as a normal throughput datapoint.

## Process Counters

`perf stat -d -d -d` over the 1/2/4/8/12 matrix reported:

- elapsed: 6.314 s
- task-clock: 1.051 s, 0.166 CPUs utilized
- context switches: 4,991
- page faults: 22,478
- instructions: 2.842B
- cycles: 2.357B
- IPC: 1.21
- branch miss rate: 3.56 percent
- L1 data miss rate: 7.05 percent
- dTLB miss rate printed as 48.56 percent of dTLB accesses, but the absolute event count is low enough that this is a hint, not a primary conclusion.

## Syscall Sketch

Small 8-thread `strace -c -f` probe (`rows_per_thread=100`, `iters=2`) was intentionally perturbative but useful for category evidence:

| syscall | time share | calls | errors | interpretation |
|---|---:|---:|---:|---|
| `futex` | 52.07% | 1,016 | 149 | contention/parking dominates under small high-contention samples |
| `clock_nanosleep` | 20.98% | 136 | 0 | retry/backoff and wait slices are a major off-CPU component |
| `pread64` | 7.64% | 1,991 | 0 | page reads remain visible but not the first-order bottleneck |
| `statx` | 5.78% | 1,395 | 319 | open/path setup remains visible in sample-shaped benchmark |
