# WAL frame assembly helper v2 candidate

Date: 2026-04-28
Agent: IcyBluff

## Candidate

Current `HEAD` includes:

`e5c83f11 perf(vdbe,wal): cache synthetic page-1 hint and unify WAL frame assembly`

That commit introduced `push_wal_frame_bytes`, which appends each frame header
field and payload into the reusable WAL scratch buffer. The v2 candidate changed
that helper to build a local 24-byte header and append only the header plus page
payload, reducing per-frame `Vec::extend_from_slice` calls from six to two.

## Verdict

Rejected. The direct current-head A/B showed the committed v1 helper was slightly
faster than the v2 header-buffer helper.

## Benchmark

V1 baseline build:

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260428-wal-assembly-head-v1 \
  cargo test -p fsqlite-wal --profile release-perf wal_frame_scratch_benchmark_report --no-run
```

V2 candidate build:

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/cargo-target-icybluff-20260428-wal-assembly-candidate \
  cargo test -p fsqlite-wal --profile release-perf wal_frame_scratch_benchmark_report --no-run
```

Measured command:

```bash
hyperfine --warmup 2 --runs 30 \
  --export-json tests/artifacts/perf/20260428T0920Z-icybluff-wal-frame-assembly/hyperfine-head-v1-v2.json \
  --command-name head-v1-helper '/data/tmp/cargo-target-icybluff-20260428-wal-assembly-head-v1/release-perf/deps/fsqlite_wal-bba5890cb2f6611c --ignored --exact wal::tests::wal_frame_scratch_benchmark_report --nocapture' \
  --command-name head-v2-header-helper '/data/tmp/cargo-target-icybluff-20260428-wal-assembly-candidate/release-perf/deps/fsqlite_wal-bba5890cb2f6611c --ignored --exact wal::tests::wal_frame_scratch_benchmark_report --nocapture'
```

| Scenario | Mean | Sigma | Verdict |
| --- | ---: | ---: | --- |
| `head-v1-helper` | 327.444ms | 4.579ms | keep |
| `head-v2-header-helper` | 330.427ms | 10.946ms | reject |

Raw benchmark reports:

- `head-v1-report.txt`
- `head-v2-report.txt`
- `hyperfine-head-v1-v2.json`

## Follow-up

No code change was kept for this candidate. The worktree was restored to the
committed v1 helper in `crates/fsqlite-wal/src/wal.rs`.
