# Profiling Run Definition — mt_mvcc current HEAD

- Run id: `profiling-mt-mvcc-20260424T161631Z`
- Date: 2026-04-24 UTC
- Repo: `/data/projects/frankensqlite`
- Skill driver: `profiling-software-performance`, with handoff to `extreme-software-optimization` using `alien-artifact-coding` and `alien-graveyard` concepts
- Primary executable: `fsqlite-e2e` binary `mt-mvcc-bench`
- Build profile: `release-perf` with frame pointers (`RUSTFLAGS=-C force-frame-pointers=yes`)

## Scenario

Profile the current multi-threaded concurrent-writer benchmark after the recent prepared-cache, pager, MVCC, and VDBE perf work. The benchmark uses a fresh file-backed database per sample, one connection per writer thread, `BEGIN CONCURRENT`, prepared INSERT statements, and a synchronized startup gate.

Primary baseline matrix:
- rows per thread: `500`
- iterations: `10`
- thread counts: `1,2,4,8,12`
- output: JSON + Markdown summary under this artifact directory

Focused profiles:
- CPU callgraph: 2-thread cliff and 8-thread steady contention case
- CPU flat histogram: first-pass hotspot selection per repo memory guidance
- perf stat: task-clock, context switches, faults, cache and branch counters where supported
- syscall/I/O sketch: small `strace -c -f` probe if available
- 16-thread case: guarded pathological probe (`timeout 240s`, 3 iterations) because prior runs showed high-thread hangs/variance.

## Non-goals

- No optimization code changes in the profiling phase.
- No kernel tuning or destructive cleanup.
- No changes to concurrent-writer defaults.

## Handoff Boundary

The handoff should contain ranked, evidence-backed optimization opportunities. Each candidate needs a clear mechanism, expected impact, correctness proof boundary, rollback criterion, and an alien-graveyard / alien-artifact design card where useful.
