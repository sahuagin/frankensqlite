# bd-c0bsc progress

Summary:
- Re-read `bd-c0bsc` and kept the slice narrow around same-connection mixed read/write overlap.
- Chose the retained `:memory:` prepared-read path because Track W still calls out mixed-OLTP prepared-query cost and the current tree was conservatively flushing supported prepared reads on retained in-memory autocommit batches.
- Enabled the existing retained-autocommit prepared-read overlay for supported `:memory:` shapes by removing the memory-only exclusion in `retained_autocommit_overlay_dirty_fast_path`.
- Updated focused tests to prove supported retained `:memory:` prepared reads now stay on the overlay path without forcing read-after-write flushes:
  - rowid lookup
  - `COUNT(*)` rowid range
  - `COUNT(*), SUM(score)` aggregate

Constraints held:
- `concurrent_mode_default` remains `true`
- no `unsafe_code`
- no Tokio ecosystem
- manual edits only

Next:
- run focused `fsqlite-core` tests for the new memory-overlay cases
- run required `check`, `clippy`, and `fmt` verification

Verification:
- `br comments add bd-c0bsc -f progress_bd-c0bsc.md`
  - Passed.
- `timeout 300 cargo test -p fsqlite-core memory_retained_autocommit_prepared_ -- --nocapture`
  - First run reached the tests and proved the new `COUNT(*)` range and `COUNT(*), SUM(score)` memory-overlay cases pass.
  - The original rowid lookup test shape failed because it used `SELECT val ... WHERE id = ?1`, which is not the current overlay-supported point-lookup shape. The test was corrected to the mixed-benchmark `SELECT * ... WHERE id = ?1` shape.
  - Re-run after that edit was blocked by unrelated compile errors already present in `crates/fsqlite-pager/src/pager.rs` (`TransactionFrameBatchContext` import missing).
- `cargo check --workspace --all-targets`
  - Blocked by unrelated existing compile errors in `crates/fsqlite-pager/src/pager.rs`:
    - missing `TransactionFrameBatchContext` at lines 5741 and 5759
- `cargo clippy --workspace --all-targets -- -D warnings`
  - Blocked by the same unrelated pager compile errors
  - Also reported an unrelated pre-existing `clippy::useless_conversion` in `crates/fsqlite-types/src/record.rs:1430`
- `cargo fmt --check`
  - Blocked by unrelated formatting drift in multiple files outside this bead, including:
    - `crates/fsqlite-core/src/wal_adapter.rs`
    - `crates/fsqlite-e2e/tests/correctness_transactions.rs`
    - `crates/fsqlite-pager/src/pager.rs`
    - `crates/fsqlite-types/src/value.rs`
    - `crates/fsqlite-wal/src/lib.rs`
- `ubs crates/fsqlite-core/src/connection.rs`
  - Ran, but reported numerous pre-existing whole-file findings that are not specific to the new `bd-c0bsc` hunk.
