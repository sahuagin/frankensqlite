# JSON path index parser candidate

Date: 2026-04-28
Agent: IcyBluff

## Candidate

Add a fast ASCII-digit parser for JSON path array indexes and from-end indexes,
falling back to the existing `usize::from_str` behavior for all non-fast cases.

Target path:

```text
crates/fsqlite-ext-json/src/lib.rs::resolve_path
```

Benchmark probe:

```text
tests::perf_json_extract_deep_single_path
```

## Verdict

Rejected. The candidate did not produce a stable process-level improvement.

## Benchmark

Baseline build:

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260428-json-baseline \
  cargo test -p fsqlite-ext-json --profile release-perf perf_json_extract_deep_single_path --no-run
```

Candidate build:

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260428-json-candidate \
  cargo test -p fsqlite-ext-json --profile release-perf perf_json_extract_deep_single_path --no-run
```

Forward A/B:

```bash
hyperfine --warmup 2 --runs 30 \
  --export-json tests/artifacts/perf/20260428T1845Z-icybluff-json-path-index/hyperfine-baseline-candidate.json \
  --command-name baseline-json-deep-path '/data/tmp/cargo-target-icybluff-20260428-json-baseline/release-perf/deps/fsqlite_ext_json-23499f9012177582 --ignored --exact tests::perf_json_extract_deep_single_path --nocapture' \
  --command-name candidate-json-deep-path '/data/tmp/cargo-target-icybluff-20260428-json-candidate/release-perf/deps/fsqlite_ext_json-23499f9012177582 --ignored --exact tests::perf_json_extract_deep_single_path --nocapture'
```

Reverse A/B:

```bash
hyperfine --warmup 2 --runs 30 \
  --export-json tests/artifacts/perf/20260428T1845Z-icybluff-json-path-index/hyperfine-candidate-baseline.json \
  --command-name candidate-json-deep-path '/data/tmp/cargo-target-icybluff-20260428-json-candidate/release-perf/deps/fsqlite_ext_json-23499f9012177582 --ignored --exact tests::perf_json_extract_deep_single_path --nocapture' \
  --command-name baseline-json-deep-path '/data/tmp/cargo-target-icybluff-20260428-json-baseline/release-perf/deps/fsqlite_ext_json-23499f9012177582 --ignored --exact tests::perf_json_extract_deep_single_path --nocapture'
```

| Run | Baseline mean | Candidate mean | Outcome |
| --- | ---: | ---: | --- |
| forward | 711.238ms | 731.814ms | baseline faster |
| reverse | 726.703ms | 717.422ms | candidate faster, noisy |

The reversed run reported a large first-run baseline outlier. Taken together,
the runs do not clear the stability bar for a hot-path parser change.

## Follow-up

No code change was kept. `crates/fsqlite-ext-json/src/lib.rs` was restored to
the committed implementation.
