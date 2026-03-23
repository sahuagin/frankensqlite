#!/usr/bin/env bash
# Verification gate for bd-db300.7.5.5:
# regime atlas / activation-frontier contract across E2, E3, and D1.

set -euo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BEAD_ID="bd-db300.7.5.5"
CONTRACT_PATH="${WORKSPACE_ROOT}/db300_regime_atlas_contract.toml"
SCENARIO_ID="${SCENARIO_ID:-G5-5-REGIME-ATLAS}"
SEED="${SEED:-755}"
TIMESTAMP_UTC="$(date -u +%Y%m%dT%H%M%SZ)"
RUN_ID="${RUN_ID:-${BEAD_ID}-${TIMESTAMP_UTC}-${SEED}}"
TRACE_ID="${TRACE_ID:-trace-${RUN_ID}}"
ARTIFACT_DIR="${WORKSPACE_ROOT}/artifacts/${BEAD_ID}/${RUN_ID}"
EVENTS_JSONL="${ARTIFACT_DIR}/events.jsonl"
SUMMARY_MD="${ARTIFACT_DIR}/summary.md"
MANIFEST_JSON="${ARTIFACT_DIR}/regime_atlas_manifest.json"
FRONTIERS_JSON="${ARTIFACT_DIR}/activation_frontiers.json"
HASHES_TXT="${ARTIFACT_DIR}/artifact_hashes.txt"
USE_RCH="${USE_RCH:-0}"
CARGO_TARGET_DIR_BASE="${CARGO_TARGET_DIR_BASE:-${WORKSPACE_ROOT}/.codex-target/g5_5_regime_atlas}"

mkdir -p "${ARTIFACT_DIR}"
: > "${EVENTS_JSONL}"

export NO_COLOR="${NO_COLOR:-1}"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"

emit_event() {
  local phase="$1"
  local event_type="$2"
  local outcome="$3"
  local elapsed_ms="$4"
  local message="$5"
  local surface_id="${6:-atlas_contract}"
  local pillar_id="${7:-G5}"
  local regime_id="${8:-atlas_contract}"
  local activation_state="${9:-shadow_only}"
  local frontier_reason="${10:-contract_render}"
  local breakpoint_metric="${11:-throughput_break_even}"
  local placement_profile="${12:-contract}"
  local topology_class="${13:-contract}"
  local fallback_state="${14:-inactive}"
  local baseline_comparator="${15:-contract_manifest}"

  python3 - "${EVENTS_JSONL}" \
    "${TRACE_ID}" "${SCENARIO_ID}" "${BEAD_ID}" "${phase}" "${event_type}" \
    "${outcome}" "${elapsed_ms}" "${message}" "${surface_id}" "${pillar_id}" \
    "${regime_id}" "${activation_state}" "${frontier_reason}" \
    "${breakpoint_metric}" "${placement_profile}" "${topology_class}" \
    "${fallback_state}" "${baseline_comparator}" <<'PY'
import json
import sys
from datetime import datetime, timezone

path = sys.argv[1]
trace_id, scenario_id, bead_id, phase, event_type, outcome, elapsed_ms, message = sys.argv[2:10]
surface_id, pillar_id, regime_id, activation_state, frontier_reason = sys.argv[10:15]
breakpoint_metric, placement_profile, topology_class = sys.argv[15:18]
fallback_state, baseline_comparator = sys.argv[18:20]

event = {
    "trace_id": trace_id,
    "scenario_id": scenario_id,
    "bead_id": bead_id,
    "phase": phase,
    "event_type": event_type,
    "outcome": outcome,
    "elapsed_ms": int(elapsed_ms),
    "timestamp": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    "message": message,
    "surface_id": surface_id,
    "pillar_id": pillar_id,
    "regime_id": regime_id,
    "activation_state": activation_state,
    "frontier_reason": frontier_reason,
    "breakpoint_metric": breakpoint_metric,
    "placement_profile": placement_profile,
    "topology_class": topology_class,
    "fallback_state": fallback_state,
    "baseline_comparator": baseline_comparator,
}
with open(path, "a", encoding="utf-8") as handle:
    handle.write(json.dumps(event, sort_keys=True) + "\n")
PY
}

run_contract_test() {
  local logfile="${ARTIFACT_DIR}/contract_test.log"
  local started finished elapsed
  local -a cmd=(
    env
    "CARGO_TARGET_DIR=${CARGO_TARGET_DIR_BASE}"
    "CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS}"
    "NO_COLOR=${NO_COLOR}"
    cargo test -p fsqlite-harness --test bd_db300_7_5_5_regime_atlas_contract -- --nocapture
  )
  if [[ "${USE_RCH}" == "1" ]]; then
    cmd=(rch exec -- "${cmd[@]}")
  fi

  emit_event "contract_test" "start" "running" 0 "running regime-atlas contract test"
  started="$(date +%s%3N)"
  if (
    cd "${WORKSPACE_ROOT}"
    "${cmd[@]}"
  ) 2>&1 | tee "${logfile}"; then
    finished="$(date +%s%3N)"
    elapsed="$((finished - started))"
    if ! grep -Eq '^test result: ok\.' "${logfile}"; then
      emit_event "contract_test" "fail" "fail" "${elapsed}" "contract test completed without a passing test result"
      return 1
    fi
    emit_event "contract_test" "pass" "pass" "${elapsed}" "contract test passed"
  else
    finished="$(date +%s%3N)"
    elapsed="$((finished - started))"
    emit_event "contract_test" "fail" "fail" "${elapsed}" "contract test command failed"
    return 1
  fi
}

render_contract_artifacts() {
  emit_event "render" "start" "running" 0 "rendering regime atlas manifest"
  python3 - "${CONTRACT_PATH}" "${MANIFEST_JSON}" "${FRONTIERS_JSON}" "${SUMMARY_MD}" "${TRACE_ID}" "${SCENARIO_ID}" "${RUN_ID}" "${BEAD_ID}" <<'PY'
import json
import sys
import tomllib
from pathlib import Path

contract_path = Path(sys.argv[1])
manifest_path = Path(sys.argv[2])
frontiers_path = Path(sys.argv[3])
summary_path = Path(sys.argv[4])
trace_id = sys.argv[5]
scenario_id = sys.argv[6]
run_id = sys.argv[7]
bead_id = sys.argv[8]

with contract_path.open("rb") as handle:
    document = tomllib.load(handle)

axis_ids = [axis["axis_id"] for axis in document["regime_axis"]]
activation_ids = [state["state_id"] for state in document["activation_state"]]
frontier_ids = [rule["rule_id"] for rule in document["frontier_rule"]]
gap_rule_ids = [rule["gap_id"] for rule in document["gap_conversion_rule"]]

manifest = {
    "trace_id": trace_id,
    "scenario_id": scenario_id,
    "run_id": run_id,
    "bead_id": bead_id,
    "meta": document["meta"],
    "global_defaults": document["global_defaults"],
    "logging": document["logging"],
    "regime_axes": axis_ids,
    "activation_states": activation_ids,
    "frontier_rules": frontier_ids,
    "gap_conversion_rules": gap_rule_ids,
    "surfaces": [
        {
            "surface_id": surface["surface_id"],
            "pillar_id": surface["pillar_id"],
            "baseline_comparator": surface["baseline_comparator"],
            "named_scripts": surface["named_scripts"],
            "supported_activation_states": surface["supported_activation_states"],
            "frontier_rules": surface["frontier_rules"],
            "deterministic_fallback": surface["deterministic_fallback"],
        }
        for surface in document["surface_contract"]
    ],
}

frontiers = {
    "trace_id": trace_id,
    "scenario_id": scenario_id,
    "run_id": run_id,
    "surfaces": [
        {
            "surface_id": surface["surface_id"],
            "pillar_id": surface["pillar_id"],
            "baseline_comparator": surface["baseline_comparator"],
            "frontier_rules": [
                next(rule for rule in document["frontier_rule"] if rule["rule_id"] == frontier_id)
                for frontier_id in surface["frontier_rules"]
            ],
            "fallback": surface["deterministic_fallback"],
            "unit_test_obligations": surface["unit_test_obligations"],
            "e2e_obligations": surface["e2e_obligations"],
        }
        for surface in document["surface_contract"]
    ],
}

manifest_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
frontiers_path.write_text(json.dumps(frontiers, indent=2, sort_keys=True) + "\n", encoding="utf-8")

summary_lines = [
    f"# {bead_id} Regime Atlas Contract",
    "",
    f"- run_id: `{run_id}`",
    f"- trace_id: `{trace_id}`",
    f"- scenario_id: `{scenario_id}`",
    "",
    "## Default Posture",
    "",
    f"- default_activation_state: `{document['global_defaults']['default_activation_state']}`",
    f"- unclassified_regime_action: `{document['global_defaults']['unclassified_regime_action']}`",
    f"- hostile_regime_action: `{document['global_defaults']['hostile_regime_action']}`",
    "",
    "## Regime Axes",
    "",
]
summary_lines.extend(f"- `{axis_id}`" for axis_id in axis_ids)
summary_lines.extend([
    "",
    "## Activation States",
    "",
])
summary_lines.extend(f"- `{state_id}`" for state_id in activation_ids)
summary_lines.extend([
    "",
    "## Frontier Rules",
    "",
])
summary_lines.extend(f"- `{rule_id}`" for rule_id in frontier_ids)
summary_lines.extend([
    "",
    "## Gap Conversion Rules",
    "",
])
summary_lines.extend(f"- `{gap_id}`" for gap_id in gap_rule_ids)
summary_lines.extend([
    "",
    "## Surface Contracts",
    "",
])
for surface in document["surface_contract"]:
    summary_lines.append(
        f"- `{surface['surface_id']}` ({surface['pillar_id']}): baseline=`{surface['baseline_comparator']}`, "
        f"states={','.join(surface['supported_activation_states'])}, "
        f"frontiers={','.join(surface['frontier_rules'])}"
    )
summary_path.write_text("\n".join(summary_lines) + "\n", encoding="utf-8")
PY

  while IFS= read -r surface_line; do
    local surface_id pillar_id baseline_comparator
    surface_id="$(printf '%s' "${surface_line}" | cut -d'|' -f1)"
    pillar_id="$(printf '%s' "${surface_line}" | cut -d'|' -f2)"
    baseline_comparator="$(printf '%s' "${surface_line}" | cut -d'|' -f3)"
    emit_event \
      "render" \
      "surface_manifest" \
      "pass" \
      0 \
      "rendered surface ${surface_id}" \
      "${surface_id}" \
      "${pillar_id}" \
      "atlas_contract.${surface_id}" \
      "shadow_only" \
      "contract_render" \
      "throughput_break_even" \
      "contract" \
      "contract" \
      "inactive" \
      "${baseline_comparator}"
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
        f"{surface['surface_id']}|{surface['pillar_id']}|{surface['baseline_comparator']}"
    )
PY
  )
  emit_event "render" "pass" "pass" 0 "rendered atlas manifest and frontier summary"
}

hash_artifacts() {
  : > "${HASHES_TXT}"
  (
    cd "${ARTIFACT_DIR}"
    sha256sum ./*.json ./*.md ./*.log ./events.jsonl 2>/dev/null
  ) > "${HASHES_TXT}"
}

echo "=== ${BEAD_ID}: regime atlas contract ==="
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
echo "  - ${MANIFEST_JSON}"
echo "  - ${FRONTIERS_JSON}"
echo "  - ${SUMMARY_MD}"
echo "  - ${EVENTS_JSONL}"
echo "  - ${HASHES_TXT}"
