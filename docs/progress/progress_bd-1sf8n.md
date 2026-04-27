# bd-1sf8n progress

Current slice: final fresh-eyes review of the dedicated Phase 9 time-travel gate traceability work, specifically fixing the gap where the prior "contract" test only rechecked local constants instead of the actual shell runner and canonical inventory.

Implemented in this increment:
- Audited the recent `bd-1sf8n` commits with a fresh-eyes review and found that the earlier traceability-contract test in `crates/fsqlite-harness/tests/bd_1sf8n_phase9_time_travel_gate.rs` could still pass if the external shell runner or canonical inventory drifted, because it only compared local constants against other local literals.
- Fixed that gap by binding the test to the actual canonical traceability inventory via `fsqlite_harness::e2e_traceability::build_canonical_inventory()` and asserting the registered Rust harness entry plus the shell utility entry both still advertise `bd-1sf8n`, `MVCC-7`, and the expected replay command.
- Added a direct shell-runner contract check against `scripts/verify_bd_1sf8n_phase9_time_travel.sh`, so the Phase 9 script's `SCENARIO_ID`, `REPLAY_COMMAND`, and minimum scenario-outcome count cannot silently drift from the Rust gate test.

Notes:
- `bd-1mt2x` is still blocked by `bd-3mgq5`, so this commit remains an epic-level verification increment rather than a claim/closure of the child bead.
- Guardrails preserved: `concurrent_mode_default` remains on by default, no `unsafe`, no Tokio, manual edits only.
- The repo already had unrelated local edits in other files; this slice stays confined to the dedicated `bd_1sf8n_phase9_time_travel_gate.rs` harness test and this progress note.

Verification target for this increment:
- `cargo test -p fsqlite-harness --test bd_1sf8n_phase9_time_travel_gate -- --nocapture --test-threads=1`
- `cargo test -p fsqlite-harness scenario_catalog_matches_phase9_traceability_contract -- --nocapture`
- `ubs crates/fsqlite-harness/tests/bd_1sf8n_phase9_time_travel_gate.rs`
- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo fmt --all --check`
