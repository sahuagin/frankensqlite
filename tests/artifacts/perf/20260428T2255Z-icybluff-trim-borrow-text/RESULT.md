# Trim Text Argument Borrowing

## Scenario

- Run ID: `20260428T2255Z-icybluff-trim-borrow-text`
- Base revision: `2c12f8fb`
- Workload: ignored unit benchmark `cargo test -p fsqlite-func perf_trim_text_args -- --ignored --nocapture`
- Iterations: 100,000 invocations per case, best of 5 repeats
- Target: `trim`, `ltrim`, and `rtrim` with text inputs
- Environment note: `/tmp` was full, so build and test commands used `TMPDIR=/data/tmp` and `CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260428-trim-*`.
- Toolchain: `rustc 1.97.0-nightly (52b6e2c20 2026-04-27)`, `cargo 1.97.0-nightly (eb9b60f1f 2026-04-24)`

## Opportunity Matrix

| Hotspot | Impact | Confidence | Effort | Score | Evidence |
|---|---:|---:|---:|---:|---|
| `TrimFunc`/`LtrimFunc`/`RtrimFunc` text args cloned through `to_text()` | 4 | 5 | 1 | 20.0 | `baseline.txt`, `candidate.txt` |

The optimization keeps the existing trim algorithm and only changes text argument access from eager `String` allocation to borrowed `Cow<str>` when the value is already text.

## Results

| Case | Baseline best ns | Candidate best ns | Change |
|---|---:|---:|---:|
| `trim("   payload   ")` | 11,527,846 | 8,691,112 | -24.608% |
| `ltrim("   payload   ")` | 10,759,737 | 7,050,709 | -34.471% |
| `rtrim("   payload   ")` | 10,694,254 | 7,245,874 | -32.245% |
| `trim("xxxpayloadxxx", "x")` | 12,977,452 | 8,934,928 | -31.150% |

Raw output:

- `baseline.txt`
- `candidate.txt`

## Isomorphism Proof

- Ordering preserved: yes. The trim helpers and Rust standard trim traversal are unchanged.
- Tie-breaking unchanged: N/A.
- Floating-point: N/A.
- RNG seeds: N/A.
- NULL behavior: unchanged. The early `args[0].is_null()` return remains, and a NULL trim character argument still falls back to a single ASCII space.
- Type coercion behavior: unchanged for non-text values. `text_arg` falls back to `SqliteValue::to_text()` when a borrowed text view is unavailable.
- Output materialization: unchanged. The final trimmed result is still materialized through `SmallText::new(...)`.

## Verification

- `rustfmt --edition 2024 --check crates/fsqlite-func/src/builtins.rs`
- `git diff --check -- crates/fsqlite-func/src/builtins.rs`
- `TMPDIR=/data/tmp rch exec -- env TMPDIR=/data/tmp CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260428-trim-verify cargo test -p fsqlite-func trim -- --nocapture`
- `cargo fmt --check`
- `TMPDIR=/data/tmp rch exec -- env TMPDIR=/data/tmp CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260428-trim-verify cargo check --workspace --all-targets`
- `TMPDIR=/data/tmp rch exec -- env TMPDIR=/data/tmp CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260428-trim-verify cargo clippy --workspace --all-targets -- -D warnings`
- `TMPDIR=/data/tmp ubs crates/fsqlite-func/src/builtins.rs tests/artifacts/perf/20260428T2255Z-icybluff-trim-borrow-text/RESULT.md`
