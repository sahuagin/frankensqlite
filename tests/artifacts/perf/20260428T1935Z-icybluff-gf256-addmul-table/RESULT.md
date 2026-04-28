# GF(256) symbol multiply-add table fast path

Agent: IcyBluff
Date: 2026-04-28
Target: `crates/fsqlite-core/src/lib.rs`

## Candidate

For non-trivial GF(256) coefficients, build a 256-entry multiplication table
once per symbol operation and use byte-indexed table lookups inside
`symbol_mul_into` and `symbol_addmul_assign`.

This preserves the existing `coeff == 0` and `coeff == 1` special cases:

- `0`: no-op or zero-fill
- `1`: copy or XOR path
- other coefficients: `gf256_mul_byte(coeff, src_byte)` is replaced by the
  precomputed equivalent `table[src_byte]`

## Build Commands

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260428-gf256-baseline cargo bench -p fsqlite-core --profile release-perf --bench symbol_ops --no-run
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260428-gf256-candidate cargo bench -p fsqlite-core --profile release-perf --bench symbol_ops --no-run
```

## Correctness Check

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260428-gf256-verify cargo test -p fsqlite-core gf256 -- --nocapture
```

Result: 4 matching GF(256)/symbol tests passed.

## Benchmark

Criterion, 100 samples:

| Scenario | Baseline | Candidate | Change |
| --- | ---: | ---: | ---: |
| `symbol_ops/symbol_addmul_c53/4096` | 26.578 us | 3.1199 us | -88.224% |
| `symbol_ops/symbol_addmul_c53/512` | 3.2785 us | 1.7101 us | -47.616% |
| `raptorq_paths/decode_fallback_addmul` | 190.56 us | 21.595 us | -88.696% |

Throughput moved from `146.97 MiB/s` to `1.2227 GiB/s` for the 4096-byte
symbol addmul scenario, and from `163.99 MiB/s` to `1.4132 GiB/s` for the
decode fallback addmul scenario.

## Isomorphism

- Ordering preserved: yes; loops still visit destination/source bytes in order.
- Tie-breaking unchanged: N/A.
- Floating point: N/A.
- RNG seeds: unchanged; benchmark data generation is untouched.
- Output equivalence: covered by the existing GF(256) and symbol-operation tests
  comparing chunked operations against scalar/asupersync references.

## Decision

Keep. The table lookup adds a tiny fixed setup cost and removes per-byte
Russian-peasant multiplication in hot RaptorQ symbol paths. All measured
scenarios improved, including the smaller 512-byte symbol case.
