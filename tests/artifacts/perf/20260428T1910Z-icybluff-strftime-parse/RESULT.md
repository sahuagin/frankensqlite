# strftime timestamp parse fast path

Agent: IcyBluff
Date: 2026-04-28
Target: `crates/fsqlite-func/src/datetime.rs`

## Candidate

Replace `parse_time_part`'s `splitn(...).collect::<Vec<_>>()` plus generic
integer parsing with a byte-indexed parser for the existing accepted forms:

- `HH:MM`
- `HH:MM:SS`
- `HH:MM:SS.frac`

The fractional seconds suffix still uses Rust's `f64` parser to preserve the
existing behavior for supported fractional syntax.

## Build Commands

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260428-strftime-baseline cargo test -p fsqlite-func --profile release-perf perf_strftime_timestamp_rows --no-run
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260428-strftime-candidate cargo test -p fsqlite-func --profile release-perf perf_strftime_timestamp_rows --no-run
```

## Correctness Check

```bash
RUST_TEST_THREADS=1 /data/tmp/cargo-target-icybluff-20260428-strftime-candidate/release-perf/deps/fsqlite_func-5bd29936596e0e29 datetime::tests:: --nocapture
```

Result: 56 passed, 1 ignored.

## Benchmark

Inner benchmark:

| Variant | best_ns |
| --- | ---: |
| baseline | 65013362 |
| candidate | 54176029 |

Forward hyperfine, 25 runs:

| Variant | Mean | Stddev | Range |
| --- | ---: | ---: | ---: |
| baseline | 325.0 ms | 3.6 ms | 318.7 ms .. 333.4 ms |
| candidate | 278.0 ms | 2.3 ms | 275.3 ms .. 283.9 ms |

Reverse hyperfine, 25 runs:

| Variant | Mean | Stddev | Range |
| --- | ---: | ---: | ---: |
| candidate | 278.3 ms | 1.8 ms | 274.9 ms .. 283.1 ms |
| baseline | 324.6 ms | 2.6 ms | 319.3 ms .. 329.7 ms |

## Decision

Keep. The candidate is about 1.17x faster in both orderings and removes a
per-call allocation from the hot timestamp parsing path.
