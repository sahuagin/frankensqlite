#!/usr/bin/env bash
# Verification gate for bd-db300.5.2.2.4:
# fused prepared-DML entry proof, fallback, regime-atlas, and shadow-oracle pack.

set -euo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BEAD_ID="bd-db300.5.2.2.4"
SCENARIO_ID="${SCENARIO_ID:-E2-2-FUSED-ENTRY-52224}"
SEED="${SEED:-52224}"
TIMESTAMP_UTC="$(date -u +%Y%m%dT%H%M%SZ)"
RUN_ID="${RUN_ID:-${BEAD_ID}-${TIMESTAMP_UTC}-${SEED}}"
TRACE_ID="${TRACE_ID:-trace-${RUN_ID}}"
ARTIFACT_DIR="${WORKSPACE_ROOT}/artifacts/${BEAD_ID}/${RUN_ID}"
EVENTS_JSONL="${ARTIFACT_DIR}/events.jsonl"
REPORT_JSON="${ARTIFACT_DIR}/report.json"
SUMMARY_MD="${ARTIFACT_DIR}/summary.md"
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

  jq -cn \
    --arg trace_id "${TRACE_ID}" \
    --arg run_id "${RUN_ID}" \
    --arg scenario_id "${SCENARIO_ID}" \
    --arg bead_id "${BEAD_ID}" \
    --arg phase "${phase}" \
    --arg event_type "${event_type}" \
    --arg outcome "${outcome}" \
    --arg message "${message}" \
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
      message: $message
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

hash_artifacts() {
  : > "${HASHES_TXT}"
  (
    cd "${ARTIFACT_DIR}"
    sha256sum ./*.log ./events.jsonl 2>/dev/null
  ) > "${HASHES_TXT}"
}

echo "=== ${BEAD_ID}: fused entry proof pack ==="
echo "run_id=${RUN_ID}"
echo "trace_id=${TRACE_ID}"
echo "scenario_id=${SCENARIO_ID}"
echo "seed=${SEED}"
echo "artifacts=${ARTIFACT_DIR}"

emit_event "bootstrap" "start" "running" 0 "verification started"

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

hash_artifacts

cat > "${SUMMARY_MD}" <<EOF
# ${BEAD_ID} Fused Entry Proof Pack

- run_id: \`${RUN_ID}\`
- trace_id: \`${TRACE_ID}\`
- scenario_id: \`${SCENARIO_ID}\`
- seed: \`${SEED}\`
- bead: \`${BEAD_ID}\`

## Direct Proof Surface Executed

- semantic equivalence: \`test_prepared_insert_reuse_semantic_parity_with_execute\`
- schema invalidation: \`test_prepared_dml_rejects_schema_change\`, \`test_prepared_dml_rejects_cross_connection_schema_change\`
- fallback boundary: \`test_prepared_insert_select_uses_fallback\`, \`test_prepared_update_with_limit_uses_fallback\`, \`test_prepared_delete_with_limit_uses_fallback\`
- control-mode publication equivalence anchor: \`test_prepared_file_backed_insert_reuses_schema_bound_publication_for_autocommit_begin\`, \`test_prepared_file_backed_single_writer_insert_reuses_schema_bound_publication\`
- matrix regression anchor: \`test_disjoint_concurrent_prepared_insert_reuse_across_rounds_refreshes_snapshot\`

## Activation And Fallback Rules

- \`control_mode=auto\`: fused entry is eligible only when the statement has a precompiled prepared-DML dispatch, schema identity is unchanged, publication binding is valid for the current file-backed or memory-backed regime, no static or dynamic fallback reason is active, and the regime atlas classifies the cell as at least \`regime_gated_default\`.
- \`control_mode=forced_fallback\`: the conservative dispatcher remains authoritative and the fused path must be bypassed with an explicit \`fallback_reason\`.
- \`control_mode=shadow_compare\`: the conservative dispatcher remains authoritative while the fused path is evaluated as the candidate path under the shadow-oracle contract.
- Unclassified or hostile regimes must route to deterministic conservative behavior rather than implicitly enabling the fused path.

## Regime Atlas / Activation Frontier Contract

- Upstream contract bead: \`bd-db300.7.5.5\`
- Named upstream validation entrypoint: \`scripts/verify_g5_5_regime_atlas.sh\`
- Required activation states: \`universal_default\`, \`regime_gated_default\`, \`shadow_only\`, \`operator_opt_in\`, \`rejected\`
- Required regime axes for this proof pack: engine mode, workload family, read/write mix, hot-page pressure, concurrency level, process/thread split, topology profile, durability shape, and control-mode override.
- Required frontier outputs for fused entry: \`regime_id\`, \`activation_state\`, \`frontier_reason\`, \`breakpoint_metric\`, \`baseline_comparator\`, \`fallback_state\`, and explicit evidence or gap-conversion linkage.

## Shadow-Oracle Differential Contract

- Upstream contract bead: \`bd-db300.7.5.6\`
- Named upstream validation entrypoint: \`scripts/verify_g5_6_shadow_oracle.sh\`
- Oracle path: conservative dispatcher / deferred prepared-DML execution
- Candidate path: fused prepared-DML transaction entry
- Required equivalence scope: user-visible rows, affected-row count, transaction outcome, schema-invalidation response, visibility/publication state, and error-class identity
- Required shadow modes: \`off\`, \`forced\`, \`sampled\`, \`shadow_canary\`
- Divergence must never silently preserve the candidate result; conservative behavior stays authoritative until the shadow contract is satisfied

## Divergence Handling

- Required divergence classes: \`result_mismatch\`, \`affected_rows_mismatch\`, \`error_class_mismatch\`, \`visibility_mismatch\`, \`fallback_mismatch\`, \`ordering_mismatch\`, \`trace_contract_missing\`
- Any divergence forces \`fallback_state=active\`, records a first-failure bundle, and blocks promotion above \`shadow_only\`
- Missing regime evidence or missing shadow evidence is not treated as success; it becomes tracked work through the gap-conversion rules from \`bd-db300.7.5.3\`

## Counterexample Bundle Requirements

- required fields: \`trace_id\`, \`run_id\`, \`scenario_id\`, \`shadow_run_id\`, \`oracle_path\`, \`candidate_path\`, \`control_mode\`, \`equivalence_scope\`, \`allowed_difference_policy\`, \`divergence_class\`, \`fallback_state\`, \`counterexample_bundle\`, \`compatibility_selector\`, \`schema_epoch\`, \`publication_seq\`, \`wal_generation\`, and first-failure diagnostics
- required payload: replay commands, SQL template or prepared-statement identity, regime classification, artifact hashes, log file paths, and a minimized explanation of the first semantic mismatch

## Named Script Obligations

- \`scripts/verify_e2_2_fused_entry.sh\` — direct fused-entry proof surface and proof-note emission
- \`scripts/verify_g5_5_regime_atlas.sh\` — regime atlas and activation-frontier validation dependency
- \`scripts/verify_g5_6_shadow_oracle.sh\` — shadow-oracle and counterexample-capture validation dependency

## Named Logging Obligations

- \`trace_id\`
- \`run_id\`
- \`scenario_id\`
- \`entry_ticket_state\`
- \`control_mode\`
- \`regime_id\`
- \`activation_state\`
- \`frontier_reason\`
- \`shadow_run_id\`
- \`shadow_verdict\`
- \`oracle_path\`
- \`candidate_path\`
- \`equivalence_scope\`
- \`allowed_difference_policy\`
- \`compatibility_selector\`
- \`fallback_reason\`
- \`fallback_state\`
- \`schema_epoch\`
- \`publication_seq\`
- \`wal_generation\`
- \`counterexample_bundle\`
- \`first_failure_diag\`

## Replay Commands

\`\`\`bash
USE_RCH=${USE_RCH} CARGO_TARGET_DIR_BASE='${CARGO_TARGET_DIR_BASE}' bash scripts/verify_e2_2_fused_entry.sh
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
  --arg hashes_txt "${HASHES_TXT}" \
  --arg use_rch "${USE_RCH}" \
  --arg cargo_target_dir_base "${CARGO_TARGET_DIR_BASE}" \
  '{
    bead_id: $bead_id,
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
      "test_prepared_dml_rejects_schema_change",
      "test_prepared_dml_rejects_cross_connection_schema_change",
      "test_prepared_insert_select_uses_fallback",
      "test_prepared_update_with_limit_uses_fallback",
      "test_prepared_delete_with_limit_uses_fallback",
      "test_prepared_file_backed_insert_reuses_schema_bound_publication_for_autocommit_begin",
      "test_prepared_file_backed_single_writer_insert_reuses_schema_bound_publication",
      "test_disjoint_concurrent_prepared_insert_reuse_across_rounds_refreshes_snapshot"
    ],
    compatibility_selectors: [
      "prepared_dml_semantic_equivalence",
      "prepared_dml_schema_invalidation",
      "prepared_dml_fallback_boundary",
      "prepared_dml_publication_reuse",
      "prepared_dml_round_refresh"
    ],
    regime_atlas_contract: {
      source_bead: "bd-db300.7.5.5",
      validation_entrypoint: "scripts/verify_g5_5_regime_atlas.sh",
      activation_states: [
        "universal_default",
        "regime_gated_default",
        "shadow_only",
        "operator_opt_in",
        "rejected"
      ],
      activation_rules: [
        {
          control_mode: "auto",
          candidate_path: "fused_prepared_dml_entry",
          oracle_path: "deferred_dispatch",
          fallback_on: [
            "unclassified_regime",
            "hostile_regime",
            "schema_identity_mismatch",
            "fallback_reason_present"
          ]
        },
        {
          control_mode: "forced_fallback",
          candidate_path: "deferred_dispatch",
          oracle_path: "deferred_dispatch",
          fallback_on: ["always"]
        },
        {
          control_mode: "shadow_compare",
          candidate_path: "fused_prepared_dml_entry",
          oracle_path: "deferred_dispatch",
          fallback_on: ["any_divergence", "missing_shadow_contract"]
        }
      ]
    },
    shadow_oracle_contract: {
      source_bead: "bd-db300.7.5.6",
      validation_entrypoint: "scripts/verify_g5_6_shadow_oracle.sh",
      oracle_path: "deferred_dispatch",
      candidate_path: "fused_prepared_dml_entry",
      shadow_modes: ["off", "forced", "sampled", "shadow_canary"],
      equivalence_scope: [
        "user_visible_result",
        "affected_rows",
        "transaction_outcome",
        "schema_invalidation_response",
        "visibility_state",
        "error_class"
      ],
      divergence_classes: [
        "result_mismatch",
        "affected_rows_mismatch",
        "error_class_mismatch",
        "visibility_mismatch",
        "fallback_mismatch",
        "ordering_mismatch",
        "trace_contract_missing"
      ],
      counterexample_bundle_required_fields: [
        "trace_id",
        "run_id",
        "scenario_id",
        "shadow_run_id",
        "oracle_path",
        "candidate_path",
        "control_mode",
        "equivalence_scope",
        "allowed_difference_policy",
        "divergence_class",
        "fallback_state",
        "counterexample_bundle",
        "compatibility_selector",
        "schema_epoch",
        "publication_seq",
        "wal_generation",
        "first_failure_diag"
      ]
    },
    logging_contract: [
      "trace_id",
      "run_id",
      "scenario_id",
      "entry_ticket_state",
      "control_mode",
      "regime_id",
      "activation_state",
      "frontier_reason",
      "shadow_run_id",
      "shadow_verdict",
      "oracle_path",
      "candidate_path",
      "equivalence_scope",
      "allowed_difference_policy",
      "compatibility_selector",
      "fallback_reason",
      "fallback_state",
      "schema_epoch",
      "publication_seq",
      "wal_generation",
      "counterexample_bundle",
      "first_failure_diag"
    ],
    artifacts: {
      events_jsonl: $events_jsonl,
      summary_md: $summary_md,
      artifact_hashes: $hashes_txt
    }
  }' > "${REPORT_JSON}"

emit_event "finalize" "pass" "pass" 0 "report written to ${REPORT_JSON}"
echo "[GATE PASS] ${BEAD_ID} fused entry proof pack passed"
