# Mixed OLTP retained COUNT/SUM follow-up

Scenario: `comprehensive-bench --quick --filter mixed --no-html`

Baseline sequence:

- `head_9d83baff_mixed_quick.json`: FSQLite `202.642251 ms`
- `head_9d83baff_mixed_quick_repeat2.json`: FSQLite `196.785127 ms`
- `dirty_interest_seed_mixed_quick.json` on landed `d83e4d33`: FSQLite `179.367460 ms`
- `dirty_interest_seed_mixed_quick_repeat2.json` on landed `d83e4d33`: FSQLite `185.664847 ms`
- `head_d83e4d33_profiled.json`: FSQLite `186.035892 ms`

Connection-only patch validation:

- Clean detached worktree: `/data/tmp/frankensqlite-d83e-update-test`
- Applied only `crates/fsqlite-core/src/connection.rs` diff on top of `d83e4d33`
- `clean_connection_only_mixed_quick.json`: FSQLite `86.085743 ms`
- `clean_connection_only_mixed_quick_repeat2.json`: FSQLite `85.718065 ms`
- `clean_connection_only_profiled.json`: FSQLite `87.501613 ms`

Shared-worktree patch run:

- `after_update_delete_direct_*` files were captured from the shared worktree before discovering an unrelated dirty `crates/fsqlite-btree/src/cursor.rs` change.
- Those files are retained for audit but are not used for attribution.

Interpretation:

- Versus the two clean `d83e4d33` quick medians, the connection-only patch is about `53%` faster on average.
- The retained `COUNT(*) + SUM(score)` scan fell from `7.35%` in `perf_head_d83e4d33_no_children.txt` to `0.54%` in `perf_clean_connection_only_no_children.txt`.
- The remaining FSQLite-side profile is mostly commit/reload and record decode work rather than retained aggregate rescans.

Verification notes:

- `cargo fmt --check` passed.
- `git diff --check -- crates/fsqlite-core/src/connection.rs` passed.
- Focused cache tests passed for first direct insert/update/delete.
- Direct UPDATE/DELETE and metadata focused tests passed.
- `cargo check --workspace --all-targets` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `ubs crates/fsqlite-core/src/connection.rs` did not run because UBS failed to verify its Rust scanner module checksum while refreshing it.
- `test_prepared_update_write_after_write_defers_active_txn_memdb_reload_until_read_boundary` fails on clean `d83e4d33` before this patch, at its setup MemDB mirror assertion.
