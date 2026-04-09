# Certification Gates, Ratchets, and Release Evidence

**Bead:** `bd-2yqp6.7`  
**Scope:** `crates/fsqlite-harness/`, `crates/fsqlite-e2e/`, and docs  
**Purpose:** define the exact conditions for calling a release "conformant" on
the declared SQLite-compatible surface, and define the monotone ratchets that
prevent silent backslide.

## Source Of Truth

The machine-readable policy lives in:

- `crates/fsqlite-harness/src/certification_policy.rs`
- `crates/fsqlite-harness/src/release_certificate.rs`

This document is the operator-facing interpretation of that policy.

## Conformance Claim

FrankenSQLite may call a release **conformant** only when all blocking gates
pass on the declared supported surface from
[`canonical_parity_contract.md`](../canonical_parity_contract.md).

The certification profile is intentionally strict:

- declared-surface verification: `100.0%`
- required suite pass rate: `100.0%`
- HIGH-severity unresolved counterexamples: `0`
- evidence freshness budget: `24h`
- ratchet tolerance: `0.0`
- quarantine/waivers for certification ratchets: disabled

## Blocking Gates

These are the blocking gates in the canonical policy:

1. **Declared-surface parity gate**  
   `confidence_gates.rs` and `release_certificate.rs` must both report
   `release_ready=true` with `100.0%` verification on the declared surface.

2. **Required suite pass gates**  
   The following CI lanes must each pass at `100.0%`:
   - `unit`
   - `e2e-differential`
   - `e2e-correctness`
   - `e2e-recovery`
   - `schema-validation`
   - `coverage-drift`

3. **Verification-contract gate**  
   `verification_contract_enforcement.rs` must report
   `final_gate_passed=true` with:
   - `missing_evidence_beads = 0`
   - `invalid_reference_beads = 0`

4. **Release-evidence completeness gate**  
   The certification bundle must include a concrete feature -> test -> run ->
   artifact-hash chain. The release certificate embeds this in
   `certification_traceability`.

5. **Critical-path evidence gate**  
   No critical invariant may be left with mocked-only evidence, ignored
   coverage, or a missing real proof path. The harness-side validation and
   no-mock critical-path reports are the intended enforcement surface here.

## Required Evidence Bundle

The certification bundle is complete only when all of the following are
published together:

- confidence gate report
- verification-contract outcome
- release certificate JSON
- CI artifact manifest
- benchmark `results.jsonl`
- benchmark `summary.md`
- benchmark `scorecards.json`
- critical-path / real-evidence report
- ratchet state

In Track G, the e2e side contributes the benchmark and artifact-pack evidence,
while the harness side contributes the gate, contract, ratchet, and release
certificate evidence.

## Feature -> Test -> Run -> Artifact Hash

The release certificate now embeds the certification traceability view instead
of leaving it implicit:

- `feature_id` and `invariant_id` come from the parity invariant catalog
- `test_path` comes from proof obligations
- `run_id`, `lane`, and `git_sha` come from the CI artifact manifest
- `content_hash` comes from manifest artifact entries

That means a release reviewer can answer:

- which supported feature is being certified
- which executable proof covered it
- which concrete run produced the proof
- which artifact hash captures the published evidence

## Ratchets

Certification ratchets are stricter than ordinary progress tracking. They are
all monotone and all blocking:

1. **Global lower-bound ratchet**  
   The global parity lower bound must not decrease across certified releases.

2. **Per-category lower-bound ratchet**  
   Each declared-surface category lower bound must not decrease.

3. **Required-suite pass-rate ratchet**  
   The mandatory certification lanes must not backslide below the previous
   certified baseline.

4. **Traceability-link coverage ratchet**  
   The fraction of certified invariants that resolve all artifact refs into a
   concrete run/artifact hash must not decrease.

5. **Artifact-hash integrity ratchet**  
   Artifact bundle hashes remain stable unless a reviewed baseline update is
   explicitly recorded by the artifact-hash ratchet machinery.

## Operator Flow

1. Run the harness parity/correctness gates and publish the gate report plus
   verification-contract outcome.
2. Run the e2e evidence pack so `manifest.json`, `results.jsonl`,
   `summary.md`, and `scorecards.json` exist for the same run family.
3. Generate/publish the release certificate JSON so it embeds the strict Track
   G policy and certification traceability.
4. Compare against the persisted ratchet state before calling the release
   conformant.

## Failure Semantics

The release must not be called conformant when any of the following are true:

- declared-surface verification is below `100.0%`
- any mandatory CI lane is red
- verification-contract enforcement reports missing evidence or invalid refs
- the release certificate lacks a concrete artifact manifest
- traceability artifact refs do not resolve to manifest hashes
- the monotone ratchet regresses
