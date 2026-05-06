#!/usr/bin/env bash
# Verification gate for bd-db300.4.5.8:
# low-tax hot-path unit and proof surface.

set -euo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BEAD_ID="bd-db300.4.5.8"
CONTRACT_PATH="${WORKSPACE_ROOT}/docs/contracts/db300_low_tax_verification_contract.toml"
SCENARIO_ID="${SCENARIO_ID:-D5-8-LOW-TAX-VERIFICATION}"
SEED="${SEED:-458}"
TIMESTAMP_UTC="$(date -u +%Y%m%dT%H%M%SZ)"
RUN_ID="${RUN_ID:-${BEAD_ID}-${TIMESTAMP_UTC}-${SEED}}"
TRACE_ID="${TRACE_ID:-trace-${RUN_ID}}"
ARTIFACT_DIR="${WORKSPACE_ROOT}/artifacts/${BEAD_ID}/${RUN_ID}"
EVENTS_JSONL="${ARTIFACT_DIR}/events.jsonl"
SUMMARY_MD="${ARTIFACT_DIR}/summary.md"
MANIFEST_JSON="${ARTIFACT_DIR}/low_tax_verification_manifest.json"
LEDGER_JSON="${ARTIFACT_DIR}/low_tax_family_ledger.json"
HASHES_TXT="${ARTIFACT_DIR}/artifact_hashes.txt"
USE_RCH="${USE_RCH:-0}"
SKIP_TEST_RUN="${SKIP_TEST_RUN:-0}"
CARGO_TARGET_DIR_BASE="${CARGO_TARGET_DIR_BASE:-${WORKSPACE_ROOT}/.codex-target/d5_8_low_tax_verification}"

mkdir -p "${ARTIFACT_DIR}"
: > "${EVENTS_JSONL}"

export NO_COLOR="${NO_COLOR:-1}"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"

emit_event() {
  local phase="$1"
  local family_id="$2"
  local event_type="$3"
  local outcome="$4"
  local elapsed_ms="$5"
  local message="$6"
  local runner="${7:-operator_entrypoint}"
  local diagnostic_keys="${8:-none}"
  local proof_note="${9:-none}"

  python3 - "${EVENTS_JSONL}" \
    "${TRACE_ID}" "${SCENARIO_ID}" "${BEAD_ID}" "${RUN_ID}" "${phase}" \
    "${family_id}" "${event_type}" "${outcome}" "${elapsed_ms}" "${message}" \
    "${runner}" "${diagnostic_keys}" "${proof_note}" <<'PY'
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
    family_id,
    event_type,
    outcome,
    elapsed_ms,
    message,
    runner,
    diagnostic_keys,
    proof_note,
) = sys.argv[2:15]

event = {
    "artifact_manifest_key": "low_tax_verification",
    "bead_id": bead_id,
    "diagnostic_json_pointer": None,
    "diagnostic_keys": diagnostic_keys.split(",") if diagnostic_keys != "none" else [],
    "elapsed_ms": int(elapsed_ms),
    "event_type": event_type,
    "family_id": family_id,
    "message": message,
    "outcome": outcome,
    "phase": phase,
    "proof_note_requirement": proof_note,
    "run_id": run_id,
    "runner": runner,
    "scenario_id": scenario_id,
    "timestamp": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    "trace_id": trace_id,
}
with open(path, "a", encoding="utf-8") as handle:
    handle.write(json.dumps(event, sort_keys=True) + "\n")
PY
}

run_family() {
  local family_id="$1"
  local runner="$2"
  local diagnostic_keys="$3"
  local proof_note="$4"
  local logfile="${ARTIFACT_DIR}/${family_id}.log"
  local started finished elapsed
  emit_event "family_run" "${family_id}" "start" "running" 0 "running ${family_id}" "${runner}" "${diagnostic_keys}" "${proof_note}"
  started="$(date +%s%3N)"

  run_one_command() {
    local subcommand="$1"
    local -a argv
    read -r -a argv <<< "${subcommand}"
    if [[ "${USE_RCH}" == "1" ]]; then
      (
        cd "${WORKSPACE_ROOT}"
        rch exec -- "${argv[@]}"
      )
    else
      (
        cd "${WORKSPACE_ROOT}"
        env \
          "CARGO_TARGET_DIR=${CARGO_TARGET_DIR_BASE}" \
          "CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS}" \
          "NO_COLOR=${NO_COLOR}" \
          "${argv[@]}"
      )
    fi
  }

  if (
    remaining="${runner}"
    while [[ -n "${remaining}" ]]; do
      if [[ "${remaining}" == *"&&"* ]]; then
        subcommand="${remaining%%&&*}"
        remaining="${remaining#*&&}"
      else
        subcommand="${remaining}"
        remaining=""
      fi
      subcommand="${subcommand#"${subcommand%%[![:space:]]*}"}"
      subcommand="${subcommand%"${subcommand##*[![:space:]]}"}"
      [[ -z "${subcommand}" ]] && continue
      run_one_command "${subcommand}"
    done
  ) 2>&1 | tee "${logfile}"; then
    finished="$(date +%s%3N)"
    elapsed="$((finished - started))"
    if ! grep -Eq '^test result: ok\.' "${logfile}"; then
      emit_event "family_run" "${family_id}" "fail" "fail" "${elapsed}" "family completed without passing cargo test result" "${runner}" "${diagnostic_keys}" "${proof_note}"
      return 1
    fi
    emit_event "family_run" "${family_id}" "pass" "pass" "${elapsed}" "family passed" "${runner}" "${diagnostic_keys}" "${proof_note}"
  else
    finished="$(date +%s%3N)"
    elapsed="$((finished - started))"
    emit_event "family_run" "${family_id}" "fail" "fail" "${elapsed}" "family command failed" "${runner}" "${diagnostic_keys}" "${proof_note}"
    return 1
  fi
}

render_contract_artifacts() {
  emit_event "render" "contract" "start" "running" 0 "rendering low-tax verification artifacts"
  python3 - "${CONTRACT_PATH}" "${MANIFEST_JSON}" "${LEDGER_JSON}" "${SUMMARY_MD}" "${TRACE_ID}" "${SCENARIO_ID}" "${RUN_ID}" "${BEAD_ID}" <<'PY'
import json
import sys
import tomllib
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

families = document.get("test_family", [])
covered = document.get("covered_bead", [])
deferred = document.get("deferred_extension", [])

manifest = {
    "trace_id": trace_id,
    "scenario_id": scenario_id,
    "run_id": run_id,
    "bead_id": bead_id,
    "meta": document["meta"],
    "global_defaults": document["global_defaults"],
    "covered_beads": covered,
    "deferred_extensions": deferred,
    "family_count": len(families),
}
manifest_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")

ledger = {
    "trace_id": trace_id,
    "scenario_id": scenario_id,
    "run_id": run_id,
    "bead_id": bead_id,
    "families": families,
}
ledger_path.write_text(json.dumps(ledger, indent=2, sort_keys=True) + "\n", encoding="utf-8")

summary_lines = [
    f"# {bead_id} Low-Tax Verification",
    "",
    f"- run_id: `{run_id}`",
    f"- trace_id: `{trace_id}`",
    f"- scenario_id: `{scenario_id}`",
    f"- family_count: `{len(families)}`",
    "",
    "## Covered Beads",
    "",
]
for row in covered:
    summary_lines.append(f"- `{row['bead_id']}`: {row['title']}")
summary_lines.extend([
    "",
    "## Deferred Extensions",
    "",
])
for row in deferred:
    summary_lines.append(f"- `{row['bead_id']}`: {row['reason']}")
summary_lines.extend([
    "",
    "## Test Families",
    "",
])
for family in families:
    summary_lines.append(f"### `{family['family_id']}`")
    summary_lines.append("")
    summary_lines.append(f"- owner_crate: `{family['owner_crate']}`")
    summary_lines.append(f"- source_path: `{family['source_path']}`")
    summary_lines.append(f"- runner: `{family['runner']}`")
    summary_lines.append(f"- supports_beads: `{', '.join(family['supports_beads'])}`")
    summary_lines.append(f"- test_names: `{', '.join(family['test_names'])}`")
    summary_lines.append(f"- diagnostic_keys: `{', '.join(family['diagnostic_keys'])}`")
    summary_lines.append(f"- behavior_preservation_scope: {family['behavior_preservation_scope']}")
    summary_lines.append(f"- proof_note_requirement: {family['proof_note_requirement']}")
    summary_lines.append("")
summary_path.write_text("\n".join(summary_lines) + "\n", encoding="utf-8")
PY
  emit_event "render" "contract" "pass" "pass" 0 "rendered low-tax verification artifacts"
}

hash_artifacts() {
  local files=(
    "$(basename "${EVENTS_JSONL}")"
    "$(basename "${MANIFEST_JSON}")"
    "$(basename "${LEDGER_JSON}")"
    "$(basename "${SUMMARY_MD}")"
  )
  shopt -s nullglob
  local logs=("${ARTIFACT_DIR}"/*.log)
  shopt -u nullglob
  for log in "${logs[@]}"; do
    files+=("$(basename "${log}")")
  done
  (
    cd "${ARTIFACT_DIR}"
    sha256sum "${files[@]}" > "${HASHES_TXT##${ARTIFACT_DIR}/}"
  )
  emit_event "hash" "contract" "pass" "pass" 0 "hashed rendered artifacts"
}

main() {
  if [[ ! -f "${CONTRACT_PATH}" ]]; then
    echo "missing contract: ${CONTRACT_PATH}" >&2
    exit 1
  fi

  render_contract_artifacts

  if [[ "${SKIP_TEST_RUN}" != "1" ]]; then
    run_family \
      "prepared_reuse_and_refresh" \
      "cargo test -p fsqlite-core --test prepared_hit_rate_proof -- --nocapture" \
      "prepared_insert_fast_lane_hits,prepared_schema_refreshes,prepared_schema_full_reloads,pager_publication_refreshes" \
      "Prepared fast lane + publication reuse equivalence"
    run_family \
      "boundary_duplication_census" \
      "cargo test -p fsqlite-core --test boundary_duplication_census -- --test-threads=1 --nocapture" \
      "publication_bind,prepared_hits,fast_lane_hits,schema_invalidation" \
      "Boundary duplication census must keep invalidation and duplicate-bind visibility"
    run_family \
      "result_row_register_reuse" \
      "cargo test -p fsqlite-vdbe --lib test_disabling_result_row_collection_still_clears_result_registers -- --nocapture && cargo test -p fsqlite-vdbe --lib test_multiple_result_rows -- --nocapture && cargo test -p fsqlite-vdbe --lib test_result_row_negative_start_register_stays_null -- --nocapture" \
      "row_count,register_clear_semantics,negative_register_handling" \
      "ResultRow register reuse must preserve row semantics and null behavior"
    run_family \
      "record_decode_scratch" \
      "cargo test -p fsqlite-types --lib record_decode_scratch_reuses_small_record_state -- --nocapture && cargo test -p fsqlite-types --lib record_decode_scratch_eagerly_materializes_wide_records -- --nocapture" \
      "scratch_record_calls,legacy_parse_record_calls,wide_record_materialization" \
      "Scratch decode must preserve decode equivalence while changing allocation discipline"
  else
    emit_event "family_run" "all" "skip" "skipped" 0 "skipping test runs because SKIP_TEST_RUN=1"
  fi

  hash_artifacts

  printf '%s\n' \
    "${BEAD_ID} artifacts ready:" \
    "  artifact_dir: ${ARTIFACT_DIR}" \
    "  summary: ${SUMMARY_MD}" \
    "  manifest: ${MANIFEST_JSON}" \
    "  ledger: ${LEDGER_JSON}" \
    "  events: ${EVENTS_JSONL}" \
    "  hashes: ${HASHES_TXT}"
}

main "$@"
