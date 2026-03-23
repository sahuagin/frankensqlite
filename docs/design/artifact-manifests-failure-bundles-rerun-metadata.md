# Artifact Manifests, Failure Bundles, and Rerun Metadata

**Bead:** `bd-db300.7.6.3` (G6.3)
**Date:** 2026-03-23
**Status:** Design contract — ready for implementation; dependency gaps noted in §8
**Depends on:** G6.1 (logging schema, OPEN), G5.3 (evidence ownership, OPEN), G6.2 (emission points, OPEN)

---

## Purpose

Define the canonical artifact-bundle shape so every verification run —
passing or failing, correctness or performance, file-backed or in-memory —
produces a mechanically comparable package. A human, CI system, or future
agent should answer "what failed, where are the artifacts, and how do I
rerun it" from the bundle alone.

This contract extends the in-tree structures: `VerifySuitePackage`,
`VerifySuiteCounterexampleBundle` (realdb_e2e.rs), `HotPathArtifactManifest`,
`HotPathArtifactProvenance` (perf_runner.rs), and `ArtifactManifest`
(ci_artifacts.rs).

---

## 1. Universal Bundle Envelope

Every artifact bundle — regardless of verification class — is rooted by a
single manifest file that acts as the entry point.

```
{artifact_root}/
├── manifest.json              ← Universal envelope (this section)
├── env.json                   ← Environment/provenance snapshot
├── rerun.sh                   ← Exact rerun command
├── logs/
│   └── run.jsonl              ← Structured JSONL log for the run
├── results/                   ← Class-specific result files
│   └── (varies by class)
└── first_failure/             ← Present ONLY on failure
    ├── bundle.json            ← First-failure bundle (§3)
    ├── counterexample.json    ← Divergence details if shadow-oracle
    └── context/               ← Relevant log slices, page dumps, etc.
```

### 1.1 `manifest.json` Schema

```json
{
  "schema_version": "fsqlite-e2e.artifact_manifest.v2",

  "_comment_run_identity": "Who/what/when",
  "trace_id": "uuid-v4",
  "scenario_id": "MVCC-COR-01",
  "suite_id": "db300_verification",
  "run_id": "20260323T075500_MVCC-COR-01_c4",
  "bead_id": "bd-db300.7.7.2",

  "_comment_classification": "What kind of run",
  "bundle_kind": "correctness | crash_fault | topology_stress | perf_attribution | decision_plane",
  "verification_class": "COR | CRF | RBR | TOP | PFA",
  "mode": "sqlite_reference | fsqlite_mvcc | fsqlite_single_writer",
  "placement_profile_id": "baseline_unpinned | recommended_pinned | adversarial_cross_node",
  "activation_regime": "red_path_correctness | low_concurrency_fixed_cost | mid_concurrency_scaling | many_core_parallel | hostile_or_unclassified",
  "quick_or_full": "quick | full",

  "_comment_workload": "What was run",
  "workload": "commutative_inserts_disjoint_keys",
  "concurrency": 4,
  "seed": 42,
  "scale": 100,
  "db_selector": "beads-proj-a",

  "_comment_outcome": "What happened",
  "result": "pass | fail | skip | partial",
  "pass_fail_signature": "sha256 of sorted pass/fail row",
  "duration_ms": 1234,
  "first_failure_path": "first_failure/bundle.json | null",

  "_comment_rerun": "How to reproduce",
  "rerun_entrypoint": "rerun.sh",
  "rerun_command": "realdb-e2e verify-suite --scenario MVCC-COR-01 --depth quick --placement baseline_unpinned",
  "focused_rerun_command": "cargo test -p fsqlite-mvcc test_name -- --nocapture",

  "_comment_oracle": "What was compared against",
  "oracle": "rusqlite | wal_invariant | self_consistency | none",
  "compatibility_surface": "row_level | integrity_check | topology_metrics | hot_path_profile",

  "_comment_provenance": "Build and environment context",
  "env_path": "env.json",

  "_comment_artifacts": "What files are in this bundle",
  "artifact_root": "/artifacts/MVCC-COR-01/20260323T075500/",
  "retention_class": "ephemeral | ci_30d | release_permanent",
  "files": [
    {"path": "logs/run.jsonl", "bytes": 45000, "sha256": "abc123..."},
    {"path": "results/comparison.jsonl", "bytes": 12000, "sha256": "def456..."},
    {"path": "env.json", "bytes": 800, "sha256": "ghi789..."},
    {"path": "rerun.sh", "bytes": 200, "sha256": "jkl012..."}
  ]
}
```

### 1.2 `env.json` Schema (Provenance)

Extends the existing `HotPathArtifactProvenance` and `EnvironmentMeta`:

```json
{
  "schema_version": "fsqlite-e2e.env.v2",
  "commit_sha": "abc123",
  "branch": "main",
  "dirty": false,
  "cargo_profile": "release-perf",
  "rustc_version": "1.86.0-nightly",
  "kernel_release": "6.17.0-19-generic",
  "hostname": "builder-01",
  "cpu_model": "AMD Ryzen Threadripper PRO 5995WX",
  "cpu_cores": 64,
  "numa_nodes": 4,
  "memory_gb": 512,
  "smt_enabled": true,
  "governor": "performance",
  "huge_pages": "transparent",
  "timestamp_utc": "2026-03-23T07:55:00Z",
  "tool_versions": [
    {"tool": "perf", "version": "6.17"},
    {"tool": "numactl", "version": "2.0.18"}
  ]
}
```

### 1.3 `rerun.sh` Contract

```bash
#!/usr/bin/env bash
# Auto-generated rerun script for {scenario_id}
# Trace ID: {trace_id}
# Original run: {timestamp}
set -euo pipefail
cd "$(dirname "$0")/../.."  # Navigate to workspace root

{exact_command_that_was_run}
```

The rerun script must be executable, self-contained (no environment variables
beyond PATH), and produce a new artifact bundle in a timestamped directory.

---

## 2. Class-Specific Result Files

### 2.1 Correctness (COR) and Rollback/Recovery (RBR)

```
results/
├── comparison.jsonl    ← Per-row oracle comparison (fsqlite vs rusqlite)
├── oracle_diff.json    ← First divergence details (null if pass)
└── summary.json        ← Row counts, pass/fail, timing
```

`comparison.jsonl` row:
```json
{"row_idx": 0, "query": "SELECT ...", "fsqlite": ["val1", "val2"], "oracle": ["val1", "val2"], "match": true}
```

`oracle_diff.json` (on failure):
```json
{
  "row_idx": 42,
  "query": "SELECT count(*) FROM t WHERE x > 10",
  "fsqlite_value": ["99"],
  "oracle_value": ["100"],
  "diff_type": "value_mismatch | missing_row | extra_row | type_mismatch"
}
```

### 2.2 Crash/Fault (CRF)

```
results/
├── pre_crash_state.jsonl     ← Database state snapshot before fault injection
├── fault_description.json    ← What fault was injected and where
├── post_recovery_state.jsonl ← Database state after WAL replay / recovery
├── integrity_check.txt       ← Output of PRAGMA integrity_check
├── wal_replay_log.jsonl      ← WAL frame replay sequence
└── summary.json              ← Pass/fail, committed rows present, uncommitted rows absent
```

`fault_description.json`:
```json
{
  "fault_type": "torn_write | kill_at_commit | kill_at_checkpoint | io_error | concurrent_kill",
  "injection_point": "wal_frame_write | pager_commit | checkpoint_passive",
  "timing": "pre_sync | post_sync_pre_publish | post_publish",
  "affected_pages": [42, 43],
  "seed": 42
}
```

### 2.3 Topology-Stress (TOP)

```
results/
├── topology_metrics.jsonl     ← Per-interval NUMA/cache/lock counters
├── contention_summary.json    ← Aggregate contention metrics
├── self_consistency.json      ← CommitIndex monotonicity, lock exclusivity checks
└── summary.json               ← Pass/fail, duration, deadlock count
```

`contention_summary.json`:
```json
{
  "total_txns": 10000,
  "completed": 9500,
  "aborted": 500,
  "deadlocks": 0,
  "max_lock_wait_ns": 5000000,
  "p99_lock_wait_ns": 200000,
  "cross_numa_lock_handoffs": 342,
  "remote_hitm_estimate": 1200
}
```

### 2.4 Performance-Attribution (PFA)

```
results/
├── hot_path_profile.jsonl     ← Full hot-path profile (reuses HotPathProfileReport)
├── regime_metrics.jsonl       ← Per-workload-phase metrics
├── attribution_summary.json   ← Top-N attributed cost centers
├── baseline_comparison.json   ← Delta vs SQLite reference (if available)
└── summary.json               ← Pass/fail (artifact presence), duration
```

Reuses existing `HotPathArtifactManifest` and `HotPathProfileReport`
structures from `perf_runner.rs`. The `attribution_summary.json` extends
the existing `build_hot_path_actionable_ranking` output.

### 2.5 Decision-Plane (future — for controller shadow runs)

```
results/
├── controller_decisions.jsonl  ← Per-decision structured log (E4.3 guardrail, DRO, etc.)
├── shadow_comparison.json      ← Conservative-vs-aggressive decision divergence
├── policy_snapshot.json        ← Controller config at run start
└── summary.json                ← Pass/fail, divergence count, regret estimate
```

---

## 3. First-Failure Bundle

Every failing scenario must produce a first-failure bundle. This is the
minimum package needed to diagnose the failure without access to the full
artifact tree.

### 3.1 `first_failure/bundle.json` Schema

```json
{
  "schema_version": "fsqlite-e2e.first_failure_bundle.v2",
  "trace_id": "...",
  "scenario_id": "MVCC-COR-01",
  "suite_id": "db300_verification",
  "run_id": "...",
  "verification_class": "COR",
  "mode": "fsqlite_mvcc",
  "placement_profile_id": "baseline_unpinned",
  "activation_regime": "red_path_correctness",
  "quick_or_full": "quick",

  "failure_type": "row_mismatch | crash_corruption | timeout | deadlock | artifact_missing | invariant_violation",
  "failure_summary": "Row 42 of SELECT count(*) FROM t: expected 100, got 99",

  "oracle": "rusqlite",
  "compatibility_surface": "row_level",

  "rerun_command": "realdb-e2e verify-suite --scenario MVCC-COR-01 ...",
  "focused_rerun_command": "cargo test -p fsqlite-core test_specific_case -- --nocapture",

  "context_files": [
    "context/log_slice.jsonl",
    "context/oracle_diff.json",
    "context/page_dump_42.bin"
  ],

  "env_snapshot": {
    "commit_sha": "abc123",
    "cargo_profile": "release-perf",
    "rustc_version": "1.86.0-nightly"
  }
}
```

### 3.2 Partial/Failed Run Handling

When a run fails mid-execution (crash, timeout, OOM):

1. The manifest is still written with `"result": "partial"`.
2. Available artifacts are listed in `files` — incomplete files are marked
   with `"complete": false`.
3. The first-failure bundle is written with whatever context is available.
4. The `rerun.sh` script is always written (it was generated before execution).

**Rule:** A partial bundle with provenance is more valuable than no bundle.
Never skip manifest writing on failure.

---

## 4. Rerun Metadata Contract

Every bundle carries three levels of rerun capability:

| Level | File | What It Does | When to Use |
|-------|------|-------------|-------------|
| **Exact rerun** | `rerun.sh` | Reproduces the entire scenario end-to-end | First attempt at reproduction |
| **Focused rerun** | `manifest.json:focused_rerun_command` | Runs only the specific failing test | After identifying the failing case |
| **Manual rerun** | `manifest.json:rerun_command` | CLI command with all flags spelled out | When the script isn't available |

### Rerun Invariants

1. `rerun.sh` must work from a clean checkout at the same commit SHA.
2. `focused_rerun_command` must be a valid `cargo test` invocation.
3. Neither rerun path may require environment variables beyond PATH.
4. The seed in the rerun command must match the original run.

---

## 5. Comparison-Friendly Identifiers

Bundles use a composite key for cross-run comparison:

```
{scenario_id}:{mode}:{placement_profile_id}:{concurrency}:{seed}
```

Example: `MVCC-COR-01:fsqlite_mvcc:baseline_unpinned:c4:42`

This key is stored in `manifest.json` and used by:
- CI to diff two runs of the same scenario across commits.
- The scorecard (G4) to correlate performance cells.
- The regime atlas (G5.5) to classify cells by activation regime.

---

## 6. Retention Policy

| Class | Default Retention | Override |
|-------|-------------------|---------|
| `ephemeral` | Deleted after CI pipeline completes | — |
| `ci_30d` | Kept for 30 days in CI artifact store | `FSQLITE_ARTIFACT_RETENTION_DAYS` |
| `release_permanent` | Kept indefinitely in release archive | — |

Assignment rules:
- Quick-suite passing runs → `ephemeral`
- Quick-suite failing runs → `ci_30d`
- Full-suite runs → `ci_30d`
- Release certification runs → `release_permanent`

---

## 7. Structured Log Fields for Bundle Events

```rust
tracing::info!(
    target: "fsqlite::artifact::bundle",
    trace_id = %trace_id,
    scenario_id = %scenario_id,
    suite_id = %suite_id,
    bundle_kind = %kind,
    artifact_manifest_key = %manifest_path,
    artifact_root = %root,
    result = %result,
    pass_fail_signature = %sig,
    rerun_entrypoint = %rerun_path,
    compatibility_surface = %compat,
    oracle = %oracle,
    retention_class = %retention,
    files_count = file_count,
    total_bytes = total_bytes,
    first_failure_present = has_first_failure,
    partial = is_partial,
    duration_ms = elapsed,
);
```

---

## 8. Dependency Gaps

| Blocking Bead | Status | Impact | Mitigation |
|--------------|--------|--------|------------|
| **bd-db300.7.6.1** (G6.1: logging schema) | OPEN | `run.jsonl` field names should align with the logging schema contract. | This artifact specifies the bundle-level fields. Per-event log field names are G6.1's scope. Compatible by construction — both use `trace_id`/`scenario_id` join keys. |
| **bd-db300.7.5.3** (G5.3: evidence ownership) | OPEN | `claim_id`, `evidence_id`, and gap-conversion rules should feed into manifest metadata. | These fields are reserved in the manifest schema but not yet populated. Implementers add them when G5.3 lands. |
| **bd-db300.7.6.2** (G6.2: emission points) | OPEN | Which code paths emit which log families to `run.jsonl`. | This artifact specifies the bundle layout. G6.2 specifies what goes into the log files within it. |

None of these gaps block the bundle shape definition. Implementers can build
bundles now and add G5.3/G6.1/G6.2 fields when those beads close.

---

## 9. Validation Entrypoint

```bash
scripts/verify_g6_3_artifact_bundle_shape.sh
```

This script:
1. Generates a mock bundle for each verification class (COR, CRF, RBR, TOP, PFA).
2. Validates each bundle against the manifest schema (required fields present).
3. Verifies `rerun.sh` is executable and syntactically valid.
4. Verifies `env.json` has all required provenance fields.
5. Verifies `files` array entries exist and SHA-256 checksums match.
6. For failure bundles: verifies `first_failure/bundle.json` is present and valid.
7. Reports missing fields or broken references.
8. Exits 0 if all bundles valid, 1 otherwise.

---

## 10. Consequences for Downstream

| Downstream | What This Provides |
|------------|-------------------|
| **bd-db300.7.7.2** (G7.2: suite composition) | First-failure bundle schema, artifact layout per verification class |
| **bd-db300.7.7.3** (G7.3: CI packaging) | Bundle shape, retention policy, manifest schema for CI attachment |
| **bd-db300.7.8.3** (G8.3: interference mapping) | Topology-stress bundle contents for interference analysis |
| **bd-db300.7.4** (G4: final scorecard) | PFA attribution_summary.json and comparison keys for scorecard cells |
