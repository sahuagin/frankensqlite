## bd-iuvw4 Progress

- Read `/data/projects/frankensqlite/AGENTS.md` first and ran `br show bd-iuvw4`.
- Verified `bd-otbu1` and `bd-m1nte` are already `CLOSED`, so `bd-iuvw4` is stale rather than genuinely blocked.
- Confirmed the retained-autocommit unit coverage already exists in `crates/fsqlite-core/src/connection.rs` for batch reuse, read-after-write overlay correctness, error-triggered flush preservation, explicit `BEGIN` flush, close flush, adaptive thresholds, and flush boundaries for `SAVEPOINT`, `PRAGMA`, schema changes, `ATTACH`/`DETACH`, and `VACUUM`.
- Confirmed the retained-autocommit e2e coverage already exists in `crates/fsqlite-e2e/tests/correctness_transactions.rs` for interleaved read/write parity, schema-boundary parity, 10K retained-autocommit profiling, and crash recovery discarding the unflushed batch.
- No source-code patch was required in this pass because the bead-scoped tests were already present in-tree and passed as written.

Verification:

- `rch exec -- cargo test -p fsqlite-core test_retained_autocommit -- --nocapture`
- Result: 15 passed, 0 failed, 0 ignored
- `rch exec -- cargo test -p fsqlite-e2e --test correctness_transactions retained_autocommit -- --nocapture --test-threads=1`
- Result: 4 passed, 0 failed, 1 ignored (`retained_autocommit_crash_helper_entrypoint`, invoked via subprocess by the crash-recovery test)
- 10K retained-autocommit probe output: `bd-iuvw4 retained-autocommit-10k elapsed_ms=205264 reuses=9961 parks=9962 flushes=0`

Constraints held:

- `concurrent_mode_default` remains `true`
- no `unsafe_code`
- no Tokio ecosystem
- manual edits only
- unrelated pre-existing worktree changes stay out of the `bd-iuvw4` commit
