# strftime numeric formatter fast path

Agent: IcyBluff
Date: 2026-04-28
Target: `crates/fsqlite-func/src/datetime.rs`

## Candidate

Add manual ASCII append helpers for small zero-padded and space-padded numeric
fields in `format_strftime`. Fast-pathed specifiers:

- `%d`, `%H`, `%I`, `%m`, `%M`, `%S`, `%W`, `%g`, `%V`
- `%e`, `%k`, `%l`
- `%j`
- `%Y`, `%G`
- composite `%R` and `%T`

Values outside the small non-negative ranges fall back to the existing
formatting machinery.

## Baseline

Baseline is current `HEAD` after `b8d01172`, using the release-perf binary from
the prior strftime parse pass:

```bash
/data/tmp/cargo-target-icybluff-20260428-strftime-candidate/release-perf/deps/fsqlite_func-5bd29936596e0e29
```

A fresh equivalent baseline build with
`/data/tmp/cargo-target-icybluff-20260428-strftime-format-baseline` was
terminated by the remote helper before completion, so the already-built current
`HEAD` binary was used for the comparison.

## Correctness Check

```bash
RUST_TEST_THREADS=1 /data/tmp/cargo-target-icybluff-20260428-strftime-format-candidate/release-perf/deps/fsqlite_func-5bd29936596e0e29 datetime::tests:: --nocapture
```

Result: 56 passed, 1 ignored.

## Benchmark

Inner benchmark:

| Variant | best_ns |
| --- | ---: |
| baseline | 54455616 |
| candidate | 51072925 |

Baseline-only hyperfine, 20 runs:

| Variant | Mean | Stddev | Range |
| --- | ---: | ---: | ---: |
| baseline | 279.0 ms | 2.1 ms | 275.3 ms .. 282.6 ms |

Forward hyperfine, 25 runs:

| Variant | Mean | Stddev | Range |
| --- | ---: | ---: | ---: |
| baseline | 279.9 ms | 3.4 ms | 275.8 ms .. 288.7 ms |
| candidate | 262.6 ms | 3.2 ms | 257.9 ms .. 269.8 ms |

Reverse hyperfine, 25 runs:

| Variant | Mean | Stddev | Range |
| --- | ---: | ---: | ---: |
| candidate | 280.6 ms | 3.9 ms | 274.3 ms .. 288.4 ms |
| baseline | 308.5 ms | 14.9 ms | 287.4 ms .. 348.7 ms |

## Decision

Keep. The forward comparison shows a stable 1.07x speedup over the parse-fast
baseline; the reverse comparison is noisier but still favors the candidate.
