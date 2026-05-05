# Pure DML Profile - PurpleCoast - 2026-05-05

## Source Change

`perf-update-delete` now accepts an optional fourth positional engine selector:
`fsqlite`, `sqlite`, or `compare`. The default stays `fsqlite`, preserving the
previous profiler behavior. `compare` runs the same prepopulate, UPDATE, and
DELETE shape against FrankenSQLite and C SQLite, then reports FSQLite/C SQLite
time ratios for setup and isolated mutation phases.

## Verification

- `cargo fmt --check`
- `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-perf-update-delete-target cargo test -p fsqlite-e2e --bin perf-update-delete -- --nocapture`
- `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-perf-update-delete-target cargo clippy -p fsqlite-e2e --bin perf-update-delete -- -D warnings`
- `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-workspace-verify-purplecoast cargo check --workspace --all-targets`
- `rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-workspace-verify-purplecoast cargo clippy --workspace --all-targets -- -D warnings`

## Profile Command

```bash
rch exec -- env CARGO_TARGET_DIR=/data/tmp/frankensqlite-perf-update-delete-target \
  cargo run --profile release-perf -p fsqlite-e2e --bin perf-update-delete -- \
  10000 100 both compare \
  > tests/artifacts/perf/pure-dml-profile-purplecoast-20260505T1805Z/both-compare.log 2>&1
```

## Result

| Phase | FSQLite | C SQLite | F/C time ratio |
|---|---:|---:|---:|
| Total | 989 ms | 375 ms | 2.64x |
| Populate | 662 ms | 311 ms | 2.13x |
| UPDATE | 166 ms | 38 ms | 4.31x |
| DELETE | 110 ms | 16 ms | 6.63x |

Per-row mutation costs from the same run:

| Mutation | FSQLite | C SQLite |
|---|---:|---:|
| UPDATE | 1666 ns | 386 ns |
| DELETE | 2213 ns | 334 ns |

## Interpretation

The UPDATE/DELETE matrix gap is not only INSERT setup noise. The populate phase
is still slower, but isolated rowid UPDATE and DELETE mutation phases are much
farther behind C SQLite. Future DML optimization should profile and improve the
direct UPDATE/DELETE B-tree mutation paths before retrying setup-only DML ideas.
