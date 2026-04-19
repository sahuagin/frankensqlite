# Benchmark methodology: concurrent writers

## TL;DR

`crates/fsqlite-e2e/src/bin/comprehensive_bench.rs::bench_concurrent_writers`
runs FrankenSQLite writers *sequentially on a single Connection* and compares
them against *multi-threaded* C SQLite WAL. Any speedup/slowdown ratio
reported by that scenario is **apples-to-oranges** for evaluating
multi-writer MVCC performance.

Use `crates/fsqlite-e2e/src/bin/mt_mvcc_bench.rs` (IMPL-4a) when you want a
real multi-threaded throughput number.

## Background

FrankenSQLite's `Connection` type is `!Send + !Sync` — its internal state
includes `RefCell` and `Rc` fields that cannot cross thread boundaries.
For a benchmark to run true concurrent writers, it must construct
*one Connection per OS thread*, each bound to the same file-backed
database, and coordinate them at the MVCC/WAL layer below the Connection
API.

`bench_concurrent_writers` was originally written against the `rusqlite`
baseline, which *does* have a `Send` Connection. When the FrankenSQLite
baseline was added, the loop iterating 1..N "writers" was left as a
sequential for-loop over a single Connection, with each "writer"
performing a transaction serially. The benchmark continued to report
timing ratios, but the reported "fsqlite at 8 writers" was actually a
single-threaded workload divided into 8 short sequential batches, while
"sqlite3 at 8 writers" was 8 OS threads contending on the WAL_WRITE_LOCK.

## Why this matters

Several optimization items in the current campaign (IMPL-4 flat-combining
page lock table, IMPL-14 Cicada read-ts batching, IMPL-15 Hekaton TID
gap reservation, IMPL-16 Silo epoch group commit, IMPL-24 MICA
partitioned commit log) target multi-writer contention. None of them
can be measured accurately by `bench_concurrent_writers`. Evidence from
the 2026-04-18 campaign:

- `IMPL-4` (flat-combining) was **refused** by the implementing agent
  after discovering that the feature was already wired behind
  `mvcc-flat-combining` and that the bench could not observe any
  difference because writers were sequential.
- Apparent "4.72× faster at 8 writers" in earlier reports was not a
  FrankenSQLite win — it was a sequential-vs-multi-threaded comparison
  that happened to favor the sequential side under low per-op cost.

## What IMPL-4a provides

`mt_mvcc_bench` spawns N OS threads, each opening its own
`Connection::open(path)` against a shared file-backed database, each
running BEGIN CONCURRENT (or BEGIN for fallback), and each committing a
fixed number of rows. It measures wall-clock throughput and compares
against a matched rusqlite WAL-mode workload.

The numbers it reports are directly comparable because both sides run
the same count of OS threads performing the same count of transactions.

## When to use which bench

| Use case | Use |
|---|---|
| Single-connection latency | `comprehensive_bench::bench_*` (all but concurrent_writers) |
| Single-connection stmt ceremony | `comprehensive_bench::bench_concurrent_writers` (effectively) |
| Real multi-thread MVCC throughput | `mt_mvcc_bench` (IMPL-4a) |
| Cross-process conflict | `swarm_multiprocess` / `swarm_peer_visibility` |

## Before you modify `bench_concurrent_writers`

Do not rename it to something less misleading (like
`bench_small_txn_ceremony`) until we've stopped citing its numbers in
memory and release notes — grep the repo for mentions first. If the
rename is worth it, do it in a separate commit with a deprecation note
in the old name.

## Related

- Campaign memory: `session_2026_04_18_ag_aac_campaign.md` — INSIGHT #75
- Blocked-by: IMPL-4, IMPL-14, IMPL-15, IMPL-16, IMPL-24
