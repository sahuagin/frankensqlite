#!/usr/bin/env bash
# verify_corpus_ingest.sh — verification gate for corpus ingestion + user repro
# intake/minimization pipeline.
#
# Primary bead: bd-2yqp6.3.5
# Legacy module lineage: bd-1dp9.2.1
#
# Structured logging contract:
#   emits JSONL events with trace_id/run_id/scenario_id/seed/timing/outcome
#   to artifacts/bd-2yqp6.3.5/<run_id>/events.jsonl
#
# Usage: ./scripts/verify_corpus_ingest.sh [--json]

set -euo pipefail

BEAD_ID="bd-2yqp6.3.5"
LEGACY_BEAD_ID="bd-1dp9.2.1"
SEED="${SEED:-3520}"
TIMESTAMP_UTC="$(date -u +%Y%m%dT%H%M%SZ)"
RUN_ID="${RUN_ID:-${BEAD_ID}-${TIMESTAMP_UTC}-${SEED}}"
TRACE_ID="${TRACE_ID:-trace-${RUN_ID}}"
SCENARIO_ID="${SCENARIO_ID:-PARITY-C5-INTAKE}"
ARTIFACT_DIR="artifacts/${BEAD_ID}/${RUN_ID}"
EVENTS_JSONL="${ARTIFACT_DIR}/events.jsonl"
TEST_LOG="${ARTIFACT_DIR}/test.log"
REPORT_JSON="${ARTIFACT_DIR}/report.json"

JSON_OUTPUT=false
if [[ "${1:-}" == "--json" ]]; then
  JSON_OUTPUT=true
fi

mkdir -p "${ARTIFACT_DIR}"
start_ns="$(date +%s%N)"

if ! command -v jq >/dev/null 2>&1; then
  echo "ERROR: jq is required for structured event emission and schema validation" >&2
  exit 2
fi

emit_event() {
  local phase="$1"
  local event_type="$2"
  local outcome="$3"
  local message="$4"
  local first_failure="${5:-}"
  local now_ns elapsed_ms
  now_ns="$(date +%s%N)"
  elapsed_ms="$(( (now_ns - start_ns) / 1000000 ))"
  jq -cn \
    --arg trace_id "${TRACE_ID}" \
    --arg run_id "${RUN_ID}" \
    --arg scenario_id "${SCENARIO_ID}" \
    --argjson seed "${SEED}" \
    --arg phase "${phase}" \
    --arg event_type "${event_type}" \
    --arg outcome "${outcome}" \
    --argjson elapsed_ms "${elapsed_ms}" \
    --arg timestamp "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
    --arg message "${message}" \
    --arg first_failure "${first_failure}" \
    '{
      trace_id: $trace_id,
      run_id: $run_id,
      scenario_id: $scenario_id,
      seed: $seed,
      phase: $phase,
      event_type: $event_type,
      outcome: $outcome,
      elapsed_ms: $elapsed_ms,
      timestamp: $timestamp,
      message: $message,
      first_failure: $first_failure
    }' >> "${EVENTS_JSONL}"
}

run_gate() {
  local phase="$1"
  shift
  emit_event "${phase}" "start" "running" "running: $*"
  if "$@" 2>&1 | tee -a "${TEST_LOG}"; then
    emit_event "${phase}" "pass" "pass" "command passed"
    return 0
  fi
  local first_failure
  first_failure="$(
    grep -m1 -E '(^error:|FAILED|panicked at)' "${TEST_LOG}" 2>/dev/null \
      | sed 's/[[:space:]]\+/ /g' \
      || true
  )"
  emit_event "${phase}" "fail" "fail" "command failed" "${first_failure}"
  return 1
}

validate_events_schema() {
  jq -e -s '
    all(
      .[];
      has("trace_id")
      and has("run_id")
      and has("scenario_id")
      and has("seed")
      and has("phase")
      and has("event_type")
      and has("outcome")
      and has("elapsed_ms")
      and has("timestamp")
      and has("message")
      and has("first_failure")
      and (.trace_id | type == "string" and length > 0)
      and (.run_id | type == "string" and length > 0)
      and (.scenario_id | type == "string" and length > 0)
      and (.seed | type == "number")
      and (.elapsed_ms | type == "number")
      and (.outcome | type == "string")
      and (.first_failure | type == "string")
    )
  ' "${EVENTS_JSONL}" > /dev/null
}

echo "=== ${BEAD_ID}: corpus ingestion verification ==="
echo "run_id=${RUN_ID}"
echo "trace_id=${TRACE_ID}"
echo "scenario_id=${SCENARIO_ID}"

emit_event "bootstrap" "start" "running" "verification started"

TEST_RESULT="pass"
if ! run_gate "corpus_ingest_tests" \
  rch exec -- cargo test -p fsqlite-harness --test corpus_ingest_test -- --nocapture; then
  TEST_RESULT="fail"
fi

EVENT_SCHEMA_OK="false"
if validate_events_schema; then
  EVENT_SCHEMA_OK="true"
else
  emit_event "schema" "fail" "fail" "events schema validation failed"
fi

TEST_COUNT=0
TEST_PASSED=0
TEST_FAILED=0
if grep -q "test result:" "${TEST_LOG}" 2>/dev/null; then
  RESULT_LINE="$(grep "test result:" "${TEST_LOG}" | tail -1)"
  TEST_PASSED="$(echo "${RESULT_LINE}" | grep -o '[0-9]* passed' | grep -o '[0-9]*' || echo 0)"
  TEST_FAILED="$(echo "${RESULT_LINE}" | grep -o '[0-9]* failed' | grep -o '[0-9]*' || echo 0)"
  TEST_COUNT=$((TEST_PASSED + TEST_FAILED))
fi

ARTIFACTS_OK="true"
for f in \
  "crates/fsqlite-harness/src/corpus_ingest.rs" \
  "crates/fsqlite-harness/tests/corpus_ingest_test.rs" \
  "scripts/verify_corpus_ingest.sh" \
; do
  if [[ ! -f "$f" ]]; then
    ARTIFACTS_OK="false"
    echo "ERROR: Missing artifact: $f" >&2
  fi
done

MODULE_REGISTERED="false"
if grep -q "pub mod corpus_ingest" crates/fsqlite-harness/src/lib.rs; then
  MODULE_REGISTERED="true"
fi

MODULE_HASH="$(sha256sum crates/fsqlite-harness/src/corpus_ingest.rs | awk '{print $1}')"
TEST_HASH="$(sha256sum crates/fsqlite-harness/tests/corpus_ingest_test.rs | awk '{print $1}')"
SCRIPT_HASH="$(sha256sum scripts/verify_corpus_ingest.sh | awk '{print $1}')"
EVENTS_HASH="$(sha256sum "${EVENTS_JSONL}" | awk '{print $1}')"
TEST_LOG_HASH="$(sha256sum "${TEST_LOG}" | awk '{print $1}')"

ERRORS=0
[[ "${TEST_RESULT}" == "pass" ]] || ERRORS=$((ERRORS + 1))
[[ "${MODULE_REGISTERED}" == "true" ]] || ERRORS=$((ERRORS + 1))
[[ "${ARTIFACTS_OK}" == "true" ]] || ERRORS=$((ERRORS + 1))
[[ "${EVENT_SCHEMA_OK}" == "true" ]] || ERRORS=$((ERRORS + 1))

RESULT_LABEL="$([ "${ERRORS}" -eq 0 ] && echo "pass" || echo "fail")"
cat > "${REPORT_JSON}" <<EOF_JSON
{
  "bead_id": "${BEAD_ID}",
  "legacy_bead_id": "${LEGACY_BEAD_ID}",
  "trace_id": "${TRACE_ID}",
  "run_id": "${RUN_ID}",
  "scenario_id": "${SCENARIO_ID}",
  "seed": ${SEED},
  "unit_test_result": "${TEST_RESULT}",
  "test_counts": {
    "total": ${TEST_COUNT},
    "passed": ${TEST_PASSED},
    "failed": ${TEST_FAILED}
  },
  "module_registered": ${MODULE_REGISTERED},
  "artifacts_present": ${ARTIFACTS_OK},
  "events_schema_valid": ${EVENT_SCHEMA_OK},
  "artifact_hashes": {
    "corpus_ingest_module": "${MODULE_HASH}",
    "corpus_ingest_test": "${TEST_HASH}",
    "verify_script": "${SCRIPT_HASH}",
    "events_jsonl": "${EVENTS_HASH}",
    "test_log": "${TEST_LOG_HASH}"
  },
  "artifacts": {
    "events_jsonl": "${EVENTS_JSONL}",
    "test_log": "${TEST_LOG}",
    "report_json": "${REPORT_JSON}"
  },
  "validation_errors": ${ERRORS},
  "result": "${RESULT_LABEL}"
}
EOF_JSON

emit_event "finalize" "info" "${RESULT_LABEL}" "report written to ${REPORT_JSON}"

if $JSON_OUTPUT; then
  cat "${REPORT_JSON}"
else
  echo ""
  echo "--- Unit Tests ---"
  echo "Result:           ${TEST_RESULT}"
  echo "Total:            ${TEST_COUNT}"
  echo "  Passed:         ${TEST_PASSED}"
  echo "  Failed:         ${TEST_FAILED}"
  echo ""
  echo "--- Validation ---"
  echo "Module registered: ${MODULE_REGISTERED}"
  echo "Artifacts present: ${ARTIFACTS_OK}"
  echo "Events schema:     ${EVENT_SCHEMA_OK}"
  echo "Errors:            ${ERRORS}"
  echo "Result:            $([ "${ERRORS}" -eq 0 ] && echo 'PASS' || echo 'FAIL')"
  echo "Artifacts:         ${ARTIFACT_DIR}"
fi

exit "${ERRORS}"
