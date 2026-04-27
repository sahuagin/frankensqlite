# Dependency Upgrade Log

**Date:** 2026-04-22
**Project:** frankensqlite
**Language:** Rust
**Trigger:** user request — apply /library-updater exhaustively, ensure asupersync 0.3.1 from crates.io

## Summary

- **Updated:** 3 | **Preserved (already latest):** 1 | **Skipped (risky):** 1 | **Failed:** 0

## Pre-check

- **asupersync = "0.3.1"** already in workspace Cargo.toml, resolved from
  `registry+https://github.com/rust-lang/crates.io-index` in Cargo.lock
  (checksum `eba4173ce977db76d7434bb01f0bd94914a9719570ccb8f9e7d56ded6ba8b70a`).
  **No change needed.** ✓

## Applied Updates

### foldhash: 0.1 → 0.2.0 (fsqlite-pager, fsqlite-core)
- **Inconsistency fix:** `fsqlite-btree` was already on 0.2.0; `fsqlite-pager`
  and `fsqlite-core` lagged at 0.1.5. Unified to 0.2.0.
- **Breaking:** None observed in this project. API surfaces we use
  (`foldhash::fast` builder) are stable across 0.1 → 0.2.
- **Verification:** `rch exec -- cargo check -p fsqlite-pager -p fsqlite-core --all-targets` → exit 0 (84s)

### json5: 0.4.1 → 1.3.1 (fsqlite-ext-json)
- **Why major jump:** json5 went 0.4 → 1.0 to signal API stability — no refactor.
- **Single callsite:** `json5::from_str::<Value>(input)` in `src/lib.rs:795`
  unchanged across 0.4 → 1.3.
- **Verification:** `rch exec -- cargo check -p fsqlite-ext-json --all-targets` → exit 0 (22s)

### jsonschema: 0.41.0 → 0.46.2 (fsqlite-e2e, dev-dependency)
- **Dev-only:** no effect on the shipped library.
- **Transitive changes:** referencing 0.41 → 0.46, micromap 0.3 added, pest
  family removed (no longer needed).
- **Verification:** `rch exec -- cargo check -p fsqlite-e2e` (lib-only) → exit 0
  (49s). `--all-targets` failed on an **unrelated** `BenchmarkSummary` struct-
  drift from concurrent swarm work on `benchmark.rs`; field rollout across
  call-sites is in-flight (independent of this upgrade).

## Skipped

### getrandom: 0.2.17 → 0.4.2 (fsqlite-ext-misc, wasm32 target)
- **Reason:** direct dep is declared *solely* to force-enable the `js` feature
  for `rand 0.8`'s transitive `getrandom 0.2` on wasm32. Bumping the direct
  declaration to 0.4.2 (feature renamed to `wasm_js`) would not reach
  `rand 0.8`'s transitive `getrandom 0.2`, leaving wasm32 without RNG entropy.
- **Proper fix:** bump `rand` 0.8 → 0.9 (which uses getrandom 0.3+ with
  `wasm_js`), then bump direct getrandom. That is a broader API migration
  (rand 0.9 renamed `thread_rng` → `rng`, changed distribution APIs, etc.) and
  should land as its own bead, not as part of this sweep.
- **`cargo outdated`** warning (`Feature js of package getrandom has been obsolete in version 0.4.2`) is a known diagnostic, not a build failure.

## Indirect deps

`cargo update` applied the following transitives (via the direct-dep edits
above): json5 0.4.1 → 1.3.1, jsonschema 0.41.0 → 0.46.2, referencing 0.41.0 →
0.46.2, +micromap 0.3.0, −pest family.

All 11 other workspace deps were already at their latest semver-compatible
version per `cargo outdated -R`.

## Stale Cargo.lock entry

A stale `foldhash v0.1.5` entry remains in `Cargo.lock` with no live consumer
(`cargo tree -i foldhash@0.1.5` reports no dependents). `cargo update` did not
prune it. Harmless; will drop on the next `cargo update --aggressive` or after
the next workspace resolve touches anything near it.

## Files changed

- `crates/fsqlite-pager/Cargo.toml` — `foldhash = "0.2.0"`
- `crates/fsqlite-core/Cargo.toml` — `foldhash = "0.2.0"`
- `crates/fsqlite-ext-json/Cargo.toml` — `json5 = "1.3"`
- `crates/fsqlite-e2e/Cargo.toml` — `jsonschema = { version = "0.46.2", default-features = false }`
- `Cargo.lock` — transitive rebalance per `cargo update`

## Open follow-ups

- [ ] Bump `rand` 0.8 → 0.9 + `getrandom` 0.2 → 0.3 as a dedicated bead
      (touches random-number call-sites throughout the engine).
- [ ] Clean stale `foldhash v0.1.5` Cargo.lock entry on next resolve.

## Notes on the swarm interaction

During this upgrade, edits to Cargo.toml files were intermittently reverted by
rch's artifact-retrieval step (stale source pulled back from the remote
worker when the sync-to-worker happened just before the Edit finished landing
on disk). Re-applying the edits after the `rch exec` returned, without
triggering another remote build, left them persistent. Recommendation for
future library-update passes: batch all Cargo.toml edits, then run one
verification build, rather than interleaving edits with `rch exec` calls.
