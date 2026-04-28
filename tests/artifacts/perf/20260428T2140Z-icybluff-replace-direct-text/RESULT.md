# replace Direct Text Borrow Perf Proof

Date: 2026-04-28
Agent: IcyBluff
Baseline function code: parent of this commit
Note: the baseline command ran while `8d764858` was `HEAD` and an unrelated
`connection.rs` perf slice was staged; that slice later landed as `80777b6b`
without changing `crates/fsqlite-func/src/builtins.rs`.
Candidate code: this commit

## Scenario

`replace(X, Y, Z)` on already-text arguments. The prior path converted all three
arguments through `SqliteValue::to_text()` before scanning/replacing, which
allocates for `SmallText` inputs that can be borrowed.

## Opportunity Matrix

| Hotspot | Impact | Confidence | Effort | Score |
| --- | ---: | ---: | ---: | ---: |
| `ReplaceFunc::invoke` text argument conversion | 4 | 5 | 1 | 20 |

## Candidate

Use the existing `text_arg()` helper for `X`, `Y`, and `Z`. Text arguments borrow
their existing `SmallText` contents; non-text arguments continue to use
`SqliteValue::to_text()` through the helper. Replacement output still allocates
one owned result string.

## Isomorphism

- Ordering preserved: yes. `replace` remains a single left-to-right string
  replacement over the same `X`, `Y`, and `Z` values.
- Tie-breaking unchanged: N/A.
- Floating-point: N/A; non-text values still flow through `to_text()`.
- NULL behavior: unchanged. `null_propagate(args)` still returns `NULL` before
  any text conversion.
- Empty needle behavior: unchanged. Empty `Y` returns text `X`.
- Size guard: unchanged. Expansion still counts `x.matches(y).count()` before
  allocating the replacement result.

## Benchmark Command

Baseline:

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260428-replace-baseline cargo test -p fsqlite-func perf_replace_text_args -- --ignored --nocapture
```

Candidate:

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260428-replace-candidate cargo test -p fsqlite-func perf_replace_text_args -- --ignored --nocapture
```

Raw output: `baseline.txt`, `candidate.txt`.

## Results

| Benchmark | Baseline best | Candidate best | Delta |
| --- | ---: | ---: | ---: |
| `perf_replace_text_args` | 22,095,144 ns | 11,142,585 ns | -49.570% |

Workload: 100,000 invocations, 3 text arguments, 5 repeats. Output length stayed
23 bytes.

## Verification

```bash
rustfmt --edition 2024 --check crates/fsqlite-func/src/builtins.rs
git diff --check -- crates/fsqlite-func/src/builtins.rs
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260428-replace-verify cargo test -p fsqlite-func replace -- --nocapture
cargo fmt --check
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260428-replace-verify cargo check --workspace --all-targets
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260428-replace-verify cargo clippy --workspace --all-targets -- -D warnings
ubs crates/fsqlite-func/src/builtins.rs tests/artifacts/perf/20260428T2140Z-icybluff-replace-direct-text/RESULT.md
```

All verification commands passed. The workspace check and clippy commands were
run against the live tree, which already contained an unrelated staged
`connection.rs` perf slice. UBS exited 0 with 0 critical issues; it also
reported the existing warning inventory for `builtins.rs`.
