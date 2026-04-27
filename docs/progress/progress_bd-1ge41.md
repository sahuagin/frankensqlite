# bd-1ge41 Progress

## 2026-04-11

- Follow-up scope landed in this slice: reject non-text `VACUUM INTO` targets instead of coercing them into literal filesystem paths.
- Why this slice: the pager-backed `VACUUM` / `VACUUM INTO` pipeline already rebuilt databases correctly, but `NULL`, integers, floats, and blobs still slipped through the INTO expression path and were turned into bogus filenames instead of raising SQLite's `non-text filename` error.
- Guardrails preserved:
  - `concurrent_mode_default` remains untouched and stays `true`.
  - No file-lock serialization was added.
  - No `unsafe` code or async runtime changes were introduced.
- Planned verification:
  - targeted `fsqlite-core` VACUUM filename validation tests
  - `cargo fmt --check`
  - `cargo check --workspace --all-targets`
  - `cargo clippy --workspace --all-targets -- -D warnings`

## 2026-04-10

- Scope landed in this slice: tighten in-place `VACUUM` cleanup so the rebuild temp file is removed after the compacted database replaces the original file.
- Why this slice: the core `VACUUM` / `VACUUM INTO` feature already existed, but the in-place rebuild path still leaked `*.fsqlite-vacuum-*.tmp` artifacts in the source directory after success.
- Guardrails preserved:
  - `concurrent_mode_default` remains untouched and stays `true`.
  - No `unsafe` code was introduced.
  - No new async/runtime usage was added.
- Planned verification:
  - targeted `fsqlite-core` VACUUM regression tests
  - `cargo fmt --check`
  - `cargo check --workspace --all-targets`
  - `cargo clippy --workspace --all-targets -- -D warnings`

## Verification Notes

- `rch exec -- cargo test -p fsqlite-core test_resolve_vacuum_into_target_rejects_non_text_values -- --nocapture`: pending
- `rch exec -- cargo test -p fsqlite-core test_vacuum_into_null_parameter_reports_non_text_filename -- --nocapture`: pending
- `cargo fmt --check`: pending
- `rch exec -- cargo check --workspace --all-targets`: pending
- `rch exec -- cargo clippy --workspace --all-targets -- -D warnings`: pending
- `rch exec -- cargo test -p fsqlite-core test_vacuum_in_place_removes_rebuild_temp_file -- --nocapture`: passed
- `cargo fmt --check`: passed after `rustfmt --edition 2024 crates/fsqlite-core/src/vacuum.rs`
- `rch exec -- cargo check --workspace --all-targets`: passed
- `rch exec -- cargo clippy --workspace --all-targets -- -D warnings`: blocked by an unrelated pre-existing `clippy::struct_field_names` error at `crates/fsqlite-core/src/connection.rs:5682` (`connection_pool_metrics`)
