# bd-1333e Progress

## 2026-04-10

- Selected sub-task: add `EXPLAIN QUERY PLAN` virtual-table scan details for eponymous table-valued functions and cover `json_each` / `json_tree` with SQLite-parity regression tests.
- Why this slice: it is a bounded Phase 8 planner-and-extension wiring gap. JSON table-valued execution already works, but planner output still needed SQLite-style `VIRTUAL TABLE` detail rows for validation coverage.
- Follow-on slice landed in this session: recurse fallback EQP source-detail extraction through simple subquery wrappers so `FROM (SELECT * FROM json_each(...))` reports the same `SCAN ... VIRTUAL TABLE INDEX 1:` row that SQLite emits.
- Planned verification:
  - targeted `fsqlite-core` EXPLAIN QUERY PLAN regression tests for `json_each` / `json_tree`
  - targeted `fsqlite-core` regression for subquery-wrapped `json_each(...)` EQP output
  - `rustfmt --edition 2024 crates/fsqlite-core/src/connection.rs`
  - `git diff --check -- crates/fsqlite-core/src/connection.rs progress_bd-1333e.md`
  - `cargo check -p fsqlite-core --lib --tests`

## 2026-04-11

- Selected sub-task: enable the safe subset of LIKE prefix planning for ASCII-case-stable prefixes in `crates/fsqlite-planner/src/lib.rs`.
- Why this slice: it advances Phase 8's LIKE/GLOB optimization work without needing new planner context plumbing. Prefixes with no ASCII letters are safe under SQLite's default LIKE semantics because built-in case folding only affects ASCII letters.
- Scope:
  - lower `LIKE '123%'`-style predicates into `WhereTermKind::LikePrefix`
  - keep `LIKE 'abc%'` on the conservative full-scan path until collation / `case_sensitive_like` metadata is available
  - add planner tests covering classification, index usability, and access-path selection
- Planned verification:
  - `cargo fmt --check`
  - `rch exec -- cargo test -p fsqlite-planner like_case_stable_prefix -- --nocapture`
  - `rch exec -- cargo test -p fsqlite-planner test_classify_where_term_like_is_other -- --nocapture`
  - `rch exec -- cargo check --workspace --all-targets`
  - `rch exec -- cargo clippy --workspace --all-targets -- -D warnings`

## Verification Notes

- `rustfmt --edition 2024 crates/fsqlite-core/src/connection.rs`: passed
- `git diff --check -- crates/fsqlite-core/src/connection.rs progress_bd-1333e.md`: passed
- `sqlite3 ':memory:' "EXPLAIN QUERY PLAN SELECT key, value FROM json_each(json_array(10,20));"`: reference detail is `SCAN json_each VIRTUAL TABLE INDEX 1:`
- `sqlite3 ':memory:' "EXPLAIN QUERY PLAN SELECT jt.fullkey, jt.atom FROM json_tree('{\"a\":{\"b\":1},\"c\":[2]}') AS jt WHERE jt.atom IS NOT NULL ORDER BY jt.id;"`: reference details are `SCAN jt VIRTUAL TABLE INDEX 1:` and `USE TEMP B-TREE FOR ORDER BY`
- `rch exec -- cargo test -p fsqlite-core test_explain_query_plan_json_each_matches_sqlite_virtual_table_detail -- --nocapture`: blocked by unrelated dirty-worktree compile failure in `crates/fsqlite-core/src/connection.rs` around `cache_page_snapshots()` / `PageCachePageSnapshot`
- `rch exec -- cargo check -p fsqlite-core --lib --tests`: blocked by the same unrelated dirty-worktree compile failure in `crates/fsqlite-core/src/connection.rs`
- `rustfmt --edition 2024 crates/fsqlite-planner/src/lib.rs`: passed
- `git diff --check -- crates/fsqlite-planner/src/lib.rs progress_bd-1333e.md`: passed
- `cargo fmt --check`: passed
- `rch exec -- cargo test -p fsqlite-planner like_ -- --nocapture`: passed (13 LIKE-related planner tests, including the new case-stable prefix coverage)
- `rch exec -- cargo check --workspace --all-targets`: passed; `fsqlite-core` emitted two pre-existing `vacuum.rs` dead-code warnings (`NON_TEXT_FILENAME`, `resolve_vacuum_into_target`)
- `rch exec -- cargo clippy --workspace --all-targets -- -D warnings`: failed on the same pre-existing `crates/fsqlite-core/src/vacuum.rs` dead-code items, unrelated to this planner change
- `rch exec -- cargo clippy -p fsqlite-planner --all-targets -- -D warnings`: passed
- `ubs crates/fsqlite-planner/src/lib.rs`: reports existing whole-file findings elsewhere in the planner crate and exits non-zero, but the changed LIKE-prefix slice passed `cargo check`, tests, and crate-scoped clippy
