## bd-ooc2c progress

Current status:
- Switched off the shared `progress.md` file per rm-wide coordination; this bead now logs to `progress_bd-ooc2c.md`.
- Fixed the flaky profiling assertion in `crates/fsqlite-types/src/record.rs` so it only checks scope-local counters under parallel test execution.
- The actual `bd-ooc2c` safe fallback implementation is now being applied in `value.rs`, `record.rs`, and `engine.rs`.

Constraints held:
- `concurrent_mode_default` remains `true`
- no `unsafe_code`
- no Tokio ecosystem
- manual edits only
