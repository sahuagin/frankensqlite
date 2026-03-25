#!/usr/bin/env bash
# Verification gate for bd-db300.5.2.2.4 (E2.2.d):
# fused prepared-DML entry proof, fallback, and matrix regression pack.
#
# Covers: semantic equivalence, schema invalidation, fallback boundary,
# publication reuse, concurrent round-refresh, fast-path separation,
# statement-cache invalidation, regime-atlas contract, and behavior-
# preservation proof note.
#
# Shadow-oracle differential and counterexample-capture remain deferred
# until bd-db300.7.5.6 and packaging infrastructure land.

set -euo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BEAD_ID="${BEAD_ID:-bd-db300.5.2.2.4}"
SCENARIO_ID="${SCENARIO_ID:-E2-2-D-PROOF-PACK-52224}"
SEED="${SEED:-52224}"
TIMESTAMP_UTC="$(date -u +%Y%m%dT%H%M%SZ)"
RUN_ID="${RUN_ID:-${BEAD_ID}-${TIMESTAMP_UTC}-${SEED}}"
TRACE_ID="${TRACE_ID:-trace-${RUN_ID}}"
ARTIFACT_DIR="${WORKSPACE_ROOT}/artifacts/${BEAD_ID}/${RUN_ID}"
EVENTS_JSONL="${ARTIFACT_DIR}/events.jsonl"
REPORT_JSON="${ARTIFACT_DIR}/report.json"
SUMMARY_MD="${ARTIFACT_DIR}/summary.md"
PROOF_NOTE_MD="${ARTIFACT_DIR}/proof_note.md"
HASHES_TXT="${ARTIFACT_DIR}/artifact_hashes.txt"
USE_RCH="${USE_RCH:-0}"
CARGO_TARGET_DIR_BASE="${CARGO_TARGET_DIR_BASE:-${WORKSPACE_ROOT}/.codex-target/e2_2_fused_entry}"

mkdir -p "${ARTIFACT_DIR}"
: > "${EVENTS_JSONL}"

export RUST_LOG="${RUST_LOG:-fsqlite.statement_reuse=info,fsqlite.execution=debug,fsqlite.connection=debug}"
export RUST_TEST_THREADS="${RUST_TEST_THREADS:-1}"
export NO_COLOR="${NO_COLOR:-1}"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"

emit_event() {
  local phase="$1"
  local event_type="$2"
  local outcome="$3"
  local elapsed_ms="$4"
  local message="$5"
  local control_mode="${6:-auto}"

  jq -cn \
    --arg trace_id "${TRACE_ID}" \
    --arg run_id "${RUN_ID}" \
    --arg scenario_id "${SCENARIO_ID}" \
    --arg bead_id "${BEAD_ID}" \
    --arg phase "${phase}" \
    --arg event_type "${event_type}" \
    --arg outcome "${outcome}" \
    --arg message "${message}" \
    --arg control_mode "${control_mode}" \
    --arg timestamp "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
    --argjson seed "${SEED}" \
    --argjson elapsed_ms "${elapsed_ms}" \
    '{
      trace_id: $trace_id,
      run_id: $run_id,
      scenario_id: $scenario_id,
      bead_id: $bead_id,
      phase: $phase,
      event_type: $event_type,
      outcome: $outcome,
      elapsed_ms: $elapsed_ms,
      timestamp: $timestamp,
      seed: $seed,
      message: $message,
      control_mode: $control_mode
    }' >> "${EVENTS_JSONL}"
}

run_phase() {
  local phase="$1"
  local logfile="$2"
  shift 2

  emit_event "${phase}" "start" "running" 0 "running: $*"
  local started finished elapsed
  started="$(date +%s%3N)"

  if (
    cd "${WORKSPACE_ROOT}"
    "$@"
  ) 2>&1 | tee "${logfile}"; then
    finished="$(date +%s%3N)"
    elapsed="$((finished - started))"
    if ! grep -Eq '^running [1-9][0-9]* tests?$' "${logfile}"; then
      emit_event "${phase}" "fail" "fail" "${elapsed}" "command completed without executing tests"
      return 1
    fi
    emit_event "${phase}" "pass" "pass" "${elapsed}" "completed successfully"
  else
    finished="$(date +%s%3N)"
    elapsed="$((finished - started))"
    emit_event "${phase}" "fail" "fail" "${elapsed}" "command failed"
    return 1
  fi
}

run_test_phase() {
  local phase="$1"
  local logfile="$2"
  local test_filter="$3"
  local compatibility_selector="$4"

  local -a cmd=(
    env
    "CARGO_TARGET_DIR=${CARGO_TARGET_DIR_BASE}"
    "CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS}"
    "RUST_LOG=${RUST_LOG}"
    "RUST_TEST_THREADS=${RUST_TEST_THREADS}"
    "NO_COLOR=${NO_COLOR}"
    cargo test -p fsqlite-core --lib "${test_filter}" -- --nocapture
  )
  if [[ "${USE_RCH}" == "1" ]]; then
    cmd=(rch exec -- "${cmd[@]}")
  fi

  run_phase "${phase}" "${logfile}" "${cmd[@]}"
  emit_event \
    "${phase}" \
    "compatibility_selector" \
    "pass" \
    0 \
    "compatibility_selector=${compatibility_selector}"
}

run_integration_test_phase() {
  local phase="$1"
  local logfile="$2"
  local test_target="$3"
  local compatibility_selector="$4"

  local -a cmd=(
    env
    "CARGO_TARGET_DIR=${CARGO_TARGET_DIR_BASE}"
    "CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS}"
    "RUST_LOG=${RUST_LOG}"
    "RUST_TEST_THREADS=${RUST_TEST_THREADS}"
    "NO_COLOR=${NO_COLOR}"
    cargo test -p fsqlite-core --test "${test_target}" -- --nocapture
  )
  if [[ "${USE_RCH}" == "1" ]]; then
    cmd=(rch exec -- "${cmd[@]}")
  fi

  run_phase "${phase}" "${logfile}" "${cmd[@]}"
  emit_event \
    "${phase}" \
    "compatibility_selector" \
    "pass" \
    0 \
    "compatibility_selector=${compatibility_selector}"
}

run_harness_test_phase() {
  local phase="$1"
  local logfile="$2"
  local test_target="$3"
  local compatibility_selector="$4"

  local -a cmd=(
    env
    "CARGO_TARGET_DIR=${CARGO_TARGET_DIR_BASE}"
    "CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS}"
    "NO_COLOR=${NO_COLOR}"
    cargo test -p fsqlite-harness --test "${test_target}" -- --nocapture
  )
  if [[ "${USE_RCH}" == "1" ]]; then
    cmd=(rch exec -- "${cmd[@]}")
  fi

  run_phase "${phase}" "${logfile}" "${cmd[@]}"
  emit_event \
    "${phase}" \
    "compatibility_selector" \
    "pass" \
    0 \
    "compatibility_selector=${compatibility_selector}"
}

generate_proof_note() {
  cat > "${PROOF_NOTE_MD}" <<'PROOF'
# Behavior-Preservation Proof Note (E2.2.d)

## Ordering
The fused entry path (PreparedDmlEntryProof) threads the same prebound
publication and op_cx through the same function call sequence as the
conservative path.  No reordering of schema validation, publication bind,
or autocommit begin is introduced.  The ensure_schema_unchanged check
runs before ensure_autocommit_txn in both paths.

## Visibility
Publication binding uses the same BoundPagerPublication snapshot in both
paths.  The fused path reuses the snapshot captured during schema
validation rather than re-binding, but the snapshot content is identical
because no schema change can intervene within one execute_prepared call.

## Crash-Safety
The fused path does not change the pager commit protocol (Phase A/B/C),
WAL append, sync, or checkpoint behavior.  All durable state transitions
remain under the same lock discipline.  The PreparedDmlEntryProof is a
transient in-memory value that does not survive process restart.

## Fallback Semantics
When PreparedDmlEntryProof carries no_publication() (deferred-DML path),
ensure_autocommit_txn_with_publication_hint receives None and falls
through to the conservative refresh_memdb_if_stale_with_publication path.
No fast-path bypass occurs without a valid prebound publication.

## Control-Mode Equivalence
The current implementation uses control_mode=auto exclusively.  The
forced_fallback and shadow_compare modes are not yet implemented.  When
they are, the PreparedDmlEntryProof must be extended with a control_mode
field and the fast path must check it before bypassing any ceremony.
This is tracked as deferred work.
PROOF
  emit_event "proof_note" "artifact" "pass" 0 "generated proof_note.md"
}

hash_artifacts() {
  : > "${HASHES_TXT}"
  (
    cd "${ARTIFACT_DIR}"
    sha256sum ./*.log ./*.md ./*.json ./events.jsonl 2>/dev/null || true
  ) > "${HASHES_TXT}"
}

echo "=== ${BEAD_ID}: fused entry proof pack (E2.2.d) ==="
echo "run_id=${RUN_ID}"
echo "trace_id=${TRACE_ID}"
echo "scenario_id=${SCENARIO_ID}"
echo "seed=${SEED}"
echo "artifacts=${ARTIFACT_DIR}"

emit_event "bootstrap" "start" "running" 0 "verification started"

# ── E2.2.c phases (carried forward) ──────────────────────────────────

run_test_phase \
  "semantic_equivalence" \
  "${ARTIFACT_DIR}/semantic_equivalence.log" \
  "test_prepared_insert_reuse_semantic_parity_with_execute" \
  "prepared_dml_semantic_equivalence"

run_test_phase \
  "schema_invalidation" \
  "${ARTIFACT_DIR}/schema_invalidation.log" \
  "test_prepared_dml_rejects_" \
  "prepared_dml_schema_invalidation"

run_test_phase \
  "fallback_boundary" \
  "${ARTIFACT_DIR}/fallback_boundary.log" \
  "uses_fallback" \
  "prepared_dml_fallback_boundary"

run_test_phase \
  "publication_equivalence" \
  "${ARTIFACT_DIR}/publication_equivalence.log" \
  "schema_bound_publication" \
  "prepared_dml_publication_reuse"

run_test_phase \
  "concurrent_round_refresh" \
  "${ARTIFACT_DIR}/concurrent_round_refresh.log" \
  "test_disjoint_concurrent_prepared_insert_reuse_across_rounds_refreshes_snapshot" \
  "prepared_dml_round_refresh"

run_integration_test_phase \
  "fast_path_separation" \
  "${ARTIFACT_DIR}/fast_path_separation.log" \
  "fast_path_separation" \
  "prepared_dml_fast_path_separation"

run_integration_test_phase \
  "statement_cache_invalidation" \
  "${ARTIFACT_DIR}/statement_cache_invalidation.log" \
  "statement_cache_invalidation" \
  "prepared_dml_statement_cache_invalidation"

# ── E2.2.d new phases ────────────────────────────────────────────────

run_harness_test_phase \
  "regime_atlas_contract" \
  "${ARTIFACT_DIR}/regime_atlas_contract.log" \
  "bd_db300_7_5_5_regime_atlas_contract" \
  "regime_atlas_activation_frontier"

generate_proof_note

# ── Finalize ─────────────────────────────────────────────────────────

hash_artifacts

cat > "${SUMMARY_MD}" <<EOF
# ${BEAD_ID} Fused Entry Proof Pack (E2.2.d)

- run_id: \`${RUN_ID}\`
- trace_id: \`${TRACE_ID}\`
- scenario_id: \`${SCENARIO_ID}\`
- seed: \`${SEED}\`
- bead: \`${BEAD_ID}\`
- stage: E2.2.d (proof, fallback, and matrix regression pack)

## Direct Proof Surface Executed

- semantic equivalence: \`test_prepared_insert_reuse_semantic_parity_with_execute\`
- schema invalidation: \`test_prepared_dml_rejects_*\`
- fallback boundary: \`*_uses_fallback\`
- publication reuse: \`*schema_bound_publication*\`
- concurrent round-refresh: \`test_disjoint_concurrent_prepared_insert_reuse_across_rounds_refreshes_snapshot\`
- fast-path separation: \`fast_path_separation\` integration test suite
- statement-cache invalidation: \`statement_cache_invalidation\` integration test suite
- regime-atlas contract: \`bd_db300_7_5_5_regime_atlas_contract\` harness test

## Behavior-Preservation Proof Note

See \`proof_note.md\` in this artifact directory.  Covers: ordering,
visibility, crash-safety, fallback semantics, and control-mode equivalence.

## Delivered in E2.2.d (previously deferred)

- Regime-atlas activation frontier contract (bd-db300.7.5.5 test phase)
- Behavior-preservation proof note artifact

## Honestly Deferred

- Shadow-oracle differential equivalence (awaiting bd-db300.7.5.6)
- Counterexample-capture bundle packaging (no infrastructure yet)
- Control-mode override routing tests (forced_fallback/shadow_compare not implemented)

## Replay Command

\`\`\`bash
BEAD_ID=${BEAD_ID} USE_RCH=${USE_RCH} CARGO_TARGET_DIR_BASE='${CARGO_TARGET_DIR_BASE}' bash scripts/verify_e2_2_fused_entry.sh
\`\`\`

## Artifact Hashes

\`\`\`
$(cat "${HASHES_TXT}")
\`\`\`
EOF

jq -n \
  --arg bead_id "${BEAD_ID}" \
  --arg run_id "${RUN_ID}" \
  --arg trace_id "${TRACE_ID}" \
  --arg scenario_id "${SCENARIO_ID}" \
  --arg seed "${SEED}" \
  --arg events_jsonl "${EVENTS_JSONL}" \
  --arg summary_md "${SUMMARY_MD}" \
  --arg proof_note_md "${PROOF_NOTE_MD}" \
  --arg hashes_txt "${HASHES_TXT}" \
  --arg use_rch "${USE_RCH}" \
  --arg cargo_target_dir_base "${CARGO_TARGET_DIR_BASE}" \
  '{
    bead_id: $bead_id,
    stage: "E2.2.d",
    run_id: $run_id,
    trace_id: $trace_id,
    scenario_id: $scenario_id,
    seed: ($seed | tonumber),
    result: "pass",
    replay: {
      entrypoint: "scripts/verify_e2_2_fused_entry.sh",
      use_rch: ($use_rch == "1"),
      cargo_target_dir_base: $cargo_target_dir_base
    },
    direct_test_surface: [
      "test_prepared_insert_reuse_semantic_parity_with_execute",
      "test_prepared_dml_rejects_*",
      "*_uses_fallback",
      "*schema_bound_publication*",
      "test_disjoint_concurrent_prepared_insert_reuse_across_rounds_refreshes_snapshot",
      "fast_path_separation (integration suite)",
      "statement_cache_invalidation (integration suite)",
      "bd_db300_7_5_5_regime_atlas_contract (harness suite)"
    ],
    compatibility_selectors: [
      "prepared_dml_semantic_equivalence",
      "prepared_dml_schema_invalidation",
      "prepared_dml_fallback_boundary",
      "prepared_dml_publication_reuse",
      "prepared_dml_round_refresh",
      "prepared_dml_fast_path_separation",
      "prepared_dml_statement_cache_invalidation",
      "regime_atlas_activation_frontier"
    ],
    delivered_in_e22d: [
      "regime_atlas_contract phase",
      "behavior_preservation proof_note.md"
    ],
    honestly_deferred: [
      "shadow_oracle_contract (awaiting bd-db300.7.5.6)",
      "counterexample_bundle packaging (no infrastructure)",
      "control_mode override routing (forced_fallback/shadow_compare not implemented)"
    ],
    artifacts: {
      events_jsonl: $events_jsonl,
      summary_md: $summary_md,
      proof_note_md: $proof_note_md,
      artifact_hashes: $hashes_txt
    }
  }' > "${REPORT_JSON}"

emit_event "finalize" "pass" "pass" 0 "report written to ${REPORT_JSON}"
echo "[GATE PASS] ${BEAD_ID} fused entry proof pack (E2.2.d) passed"
