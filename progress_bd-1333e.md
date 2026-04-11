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

- Selected sub-task: treat trailing `LIKE`/`GLOB` prefixes after an equality prefix on composite indexes as planner-usable range probes in `crates/fsqlite-planner/src/lib.rs`.
- Why this slice: commit `0987171a` taught the planner to classify safe `LIKE` prefixes, but composite indexes still ignored those prefixes on the column immediately after an equality prefix. That left `(a, b)` indexes planning `WHERE a = ? AND b LIKE '123%'` the same way as `WHERE a = ?`, which is an avoidable Phase 8 planner gap.
- Scope:
  - extend `MultiColumnTrailingConstraint` with a dedicated trailing-prefix case
  - preserve the tighter prefix selectivity heuristic instead of collapsing composite `LIKE`/`GLOB` prefixes into a generic range estimate
  - add planner tests for index usability plus access-path row estimates for composite trailing `LIKE` and `GLOB` prefixes
- Planned verification:
  - `rustfmt --edition 2024 crates/fsqlite-planner/src/lib.rs`
  - `git diff --check -- crates/fsqlite-planner/src/lib.rs progress_bd-1333e.md`
  - `cargo fmt --check`
  - `rch exec -- cargo test -p fsqlite-planner multicolumn_trailing_ -- --nocapture`
  - `rch exec -- cargo check --workspace --all-targets`
  - `rch exec -- cargo clippy --workspace --all-targets -- -D warnings`
  - `ubs crates/fsqlite-planner/src/lib.rs progress_bd-1333e.md`

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
- `rustfmt --edition 2024 crates/fsqlite-planner/src/lib.rs`: passed
- `git diff --check -- crates/fsqlite-planner/src/lib.rs progress_bd-1333e.md`: passed
- `cargo fmt --check`: passed
- `rch exec -- cargo test -p fsqlite-planner multicolumn_trailing_ -- --nocapture`: passed (5 targeted tests covering composite trailing `IN`, `LIKE`, and `GLOB` index planning)
- `rch exec -- cargo check --workspace --all-targets`: passed
- `rch exec -- cargo clippy --workspace --all-targets -- -D warnings`: passed
- `ubs crates/fsqlite-planner/src/lib.rs progress_bd-1333e.md`: exits non-zero on broad pre-existing `fsqlite-planner` findings outside this slice; the scanner still reports formatting/clippy/build health clean for the changed planner file

- Selected sub-task: add escaped-`LIKE` planner regression coverage after the 2026-04-11 `bc8e5ad4` narrowing of `LIKE`/`GLOB` prefix range scans in `crates/fsqlite-planner/src/lib.rs`.
- Why this slice: the planner now accepts a safe subset of `LIKE ... ESCAPE ...` prefixes, but there was no focused regression coverage for extraction, term classification, or access-path selection. That left Phase 8's prefix-planning behavior vulnerable to silent regressions even though the code path now exists in-tree.
- Scope:
  - add a test helper for `LIKE` terms with literal `ESCAPE`
  - cover escaped `%` and `_` inside constant prefixes for extraction
  - cover safe case-stable escaped prefixes using index scans, while keeping escaped ASCII prefixes on the conservative fallback path
- Planned verification:
  - `rustfmt --edition 2024 crates/fsqlite-planner/src/lib.rs`
  - `git diff --check -- crates/fsqlite-planner/src/lib.rs progress_bd-1333e.md`
  - `cargo fmt --check`
  - `rch exec -- cargo test -p fsqlite-planner like_escape -- --nocapture`
  - `rch exec -- cargo test -p fsqlite-planner extract_like_prefix -- --nocapture`
  - `rch exec -- cargo check --workspace --all-targets`
  - `rch exec -- cargo check -p fsqlite-planner --all-targets`
  - `rch exec -- cargo clippy -p fsqlite-planner --all-targets -- -D warnings`
  - `ubs crates/fsqlite-planner/src/lib.rs progress_bd-1333e.md`

- `rustfmt --edition 2024 crates/fsqlite-planner/src/lib.rs`: passed
- `git diff --check -- crates/fsqlite-planner/src/lib.rs progress_bd-1333e.md`: passed
- `rch exec -- cargo test -p fsqlite-planner like_escape -- --nocapture`: passed (3 targeted planner tests covering escaped-prefix classification and access-path selection)
- `rch exec -- cargo test -p fsqlite-planner extract_like_prefix -- --nocapture`: passed (8 targeted extraction tests, including escaped `%` / `_` prefixes and invalid escape literals)
- `rch exec -- cargo check --workspace --all-targets`: blocked by unrelated dirty-tree failure in `crates/fsqlite-pager/src/page_cache.rs` (`stable_snapshot_impl(None)` no longer matches the method signature, plus an unused `attempt` warning)
- `rch exec -- cargo check -p fsqlite-planner --all-targets`: passed
- `rch exec -- cargo clippy -p fsqlite-planner --all-targets -- -D warnings`: passed

- Selected sub-task: reject under-modeled multi-gap skip-scan candidates in `crates/fsqlite-planner/src/lib.rs`.
- Why this slice: the existing skip-scan heuristic only prices one skipped leading column, but it still treated a constraint on the third column of `(a, b, c)` as planner-usable. That underestimates cost for multi-gap skip-scan shapes and can make the planner choose an index it cannot price correctly.
- Scope:
  - limit skip-scan candidate detection to constraints on the immediate second index column
  - keep existing one-leading-column skip-scan behavior for `(a, b)` and `(a, b, c)` shapes where `b` is constrained
  - add regressions covering both the accepted three-column second-column case and the rejected third-column gap case
- Planned verification:
  - `rustfmt --edition 2024 crates/fsqlite-planner/src/lib.rs`
  - `cargo fmt --check`
  - `rch exec -- cargo test -p fsqlite-planner test_best_access_path_skip_scan_allows_immediate_second_column_on_three_column_index -- --nocapture`
  - `rch exec -- cargo test -p fsqlite-planner test_best_access_path_skip_scan_rejects_gapped_trailing_column -- --nocapture`
  - `rch exec -- cargo check -p fsqlite-planner --all-targets`
  - `rch exec -- cargo clippy -p fsqlite-planner --all-targets -- -D warnings`
  - `rch exec -- cargo check --workspace --all-targets`
  - `rch exec -- cargo clippy --workspace --all-targets -- -D warnings`
  - `ubs crates/fsqlite-planner/src/lib.rs`

- `rustfmt --edition 2024 crates/fsqlite-planner/src/lib.rs`: passed
- `cargo fmt --check`: blocked by unrelated formatting drift in `crates/fsqlite-observability/src/connection_pool.rs` around the `recommended_point["throughput_score"]` assertion chain
- `rch exec -- cargo test -p fsqlite-planner test_best_access_path_skip_scan_allows_immediate_second_column_on_three_column_index -- --nocapture`: passed
- `rch exec -- cargo test -p fsqlite-planner test_best_access_path_skip_scan_rejects_gapped_trailing_column -- --nocapture`: passed
- `rch exec -- cargo check -p fsqlite-planner --all-targets`: passed
- `rch exec -- cargo clippy -p fsqlite-planner --all-targets -- -D warnings`: passed
- `rch exec -- cargo check --workspace --all-targets`: passed
- `rch exec -- cargo clippy --workspace --all-targets -- -D warnings`: passed
- `ubs crates/fsqlite-planner/src/lib.rs`: exits non-zero on broad pre-existing whole-file findings in `fsqlite-planner`, but still reports formatting/clippy/build health clean for the changed file
