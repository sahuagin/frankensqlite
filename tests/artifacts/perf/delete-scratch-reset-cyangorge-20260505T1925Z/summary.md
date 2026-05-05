# Rejected direct DELETE scratch-reset narrowing - 2026-05-05

Agent: CyanGorge

## Target

The isolated DELETE profile in
`tests/artifacts/perf/dml-mutation-profile-purplecoast-20260505T1830Z/summary.md`
showed DELETE still far behind C SQLite. A small visible sub-signal was
`reset_prepared_direct_insert_statement_scratch` under prepared direct DELETE.

## Candidate

The first candidate removed the broad
`PreparedDirectInsertScratchResetGuard` from
`execute_prepared_direct_simple_delete`. A fresh-eyes reread found that direct
DELETE can still use `prepared_direct_insert_cell_scratch` and
`prepared_direct_update_row_scratch` when maintaining the retained autocommit
COUNT/SUM cache. The measured candidate therefore changed shape:

- Skip the broad INSERT scratch reset on the common direct DELETE path.
- Add a DELETE-specific reset guard only inside the retained COUNT/SUM cache
  maintenance path.
- Clear only the scratch buffers actually used by that DELETE maintenance path.

The candidate diff is saved as `candidate.diff`.

## Correctness

Focused direct DELETE unit proofs passed on the candidate:

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-cyangorge-current-target \
  cargo test -p fsqlite-core --lib prepared_delete -- --nocapture
```

Result: 4 passed, 0 failed.

The broader integration test
`fsqlite-core --test fast_path_separation test_fast_path_prepared_delete`
failed on a clean baseline worktree before considering the candidate:

```text
[T10] DELETE: fast_delta=2, ud_fast_lane_delta=0
prepared DELETE should hit update/delete fast lane: ud_fast=0
```

That failure is pre-existing on `HEAD` and was not used as candidate evidence.

## Measurement

Workload:

```bash
perf-update-delete 10000 1000 delete fsqlite isolated
```

The fair keep/reject comparison used local builds for both binaries:

- Baseline: clean worktree `/data/tmp/frankensqlite-cyangorge-delete-baseline-20260505T1916`
  at `a50dc8ac`.
- Candidate: main worktree at `4dcf22bb` plus `candidate.diff`.
- Build flags: `RUSTFLAGS='-C force-frame-pointers=yes'`
  and `cargo build --profile release-perf -p fsqlite-e2e --bin perf-update-delete`.
- Hyperfine command output is in `hyperfine-delete-isolated-local-local.json`.

| Binary | Mean | Stddev | Median | Min | Max |
| --- | ---: | ---: | ---: | ---: | ---: |
| Baseline local | 1.3775s | 0.0138s | 1.3756s | 1.3578s | 1.4134s |
| Candidate local | 1.3712s | 0.0126s | 1.3684s | 1.3553s | 1.4039s |

The candidate was only about 0.45% faster by mean, well inside the same-host
variance envelope and not enough to keep.

An earlier local-baseline vs RCH-candidate run is preserved as
`hyperfine-delete-isolated.json` for transparency, but it is intentionally not a
keep/reject signal: the candidate binary was built on the remote worker while
the baseline binary was built locally, and the result was a misleading 17%
candidate slowdown.

## Decision

Rejected and reverted. Do not retry prepared direct DELETE scratch-reset
narrowing as a standalone optimization. It can be reconsidered only if a future
profile shows statement scratch reset as a dominant top-level DELETE cost or if
it is part of a broader, measured rewrite that removes the retained cache
scratch dependency entirely.
