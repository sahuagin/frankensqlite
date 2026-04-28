# hex Borrowed Input Bytes Perf Proof

Date: 2026-04-28
Agent: IcyBluff
Baseline function code: parent of `51a3af4d` for `builtins.rs`
Candidate code commit: `51a3af4d` (`perf(func): borrow text/blob bytes in HEX() instead of cloning`)

## Scenario

`hex(X)` on already-text and blob arguments. The prior path materialized input
bytes before encoding:

- blob inputs were cloned into a `Vec<u8>`;
- text inputs were converted through `SqliteValue::to_text()` and then
  `String::into_bytes()`.

The hex encoder only needs a borrowed byte slice for both cases.

## Opportunity Matrix

| Hotspot | Impact | Confidence | Effort | Score |
| --- | ---: | ---: | ---: | ---: |
| `HexFunc::invoke` input byte materialization | 3 | 4 | 1 | 12 |

## Candidate

Represent the input as `Cow<'_, [u8]>`: borrow blob bytes directly, borrow text
bytes through `SmallText::as_bytes_direct()`, and keep owned conversion only for
non-text/non-blob values. The output allocation is unchanged.

## Isomorphism

- Ordering preserved: yes. The function still handles `NULL` first, then
  encodes bytes left-to-right.
- Tie-breaking unchanged: N/A.
- Floating-point: unchanged for float inputs; non-text values still use
  `to_text()`.
- Text bytes unchanged: yes. `SmallText::as_bytes_direct()` returns the same
  UTF-8 bytes that `to_text().into_bytes()` would have produced for text values.
- Blob bytes unchanged: yes. The same blob slice is encoded without cloning it
  first.
- Output case unchanged: yes. `write!(hex, "{b:02X}")` still emits uppercase
  hex.

## Benchmark Command

Baseline:

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260428-hex-baseline cargo test -p fsqlite-func perf_hex_text_blob_args -- --ignored --nocapture
```

Candidate:

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260428-hex-candidate cargo test -p fsqlite-func perf_hex_text_blob_args -- --ignored --nocapture
```

Raw output: `baseline.txt`, `candidate.txt`.

## Results

| Case | Baseline best | Candidate best | Delta |
| --- | ---: | ---: | ---: |
| text | 43,295,296 ns | 38,142,667 ns | -11.901% |
| blob | 41,834,951 ns | 37,999,649 ns | -9.168% |

Workload: 100,000 invocations, 24 input bytes, 5 repeats. Both result lengths
stayed 48 bytes. The text-side result clears the same-host 10% variance
threshold; the blob-side result is below that threshold and should be treated as
directional only.

## Verification

```bash
rustfmt --edition 2024 --check crates/fsqlite-func/src/builtins.rs
git diff --check -- crates/fsqlite-func/src/builtins.rs
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260428-hex-verify cargo test -p fsqlite-func hex -- --nocapture
cargo fmt --check
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260428-hex-verify cargo check --workspace --all-targets
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260428-hex-verify cargo clippy --workspace --all-targets -- -D warnings
TMPDIR=/data/tmp ubs crates/fsqlite-func/src/builtins.rs tests/artifacts/perf/20260428T2215Z-icybluff-hex-borrow-bytes/RESULT.md
```

All verification commands passed. UBS exited 0 with 0 critical issues; it also
reported the existing warning inventory for `builtins.rs`. `/tmp` was full, so
UBS was run with `TMPDIR=/data/tmp`.
