#!/usr/bin/env bash
# verify_checkpoint_scheduling.sh — deterministic verifier for bd-t6sv2.7
set -euo pipefail

BEAD_ID="bd-t6sv2.7"
SCHEMA_VERSION="1"
RUN_ID="${BEAD_ID}-$(date -u +%Y%m%dT%H%M%SZ)-$$"
REPORT_DIR="test-results"
REPORT_FILE="${REPORT_DIR}/${BEAD_ID}-checkpoint-scheduling-verify.json"
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

extract_scheduling_metric() {
  local output="$1"
  local scenario="$2"
  local metric="$3"
  local line
  line=$(echo "$output" | grep "checkpoint_scheduling_case scenario=${scenario}" | tail -1 || true)
  if [[ -z "$line" ]]; then
    echo "-1"
    return
  fi
  echo "$line" | sed -n "s/.*${metric}=\([0-9][0-9]*\).*/\1/p" | tail -1 | awk 'NF {print; found=1} END {if (!found) print "-1"}'
}

declare -A CASE_STATUS
declare -A CASE_PASSED
declare -A CASE_FAILED
declare -A CASE_EXIT
declare -A CASE_DURATION_MS
declare -A CASE_COMMAND

declare -A CASE_OUTPUT

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
  CASE_OUTPUT["$case_id"]="$output"

  if [ "$exit_code" -eq 0 ] && [ "$failed" -eq 0 ]; then
    CASE_STATUS["$case_id"]="pass"
  else
    CASE_STATUS["$case_id"]="fail"
  fi
}

run_case \
  "checkpoint_core_suite" \
  run_build env RUST_TEST_THREADS=1 cargo test -p fsqlite-core test_pragma_checkpoint_ -- --nocapture

run_case \
  "checkpoint_burst_suite" \
  run_build env RUST_TEST_THREADS=1 cargo test -p fsqlite-core test_pragma_checkpoint_autocheckpoint_bursty_default_vs_adaptive -- --nocapture

run_case \
  "checkpoint_wal_metrics_suite" \
  run_build cargo test -p fsqlite-wal metrics_ -- --nocapture

run_case \
  "clippy_core" \
  run_build cargo clippy -p fsqlite-core --all-targets --no-deps -- -D warnings

BASELINE_CHECKPOINT_COUNT=$(extract_scheduling_metric "${CASE_OUTPUT["checkpoint_burst_suite"]}" "baseline" "checkpoint_count")
BASELINE_WAL_FRAMES=$(extract_scheduling_metric "${CASE_OUTPUT["checkpoint_burst_suite"]}" "baseline" "wal_frames_estimate")
ADAPTIVE_CHECKPOINT_COUNT=$(extract_scheduling_metric "${CASE_OUTPUT["checkpoint_burst_suite"]}" "adaptive" "checkpoint_count")
ADAPTIVE_WAL_FRAMES=$(extract_scheduling_metric "${CASE_OUTPUT["checkpoint_burst_suite"]}" "adaptive" "wal_frames_estimate")

ADAPTIVE_CHECKPOINT_PRESSURE_REDUCED=false
ADAPTIVE_WAL_BUFFERING_HIGHER=false
if [ "$BASELINE_CHECKPOINT_COUNT" -ge 0 ] && [ "$ADAPTIVE_CHECKPOINT_COUNT" -ge 0 ] && [ "$ADAPTIVE_CHECKPOINT_COUNT" -le "$BASELINE_CHECKPOINT_COUNT" ]; then
  ADAPTIVE_CHECKPOINT_PRESSURE_REDUCED=true
fi
if [ "$BASELINE_WAL_FRAMES" -ge 0 ] && [ "$ADAPTIVE_WAL_FRAMES" -ge 0 ] && [ "$ADAPTIVE_WAL_FRAMES" -ge "$BASELINE_WAL_FRAMES" ]; then
  ADAPTIVE_WAL_BUFFERING_HIGHER=true
fi

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

if ! $ADAPTIVE_CHECKPOINT_PRESSURE_REDUCED || ! $ADAPTIVE_WAL_BUFFERING_HIGHER; then
  VERDICT="fail"
fi

TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
REPORT_CONTENT=$(cat <<ENDJSON
{
  "schema_version": ${SCHEMA_VERSION},
  "bead_id": "${BEAD_ID}",
  "run_id": "${RUN_ID}",
  "timestamp": "${TIMESTAMP}",
  "verdict": "${VERDICT}",
  "cases": {
    "checkpoint_core_suite": {
      "status": "${CASE_STATUS["checkpoint_core_suite"]}",
      "command": "${CASE_COMMAND["checkpoint_core_suite"]}",
      "exit_code": ${CASE_EXIT["checkpoint_core_suite"]},
      "duration_ms": ${CASE_DURATION_MS["checkpoint_core_suite"]},
      "passed": ${CASE_PASSED["checkpoint_core_suite"]},
      "failed": ${CASE_FAILED["checkpoint_core_suite"]}
    },
    "checkpoint_burst_suite": {
      "status": "${CASE_STATUS["checkpoint_burst_suite"]}",
      "command": "${CASE_COMMAND["checkpoint_burst_suite"]}",
      "exit_code": ${CASE_EXIT["checkpoint_burst_suite"]},
      "duration_ms": ${CASE_DURATION_MS["checkpoint_burst_suite"]},
      "passed": ${CASE_PASSED["checkpoint_burst_suite"]},
      "failed": ${CASE_FAILED["checkpoint_burst_suite"]}
    },
    "checkpoint_wal_metrics_suite": {
      "status": "${CASE_STATUS["checkpoint_wal_metrics_suite"]}",
      "command": "${CASE_COMMAND["checkpoint_wal_metrics_suite"]}",
      "exit_code": ${CASE_EXIT["checkpoint_wal_metrics_suite"]},
      "duration_ms": ${CASE_DURATION_MS["checkpoint_wal_metrics_suite"]},
      "passed": ${CASE_PASSED["checkpoint_wal_metrics_suite"]},
      "failed": ${CASE_FAILED["checkpoint_wal_metrics_suite"]}
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
  "comparison": {
    "baseline_checkpoint_count": ${BASELINE_CHECKPOINT_COUNT},
    "baseline_wal_frames_estimate": ${BASELINE_WAL_FRAMES},
    "adaptive_checkpoint_count": ${ADAPTIVE_CHECKPOINT_COUNT},
    "adaptive_wal_frames_estimate": ${ADAPTIVE_WAL_FRAMES},
    "adaptive_checkpoint_pressure_reduced": ${ADAPTIVE_CHECKPOINT_PRESSURE_REDUCED},
    "adaptive_wal_buffering_higher": ${ADAPTIVE_WAL_BUFFERING_HIGHER}
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
  echo "  checkpoint_core_suite: ${CASE_STATUS["checkpoint_core_suite"]}"
  echo "  checkpoint_burst_suite: ${CASE_STATUS["checkpoint_burst_suite"]}"
  echo "  checkpoint_wal_metrics_suite: ${CASE_STATUS["checkpoint_wal_metrics_suite"]}"
  echo "  clippy_core: ${CASE_STATUS["clippy_core"]}"
  echo "  baseline_checkpoint_count=${BASELINE_CHECKPOINT_COUNT}"
  echo "  adaptive_checkpoint_count=${ADAPTIVE_CHECKPOINT_COUNT}"
  echo "  baseline_wal_frames_estimate=${BASELINE_WAL_FRAMES}"
  echo "  adaptive_wal_frames_estimate=${ADAPTIVE_WAL_FRAMES}"
  echo "  report_path=${REPORT_FILE}"
  echo "  report_sha256=${REPORT_SHA}"
fi

if [ "$VERDICT" = "pass" ]; then
  exit 0
fi
exit 1
