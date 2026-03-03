#!/usr/bin/env bash
# verify_txn_lifecycle.sh — deterministic verifier for bd-t6sv2.5
set -euo pipefail

BEAD_ID="bd-t6sv2.5"
SCHEMA_VERSION="1"
RUN_ID="${BEAD_ID}-$(date -u +%Y%m%dT%H%M%SZ)-$$"
REPORT_DIR="test-results"
REPORT_FILE="${REPORT_DIR}/${BEAD_ID}-txn-lifecycle-verify.json"
JSON_MODE=false
NO_RCH=false

SCRIPT_PATH="${BASH_SOURCE[0]}"
if [[ "$SCRIPT_PATH" != /* ]]; then
  SCRIPT_PATH="$(pwd)/$SCRIPT_PATH"
fi

for arg in "$@"; do
  case "$arg" in
    --json) JSON_MODE=true ;;
    --no-rch) NO_RCH=true ;;
    *)
      echo "ERROR: unknown argument '$arg'" >&2
      exit 2
      ;;
  esac
done

mkdir -p "$REPORT_DIR"

run_build() {
  if ! $NO_RCH && command -v rch >/dev/null 2>&1; then
    rch exec -- "$@"
  else
    "$@"
  fi
}

parse_passed() {
  echo "$1" | grep 'test result:' | grep -oP '\d+ passed' | grep -oP '\d+' | tail -1 || echo "0"
}

parse_failed() {
  echo "$1" | grep 'test result:' | grep -oP '\d+ failed' | grep -oP '\d+' | tail -1 || echo "0"
}

declare -A CASE_STATUS
declare -A CASE_PASSED
declare -A CASE_FAILED
declare -A CASE_EXIT
declare -A CASE_DURATION_MS
declare -A CASE_COMMAND

run_case() {
  local case_id="$1"
  shift

  CASE_COMMAND["$case_id"]="$*"
  echo "phase=${case_id} bead_id=${BEAD_ID} run_id=${RUN_ID}"

  local output
  local exit_code
  local started_ns
  local finished_ns
  local elapsed_ms
  started_ns=$(date +%s%N)
  set +e
  output=$("$@" 2>&1)
  exit_code=$?
  set -e
  finished_ns=$(date +%s%N)
  elapsed_ms=$(((finished_ns - started_ns) / 1000000))
  local passed
  local failed
  passed=$(parse_passed "$output")
  failed=$(parse_failed "$output")

  CASE_PASSED["$case_id"]="$passed"
  CASE_FAILED["$case_id"]="$failed"
  CASE_EXIT["$case_id"]="$exit_code"
  CASE_DURATION_MS["$case_id"]="$elapsed_ms"

  if [ "$exit_code" -eq 0 ] && [ "$failed" -eq 0 ]; then
    CASE_STATUS["$case_id"]="pass"
  else
    CASE_STATUS["$case_id"]="fail"
  fi
}

run_case \
  "txn_core_suite" \
  run_build cargo test -p fsqlite-core test_pragma_txn_ -- --nocapture

run_case \
  "txn_live_table_suite" \
  run_build cargo test -p fsqlite-core test_pragma_fsqlite_transactions -- --nocapture

run_case \
  "clippy_core" \
  run_build cargo clippy -p fsqlite-core --all-targets -- -D warnings

TOTAL_PASSED=0
TOTAL_FAILED=0
VERDICT="pass"
for case_id in "${!CASE_STATUS[@]}"; do
  TOTAL_PASSED=$((TOTAL_PASSED + CASE_PASSED["$case_id"]))
  TOTAL_FAILED=$((TOTAL_FAILED + CASE_FAILED["$case_id"]))
  if [ "${CASE_STATUS["$case_id"]}" != "pass" ]; then
    VERDICT="fail"
  fi
done

TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
REPORT_CONTENT=$(cat <<ENDJSON
{
  "schema_version": ${SCHEMA_VERSION},
  "bead_id": "${BEAD_ID}",
  "run_id": "${RUN_ID}",
  "timestamp": "${TIMESTAMP}",
  "verdict": "${VERDICT}",
  "cases": {
    "txn_core_suite": {
      "status": "${CASE_STATUS["txn_core_suite"]}",
      "command": "${CASE_COMMAND["txn_core_suite"]}",
      "exit_code": ${CASE_EXIT["txn_core_suite"]},
      "duration_ms": ${CASE_DURATION_MS["txn_core_suite"]},
      "passed": ${CASE_PASSED["txn_core_suite"]},
      "failed": ${CASE_FAILED["txn_core_suite"]}
    },
    "txn_live_table_suite": {
      "status": "${CASE_STATUS["txn_live_table_suite"]}",
      "command": "${CASE_COMMAND["txn_live_table_suite"]}",
      "exit_code": ${CASE_EXIT["txn_live_table_suite"]},
      "duration_ms": ${CASE_DURATION_MS["txn_live_table_suite"]},
      "passed": ${CASE_PASSED["txn_live_table_suite"]},
      "failed": ${CASE_FAILED["txn_live_table_suite"]}
    },
    "clippy_core": {
      "status": "${CASE_STATUS["clippy_core"]}",
      "command": "${CASE_COMMAND["clippy_core"]}",
      "exit_code": ${CASE_EXIT["clippy_core"]},
      "duration_ms": ${CASE_DURATION_MS["clippy_core"]},
      "passed": ${CASE_PASSED["clippy_core"]},
      "failed": ${CASE_FAILED["clippy_core"]}
    }
  },
  "totals": {
    "passed": ${TOTAL_PASSED},
    "failed": ${TOTAL_FAILED}
  }
}
ENDJSON
)

echo "$REPORT_CONTENT" > "$REPORT_FILE"
REPORT_SHA=$(sha256sum "$REPORT_FILE" | cut -d' ' -f1)

if $JSON_MODE; then
  echo "$REPORT_CONTENT"
else
  echo "phase=complete bead_id=${BEAD_ID} run_id=${RUN_ID} verdict=${VERDICT}"
  echo "  txn_core_suite: ${CASE_STATUS["txn_core_suite"]}"
  echo "  txn_live_table_suite: ${CASE_STATUS["txn_live_table_suite"]}"
  echo "  clippy_core: ${CASE_STATUS["clippy_core"]}"
  echo "  report_path=${REPORT_FILE}"
  echo "  report_sha256=${REPORT_SHA}"
fi

if [ "$VERDICT" = "pass" ]; then
  exit 0
fi
exit 1
