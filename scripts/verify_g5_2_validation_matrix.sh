#!/usr/bin/env bash
# Verification gate for bd-db300.7.5.2:
# crash, fault, interference, and canonical/e2e obligation ledger for the
# 2026-03-23 benchmark-gap overlay.

set -euo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BEAD_ID="bd-db300.7.5.2"
CONTRACT_PATH="${WORKSPACE_ROOT}/docs/contracts/db300_validation_matrix.toml"
SCENARIO_ID="${SCENARIO_ID:-G5-2-VALIDATION-MATRIX}"
SEED="${SEED:-752}"
TIMESTAMP_UTC="$(date -u +%Y%m%dT%H%M%SZ)"
RUN_ID="${RUN_ID:-${BEAD_ID}-${TIMESTAMP_UTC}-${SEED}}"
TRACE_ID="${TRACE_ID:-trace-${RUN_ID}}"
ARTIFACT_DIR="${WORKSPACE_ROOT}/artifacts/${BEAD_ID}/${RUN_ID}"
EVENTS_JSONL="${ARTIFACT_DIR}/events.jsonl"
SUMMARY_MD="${ARTIFACT_DIR}/summary.md"
MANIFEST_JSON="${ARTIFACT_DIR}/validation_matrix_manifest.json"
LEDGER_JSON="${ARTIFACT_DIR}/validation_obligation_ledger.json"
HASHES_TXT="${ARTIFACT_DIR}/artifact_hashes.txt"
USE_RCH="${USE_RCH:-0}"
SKIP_CONTRACT_TEST="${SKIP_CONTRACT_TEST:-0}"
CARGO_TARGET_DIR_BASE="${CARGO_TARGET_DIR_BASE:-${WORKSPACE_ROOT}/.codex-target/g5_2_validation_matrix}"

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
  local validation_class="${6:-matrix_contract}"
  local entrypoint="${7:-scripts/verify_g5_2_validation_matrix.sh}"
  local placement_profile="${8:-contract}"
  local workload_row="${9:-contract}"
  local failure_mode="${10:-contract_render}"
  local log_family="${11:-validation_matrix}"
  local seed_policy="${12:-fixed_seed_42}"
  local stop_on_failure="${13:-true}"

  python3 - "${EVENTS_JSONL}" \
    "${TRACE_ID}" "${SCENARIO_ID}" "${BEAD_ID}" "${phase}" "${event_type}" \
    "${outcome}" "${elapsed_ms}" "${message}" "${validation_class}" \
    "${entrypoint}" "${placement_profile}" "${workload_row}" "${failure_mode}" \
    "${log_family}" "${seed_policy}" "${stop_on_failure}" <<'PY'
import json
import sys
from datetime import datetime, timezone

path = sys.argv[1]
(
    trace_id,
    scenario_id,
    bead_id,
    phase,
    event_type,
    outcome,
    elapsed_ms,
    message,
    validation_class,
    entrypoint,
    placement_profile,
    workload_row,
    failure_mode,
    log_family,
    seed_policy,
    stop_on_failure,
) = sys.argv[2:18]

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
    "validation_class": validation_class,
    "entrypoint": entrypoint,
    "placement_profile": placement_profile,
    "workload_row": workload_row,
    "failure_mode": failure_mode,
    "log_family": log_family,
    "seed_policy": seed_policy,
    "stop_on_failure": stop_on_failure == "true",
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
    cargo test -p fsqlite-harness --test bd_db300_7_5_2_validation_matrix -- --nocapture
  )
  if [[ "${USE_RCH}" == "1" ]]; then
    cmd=(rch exec -- "${cmd[@]}")
  fi

  emit_event "contract_test" "start" "running" 0 "running validation-matrix contract test"
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
  emit_event "render" "start" "running" 0 "rendering validation obligation ledger"
  python3 - "${CONTRACT_PATH}" "${MANIFEST_JSON}" "${LEDGER_JSON}" "${SUMMARY_MD}" "${TRACE_ID}" "${SCENARIO_ID}" "${RUN_ID}" "${BEAD_ID}" <<'PY'
import json
import sys
import tomllib
from collections import defaultdict
from pathlib import Path

contract_path = Path(sys.argv[1])
manifest_path = Path(sys.argv[2])
ledger_path = Path(sys.argv[3])
summary_path = Path(sys.argv[4])
trace_id = sys.argv[5]
scenario_id = sys.argv[6]
run_id = sys.argv[7]
bead_id = sys.argv[8]

with contract_path.open("rb") as handle:
    document = tomllib.load(handle)

rows = document.get("obligation_row", [])
excluded = document.get("excluded_bead", [])
validation_classes = document.get("validation_class", [])

rows_by_bead = defaultdict(list)
for row in rows:
    rows_by_bead[row["bead_id"]].append(row)

manifest = {
    "trace_id": trace_id,
    "scenario_id": scenario_id,
    "run_id": run_id,
    "bead_id": bead_id,
    "meta": document["meta"],
    "global_defaults": document["global_defaults"],
    "validation_classes": validation_classes,
    "included_bead_ids": sorted(rows_by_bead.keys()),
    "excluded_beads": excluded,
    "row_count": len(rows),
    "excluded_count": len(excluded),
}

ledger = {
    "trace_id": trace_id,
    "scenario_id": scenario_id,
    "run_id": run_id,
    "bead_id": bead_id,
    "rows": rows,
    "beads": [
        {
            "bead_id": bead_id_key,
            "validation_classes": sorted({row["validation_class"] for row in bead_rows}),
            "entrypoints": [row["entrypoint"] for row in bead_rows],
            "rows": bead_rows,
        }
        for bead_id_key, bead_rows in sorted(rows_by_bead.items())
    ],
}

manifest_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
ledger_path.write_text(json.dumps(ledger, indent=2, sort_keys=True) + "\n", encoding="utf-8")

summary_lines = [
    f"# {bead_id} Validation Matrix",
    "",
    f"- run_id: `{run_id}`",
    f"- trace_id: `{trace_id}`",
    f"- scenario_id: `{scenario_id}`",
    f"- included_beads: `{len(rows_by_bead)}`",
    f"- excluded_beads: `{len(excluded)}`",
    f"- obligation_rows: `{len(rows)}`",
    "",
    "## Validation Classes",
    "",
]
for row in validation_classes:
    summary_lines.append(f"- `{row['id']}`: {row['description']}")
summary_lines.extend([
    "",
    "## Included Beads",
    "",
])
for bead_id_key, bead_rows in sorted(rows_by_bead.items()):
    summary_lines.append(f"### `{bead_id_key}`")
    summary_lines.append("")
    summary_lines.append(f"- validation_classes: `{', '.join(sorted({row['validation_class'] for row in bead_rows}))}`")
    for row in bead_rows:
        summary_lines.append(
            "- "
            + f"`{row['validation_class']}` via `{row['entrypoint']}` "
            + f"(placement=`{row['placement_profile']}`, workload=`{row['workload_row']}`, failure_mode=`{row['failure_mode']}`, "
            + f"log_family=`{row['log_family']}`, seed_policy=`{row['seed_policy']}`, stop_on_failure=`{str(row['stop_on_failure']).lower()}`)"
        )
        summary_lines.append(f"  negative: {row['negative_expectation']}")
    summary_lines.append("")
    summary_lines.append("")
summary_lines.extend([
    "## Excluded Beads",
    "",
])
for row in excluded:
    summary_lines.append(f"- `{row['bead_id']}`: {row['reason']}")
    summary_lines.append(f"  gap_rule: {row['gap_rule']}")

summary_path.write_text("\n".join(summary_lines) + "\n", encoding="utf-8")
PY
  emit_event "render" "pass" "pass" 0 "rendered validation obligation ledger"
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
  emit_event "hash" "pass" "pass" 0 "hashed rendered artifacts"
}

if [[ "${SKIP_CONTRACT_TEST}" == "1" ]]; then
  emit_event "contract_test" "skip" "pass" 0 "skipping contract test because SKIP_CONTRACT_TEST=1"
else
  run_contract_test
fi
render_contract_artifacts
hash_artifacts

echo "=== ${BEAD_ID} validation matrix complete ==="
echo "Artifact dir: ${ARTIFACT_DIR}"
echo "Summary:      ${SUMMARY_MD}"
echo "Manifest:     ${MANIFEST_JSON}"
echo "Ledger:       ${LEDGER_JSON}"
echo "Events:       ${EVENTS_JSONL}"
echo "Hashes:       ${HASHES_TXT}"
