# UNHEX Single-Pass Decode

## Scenario

- Run ID: `20260429T0208Z-icybluff-unhex-single-pass`
- Parent revision: `f6515074` (`perf(func): avoid compileoption text allocation`)
- Code revision: this commit
- Workload: `cargo test -p fsqlite-func perf_unhex_text_args -- --ignored --nocapture`
- Iterations: 300,000 calls per case, best of 7 repeats
- Target: `unhex(X[,Y])` plain and ignored-character text paths
- Toolchain: `rustc 1.97.0-nightly (52b6e2c20 2026-04-27)`, `cargo 1.97.0-nightly (eb9b60f1f 2026-04-24)`

The optimization replaces the filtered `String` plus second `Vec<char>` decode
pass with a single pass over filtered input characters. It keeps the existing
`text_arg` borrowing path and still materializes the result bytes once.

## Opportunity Matrix

| Hotspot | Impact | Confidence | Effort | Score |
|---|---:|---:|---:|---:|
| `UnhexFunc::invoke` filtered-string and char-vector allocation | 5 | 5 | 2 | 12.5 |

## Results

| Case | Baseline best ns | Candidate best ns | Delta | Checksum |
|---|---:|---:|---:|---:|
| `unhex("48656C6C6F776F726C64")` | 103,534,263 | 33,635,174 | -67.513% | 31,500,000 |
| `unhex("48-65-6C-6C-6F", "-")` | 81,859,448 | 34,888,170 | -57.380% | 31,500,000 |

Artifacts:

- `baseline.txt`
- `candidate.txt`

## Isomorphism

- Ordering preserved: yes. The input is still scanned left to right.
- Ignore filtering: unchanged. Characters listed in the optional second argument are skipped before hex decoding.
- Pairing rule: unchanged for valid inputs. Two hex digits produce one byte.
- Odd digit count: unchanged. A trailing unmatched high nibble returns NULL.
- Invalid digit behavior: unchanged. Any non-ignored non-hex character returns NULL.
- Floating point: not applicable.
- RNG seeds: not applicable.
- NULL behavior: unchanged. NULL input still returns NULL before conversion.
- Output equivalence: checksum stayed at `31,500,000`.

## Verification

Commands run for this slice:

```bash
rustfmt --edition 2024 crates/fsqlite-func/src/builtins.rs
TMPDIR=/data/tmp rch exec -- env TMPDIR=/data/tmp CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260429-unhex-single-baseline cargo test -p fsqlite-func perf_unhex_text_args -- --ignored --nocapture
TMPDIR=/data/tmp rch exec -- env TMPDIR=/data/tmp CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260429-unhex-single-candidate cargo test -p fsqlite-func perf_unhex_text_args -- --ignored --nocapture
```

Workspace verification is recorded in the session closeout. Full `cargo fmt
--check` is expected to remain blocked by unrelated pre-existing formatting in
`crates/fsqlite-core/src/connection.rs`.
