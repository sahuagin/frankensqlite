# SOUNDEX Text Argument Borrowing

## Scenario

- Run ID: `20260429T0054Z-icybluff-soundex-borrow-text`
- Code revision: `cf1c5b5c`
- Workload: ignored unit benchmark `cargo test -p fsqlite-func perf_soundex_text_arg -- --ignored --nocapture`
- Iterations: 1,000,000 invocations, best of 7 repeats
- Target: `soundex(X)` with a text input
- Build/test shape: `TMPDIR=/data/tmp rch exec -- env TMPDIR=/data/tmp CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260429-soundex-* cargo ...`
- Isolated workspace gate shape: `TMPDIR=/data/tmp CARGO_HOME=/data/tmp/cargo-home-icybluff-20260429-soundex rch exec -- env TMPDIR=/data/tmp CARGO_HOME=/data/tmp/cargo-home-icybluff-20260429-soundex CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260429-soundex-isolated cargo ...`
- Toolchain: `rustc 1.97.0-nightly (52b6e2c20 2026-04-27)`, `cargo 1.97.0-nightly (eb9b60f1f 2026-04-24)`

## Opportunity Matrix

| Hotspot | Impact | Confidence | Effort | Score | Evidence |
|---|---:|---:|---:|---:|---|
| `SoundexFunc` clones text with `to_text()` before passing a borrowed view to `soundex()` | 4 | 5 | 1 | 20.0 | `baseline.txt`, `candidate.txt` |

The optimization keeps the existing SOUNDEX algorithm and only changes text argument access from eager `String` allocation to borrowed `Cow<str>` when the value is already text.

## Results

| Case | Baseline best ns | Candidate best ns | Change |
|---|---:|---:|---:|
| `soundex("Robert")` | 86,311,136 | 60,939,064 | -29.396% |

Both runs reported `checksum=28000000`.

Raw output:

- `baseline.txt`
- `candidate.txt`

## Isomorphism Proof

- Ordering preserved: yes. The same `soundex()` helper still scans characters in the same order.
- Tie-breaking unchanged: N/A.
- Floating-point: N/A.
- RNG seeds: N/A.
- NULL behavior: unchanged. `NULL` still returns `?000`.
- Type coercion behavior: unchanged for non-text values. `text_arg` falls back to `SqliteValue::to_text()` when a borrowed text view is unavailable.
- Output materialization: unchanged. The SOUNDEX result is still materialized through `SmallText::from_string(...)`.

## Verification

- `rustfmt --edition 2024 --check crates/fsqlite-func/src/builtins.rs`
- `git diff --check -- crates/fsqlite-func/src/builtins.rs`
- `TMPDIR=/data/tmp rch exec -- env TMPDIR=/data/tmp CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260429-soundex-verify cargo test -p fsqlite-func soundex -- --nocapture`
- `TMPDIR=/data/tmp CARGO_HOME=/data/tmp/cargo-home-icybluff-20260429-soundex rch exec -- env TMPDIR=/data/tmp CARGO_HOME=/data/tmp/cargo-home-icybluff-20260429-soundex CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260429-soundex-isolated cargo check --workspace --all-targets`
- `TMPDIR=/data/tmp CARGO_HOME=/data/tmp/cargo-home-icybluff-20260429-soundex rch exec -- env TMPDIR=/data/tmp CARGO_HOME=/data/tmp/cargo-home-icybluff-20260429-soundex CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260429-soundex-isolated cargo clippy --workspace --all-targets -- -D warnings`
- `TMPDIR=/data/tmp ubs crates/fsqlite-func/src/builtins.rs tests/artifacts/perf/20260429T0054Z-icybluff-soundex-borrow-text/RESULT.md`

`cargo fmt --check` was attempted during the slice and initially failed on unrelated dirty work in `crates/fsqlite-core/src/connection.rs`. That peer work was not part of this artifact bundle.
