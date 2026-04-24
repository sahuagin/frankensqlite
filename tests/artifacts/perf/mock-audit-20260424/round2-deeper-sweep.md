# `/mock-code-finder` round 2 — deeper sweep (2026-04-24)

Second pass over `crates/fsqlite-pager/src/` and
`crates/fsqlite-wal/src/` with a broader pattern set than round 1
(`summary.md` in this directory). Goal: confirm round 1 and look for
anything it missed.

## Round 1 findings — still the only dead-code beads

- `bd-q7zls` P1: `fsqlite-pager::arc_cache` pub-exported, zero
  production consumers (3,866 LOC).
- `bd-5ftij` P2: `fsqlite-pager::thompson_partitioner` pub-exported,
  zero production consumers (350 LOC).

Both still open, awaiting user disposition (delete / wire /
experimental-feature-gate).

## One additional acknowledged gap — already tracked

`fsqlite-wal/src/native_commit.rs:382-385`:

```rust
// SSI re-validation would check for dangerous structure here.
// For now, we accept (the writer already validated locally in step 2).
// Full SSI re-validation is deferred to the SSI witness plane
// implementation (bd-3t3.9.*).
```

This is load-bearing code — the cross-process commit acceptance
path. It IS a real gap (cross-process SSI re-validation is not
performed), but it's acknowledged in-source and tracked via the
existing `bd-3t3` MVCC-formal-model epic, which has a 12-bead
subtree including `bd-3t3.9.*`. **No new bead needed** — filing one
would duplicate the existing tracking. Recording it here so a
future auditor who greps for "for now" finds the cross-reference.

## What I scanned this pass

Patterns run against both crates; all results cross-checked to
confirm they were either (a) already tracked, (b) false positives,
or (c) documented design decisions:

| Pattern | Hits | Disposition |
|---|---:|---|
| `TODO\|FIXME\|XXX\|HACK` case-insensitive | 0 | clean |
| `todo!()\|unimplemented!()` | 0 | clean |
| `panic!()` outside `#[cfg(test)]` | 0 | clean |
| `placeholder\|stub\|dummy\|fake\|mock` | ~15 | all test fixtures + `arc_cache` self-references (already filed) |
| `kludge\|workaround\|temporary\|for now\|TBD\|TBA` | 1 prod | `native_commit.rs:382` — already tracked in bd-3t3.9.* |
| `intentionally\|by design\|best-effort\|soft (fail\|error)` | ~20 | all documented design decisions |
| `let _ = <fn>()` outside tests | 2 | both inside a `#[cfg(test)]` warm-up loop (`wal.rs:1584-1585`) |
| `deferred\|not yet wired\|later\|pending` in comments | many | all descriptive ("will be flushed later", "deferred to Phase C", etc.) — none are gaps |
| `Ok(())\|Ok(None)\|Ok(0)\|Ok(Vec::new())` single-line bodies | 30+ in `traits.rs` | all legit default trait impls; production `WalBackendAdapter` in `fsqlite-core/src/wal_adapter.rs` overrides each real one (verified via grep-count) |

## Conclusion

`crates/fsqlite-pager` and `crates/fsqlite-wal` are clean of stubs,
TODOs, placeholder code, or hidden implementation gaps. The two
filed bugs (`bd-q7zls`, `bd-5ftij`) are carry-over *abstractions*,
not half-finished code — the implementations they contain are
complete; they just don't have callers. The one acknowledged
implementation gap (`native_commit.rs` SSI re-validation) is
already tracked in the `bd-3t3` epic.

## Methodology trail

Round 1 commands (in `summary.md`):
- 5-stanza grep suite across stubs/TODOs/panic/defaults/dead-code.

Round 2 additions (this file):
- `grep -ri 'TODO|FIXME|XXX|HACK'` case-insensitive in both crates.
- `grep -rn 'kludge|workaround|temporary|for now|TBD|TBA'`.
- `grep -rn 'intentionally|by design|best-effort|soft (fail|error)'`
  (cross-checked against each hit to confirm it was design, not
  deferral).
- `grep -rn 'let _ = \w+('` outside tests to flag ignored Results.
- `grep -rn 'is deferred|to be wired|not yet wired|not yet implemented|not yet supported|not yet landed|not implemented here|will validate|will be|TBD|TBA'`
  — cross-referenced against existing beads (bd-3t3.* family).

No beads created in round 2. Round 1's two beads remain the only
open items from this audit surface.
