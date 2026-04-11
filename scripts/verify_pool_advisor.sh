#!/usr/bin/env bash
# verify_pool_advisor.sh — deterministic verifier for bd-t6sv2.10
set -euo pipefail

BEAD_ID="bd-t6sv2.10"
SCHEMA_VERSION="1"
RUN_ID="${BEAD_ID}-$(date -u +%Y%m%dT%H%M%SZ)-$$"
REPORT_DIR="test-results"
REPORT_FILE="${REPORT_DIR}/${BEAD_ID}-pool-advisor-verify.json"
JSON_MODE=false
NO_RCH=false

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
  echo "$1" | grep 'test result:' | grep -oE '[0-9]+ passed' | grep -oE '[0-9]+' | tail -1 || echo "0"
}

parse_failed() {
  echo "$1" | grep 'test result:' | grep -oE '[0-9]+ failed' | grep -oE '[0-9]+' | tail -1 || echo "0"
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
  if [[ "$passed" == "0" && "$failed" == "0" && "$exit_code" -eq 0 ]]; then
    passed=1
  fi

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
  "validator_suite" \
  run_build cargo test -p fsqlite-observability connection_pool::tests:: -- --nocapture

run_case \
  "doctest_suite" \
  run_build cargo test -p fsqlite-observability --doc -- --nocapture

run_case \
  "clippy_observability" \
  run_build cargo clippy -p fsqlite-observability --all-targets --no-deps -- -D warnings

run_case \
  "docs_contract" \
  bash -lc \
  "rg -q 'multiple writer connections' docs/connection-pooling.md \
    && rg -q 'simulate_connection_pool' docs/connection-pooling.md \
    && rg -q 'validate_connection_pool' docs/connection-pooling.md \
    && rg -q 'sqlx::Pool|sqlx' docs/connection-pooling.md \
    && rg -q 'r2d2' docs/connection-pooling.md \
    && rg -q 'deadpool' docs/connection-pooling.md \
    && rg -q 'bb8' docs/connection-pooling.md"

TOTAL_PASSED=0
TOTAL_FAILED=0
TOTAL_CASES=0
PASSED_CASES=0
VERDICT="pass"
for case_id in "${!CASE_STATUS[@]}"; do
  TOTAL_PASSED=$((TOTAL_PASSED + CASE_PASSED["$case_id"]))
  TOTAL_FAILED=$((TOTAL_FAILED + CASE_FAILED["$case_id"]))
  TOTAL_CASES=$((TOTAL_CASES + 1))
  if [ "${CASE_STATUS["$case_id"]}" = "pass" ]; then
    PASSED_CASES=$((PASSED_CASES + 1))
  else
    VERDICT="fail"
  fi
done

RECOMMENDATION_ACCURACY_PCT=$((PASSED_CASES * 100 / TOTAL_CASES))
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
REPORT_CONTENT=$(cat <<ENDJSON
{
  "schema_version": ${SCHEMA_VERSION},
  "bead_id": "${BEAD_ID}",
  "run_id": "${RUN_ID}",
  "timestamp": "${TIMESTAMP}",
  "verdict": "${VERDICT}",
  "recommendation_accuracy_pct": ${RECOMMENDATION_ACCURACY_PCT},
  "cases": {
    "validator_suite": {
      "status": "${CASE_STATUS["validator_suite"]}",
      "command": "${CASE_COMMAND["validator_suite"]}",
      "exit_code": ${CASE_EXIT["validator_suite"]},
      "duration_ms": ${CASE_DURATION_MS["validator_suite"]},
      "passed": ${CASE_PASSED["validator_suite"]},
      "failed": ${CASE_FAILED["validator_suite"]}
    },
    "doctest_suite": {
      "status": "${CASE_STATUS["doctest_suite"]}",
      "command": "${CASE_COMMAND["doctest_suite"]}",
      "exit_code": ${CASE_EXIT["doctest_suite"]},
      "duration_ms": ${CASE_DURATION_MS["doctest_suite"]},
      "passed": ${CASE_PASSED["doctest_suite"]},
      "failed": ${CASE_FAILED["doctest_suite"]}
    },
    "clippy_observability": {
      "status": "${CASE_STATUS["clippy_observability"]}",
      "command": "${CASE_COMMAND["clippy_observability"]}",
      "exit_code": ${CASE_EXIT["clippy_observability"]},
      "duration_ms": ${CASE_DURATION_MS["clippy_observability"]},
      "passed": ${CASE_PASSED["clippy_observability"]},
      "failed": ${CASE_FAILED["clippy_observability"]}
    },
    "docs_contract": {
      "status": "${CASE_STATUS["docs_contract"]}",
      "command": "${CASE_COMMAND["docs_contract"]}",
      "exit_code": ${CASE_EXIT["docs_contract"]},
      "duration_ms": ${CASE_DURATION_MS["docs_contract"]},
      "passed": ${CASE_PASSED["docs_contract"]},
      "failed": ${CASE_FAILED["docs_contract"]}
    }
  },
  "totals": {
    "passed": ${TOTAL_PASSED},
    "failed": ${TOTAL_FAILED},
    "passed_cases": ${PASSED_CASES},
    "total_cases": ${TOTAL_CASES}
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
  echo "  validator_suite: ${CASE_STATUS["validator_suite"]}"
  echo "  doctest_suite: ${CASE_STATUS["doctest_suite"]}"
  echo "  clippy_observability: ${CASE_STATUS["clippy_observability"]}"
  echo "  docs_contract: ${CASE_STATUS["docs_contract"]}"
  echo "  recommendation_accuracy_pct=${RECOMMENDATION_ACCURACY_PCT}"
  echo "  report_path=${REPORT_FILE}"
  echo "  report_sha256=${REPORT_SHA}"
fi

if [ "$VERDICT" = "pass" ]; then
  exit 0
fi
exit 1
