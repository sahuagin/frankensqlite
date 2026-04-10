## bd-abgqx Progress

- Read `/data/projects/frankensqlite/AGENTS.md` first and ran `br show bd-abgqx`.
- Confirmed the bead scope is Track S test coverage for register value lifecycle, borrow safety, and performance, with `concurrent_mode_default` staying enabled by default.
- Traced existing Track S unit coverage in [crates/fsqlite-vdbe/src/engine.rs](/data/projects/frankensqlite/crates/fsqlite-vdbe/src/engine.rs) and the new bead-scoped e2e test file in [crates/fsqlite-e2e/tests/bd_abgqx_track_s_register_values.rs](/data/projects/frankensqlite/crates/fsqlite-e2e/tests/bd_abgqx_track_s_register_values.rs).
- Identified unrelated local worktree changes in `crates/fsqlite-core/src/connection.rs` and `crates/fsqlite-types/src/value.rs`; those will stay out of the `bd-abgqx` commit unless validation shows a direct dependency.

Planned focused delta:

- keep the bead commit scoped to Track S tests and bead progress notes
- validate the new e2e test and the register sideband lifecycle unit test
- fix only the failures needed to land `bd-abgqx`

Current implementation note:

- Updated the bead-scoped e2e assertions to match the real prepared INSERT path: simple three-column prepared INSERTs use the prepared direct-insert fast lane instead of VDBE `MakeRecord`, so the test now captures both hot-path and VDBE metrics and treats `make_record_calls_total == 0` as the expected fast-path outcome.
- Reproduced the reported `cargo test` exit `101` against `cargo test -p fsqlite-vdbe test_register_ -- --nocapture --test-threads=1`. The failing test was `test_register_value_insert_avoids_make_record_blob_materialization`, and the root cause was a verifier-invalid `Rewind` jump target: the test patched `Rewind` to the `end` label resolved after `Halt`, producing `p2 target 13 is outside 0..13`.
- Fixed the failing unit test by introducing a dedicated `done` label before `Halt` and routing `Rewind` to that in [crates/fsqlite-vdbe/src/engine.rs](/data/projects/frankensqlite/crates/fsqlite-vdbe/src/engine.rs).

Verification:

- `rch exec -- cargo test -p fsqlite-vdbe test_register_ -- --nocapture --test-threads=1`
- `rch exec -- cargo test -p fsqlite-e2e --test bd_abgqx_track_s_register_values -- --nocapture --test-threads=1`
- `cargo fmt -p fsqlite-vdbe --check`

Validation blockers outside bead scope:

- `cargo fmt --check` is currently blocked by pre-existing unrelated formatting diffs in `crates/fsqlite-core/src/connection.rs`, `crates/fsqlite-ext-fts5/src/lib.rs`, and `crates/fsqlite-types/src/record.rs`.
- `rch exec -- cargo clippy -p fsqlite-vdbe --tests -- -D warnings` is currently blocked by a pre-existing unrelated lint in `crates/fsqlite-pager/src/pager.rs:1813` (`cloned_instead_of_copied`).

Constraints held:

- `concurrent_mode_default` remains `true`
- no `unsafe_code`
- no Tokio ecosystem
- manual edits only
