## bd-ooc2c progress

Current status:
- Switched off the shared `progress.md` file per rm-wide coordination; this bead now logs to `progress_bd-ooc2c.md`.
- Fixed the flaky profiling assertion in `crates/fsqlite-types/src/record.rs` so it only checks scope-local counters under parallel test execution.
- Tightened the safe slab fallback so it only retains values with genuinely reusable heap backing:
  - `SmallText::overwrite()` now detaches from its lazy shared cache without throwing away the owned `String` buffer.
  - `pool_return_reusable()` now skips inline text, `HeapShared` text, and non-unique blob `Arc`s that cannot pay off on the next overwrite.
  - Added a VDBE regression proving shared text/blob register overwrites do not pollute the reusable slab.

Verification:
- `rch exec -- cargo test -p fsqlite-types test_small_text_overwrite_detaches_from_shared_arc -- --nocapture`
- `rch exec -- cargo test -p fsqlite-types test_pool_return_reusable_keeps_only_reusable_heap_storage -- --nocapture`
- `rch exec -- cargo test -p fsqlite-vdbe test_set_reg_skips_shared_values_without_reusable_backing_storage -- --nocapture`
- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings` currently stops in unrelated code at `crates/fsqlite-wal/src/parallel_wal.rs:361` (`clippy::unnecessary_map_or`)
- `cargo clippy -p fsqlite-types -p fsqlite-vdbe --all-targets --no-deps -- -D warnings`
- `cargo fmt --check` currently stops on an unrelated existing diff in `crates/fsqlite-core/src/connection.rs`
- `rustfmt --edition 2024 --check crates/fsqlite-types/src/value.rs crates/fsqlite-vdbe/src/engine.rs`

Constraints held:
- `concurrent_mode_default` remains `true`
- no `unsafe_code`
- no Tokio ecosystem
- manual edits only
