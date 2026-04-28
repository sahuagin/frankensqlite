# VDBE context classify A/B

Date: 2026-04-28
Agent: IcyBluff

## Change under test

`573ba603 refactor(vdbe): pass ConcurrentContext to classify_concurrent_write_tier`

The write-page hot path already cloned the active `ConcurrentContext` in
`SharedTxnPageIo::write_page_internal`. The change passes that context into
`classify_concurrent_write_tier` instead of borrowing and cloning the
`Rc<RefCell<Option<ConcurrentContext>>>` a second time.

This artifact also accompanies the follow-up clippy cleanup that makes the
now-`self`-free helper an associated function.

## Scenario

Harness:

```bash
perf-update-delete 10000 100 both
perf-update-delete 10000 100 delete
```

Builds used fresh `release-perf` target directories:

```bash
env CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260428-vdbe-context-baseline cargo build -p fsqlite-e2e --profile release-perf --bin perf-update-delete
env CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260428-vdbe-context-candidate cargo build -p fsqlite-e2e --profile release-perf --bin perf-update-delete
```

The A/B held the concurrent peer `record.rs` serialization work constant by
building a fresh baseline immediately before the VDBE edit and the candidate
immediately after it.

## Hyperfine

Command:

```bash
hyperfine --warmup 2 --runs 10 \
  --export-json tests/artifacts/perf/20260428T0415Z-icybluff-vdbe-context-classify/hyperfine.json \
  --command-name baseline-both '/data/tmp/cargo-target-icybluff-20260428-vdbe-context-baseline/release-perf/perf-update-delete 10000 100 both' \
  --command-name candidate-both '/data/tmp/cargo-target-icybluff-20260428-vdbe-context-candidate/release-perf/perf-update-delete 10000 100 both' \
  --command-name baseline-delete '/data/tmp/cargo-target-icybluff-20260428-vdbe-context-baseline/release-perf/perf-update-delete 10000 100 delete' \
  --command-name candidate-delete '/data/tmp/cargo-target-icybluff-20260428-vdbe-context-candidate/release-perf/perf-update-delete 10000 100 delete'
```

| Scenario | Baseline mean | Candidate mean | Delta |
| --- | ---: | ---: | ---: |
| both | 1.293965s | 1.239578s | 4.20% faster |
| delete | 0.991693s | 0.925101s | 6.71% faster |

## Candidate perf sample

Command:

```bash
perf record -F 999 \
  -o tests/artifacts/perf/20260428T0415Z-icybluff-vdbe-context-classify/perf-candidate-both.data \
  -- /data/tmp/cargo-target-icybluff-20260428-vdbe-context-candidate/release-perf/perf-update-delete 10000 100 both
```

Run output:

```text
total=1257ms populate=716ms update=303ms delete=166ms  |  per-row-update=3040ns  per-row-delete=3321ns
```

Top flat samples from `perf-candidate-both-flat.txt`:

| Overhead | Symbol |
| ---: | --- |
| 8.79% | `__memmove_avx_unaligned_erms` |
| 7.11% | `Connection::execute_prepared_direct_simple_insert` |
| 4.64% | `BtCursor<SharedTxnPageIo>::delete` |
| 3.95% | `_int_malloc` |
| 2.52% | `serialize_record_iter_with_precomputed_header_into` |
| 2.24% | `SharedTxnPageIo::write_page_internal` |

Perf warning: kernel symbols were restricted by the host kernel settings, so
kernel samples may be unresolved. User-space symbols are still useful for this
comparison.

## Behavior proof

```bash
cargo fmt --check
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260428-vdbe-context-check cargo check --workspace --all-targets
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260428-vdbe-context-check cargo clippy --workspace --all-targets -- -D warnings
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260428-vdbe-context-test cargo test -p fsqlite-vdbe shared_txn_page_io -- --nocapture
```

Results:

- `cargo fmt --check`: passed.
- `cargo check --workspace --all-targets`: passed.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `cargo test -p fsqlite-vdbe shared_txn_page_io`: `15 passed; 0 failed; 769 filtered out`.
