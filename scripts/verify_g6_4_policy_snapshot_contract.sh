#!/usr/bin/env bash
# Verification gate for bd-db300.7.6.4:
# policy-as-data controller artifact and runtime snapshot contract.

set -euo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BEAD_ID="bd-db300.7.6.4"
CONTRACT_PATH="${WORKSPACE_ROOT}/docs/contracts/db300_policy_snapshot_contract.toml"
SCENARIO_ID="${SCENARIO_ID:-G6-4-POLICY-SNAPSHOT}"
SEED="${SEED:-764}"
TIMESTAMP_UTC="$(date -u +%Y%m%dT%H%M%SZ)"
RUN_ID="${RUN_ID:-${BEAD_ID}-${TIMESTAMP_UTC}-${SEED}}"
TRACE_ID="${TRACE_ID:-trace-${RUN_ID}}"
ARTIFACT_DIR="${WORKSPACE_ROOT}/artifacts/${BEAD_ID}/${RUN_ID}"
STRUCTURED_LOGS_JSONL="${ARTIFACT_DIR}/structured_logs.ndjson"
POLICY_ARTIFACT_JSON="${ARTIFACT_DIR}/policy_artifact.json"
RUNTIME_SNAPSHOT_JSON="${ARTIFACT_DIR}/runtime_snapshot.json"
DECISION_RECORD_JSON="${ARTIFACT_DIR}/decision_record.json"
OPERATOR_SCORECARD_JSON="${ARTIFACT_DIR}/operator_scorecard.json"
SUMMARY_MD="${ARTIFACT_DIR}/summary.md"
TEST_LOG="${ARTIFACT_DIR}/contract_test.log"
HASHES_TXT="${ARTIFACT_DIR}/artifact_hashes.txt"
USE_RCH="${USE_RCH:-0}"
CARGO_TARGET_DIR_BASE="${CARGO_TARGET_DIR_BASE:-${WORKSPACE_ROOT}/.codex-target/g6_4_policy_snapshot}"

mkdir -p "${ARTIFACT_DIR}"
: > "${STRUCTURED_LOGS_JSONL}"

export NO_COLOR="${NO_COLOR:-1}"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"

emit_event() {
  local phase="$1"
  local event_type="$2"
  local outcome="$3"
  local elapsed_ms="$4"
  local message="$5"
  local policy_id="${6:-db300_policy_snapshot_contract_v1}"
  local decision_id="${7:-contract_decision}"
  local rollout_stage="${8:-shadow_only}"
  local activation_regime_id="${9:-e2_fused_entry.file_backed.unknown_topology}"
  local activation_state="${10:-shadow_only}"
  local control_mode="${11:-auto}"
  local budget_id="${12:-shadow_consistency_zero_blocker}"
  local controller_family="${13:-contract_policy_surface}"
  local controller_calibration="${14:-static_contract}"
  local shadow_sample_rate="${15:-100%}"
  local kill_switch_state="${16:-disarmed}"
  local fallback_active="${17:-true}"
  local fallback_reason="${18:-unclassified_or_hostile_regime}"
  local last_action="${19:-remain_shadow_only}"
  local expected_loss="${20:-0.0}"
  local counterfactual_action="${21:-promote_to_canary}"
  local regret_delta="${22:-0.0}"
  local safe_by_default_boundary="${23:-Only universal_default or regime_gated_default may auto-enable.}"
  local first_failure_summary="${24:-}"
  local first_failure_artifact="${25:-}"
  local replay_command="${26:-bash scripts/verify_g6_4_policy_snapshot_contract.sh}"
  local diagnostic_json_pointer="${27:-}"

  python3 - "${STRUCTURED_LOGS_JSONL}" \
    "${TRACE_ID}" "${SCENARIO_ID}" "${policy_id}" "${decision_id}" \
    "${phase}" "${event_type}" "${outcome}" "${elapsed_ms}" "${message}" \
    "${rollout_stage}" "${activation_regime_id}" "${activation_state}" \
    "${control_mode}" "${budget_id}" "${controller_family}" \
    "${controller_calibration}" "${shadow_sample_rate}" "${kill_switch_state}" \
    "${fallback_active}" "${fallback_reason}" "${last_action}" \
    "${expected_loss}" "${counterfactual_action}" "${regret_delta}" \
    "${safe_by_default_boundary}" "${first_failure_summary}" \
    "${first_failure_artifact}" "${replay_command}" \
    "${diagnostic_json_pointer}" <<'PY'
import json
import sys
from datetime import datetime, timezone

path = sys.argv[1]
(
    trace_id,
    scenario_id,
    policy_id,
    decision_id,
    phase,
    event_type,
    outcome,
    elapsed_ms,
    message,
    rollout_stage,
    activation_regime_id,
    activation_state,
    control_mode,
    budget_id,
    controller_family,
    controller_calibration,
    shadow_sample_rate,
    kill_switch_state,
    fallback_active,
    fallback_reason,
    last_action,
    expected_loss,
    counterfactual_action,
    regret_delta,
    safe_by_default_boundary,
    first_failure_summary,
    first_failure_artifact,
    replay_command,
    diagnostic_json_pointer,
) = sys.argv[2:31]

event = {
    "trace_id": trace_id,
    "scenario_id": scenario_id,
    "policy_id": policy_id,
    "decision_id": decision_id,
    "phase": phase,
    "event_type": event_type,
    "outcome": outcome,
    "elapsed_ms": int(elapsed_ms),
    "timestamp": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    "message": message,
    "rollout_stage": rollout_stage,
    "activation_regime_id": activation_regime_id,
    "activation_state": activation_state,
    "control_mode": control_mode,
    "budget_id": budget_id,
    "controller_family": controller_family,
    "controller_calibration": controller_calibration,
    "shadow_sample_rate": shadow_sample_rate,
    "kill_switch_state": kill_switch_state,
    "fallback_active": fallback_active == "true",
    "fallback_reason": fallback_reason,
    "last_action": last_action,
    "expected_loss": float(expected_loss),
    "counterfactual_action": counterfactual_action,
    "regret_delta": float(regret_delta),
    "safe_by_default_boundary": safe_by_default_boundary,
}

if first_failure_summary:
    event["first_failure_summary"] = first_failure_summary
if first_failure_artifact:
    event["first_failure_artifact"] = first_failure_artifact
if replay_command:
    event["replay_command"] = replay_command
if diagnostic_json_pointer:
    event["diagnostic_json_pointer"] = diagnostic_json_pointer

with open(path, "a", encoding="utf-8") as handle:
    handle.write(json.dumps(event, sort_keys=True) + "\n")
PY
}

run_contract_test() {
  local started finished elapsed
  local -a cmd=(
    env
    "CARGO_TARGET_DIR=${CARGO_TARGET_DIR_BASE}"
    "CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS}"
    "NO_COLOR=${NO_COLOR}"
    cargo test -p fsqlite-harness --test bd_db300_7_6_4_policy_snapshot_contract -- --nocapture
  )
  if [[ "${USE_RCH}" == "1" ]]; then
    cmd=(rch exec -- "${cmd[@]}")
  fi

  emit_event "contract_test" "start" "running" 0 "running policy snapshot contract test"
  started="$(date +%s%3N)"
  if (
    cd "${WORKSPACE_ROOT}"
    "${cmd[@]}"
  ) 2>&1 | tee "${TEST_LOG}"; then
    finished="$(date +%s%3N)"
    elapsed="$((finished - started))"
    if ! grep -Eq '^test result: ok\.' "${TEST_LOG}"; then
      emit_event \
        "contract_test" \
        "fail" \
        "fail" \
        "${elapsed}" \
        "contract test completed without a passing test result" \
        "db300_policy_snapshot_contract_v1" \
        "${BEAD_ID}-decision-${RUN_ID}" \
        "fallback_only" \
        "unknown" \
        "shadow_only" \
        "forced_fallback" \
        "shadow_consistency_zero_blocker" \
        "contract_policy_surface" \
        "static_contract" \
        "100%" \
        "tripped" \
        "true" \
        "contract_test_missing_success" \
        "fallback_only" \
        "0.0" \
        "promote_to_canary" \
        "0.0" \
        "Only universal_default or regime_gated_default may auto-enable." \
        "missing cargo test success signature" \
        "/contract_test.log" \
        "bash scripts/verify_g6_4_policy_snapshot_contract.sh" \
        "/contract_test.log"
      return 1
    fi
    emit_event "contract_test" "pass" "pass" "${elapsed}" "contract test passed"
  else
    finished="$(date +%s%3N)"
    elapsed="$((finished - started))"
    emit_event \
      "contract_test" \
      "fail" \
      "fail" \
      "${elapsed}" \
      "contract test command failed" \
      "db300_policy_snapshot_contract_v1" \
      "${BEAD_ID}-decision-${RUN_ID}" \
      "fallback_only" \
      "unknown" \
      "shadow_only" \
      "forced_fallback" \
      "shadow_consistency_zero_blocker" \
      "contract_policy_surface" \
      "static_contract" \
      "100%" \
      "tripped" \
      "true" \
      "cargo_test_failed" \
      "fallback_only" \
      "0.0" \
      "promote_to_canary" \
      "0.0" \
      "Only universal_default or regime_gated_default may auto-enable." \
      "cargo test failed" \
      "/contract_test.log" \
      "bash scripts/verify_g6_4_policy_snapshot_contract.sh" \
      "/contract_test.log"
    return 1
  fi
}

render_contract_artifacts() {
  emit_event "render" "start" "running" 0 "rendering policy artifact, runtime snapshot, decision record, and operator scorecard"
  python3 - "${CONTRACT_PATH}" "${POLICY_ARTIFACT_JSON}" \
    "${RUNTIME_SNAPSHOT_JSON}" "${DECISION_RECORD_JSON}" \
    "${OPERATOR_SCORECARD_JSON}" "${SUMMARY_MD}" "${TRACE_ID}" \
    "${SCENARIO_ID}" "${RUN_ID}" "${BEAD_ID}" <<'PY'
import json
import sys
import tomllib
from pathlib import Path

(
    contract_path_raw,
    policy_artifact_path_raw,
    runtime_snapshot_path_raw,
    decision_record_path_raw,
    operator_scorecard_path_raw,
    summary_path_raw,
    trace_id,
    scenario_id,
    run_id,
    bead_id,
) = sys.argv[1:11]

contract_path = Path(contract_path_raw)
policy_artifact_path = Path(policy_artifact_path_raw)
runtime_snapshot_path = Path(runtime_snapshot_path_raw)
decision_record_path = Path(decision_record_path_raw)
operator_scorecard_path = Path(operator_scorecard_path_raw)
summary_path = Path(summary_path_raw)

with contract_path.open("rb") as handle:
    document = tomllib.load(handle)

policy_id = "db300_policy_snapshot_contract_v1"
decision_id = f"{bead_id}-decision-{run_id}"
activation_regime_id = "e2_fused_entry.file_backed.unknown_topology"
activation_state = "shadow_only"
safe_by_default_boundary = document["global_defaults"]["safe_by_default_rule"]
rollout_stage = document["global_defaults"]["default_rollout_stage"]
control_mode = document["global_defaults"]["default_control_mode"]
shadow_sample_rate = document["global_defaults"]["default_shadow_sample_rate"]
kill_switch_state = document["global_defaults"]["default_kill_switch_state"]
fallback_policy = document["global_defaults"]["default_fallback_policy"]
baseline_comparator = "prepared_dml_conservative_dispatch"
evidence_id = "g6_4-contract-evidence"
safety_certificate_id = "pending_shadow_and_composition_clearance"

policy_artifact = {
    "policy_id": policy_id,
    "controller_family": "contract_policy_surface",
    "policy_version": document["meta"]["schema_version"],
    "rollout_stage": rollout_stage,
    "control_mode": control_mode,
    "activation_regime_id": activation_regime_id,
    "activation_state": activation_state,
    "safe_by_default_boundary": safe_by_default_boundary,
    "budget_id": "shadow_consistency_zero_blocker",
    "baseline_comparator": baseline_comparator,
    "fallback_policy": fallback_policy,
    "shadow_sample_rate": shadow_sample_rate,
    "kill_switch_state": kill_switch_state,
    "evidence_id": evidence_id,
    "trace_id": trace_id,
    "safety_certificate_id": safety_certificate_id,
    "provenance": {
        "artifact_graph_rule": document["global_defaults"]["artifact_graph_rule"],
        "activation_contract_bead": "bd-db300.7.5.5",
        "shadow_contract_bead": "bd-db300.7.5.6",
    },
}
policy_artifact_path.write_text(
    json.dumps(policy_artifact, indent=2, sort_keys=True) + "\n",
    encoding="utf-8",
)

runtime_snapshot = {
    "policy_id": policy_id,
    "snapshot_schema_version": document["runtime_snapshot_contract"]["schema_id"],
    "decision_count": 1,
    "fallback_active": True,
    "last_action": "remain_shadow_only",
    "expected_loss": 0.0,
    "counterfactual_action": "promote_to_canary",
    "regret_delta": 0.0,
    "evidence_root": {
        "evidence_id": evidence_id,
        "trace_id": trace_id,
    },
    "activation_regime_id": activation_regime_id,
    "activation_state": activation_state,
    "rollout_stage": rollout_stage,
    "control_mode": control_mode,
    "shadow_sample_rate": shadow_sample_rate,
    "kill_switch_state": kill_switch_state,
    "safe_by_default_boundary": safe_by_default_boundary,
}
runtime_snapshot_path.write_text(
    json.dumps(runtime_snapshot, indent=2, sort_keys=True) + "\n",
    encoding="utf-8",
)

decision_record = {
    "policy_id": policy_id,
    "decision_id": decision_id,
    "controller_family": "contract_policy_surface",
    "rollout_stage": rollout_stage,
    "control_mode": control_mode,
    "activation_regime_id": activation_regime_id,
    "activation_state": activation_state,
    "safe_by_default_boundary": safe_by_default_boundary,
    "baseline_comparator": baseline_comparator,
    "fallback_active": True,
    "fallback_reason": "unclassified_or_hostile_regime",
    "fallback_annotation": "Conservative fallback remains authoritative until the regime-atlas boundary and shadow-oracle gates both clear.",
    "shadow_sample_rate": shadow_sample_rate,
    "kill_switch_state": kill_switch_state,
    "expected_loss": 0.0,
    "counterfactual_action": "promote_to_canary",
    "regret_delta": 0.0,
    "evidence_id": evidence_id,
    "trace_id": trace_id,
    "safety_certificate_id": safety_certificate_id,
}
decision_record_path.write_text(
    json.dumps(decision_record, indent=2, sort_keys=True) + "\n",
    encoding="utf-8",
)

operator_scorecard = {
    "scorecard_id": f"{bead_id}-scorecard-{run_id}",
    "policy_id": policy_id,
    "decision_id": decision_id,
    "activation_regime_id": activation_regime_id,
    "activation_state": activation_state,
    "safe_by_default_boundary": safe_by_default_boundary,
    "rollout_stage": rollout_stage,
    "rollout_annotation": "Remain shadow_only until the regime-safe default boundary and shadow-oracle evidence allow canary promotion.",
    "fallback_annotation": "Out-of-regime or unclassified cells remain on the conservative baseline with explicit operator-visible fallback.",
    "shadow_sample_rate": shadow_sample_rate,
    "kill_switch_state": kill_switch_state,
    "user_visibility": "internal_or_operator_only",
    "baseline_comparator": baseline_comparator,
    "safety_certificate_id": safety_certificate_id,
}
operator_scorecard_path.write_text(
    json.dumps(operator_scorecard, indent=2, sort_keys=True) + "\n",
    encoding="utf-8",
)

summary_lines = [
    f"# {bead_id} Policy Snapshot Contract",
    "",
    f"- run_id: `{run_id}`",
    f"- trace_id: `{trace_id}`",
    f"- scenario_id: `{scenario_id}`",
    "",
    "## Default Posture",
    "",
    f"- default_rollout_stage: `{document['global_defaults']['default_rollout_stage']}`",
    f"- default_control_mode: `{document['global_defaults']['default_control_mode']}`",
    f"- default_shadow_sample_rate: `{document['global_defaults']['default_shadow_sample_rate']}`",
    f"- default_kill_switch_state: `{document['global_defaults']['default_kill_switch_state']}`",
    "",
    "## Decision Surface",
    "",
    f"- activation_regime_id: `{activation_regime_id}`",
    f"- activation_state: `{activation_state}`",
    f"- safe_by_default_boundary: `{safe_by_default_boundary}`",
    f"- rollout_stage: `{rollout_stage}`",
    f"- fallback_annotation: `{decision_record['fallback_annotation']}`",
    "",
    "## Operator Scorecard",
    "",
    f"- user_visibility: `{operator_scorecard['user_visibility']}`",
    f"- rollout_annotation: `{operator_scorecard['rollout_annotation']}`",
    f"- kill_switch_state: `{kill_switch_state}`",
    f"- shadow_sample_rate: `{shadow_sample_rate}`",
]
summary_path.write_text("\n".join(summary_lines) + "\n", encoding="utf-8")
PY

  emit_event \
    "render" \
    "policy_artifact" \
    "pass" \
    0 \
    "rendered policy_artifact.json"
  emit_event \
    "render" \
    "runtime_snapshot" \
    "pass" \
    0 \
    "rendered runtime_snapshot.json"
  emit_event \
    "render" \
    "decision_record" \
    "pass" \
    0 \
    "rendered decision_record.json"
  emit_event \
    "render" \
    "operator_scorecard" \
    "pass" \
    0 \
    "rendered operator_scorecard.json"
}

hash_artifacts() {
  : > "${HASHES_TXT}"
  (
    cd "${ARTIFACT_DIR}"
    sha256sum ./*.json ./*.md ./*.log ./structured_logs.ndjson 2>/dev/null
  ) > "${HASHES_TXT}"
}

echo "=== ${BEAD_ID}: policy snapshot contract ==="
echo "run_id=${RUN_ID}"
echo "trace_id=${TRACE_ID}"
echo "scenario_id=${SCENARIO_ID}"
echo "seed=${SEED}"
echo "artifacts=${ARTIFACT_DIR}"

emit_event "bootstrap" "start" "running" 0 "verification started"
run_contract_test
render_contract_artifacts
hash_artifacts
emit_event "complete" "pass" "pass" 0 "verification completed"

echo
echo "Artifacts written to ${ARTIFACT_DIR}"
echo "  - ${POLICY_ARTIFACT_JSON}"
echo "  - ${RUNTIME_SNAPSHOT_JSON}"
echo "  - ${DECISION_RECORD_JSON}"
echo "  - ${OPERATOR_SCORECARD_JSON}"
echo "  - ${SUMMARY_MD}"
echo "  - ${STRUCTURED_LOGS_JSONL}"
echo "  - ${HASHES_TXT}"
