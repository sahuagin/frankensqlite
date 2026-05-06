#!/usr/bin/env bash
# Verification gate for bd-db300.7.6.2:
# unit, e2e, perf, failure-path, and decision-plane structured-log emission map.

set -euo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BEAD_ID="bd-db300.7.6.2"
CONTRACT_PATH="${WORKSPACE_ROOT}/docs/contracts/db300_log_emission_map.toml"
SCENARIO_ID="${SCENARIO_ID:-G6-2-LOG-EMISSION-MAP}"
SEED="${SEED:-762}"
TIMESTAMP_UTC="$(date -u +%Y%m%dT%H%M%SZ)"
RUN_ID="${RUN_ID:-${BEAD_ID}-${TIMESTAMP_UTC}-${SEED}}"
TRACE_ID="${TRACE_ID:-trace-${RUN_ID}}"
ARTIFACT_DIR="${WORKSPACE_ROOT}/artifacts/${BEAD_ID}/${RUN_ID}"
EVENTS_JSONL="${ARTIFACT_DIR}/events.jsonl"
SUMMARY_MD="${ARTIFACT_DIR}/summary.md"
MANIFEST_JSON="${ARTIFACT_DIR}/emission_map_manifest.json"
LEDGER_JSON="${ARTIFACT_DIR}/emission_gap_ledger.json"
HASHES_TXT="${ARTIFACT_DIR}/artifact_hashes.txt"
USE_RCH="${USE_RCH:-0}"
SKIP_CONTRACT_TEST="${SKIP_CONTRACT_TEST:-0}"
CARGO_TARGET_DIR_BASE="${CARGO_TARGET_DIR_BASE:-${WORKSPACE_ROOT}/.codex-target/g6_2_log_emission_map}"

mkdir -p "${ARTIFACT_DIR}"
: > "${EVENTS_JSONL}"

export NO_COLOR="${NO_COLOR:-1}"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"

emit_lifecycle_event() {
  local phase="$1"
  local event_type="$2"
  local outcome="$3"
  local elapsed_ms="$4"
  local message="$5"

  python3 - "${EVENTS_JSONL}" \
    "${TRACE_ID}" "${SCENARIO_ID}" "${BEAD_ID}" "${RUN_ID}" "${phase}" \
    "${event_type}" "${outcome}" "${elapsed_ms}" "${message}" <<'PY'
import json
import sys
from datetime import datetime, timezone

path = sys.argv[1]
(
    trace_id,
    scenario_id,
    bead_id,
    run_id,
    phase,
    event_type,
    outcome,
    elapsed_ms,
    message,
) = sys.argv[2:11]

event = {
    "artifact_manifest_key": "emission_map_contract",
    "bead_id": bead_id,
    "diagnostic_json_pointer": None,
    "elapsed_ms": int(elapsed_ms),
    "emitter_family": "operator_entrypoint",
    "entrypoint_name": "scripts/verify_g6_2_log_emission_map.sh",
    "event_type": event_type,
    "first_failure_artifact": "not_triggered",
    "first_failure_stage": "not_triggered",
    "first_failure_summary": "not_triggered",
    "message": message,
    "missing_field_count": 0,
    "outcome": outcome,
    "phase": phase,
    "required_event_family": "verification_bundle_summary",
    "required_field_count": 0,
    "run_id": run_id,
    "scenario_id": scenario_id,
    "timestamp": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    "trace_id": trace_id,
    "unexpected_field_count": 0,
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
    cargo test -p fsqlite-harness --test bd_db300_7_6_2_log_emission_map -- --nocapture
  )
  if [[ "${USE_RCH}" == "1" ]]; then
    cmd=(rch exec -- "${cmd[@]}")
  fi

  emit_lifecycle_event "contract_test" "start" "running" 0 "running log-emission-map contract test"
  started="$(date +%s%3N)"
  if (
    cd "${WORKSPACE_ROOT}"
    "${cmd[@]}"
  ) 2>&1 | tee "${logfile}"; then
    finished="$(date +%s%3N)"
    elapsed="$((finished - started))"
    if ! grep -Eq '^test result: ok\.' "${logfile}"; then
      emit_lifecycle_event "contract_test" "fail" "fail" "${elapsed}" "contract test completed without a passing test result"
      return 1
    fi
    emit_lifecycle_event "contract_test" "pass" "pass" "${elapsed}" "contract test passed"
  else
    finished="$(date +%s%3N)"
    elapsed="$((finished - started))"
    emit_lifecycle_event "contract_test" "fail" "fail" "${elapsed}" "contract test command failed"
    return 1
  fi
}

render_contract_artifacts() {
  emit_lifecycle_event "render" "start" "running" 0 "rendering emission map artifacts"
  python3 - "${CONTRACT_PATH}" "${MANIFEST_JSON}" "${LEDGER_JSON}" "${SUMMARY_MD}" "${EVENTS_JSONL}" "${TRACE_ID}" "${SCENARIO_ID}" "${RUN_ID}" "${BEAD_ID}" <<'PY'
import json
import sys
import tomllib
from datetime import datetime, timezone
from pathlib import Path

contract_path = Path(sys.argv[1])
manifest_path = Path(sys.argv[2])
ledger_path = Path(sys.argv[3])
summary_path = Path(sys.argv[4])
events_path = Path(sys.argv[5])
trace_id = sys.argv[6]
scenario_id = sys.argv[7]
run_id = sys.argv[8]
bead_id = sys.argv[9]

with contract_path.open("rb") as handle:
    document = tomllib.load(handle)

coverage_fields = document["coverage_log_fields"]["required_fields"]
linkage_fields = document["artifact_linkage_fields"]["required_fields"]
emitters = document.get("emitter_family", [])

manifest = {
    "trace_id": trace_id,
    "scenario_id": scenario_id,
    "run_id": run_id,
    "bead_id": bead_id,
    "meta": document["meta"],
    "global_defaults": document["global_defaults"],
    "coverage_log_fields": coverage_fields,
    "artifact_linkage_fields": linkage_fields,
    "surface_classes": document["surface_class_policy"]["required_classes"],
    "emitter_family_ids": [row["emitter_family_id"] for row in emitters],
    "emitter_count": len(emitters),
}
manifest_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")

ledger_rows = []
coverage_events = []
timestamp = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
for emitter in emitters:
    min_field_count = len(emitter["minimum_required_fields"])
    ledger_rows.append(
        {
            "emitter_family": emitter["emitter_family_id"],
            "surface_class": emitter["surface_class"],
            "entrypoint_name": emitter["entrypoint_name"],
            "artifact_manifest_key": emitter["artifact_manifest_key"],
            "bundle_kind": emitter["bundle_kind"],
            "required_event_families": emitter["required_event_families"],
            "minimum_required_fields": emitter["minimum_required_fields"],
            "mandatory_when": emitter["mandatory_when"],
            "expected_artifacts": emitter["expected_artifacts"],
            "negative_path_expectation": emitter["negative_path_expectation"],
            "gap_conversion_rule": emitter["gap_conversion_rule"],
        }
    )
    for required_event_family in emitter["required_event_families"]:
        coverage_events.append(
            {
                "artifact_manifest_key": emitter["artifact_manifest_key"],
                "bead_id": bead_id,
                "bundle_kind": emitter["bundle_kind"],
                "diagnostic_json_pointer": None,
                "emitter_family": emitter["emitter_family_id"],
                "entrypoint_name": emitter["entrypoint_name"],
                "event_type": "emitter_coverage",
                "first_failure_artifact": "not_triggered",
                "first_failure_stage": "not_triggered",
                "first_failure_summary": "not_triggered",
                "message": f"{emitter['emitter_family_id']} requires {required_event_family}",
                "missing_field_count": 0,
                "outcome": "pass",
                "phase": "coverage",
                "required_event_family": required_event_family,
                "required_field_count": min_field_count,
                "run_id": run_id,
                "scenario_id": scenario_id,
                "timestamp": timestamp,
                "trace_id": trace_id,
                "unexpected_field_count": 0,
            }
        )

ledger = {
    "trace_id": trace_id,
    "scenario_id": scenario_id,
    "run_id": run_id,
    "bead_id": bead_id,
    "emitter_rows": ledger_rows,
}
ledger_path.write_text(json.dumps(ledger, indent=2, sort_keys=True) + "\n", encoding="utf-8")

with events_path.open("a", encoding="utf-8") as handle:
    for event in coverage_events:
        handle.write(json.dumps(event, sort_keys=True) + "\n")

summary_lines = [
    f"# {bead_id} Log Emission Map",
    "",
    f"- run_id: `{run_id}`",
    f"- trace_id: `{trace_id}`",
    f"- scenario_id: `{scenario_id}`",
    f"- emitter_families: `{len(emitters)}`",
    "",
    "## Surface Classes",
    "",
]
for surface_class in document["surface_class_policy"]["required_classes"]:
    summary_lines.append(f"- `{surface_class}`")
summary_lines.extend([
    "",
    "## Emitter Families",
    "",
])
for emitter in emitters:
    summary_lines.append(f"### `{emitter['emitter_family_id']}`")
    summary_lines.append("")
    summary_lines.append(f"- surface_class: `{emitter['surface_class']}`")
    summary_lines.append(f"- entrypoint: `{emitter['entrypoint_name']}`")
    summary_lines.append(f"- artifact_manifest_key: `{emitter['artifact_manifest_key']}`")
    summary_lines.append(f"- bundle_kind: `{emitter['bundle_kind']}`")
    summary_lines.append(f"- mode_scope: `{', '.join(emitter['mode_scope'])}`")
    summary_lines.append(
        f"- required_event_families: `{', '.join(emitter['required_event_families'])}`"
    )
    summary_lines.append(
        f"- minimum_required_fields: `{', '.join(emitter['minimum_required_fields'])}`"
    )
    summary_lines.append(f"- expected_artifacts: `{', '.join(emitter['expected_artifacts'])}`")
    summary_lines.append(f"- negative_path_expectation: {emitter['negative_path_expectation']}")
    summary_lines.append(f"- gap_conversion_rule: {emitter['gap_conversion_rule']}")
    summary_lines.append("- mandatory_when:")
    for rule in emitter["mandatory_when"]:
        summary_lines.append(f"  - {rule}")
    summary_lines.append("")
summary_path.write_text("\n".join(summary_lines) + "\n", encoding="utf-8")
PY
  emit_lifecycle_event "render" "pass" "pass" 0 "rendered emission map artifacts"
}

hash_artifacts() {
  local files=(
    "$(basename "${EVENTS_JSONL}")"
    "$(basename "${MANIFEST_JSON}")"
    "$(basename "${LEDGER_JSON}")"
    "$(basename "${SUMMARY_MD}")"
  )
  if [[ -f "${ARTIFACT_DIR}/contract_test.log" ]]; then
    files+=(contract_test.log)
  fi
  (
    cd "${ARTIFACT_DIR}"
    sha256sum "${files[@]}" > "${HASHES_TXT##${ARTIFACT_DIR}/}"
  )
  emit_lifecycle_event "hash" "pass" "pass" 0 "hashed rendered artifacts"
}

main() {
  if [[ ! -f "${CONTRACT_PATH}" ]]; then
    echo "missing contract: ${CONTRACT_PATH}" >&2
    exit 1
  fi

  if [[ "${SKIP_CONTRACT_TEST}" != "1" ]]; then
    run_contract_test
  else
    emit_lifecycle_event "contract_test" "skip" "skipped" 0 "skipping contract test because SKIP_CONTRACT_TEST=1"
  fi

  render_contract_artifacts
  hash_artifacts

  cat <<EOF
bd-db300.7.6.2 artifacts ready:
  artifact_dir: ${ARTIFACT_DIR}
  summary: ${SUMMARY_MD}
  manifest: ${MANIFEST_JSON}
  ledger: ${LEDGER_JSON}
  events: ${EVENTS_JSONL}
  hashes: ${HASHES_TXT}
EOF
}

main "$@"
