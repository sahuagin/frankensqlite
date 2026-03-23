# Verification Suite Composition Contract

**Bead:** `bd-db300.7.7.2` (G7.2)
**Date:** 2026-03-23
**Status:** Design artifact — ready for implementation; dependency gaps noted in §9
**Depends on:** G7.1 (verify-suite entrypoints, IN_PROGRESS), G5.2 (crash/fault mapping, OPEN),
A6.3 (side conditions, CLOSED), G6.3 (artifact manifests, OPEN), G2 (crash coverage, OPEN)

---

## Purpose

Define the mandatory scenario composition for each one-command verification
suite so suites fail when a required scenario is missing, not silently succeed
because nobody wired it in.

This artifact uses the already-landed verify-suite operator surface in
`realdb_e2e.rs` (VerifySuitePackage, activation regimes, modes, depths,
shadow modes, divergence classes, kill-switch state) as the runtime
machinery and specifies what each suite MUST contain.

---

## 1. Suite Families

Each suite family maps to one `VerifySuiteMode` × one activation regime.

| Suite Family | Mode | Primary Regime | Purpose |
|-------------|------|----------------|---------|
| **S-REF** | SqliteReference | red_path_correctness | Prove C SQLite baseline produces expected results |
| **S-MVCC** | FsqliteMvcc | (varies by scenario class) | Prove FrankenSQLite MVCC concurrent mode is correct and performant |
| **S-SW** | FsqliteSingleWriter | low_concurrency_fixed_cost | Prove forced single-writer mode matches SQLite behavior |
| **S-CROSS** | FsqliteMvcc | many_core_parallel | Prove cross-node/NUMA correctness under topology stress |
| **S-CRASH** | FsqliteMvcc | red_path_correctness | Prove crash/fault/recovery semantics |

---

## 2. Verification Classes

Each scenario belongs to exactly one verification class. Classes determine
which artifact bundles, log families, and oracle comparisons are required.

| Class | Code | Description | Oracle |
|-------|------|-------------|--------|
| **Correctness** | `COR` | Functional equivalence with C SQLite | rusqlite reference |
| **Crash/Fault** | `CRF` | Recovery after injected crash, torn write, or I/O error | WAL invariant checker |
| **Rollback/Recovery** | `RBR` | Savepoint, transaction rollback, and restart behavior | rusqlite reference |
| **Topology-Stress** | `TOP` | Cross-NUMA, cross-CCD, remote-ownership contention | Self-consistency (no oracle) |
| **Performance-Attribution** | `PFA` | Focused benchmark explaining a specific metric delta | Hot-path profile comparator |

---

## 3. Mandatory Scenarios Per Suite

### 3.1 S-REF: SQLite Reference Baseline

| ID | Class | Scenario | Workload | Concurrency | Depth | Placement |
|----|-------|----------|----------|-------------|-------|-----------|
| REF-COR-01 | COR | Single-writer INSERT/SELECT correctness | commutative_inserts | c1 | Quick | baseline_unpinned |
| REF-COR-02 | COR | Mixed read-write correctness | mixed_read_write | c1 | Quick | baseline_unpinned |
| REF-COR-03 | COR | Hot-page single-writer correctness | hot_page_contention | c1 | Quick | baseline_unpinned |
| REF-RBR-01 | RBR | Savepoint rollback correctness | mixed_read_write | c1 | Full | baseline_unpinned |
| REF-PFA-01 | PFA | c1 baseline throughput and latency | commutative_inserts | c1 | Full | baseline_unpinned |

**Fail criteria:** Any row mismatch against expected results. REF-PFA-01 establishes
the SQLite baseline numbers that S-MVCC ratios are computed against.

### 3.2 S-MVCC: FrankenSQLite MVCC Concurrent Mode

| ID | Class | Scenario | Workload | Concurrency | Depth | Placement | Regime |
|----|-------|----------|----------|-------------|-------|-----------|----|
| MVCC-COR-01 | COR | Disjoint-key concurrent INSERT correctness | commutative_inserts | c4 | Quick | baseline_unpinned | red_path_correctness |
| MVCC-COR-02 | COR | Mixed read-write concurrent correctness | mixed_read_write | c4 | Quick | baseline_unpinned | red_path_correctness |
| MVCC-COR-03 | COR | SSI write-skew detection | (synthetic SSI schedule) | c2 | Quick | baseline_unpinned | red_path_correctness |
| MVCC-COR-04 | COR | Phantom prevention (SSI) | (synthetic SSI schedule) | c2 | Quick | baseline_unpinned | red_path_correctness |
| MVCC-COR-05 | COR | CommitIndex monotonicity under concurrent writes | commutative_inserts | c8 | Full | recommended_pinned | red_path_correctness |
| MVCC-COR-06 | COR | Page-lock exclusivity (no double-ownership) | hot_page_contention | c4 | Quick | baseline_unpinned | red_path_correctness |
| MVCC-COR-07 | COR | MVCC visibility: reader sees consistent snapshot | mixed_read_write | c4 | Quick | baseline_unpinned | red_path_correctness |
| MVCC-RBR-01 | RBR | Concurrent savepoint rollback | mixed_read_write | c4 | Full | baseline_unpinned | red_path_correctness |
| MVCC-RBR-02 | RBR | Transaction abort + retry correctness | hot_page_contention | c4 | Full | baseline_unpinned | red_path_correctness |
| MVCC-RBR-03 | RBR | Connection close during active txn (cleanup) | mixed_read_write | c2 | Full | baseline_unpinned | red_path_correctness |
| MVCC-PFA-01 | PFA | c1 disjoint throughput attribution | commutative_inserts | c1 | Full | baseline_unpinned | low_concurrency_fixed_cost |
| MVCC-PFA-02 | PFA | c4 disjoint throughput attribution | commutative_inserts | c4 | Full | recommended_pinned | mid_concurrency_scaling |
| MVCC-PFA-03 | PFA | c8 disjoint throughput attribution | commutative_inserts | c8 | Full | recommended_pinned | many_core_parallel |
| MVCC-PFA-04 | PFA | c4 hot-page contention attribution | hot_page_contention | c4 | Full | recommended_pinned | mid_concurrency_scaling |
| MVCC-PFA-05 | PFA | c4 mixed read-write attribution | mixed_read_write | c4 | Full | recommended_pinned | mid_concurrency_scaling |
| MVCC-PFA-06 | PFA | c8 mixed read-write attribution | mixed_read_write | c8 | Full | recommended_pinned | many_core_parallel |

**Fail criteria:** COR scenarios: any row mismatch against rusqlite oracle under
same workload. RBR scenarios: post-rollback state differs from oracle. PFA
scenarios: hot-path profile artifact must be emitted (no correctness gate, but
missing artifact = fail).

### 3.3 S-SW: Forced Single-Writer Mode

| ID | Class | Scenario | Workload | Concurrency | Depth | Placement |
|----|-------|----------|----------|-------------|-------|-----------|
| SW-COR-01 | COR | Single-writer parity with SQLite | commutative_inserts | c1 | Quick | baseline_unpinned |
| SW-COR-02 | COR | Mixed read-write parity | mixed_read_write | c1 | Quick | baseline_unpinned |
| SW-RBR-01 | RBR | Savepoint rollback parity | mixed_read_write | c1 | Full | baseline_unpinned |
| SW-PFA-01 | PFA | c1 single-writer overhead attribution | commutative_inserts | c1 | Full | baseline_unpinned |

**Fail criteria:** Row-level mismatch against rusqlite oracle.

### 3.4 S-CRASH: Crash/Fault/Recovery

| ID | Class | Scenario | Fault Type | Concurrency | Depth | Placement |
|----|-------|----------|-----------|-------------|-------|-----------|
| CRF-01 | CRF | WAL frame torn write recovery | Torn write at random WAL offset | c1 | Full | baseline_unpinned |
| CRF-02 | CRF | Crash during COMMIT (pre-WAL-sync) | Kill at txn.commit() entry | c2 | Full | baseline_unpinned |
| CRF-03 | CRF | Crash during COMMIT (post-WAL-sync, pre-publish) | Kill after WAL write, before CommitIndex update | c2 | Full | baseline_unpinned |
| CRF-04 | CRF | Crash during checkpoint | Kill during checkpoint PASSIVE | c1 | Full | baseline_unpinned |
| CRF-05 | CRF | I/O error during page write | Injected EIO on pager write | c1 | Full | baseline_unpinned |
| CRF-06 | CRF | Concurrent crash: 2 writers, kill one mid-commit | Kill writer A, verify writer B commits cleanly | c2 | Full | baseline_unpinned |
| CRF-07 | CRF | Database restart after unclean shutdown | No COMMIT before process exit | c1 | Full | baseline_unpinned |

**Fail criteria:** Post-recovery database must pass `PRAGMA integrity_check`.
Committed data before crash point must be present. Uncommitted data must not
be present. WAL replay must not corrupt the database.

**Dependency gap:** The concrete fault-injection machinery (G2, bd-db300.7.2) is
OPEN. CRF scenarios are specified here but cannot be fully implemented until
the injection framework lands. See §9.

### 3.5 S-CROSS: Topology-Stress

| ID | Class | Scenario | Workload | Concurrency | Depth | Placement |
|----|-------|----------|----------|-------------|-------|-----------|
| TOP-01 | TOP | Cross-NUMA disjoint writes | commutative_inserts | c8 | Full | adversarial_cross_node |
| TOP-02 | TOP | Cross-NUMA hot-page contention | hot_page_contention | c8 | Full | adversarial_cross_node |
| TOP-03 | TOP | Cross-NUMA mixed read-write | mixed_read_write | c8 | Full | adversarial_cross_node |
| TOP-04 | TOP | Remote-ownership lock handoff | hot_page_contention | c4 | Full | adversarial_cross_node |
| TOP-05 | TOP | SMT sibling interference | commutative_inserts | c8 | Full | adversarial_cross_node |

**Fail criteria:** No data corruption (self-consistency check). No deadlocks
(all transactions complete or abort within timeout). Structured topology
metrics must be emitted. No oracle comparison (topology stress is about
survivability, not parity).

---

## 4. Quick vs Full Policy

| Designation | When Run | What's Included | Time Budget |
|-------------|----------|-----------------|-------------|
| **Quick** | Every CI run, every `verify-suite --depth quick` | All COR scenarios at c1–c4, baseline_unpinned. No PFA, no TOP, minimal CRF. | ≤ 60s |
| **Full** | Nightly, pre-release, `verify-suite --depth full` | Everything: COR + RBR + CRF + TOP + PFA at all concurrency levels and placement profiles. | ≤ 30min |

Quick-suite mandatory scenarios (minimum set that must pass for any commit):

```
REF-COR-01, REF-COR-02, REF-COR-03
MVCC-COR-01, MVCC-COR-02, MVCC-COR-03, MVCC-COR-04, MVCC-COR-06, MVCC-COR-07
SW-COR-01, SW-COR-02
```

All other scenarios are Full-only.

---

## 5. Required Artifact Bundle Per Scenario Class

| Class | Artifacts | Log Families |
|-------|-----------|-------------|
| COR | `results.jsonl` (row-level comparison), `oracle_diff.json` (first mismatch) | `fsqlite::verify::correctness` |
| CRF | `pre_crash_state.jsonl`, `post_recovery_state.jsonl`, `integrity_check.txt`, `wal_replay_log.jsonl` | `fsqlite::verify::crash`, `fsqlite::verify::recovery` |
| RBR | `pre_rollback_state.jsonl`, `post_rollback_state.jsonl`, `oracle_diff.json` | `fsqlite::verify::rollback` |
| TOP | `topology_metrics.jsonl` (NUMA node, cache-line, lock-handoff counters), `contention_summary.json` | `fsqlite::verify::topology` |
| PFA | `hot_path_profile.jsonl`, `regime_metrics.jsonl`, `attribution_summary.json` | `fsqlite::verify::perf_attribution` |

### First-Failure Bundle

Every scenario must produce a first-failure bundle on failure:

```json
{
  "schema_version": "fsqlite-e2e.first_failure_bundle.v1",
  "trace_id": "...",
  "scenario_id": "MVCC-COR-01",
  "suite_id": "...",
  "verification_class": "COR",
  "placement_profile": "baseline_unpinned",
  "quick_or_full": "quick",
  "failure_type": "row_mismatch | crash_corruption | timeout | artifact_missing",
  "first_failure_row": { ... },
  "rerun_command": "realdb-e2e verify-suite --scenario MVCC-COR-01 ...",
  "artifact_manifest_key": "artifacts/MVCC-COR-01/manifest.json"
}
```

**Dependency gap:** The artifact manifest schema (G6.3, bd-db300.7.6.3) is
OPEN. The bundle format above is provisional and must align with G6.3 when
it lands. See §9.

---

## 6. Structured Log Schema

Every scenario execution emits:

```rust
tracing::info!(
    target: "fsqlite::verify::suite_scenario",
    trace_id = %trace_id,
    scenario_id = %scenario_id,       // "MVCC-COR-01"
    suite_id = %suite_id,
    verification_class = %class,      // "COR", "CRF", "RBR", "TOP", "PFA"
    placement_profile = %profile,     // "baseline_unpinned", "recommended_pinned", "adversarial_cross_node"
    quick_or_full = %depth,           // "quick", "full"
    mode = %mode,                     // "sqlite_reference", "fsqlite_mvcc", "fsqlite_single_writer"
    activation_regime = %regime,
    oracle = %oracle,                 // "rusqlite", "wal_invariant", "self_consistency", "none"
    compatibility_surface = %compat,  // "row_level", "integrity_check", "topology_metrics"
    artifact_manifest_key = %manifest,
    result = %result,                 // "pass", "fail", "skip"
    failure_type = failure_type.as_deref().unwrap_or("none"),
    duration_ms = elapsed_ms,
);
```

---

## 7. Suite Composition Validation Rules

The following rules must be enforced by the suite runner to prevent accidental
scenario dilution:

### R1: Quick-Suite Minimum Coverage

```
For every Quick run:
  count(COR scenarios in Quick set) >= 11
  Every COR scenario must use a rusqlite oracle comparison
  If any Quick-mandatory scenario is missing → suite FAILS with
    "MISSING_MANDATORY_SCENARIO" error
```

### R2: Full-Suite Class Coverage

```
For every Full run:
  count(COR) >= 11
  count(RBR) >= 4
  count(CRF) >= 3  (relaxed until G2 lands — see §9)
  count(TOP) >= 3  (relaxed until cross-node hardware available)
  count(PFA) >= 7
```

### R3: Placement Profile Coverage

```
For every Full run:
  at least 1 scenario uses baseline_unpinned
  at least 1 scenario uses recommended_pinned
  at least 1 scenario uses adversarial_cross_node (if hardware supports it)
```

### R4: Mode Coverage

```
For every Full run:
  at least 1 scenario uses SqliteReference
  at least 1 scenario uses FsqliteMvcc
  at least 1 scenario uses FsqliteSingleWriter
```

### R5: Regression Catch Guarantee

The composition must include scenarios that catch each of these regression
classes:

| Regression Class | Catching Scenario(s) |
|-----------------|---------------------|
| Synchronization (lost update, double-write) | MVCC-COR-01, MVCC-COR-06 |
| Stale cache / reuse bug | MVCC-COR-07 (snapshot visibility) |
| Allocator / restart failure | CRF-07 (restart after unclean shutdown) |
| Placement-sensitive convoy | TOP-01, TOP-02 (cross-NUMA contention) |
| SSI false negative | MVCC-COR-03, MVCC-COR-04 (write-skew, phantom) |
| WAL corruption | CRF-01, CRF-02, CRF-03 (torn write, crash at commit) |
| Rollback state leak | MVCC-RBR-01, MVCC-RBR-02 |

---

## 8. Named Operator Entrypoint

```bash
scripts/verify_g7_2_suite_composition.sh
```

This script:
1. Enumerates the scenario registry (all defined scenarios across all suites).
2. Validates Quick-suite minimum coverage (R1).
3. Validates Full-suite class coverage (R2).
4. Validates placement profile coverage (R3).
5. Validates mode coverage (R4).
6. Validates regression catch guarantee (R5).
7. Reports any missing mandatory scenarios.
8. Emits `artifacts/g7_2_composition_report.json` with pass/fail per rule.
9. Exits 0 if all rules pass, 1 if any rule fails.

**Implementation note:** This script can be implemented as a Rust integration
test that reads a static scenario registry (a `const` array or JSON manifest)
and validates the composition rules. The verify-suite runner calls this before
executing scenarios.

---

## 9. Dependency Gaps and Blockers

| Blocking Bead | Status | What's Blocked | Mitigation |
|--------------|--------|---------------|------------|
| **bd-db300.7.7.1** (G7.1: one-command entrypoints) | IN_PROGRESS | Suite runner CLI surface that invokes scenarios. Without it, scenarios are defined but not executable via one command. | Scenario definitions in this artifact are standalone. G7.1 wires them into the CLI. |
| **bd-db300.7.5.2** (G5.2: crash/fault mapping) | OPEN | CRF scenario details (fault injection points, crash timing) need G5.2's cross-epic mapping. | CRF scenarios are specified at the contract level. Injection machinery requires G2. |
| **bd-db300.7.6.3** (G6.3: artifact manifests) | OPEN | First-failure bundle schema. The format in §5 is provisional. | Provisional schema is self-consistent. Must be reconciled when G6.3 lands. |
| **bd-db300.7.2** (G2: crash/fault coverage) | OPEN | Concrete fault injection framework for CRF-01 through CRF-07. | CRF scenarios are specified but not implementable until the injection harness exists. Quick-suite coverage rule R2 relaxes CRF count until G2 lands. |

**Closure recommendation:** This bead can be closed when:
1. The scenario registry is committed (static or manifest-driven).
2. The composition validation rules (R1–R5) are testable.
3. The dependency gaps are documented (this section).
4. The downstream bead (G7.3: CI packaging) has enough to proceed.

Items that require upstream beads to close first are explicitly gated and
marked with relaxed thresholds. The composition contract is still useful
without them because it defines WHAT must exist, not HOW to implement it.

---

## 10. Consequences for Downstream Beads

| Downstream | What This Provides |
|------------|-------------------|
| **bd-db300.7.7.3** (G7.3: CI packaging) | Scenario list, Quick/Full designation, time budgets, validation rules |
| **bd-db300.7.4** (G4: final scorecard) | PFA scenario IDs that feed the scorecard cells |
| **bd-db300.7.8.4** (G8.4: controller composition) | TOP scenario IDs for interference replay |
| **bd-db300.7.5.5** (G5.5: regime atlas) | Regime-to-scenario mapping for activation-frontier classification |
