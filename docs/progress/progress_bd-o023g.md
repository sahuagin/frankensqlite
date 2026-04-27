## bd-o023g progress

- Read `/data/projects/frankensqlite/AGENTS.md` first and ran `br show bd-o023g`.
- Confirmed the bead scope is the integer-only `MakeRecord` fast path: safe AVX2 classification via nightly `portable_simd`, runtime CPU detection, and scalar fallback without touching `concurrent_mode_default`.
- While this session was in flight, the bulk SIMD landing work appeared in `HEAD` already:
  - `1c58156e` moved the integer-record serializer from unsafe AVX2 intrinsics to safe `portable_simd`
  - `cd6e61ca` finished the classifier refactor and restored `unsafe_code = "forbid"` for `fsqlite-vdbe`

Focused delta in this landing pass:

- fixed the remaining `fsqlite-types` proptest compile failure in [crates/fsqlite-types/src/record.rs](/data/projects/frankensqlite/crates/fsqlite-types/src/record.rs) by replacing the invalid inline-capture assertion message inside `prop_assert_eq!`
- test-gated the legacy [crates/fsqlite-vdbe/src/make_record_simd.rs](/data/projects/frankensqlite/crates/fsqlite-vdbe/src/make_record_simd.rs) module from [crates/fsqlite-vdbe/src/lib.rs](/data/projects/frankensqlite/crates/fsqlite-vdbe/src/lib.rs) so non-test builds stop reporting it as dead code
- added this bead-scoped progress file so the task no longer depends on a shared progress note

Verification:

- `cargo test -p fsqlite-types simd_integer_record_ -- --nocapture --test-threads=1`
  - passed locally: 6 tests, including the 10K proptest and the scalar-fallback simulation path
- `cargo test -p fsqlite-vdbe integer_record_fast_path -- --nocapture --test-threads=1`
  - passed locally: 3 tests
- `cargo bench -p fsqlite-vdbe --bench make_record -- --sample-size 10`
  - completed locally
  - current harness shows `precomputed_header` faster than `generic`, but only by about 8-13% across the 4/8/16/32-column cases on this machine
  - this benchmark compares two `MakeRecord` opcode shapes, not an explicit scalar-vs-SIMD toggle, so the bead’s `>= 20%` acceptance bar remains unproven by the current harness
- `cargo check --workspace --all-targets`
  - blocked by a pre-existing unrelated `fsqlite-core` error in `crates/fsqlite-core/src/connection.rs:615` (`missing field window_func_partitions_total`)
- `cargo clippy -p fsqlite-types -p fsqlite-vdbe --all-targets -- -D warnings`
  - blocked by a pre-existing unrelated lint in `crates/fsqlite-wal/src/parallel_wal.rs:361` (`clippy::unnecessary_map_or`)
- `cargo fmt --check`
  - blocked by pre-existing unrelated formatting drift in `crates/fsqlite-core/src/connection.rs`

Constraints held:

- `concurrent_mode_default` remains `true`
- no `unsafe_code`
- no Tokio ecosystem
- manual edits only
