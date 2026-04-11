#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

BEAD_ID="bd-1sf8n"
SCENARIO_ID="MVCC-7"
TRACE_TOPIC="phase9_time_travel"
TIMESTAMP_UTC="$(date -u +%Y%m%dT%H%M%SZ)"
RUN_ID="${BEAD_ID}-${TIMESTAMP_UTC}"
TRACE_ID="trace-${RUN_ID}"
ARTIFACT_DIR="${REPO_ROOT}/artifacts/${BEAD_ID}/${RUN_ID}"
EVENTS_JSONL="${ARTIFACT_DIR}/events.jsonl"
TEST_LOG="${ARTIFACT_DIR}/cargo-test.log"
SCENARIO_SUMMARY_JSONL="${ARTIFACT_DIR}/scenario_summary.jsonl"
MANIFEST_JSON="${ARTIFACT_DIR}/manifest.json"
REPORT_JSON="${ARTIFACT_DIR}/report.json"
REPLAY_COMMAND="cargo test -p fsqlite-harness --test bd_1sf8n_phase9_time_travel_gate -- --nocapture --test-threads=1"
TARGET_DIR="${CARGO_TARGET_DIR:-/tmp/cargo-target}"
RESULT="running"

mkdir -p "${ARTIFACT_DIR}"
mkdir -p "${TARGET_DIR}"
: > "${EVENTS_JSONL}"

emit_event() {
    local phase="$1"
    local outcome="$2"
    local message="$3"
    printf '{"trace_id":"%s","run_id":"%s","scenario_id":"%s","trace_topic":"%s","phase":"%s","outcome":"%s","timestamp":"%s","message":"%s"}\n' \
        "${TRACE_ID}" "${RUN_ID}" "${SCENARIO_ID}" "${TRACE_TOPIC}" "${phase}" "${outcome}" \
        "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "${message}" >> "${EVENTS_JSONL}"
}

run_compile_step() {
    local phase="$1"
    local description="$2"
    shift 2

    if command -v rch >/dev/null 2>&1; then
        emit_event "${phase}" "running" "${description} via rch"
        if rch exec -- "$@" 2>&1 | tee "${TEST_LOG}"; then
            emit_event "${phase}" "pass" "${description} via rch"
            return 0
        fi
        emit_event "${phase}" "running" "${description} falling back to local execution"
    fi

    if "$@" 2>&1 | tee "${TEST_LOG}"; then
        emit_event "${phase}" "pass" "${description} via local fallback"
    else
        emit_event "${phase}" "fail" "${description} via local fallback"
        return 1
    fi
}

finish() {
    local exit_code=$?
    if [[ ${exit_code} -eq 0 ]]; then
        RESULT="pass"
    else
        RESULT="fail"
    fi

    local events_sha=""
    local log_sha=""
    local summary_sha=""
    if [[ -f "${EVENTS_JSONL}" ]]; then
        events_sha="$(sha256sum "${EVENTS_JSONL}" | awk '{print $1}')"
    fi
    if [[ -f "${TEST_LOG}" ]]; then
        log_sha="$(sha256sum "${TEST_LOG}" | awk '{print $1}')"
    fi
    if [[ -f "${SCENARIO_SUMMARY_JSONL}" ]]; then
        summary_sha="$(sha256sum "${SCENARIO_SUMMARY_JSONL}" | awk '{print $1}')"
    fi

    cat > "${MANIFEST_JSON}" <<EOF
{
  "bead_id": "${BEAD_ID}",
  "scenario_id": "${SCENARIO_ID}",
  "trace_topic": "${TRACE_TOPIC}",
  "run_id": "${RUN_ID}",
  "trace_id": "${TRACE_ID}",
  "replay_command": "${REPLAY_COMMAND}",
  "artifacts": [
    {"path":"${EVENTS_JSONL}","sha256":"${events_sha}"},
    {"path":"${TEST_LOG}","sha256":"${log_sha}"},
    {"path":"${SCENARIO_SUMMARY_JSONL}","sha256":"${summary_sha}"}
  ]
}
EOF

    cat > "${REPORT_JSON}" <<EOF
{
  "bead_id": "${BEAD_ID}",
  "scenario_id": "${SCENARIO_ID}",
  "trace_topic": "${TRACE_TOPIC}",
  "run_id": "${RUN_ID}",
  "trace_id": "${TRACE_ID}",
  "result": "${RESULT}",
  "events_jsonl": "${EVENTS_JSONL}",
  "test_log": "${TEST_LOG}",
  "scenario_summary_jsonl": "${SCENARIO_SUMMARY_JSONL}",
  "manifest_json": "${MANIFEST_JSON}",
  "replay_command": "${REPLAY_COMMAND}"
}
EOF

    emit_event "finalize" "${RESULT}" "report written to ${REPORT_JSON}"

    if [[ ${exit_code} -eq 0 ]]; then
        echo "[GATE PASS] ${BEAD_ID} Phase 9 time-travel verification passed"
    else
        echo "[GATE FAIL] ${BEAD_ID} Phase 9 time-travel verification failed"
        echo "artifact_dir=${ARTIFACT_DIR}"
    fi
}
trap finish EXIT

echo "=== ${BEAD_ID}: Phase 9 time-travel verification ==="
echo "run_id=${RUN_ID}"
echo "trace_id=${TRACE_ID}"
echo "scenario_id=${SCENARIO_ID}"
echo "artifact_dir=${ARTIFACT_DIR}"

emit_event "bootstrap" "running" "verification started"
run_compile_step \
    "time_travel_gate" \
    "running Phase 9 time-travel harness gate" \
    cargo test -p fsqlite-harness --test bd_1sf8n_phase9_time_travel_gate -- --nocapture --test-threads=1

emit_event "extract_outcomes" "running" "extracting SCENARIO_OUTCOME JSON lines"
if grep -E 'SCENARIO_OUTCOME:' "${TEST_LOG}" > /dev/null; then
    sed -n 's/.*SCENARIO_OUTCOME://p' "${TEST_LOG}" > "${SCENARIO_SUMMARY_JSONL}"
else
    : > "${SCENARIO_SUMMARY_JSONL}"
fi

scenario_rows="$(wc -l < "${SCENARIO_SUMMARY_JSONL}" | tr -d ' ')"
emit_event "extract_outcomes" "pass" "scenario_summary_rows=${scenario_rows}"

if [[ "${scenario_rows}" -lt 3 ]]; then
    emit_event "extract_outcomes" "fail" "expected at least 3 scenario outcomes"
    echo "ERROR: expected at least 3 scenario outcomes, observed ${scenario_rows}" >&2
    exit 1
fi
