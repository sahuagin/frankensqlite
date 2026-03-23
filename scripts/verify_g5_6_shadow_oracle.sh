#!/usr/bin/env bash
# Verification gate for bd-db300.7.5.6:
# shadow-oracle differential verification / counterexample-capture contract
# across E2, E3, D1, and E4.

set -euo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BEAD_ID="bd-db300.7.5.6"
CONTRACT_PATH="${WORKSPACE_ROOT}/db300_shadow_oracle_contract.toml"
SCENARIO_ID="${SCENARIO_ID:-G5-6-SHADOW-ORACLE}"
SEED="${SEED:-756}"
TIMESTAMP_UTC="$(date -u +%Y%m%dT%H%M%SZ)"
RUN_ID="${RUN_ID:-${BEAD_ID}-${TIMESTAMP_UTC}-${SEED}}"
TRACE_ID="${TRACE_ID:-trace-${RUN_ID}}"
SHADOW_RUN_ID="${SHADOW_RUN_ID:-shadow-${RUN_ID}}"
ARTIFACT_DIR="${WORKSPACE_ROOT}/artifacts/${BEAD_ID}/${RUN_ID}"
STRUCTURED_LOGS_JSONL="${ARTIFACT_DIR}/structured_logs.ndjson"
SUMMARY_MD="${ARTIFACT_DIR}/summary.md"
MANIFEST_JSON="${ARTIFACT_DIR}/shadow_oracle_manifest.json"
COUNTEREXAMPLE_DIR="${ARTIFACT_DIR}/counterexample_bundle"
BUNDLE_SUMMARY_JSON="${COUNTEREXAMPLE_DIR}/bundle_summary.json"
ORACLE_OBS_JSON="${COUNTEREXAMPLE_DIR}/oracle_observation.json"
CANDIDATE_OBS_JSON="${COUNTEREXAMPLE_DIR}/candidate_observation.json"
ARTIFACT_MANIFEST_JSON="${COUNTEREXAMPLE_DIR}/artifact_manifest.json"
REPLAY_SH="${COUNTEREXAMPLE_DIR}/replay.sh"
MIN_REPRO_JSON="${COUNTEREXAMPLE_DIR}/minimal_reproduction.json"
DECISION_RECORD_JSON="${COUNTEREXAMPLE_DIR}/decision_record.json"
STATE_HASH_INPUTS_JSON="${COUNTEREXAMPLE_DIR}/state_hash_inputs.json"
TEST_LOG="${ARTIFACT_DIR}/contract_test.log"
HASHES_TXT="${ARTIFACT_DIR}/artifact_hashes.txt"
USE_RCH="${USE_RCH:-0}"
CARGO_TARGET_DIR_BASE="${CARGO_TARGET_DIR_BASE:-${WORKSPACE_ROOT}/.codex-target/g5_6_shadow_oracle}"

mkdir -p "${ARTIFACT_DIR}" "${COUNTEREXAMPLE_DIR}"
: > "${STRUCTURED_LOGS_JSONL}"

export NO_COLOR="${NO_COLOR:-1}"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"

emit_event() {
  local phase="$1"
  local event_type="$2"
  local outcome="$3"
  local elapsed_ms="$4"
  local message="$5"
  local surface_id="${6:-shadow_oracle_contract}"
  local oracle_path="${7:-contract_oracle}"
  local candidate_path="${8:-contract_candidate}"
  local equivalence_scope="${9:-user_visible_result}"
  local allowed_difference_policy="${10:-exact_identity}"
  local shadow_mode="${11:-forced}"
  local shadow_sample_rate="${12:-100%}"
  local rollout_stage="${13:-shadow_only}"
  local fallback_state="${14:-oracle_authoritative}"
  local kill_switch_state="${15:-disarmed}"
  local counterexample_bundle="${16:-counterexample_bundle/}"
  local divergence_class="${17:-none}"
  local first_failure_summary="${18:-}"
  local first_failure_json_pointer="${19:-}"
  local replay_command="${20:-bash scripts/verify_g5_6_shadow_oracle.sh}"
  local diagnostic_json_pointer="${21:-}"
  local minimal_reproduction_json_pointer="${22:-}"

  python3 - "${STRUCTURED_LOGS_JSONL}" \
    "${TRACE_ID}" "${SCENARIO_ID}" "${SHADOW_RUN_ID}" "${BEAD_ID}" "${phase}" \
    "${event_type}" "${outcome}" "${elapsed_ms}" "${message}" "${surface_id}" \
    "${oracle_path}" "${candidate_path}" "${equivalence_scope}" \
    "${allowed_difference_policy}" "${shadow_mode}" "${shadow_sample_rate}" \
    "${rollout_stage}" "${fallback_state}" "${kill_switch_state}" \
    "${counterexample_bundle}" "${divergence_class}" "${first_failure_summary}" \
    "${first_failure_json_pointer}" "${replay_command}" \
    "${diagnostic_json_pointer}" "${minimal_reproduction_json_pointer}" <<'PY'
import json
import sys
from datetime import datetime, timezone

path = sys.argv[1]
(
    trace_id,
    scenario_id,
    shadow_run_id,
    bead_id,
    phase,
    event_type,
    outcome,
    elapsed_ms,
    message,
    surface_id,
    oracle_path,
    candidate_path,
    equivalence_scope,
    allowed_difference_policy,
    shadow_mode,
    shadow_sample_rate,
    rollout_stage,
    fallback_state,
    kill_switch_state,
    counterexample_bundle,
    divergence_class,
    first_failure_summary,
    first_failure_json_pointer,
    replay_command,
    diagnostic_json_pointer,
    minimal_reproduction_json_pointer,
) = sys.argv[2:28]

event = {
    "trace_id": trace_id,
    "scenario_id": scenario_id,
    "shadow_run_id": shadow_run_id,
    "bead_id": bead_id,
    "phase": phase,
    "event_type": event_type,
    "outcome": outcome,
    "elapsed_ms": int(elapsed_ms),
    "timestamp": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    "message": message,
    "surface_id": surface_id,
    "oracle_path": oracle_path,
    "candidate_path": candidate_path,
    "equivalence_scope": equivalence_scope,
    "allowed_difference_policy": allowed_difference_policy,
    "shadow_mode": shadow_mode,
    "shadow_sample_rate": shadow_sample_rate,
    "rollout_stage": rollout_stage,
    "fallback_state": fallback_state,
    "kill_switch_state": kill_switch_state,
    "counterexample_bundle": counterexample_bundle,
    "divergence_class": divergence_class,
}

if first_failure_summary:
    event["first_failure_summary"] = first_failure_summary
if first_failure_json_pointer:
    event["first_failure_json_pointer"] = first_failure_json_pointer
if replay_command:
    event["replay_command"] = replay_command
if diagnostic_json_pointer:
    event["diagnostic_json_pointer"] = diagnostic_json_pointer
if minimal_reproduction_json_pointer:
    event["minimal_reproduction_json_pointer"] = minimal_reproduction_json_pointer

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
    cargo test -p fsqlite-harness --test bd_db300_7_5_6_shadow_oracle_contract -- --nocapture
  )
  if [[ "${USE_RCH}" == "1" ]]; then
    cmd=(rch exec -- "${cmd[@]}")
  fi

  emit_event "contract_test" "start" "running" 0 "running shadow-oracle contract test"
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
        "shadow_oracle_contract" \
        "contract_oracle" \
        "contract_candidate" \
        "user_visible_result" \
        "exact_identity" \
        "forced" \
        "100%" \
        "shadow_only" \
        "oracle_authoritative" \
        "disarmed" \
        "counterexample_bundle/" \
        "observability_gap" \
        "missing cargo test success signature" \
        "/contract_test.log" \
        "bash scripts/verify_g5_6_shadow_oracle.sh" \
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
      "shadow_oracle_contract" \
      "contract_oracle" \
      "contract_candidate" \
      "user_visible_result" \
      "exact_identity" \
      "forced" \
      "100%" \
      "shadow_only" \
      "oracle_authoritative" \
      "disarmed" \
      "counterexample_bundle/" \
      "observability_gap" \
      "cargo test failed" \
      "/contract_test.log" \
      "bash scripts/verify_g5_6_shadow_oracle.sh" \
      "/contract_test.log"
    return 1
  fi
}

render_contract_artifacts() {
  emit_event "render" "start" "running" 0 "rendering shadow-oracle manifest and counterexample bundle"
  python3 - "${CONTRACT_PATH}" "${MANIFEST_JSON}" "${SUMMARY_MD}" \
    "${BUNDLE_SUMMARY_JSON}" "${ORACLE_OBS_JSON}" "${CANDIDATE_OBS_JSON}" \
    "${ARTIFACT_MANIFEST_JSON}" "${REPLAY_SH}" "${MIN_REPRO_JSON}" \
    "${DECISION_RECORD_JSON}" "${STATE_HASH_INPUTS_JSON}" "${TRACE_ID}" \
    "${SCENARIO_ID}" "${RUN_ID}" "${SHADOW_RUN_ID}" "${BEAD_ID}" <<'PY'
import json
import os
import sys
import tomllib
from pathlib import Path

(
    contract_path_raw,
    manifest_path_raw,
    summary_path_raw,
    bundle_summary_path_raw,
    oracle_obs_path_raw,
    candidate_obs_path_raw,
    artifact_manifest_path_raw,
    replay_path_raw,
    min_repro_path_raw,
    decision_record_path_raw,
    state_hash_inputs_path_raw,
    trace_id,
    scenario_id,
    run_id,
    shadow_run_id,
    bead_id,
) = sys.argv[1:17]

contract_path = Path(contract_path_raw)
manifest_path = Path(manifest_path_raw)
summary_path = Path(summary_path_raw)
bundle_summary_path = Path(bundle_summary_path_raw)
oracle_obs_path = Path(oracle_obs_path_raw)
candidate_obs_path = Path(candidate_obs_path_raw)
artifact_manifest_path = Path(artifact_manifest_path_raw)
replay_path = Path(replay_path_raw)
min_repro_path = Path(min_repro_path_raw)
decision_record_path = Path(decision_record_path_raw)
state_hash_inputs_path = Path(state_hash_inputs_path_raw)

with contract_path.open("rb") as handle:
    document = tomllib.load(handle)

surfaces = document["surface_contract"]
scopes = [scope["scope_id"] for scope in document["equivalence_scope"]]
policies = [policy["policy_id"] for policy in document["allowed_difference_policy"]]
shadow_modes = [mode["mode_id"] for mode in document["shadow_mode"]]
divergence_classes = [entry["divergence_class_id"] for entry in document["divergence_class"]]
kill_switch_profiles = [entry["profile_id"] for entry in document["kill_switch_profile"]]

manifest = {
    "trace_id": trace_id,
    "scenario_id": scenario_id,
    "run_id": run_id,
    "shadow_run_id": shadow_run_id,
    "bead_id": bead_id,
    "meta": document["meta"],
    "global_defaults": document["global_defaults"],
    "logging": document["logging"],
    "equivalence_scopes": scopes,
    "allowed_difference_policies": policies,
    "shadow_modes": shadow_modes,
    "divergence_classes": divergence_classes,
    "kill_switch_profiles": kill_switch_profiles,
    "named_scripts": document["named_script"],
    "surfaces": [
        {
            "surface_id": surface["surface_id"],
            "pillar_id": surface["pillar_id"],
            "oracle_identity": surface["oracle_identity"],
            "candidate_identity": surface["candidate_identity"],
            "equivalence_scopes": surface["equivalence_scopes"],
            "allowed_difference_policies": surface["allowed_difference_policies"],
            "supported_shadow_modes": surface["supported_shadow_modes"],
            "divergence_classes": surface["divergence_classes"],
            "kill_switch_profiles": surface["kill_switch_profiles"],
            "named_scripts": surface["named_scripts"],
            "required_log_fields": surface["required_log_fields"],
        }
        for surface in surfaces
    ],
}
manifest_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")

primary_surface = surfaces[0]
bundle_summary = {
    "bundle_schema": document["counterexample_bundle"]["bundle_schema"],
    "bundle_id": f"{bead_id}-bundle-{run_id}",
    "trace_id": trace_id,
    "scenario_id": scenario_id,
    "shadow_run_id": shadow_run_id,
    "surface_id": primary_surface["surface_id"],
    "oracle_path": primary_surface["oracle_identity"],
    "candidate_path": primary_surface["candidate_identity"],
    "equivalence_scope": "user_visible_result",
    "allowed_difference_policy": "exact_identity",
    "shadow_mode": "forced",
    "rollout_stage": document["global_defaults"]["default_rollout_stage"],
    "divergence_class": "semantic_result_mismatch",
    "kill_switch_state": "immediate_surface_latch",
    "fallback_state": "oracle_authoritative",
    "replay_command": "bash scripts/verify_g5_6_shadow_oracle.sh",
    "diagnostic_json_pointer": "/surfaces/0",
    "artifact_entries": [
        "bundle_summary.json",
        "oracle_observation.json",
        "candidate_observation.json",
        "artifact_manifest.json",
        "structured_logs.ndjson",
        "replay.sh",
        "minimal_reproduction.json",
        "decision_record.json",
        "state_hash_inputs.json",
    ],
    "oracle_observation_hash": "sha256:contract-oracle-placeholder",
    "candidate_observation_hash": "sha256:contract-candidate-placeholder",
    "minimal_reproduction_json_pointer": "/minimal_reproduction",
}
bundle_summary_path.write_text(json.dumps(bundle_summary, indent=2, sort_keys=True) + "\n", encoding="utf-8")

oracle_observation = {
    "surface_id": primary_surface["surface_id"],
    "path_identity": primary_surface["oracle_identity"],
    "equivalence_scopes": primary_surface["equivalence_scopes"],
    "required_log_fields": primary_surface["required_log_fields"],
    "fallback_state": "oracle_authoritative",
}
oracle_obs_path.write_text(json.dumps(oracle_observation, indent=2, sort_keys=True) + "\n", encoding="utf-8")

candidate_observation = {
    "surface_id": primary_surface["surface_id"],
    "path_identity": primary_surface["candidate_identity"],
    "equivalence_scopes": primary_surface["equivalence_scopes"],
    "required_log_fields": primary_surface["required_log_fields"],
    "divergence_class": "semantic_result_mismatch",
    "kill_switch_profile": "immediate_surface_latch",
}
candidate_obs_path.write_text(json.dumps(candidate_observation, indent=2, sort_keys=True) + "\n", encoding="utf-8")

artifact_manifest = {
    "trace_id": trace_id,
    "scenario_id": scenario_id,
    "run_id": run_id,
    "shadow_run_id": shadow_run_id,
    "contract_path": os.fspath(contract_path),
    "manifest_path": os.fspath(manifest_path),
    "counterexample_bundle_root": os.fspath(bundle_summary_path.parent),
    "rendered_files": [
        os.path.basename(bundle_summary_path),
        os.path.basename(oracle_obs_path),
        os.path.basename(candidate_obs_path),
        os.path.basename(artifact_manifest_path),
        os.path.basename(replay_path),
        os.path.basename(min_repro_path),
        os.path.basename(decision_record_path),
        os.path.basename(state_hash_inputs_path),
    ],
}
artifact_manifest_path.write_text(json.dumps(artifact_manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")

minimal_reproduction = {
    "surface_id": primary_surface["surface_id"],
    "shadow_mode": "forced",
    "oracle_path": primary_surface["oracle_identity"],
    "candidate_path": primary_surface["candidate_identity"],
    "selector": primary_surface["named_scripts"][1],
}
min_repro_path.write_text(json.dumps(minimal_reproduction, indent=2, sort_keys=True) + "\n", encoding="utf-8")

decision_record = {
    "policy_id": "contract_shadow_oracle_policy",
    "decision_id": f"{bead_id}-decision-{run_id}",
    "budget_id": "forced_shadow_budget",
    "slo_id": "shadow_consistency_zero_blocker",
    "counterfactual_action": "remain_shadow_only",
    "regret_delta": 0.0,
    "fallback_active": True,
}
decision_record_path.write_text(json.dumps(decision_record, indent=2, sort_keys=True) + "\n", encoding="utf-8")

state_hash_inputs = {
    "page_bytes": "semantic_only",
    "publication_sequence": "required",
    "commit_certificate_bytes": "required_when_present",
    "wal_visibility_state": "required_when_present",
    "excluded_ephemeral_fields": [
        "trace_id",
        "timing_counters",
        "allocation_counters",
        "sampling_counters",
    ],
}
state_hash_inputs_path.write_text(json.dumps(state_hash_inputs, indent=2, sort_keys=True) + "\n", encoding="utf-8")

summary_lines = [
    f"# {bead_id} Shadow-Oracle Contract",
    "",
    f"- run_id: `{run_id}`",
    f"- trace_id: `{trace_id}`",
    f"- shadow_run_id: `{shadow_run_id}`",
    f"- scenario_id: `{scenario_id}`",
    "",
    "## Default Posture",
    "",
    f"- oracle_result_authority: `{document['global_defaults']['oracle_result_authority']}`",
    f"- default_shadow_mode: `{document['global_defaults']['default_shadow_mode']}`",
    f"- default_rollout_stage: `{document['global_defaults']['default_rollout_stage']}`",
    "",
    "## Comparison Vocabulary",
    "",
]
summary_lines.extend(f"- scope: `{scope_id}`" for scope_id in scopes)
summary_lines.extend([
    "",
    "## Allowed-Difference Policies",
    "",
])
summary_lines.extend(f"- policy: `{policy_id}`" for policy_id in policies)
summary_lines.extend([
    "",
    "## Shadow Modes",
    "",
])
summary_lines.extend(f"- mode: `{mode_id}`" for mode_id in shadow_modes)
summary_lines.extend([
    "",
    "## Surface Contracts",
    "",
])
for surface in surfaces:
    summary_lines.append(
        f"- `{surface['surface_id']}` ({surface['pillar_id']}): oracle=`{surface['oracle_identity']}`, "
        f"candidate=`{surface['candidate_identity']}`, modes={','.join(surface['supported_shadow_modes'])}"
    )
summary_path.write_text("\n".join(summary_lines) + "\n", encoding="utf-8")

replay_path.write_text(
    "\n".join(
        [
            "#!/usr/bin/env bash",
            "set -euo pipefail",
            f"WORKSPACE_ROOT=\"{contract_path.parent}\"",
            "cd \"${WORKSPACE_ROOT}\"",
            "bash scripts/verify_g5_6_shadow_oracle.sh",
            "",
        ]
    ),
    encoding="utf-8",
)
replay_path.chmod(0o755)
PY

  while IFS='|' read -r surface_id pillar_id oracle_path candidate_path first_scope first_policy; do
    emit_event \
      "render" \
      "surface_manifest" \
      "pass" \
      0 \
      "rendered surface ${surface_id}" \
      "${surface_id}" \
      "${oracle_path}" \
      "${candidate_path}" \
      "${first_scope}" \
      "${first_policy}" \
      "forced" \
      "100%" \
      "shadow_only" \
      "oracle_authoritative" \
      "disarmed" \
      "counterexample_bundle/" \
      "none"
  done < <(
    python3 - "${CONTRACT_PATH}" <<'PY'
import sys
import tomllib
from pathlib import Path

contract_path = Path(sys.argv[1])
with contract_path.open("rb") as handle:
    document = tomllib.load(handle)
for surface in document["surface_contract"]:
    print(
        "|".join(
            [
                surface["surface_id"],
                surface["pillar_id"],
                surface["oracle_identity"],
                surface["candidate_identity"],
                surface["equivalence_scopes"][0],
                surface["allowed_difference_policies"][0],
            ]
        )
    )
PY
  )
  emit_event "render" "pass" "pass" 0 "rendered manifest and representative counterexample bundle"
}

finalize_bundle_logs() {
  cp "${STRUCTURED_LOGS_JSONL}" "${COUNTEREXAMPLE_DIR}/structured_logs.ndjson"
}

hash_artifacts() {
  : > "${HASHES_TXT}"
  (
    cd "${ARTIFACT_DIR}"
    sha256sum ./*.json ./*.md ./*.log ./*.ndjson ./counterexample_bundle/* 2>/dev/null
  ) > "${HASHES_TXT}"
}

echo "=== ${BEAD_ID}: shadow-oracle contract ==="
echo "run_id=${RUN_ID}"
echo "trace_id=${TRACE_ID}"
echo "shadow_run_id=${SHADOW_RUN_ID}"
echo "scenario_id=${SCENARIO_ID}"
echo "seed=${SEED}"
echo "artifacts=${ARTIFACT_DIR}"

emit_event "bootstrap" "start" "running" 0 "verification started"
run_contract_test
render_contract_artifacts
emit_event "complete" "pass" "pass" 0 "verification completed"
finalize_bundle_logs
hash_artifacts

echo
echo "Artifacts written to ${ARTIFACT_DIR}"
echo "  - ${MANIFEST_JSON}"
echo "  - ${SUMMARY_MD}"
echo "  - ${STRUCTURED_LOGS_JSONL}"
echo "  - ${COUNTEREXAMPLE_DIR}"
echo "  - ${HASHES_TXT}"
