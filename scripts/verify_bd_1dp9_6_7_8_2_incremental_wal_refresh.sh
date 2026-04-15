#!/usr/bin/env bash
# verify_bd_1dp9_6_7_8_2_incremental_wal_refresh.sh
#
# E2E verification script for bd-1dp9.6.7.8.2:
# "Incremental refresh and steady-state lookup cutover off reverse scans"
#
# Tests:
# - Steady-state authoritative index lookup
# - Incremental refresh scaling with appended frames
# - Generation reset index invalidation
# - Cross-connection visibility with refresh
# - Many-pages performance comparison
#
# Usage:
#   ./scripts/verify_bd_1dp9_6_7_8_2_incremental_wal_refresh.sh
#
# Environment variables:
#   FSQLITE_VERBOSE=1  - Show full test output
#   FSQLITE_TRACE=1    - Enable trace-level logging

set -euo pipefail

BEAD_ID="bd-1dp9.6.7.8.2"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TEST_NAME="bd_1dp9_6_7_8_2_incremental_wal_refresh"
ARTIFACT_DIR="${PROJECT_ROOT}/target/verification/${BEAD_ID}"

cd "$PROJECT_ROOT"

echo "=== ${BEAD_ID}: Incremental WAL Refresh E2E Verification ==="
echo "Project root: $PROJECT_ROOT"
echo "Artifact dir: $ARTIFACT_DIR"
echo ""

mkdir -p "$ARTIFACT_DIR"

# Set up logging environment
export RUST_LOG="${RUST_LOG:-fsqlite=debug,fsqlite.wal_publication=trace}"
if [[ "${FSQLITE_TRACE:-}" == "1" ]]; then
    export RUST_LOG="trace"
fi

# Run the E2E tests
echo "Running E2E tests..."
TEST_OUTPUT="${ARTIFACT_DIR}/test_output.txt"

if [[ "${FSQLITE_VERBOSE:-}" == "1" ]]; then
    cargo test -p fsqlite-e2e --test "$TEST_NAME" -- --nocapture --test-threads=1 2>&1 | tee "$TEST_OUTPUT"
else
    cargo test -p fsqlite-e2e --test "$TEST_NAME" -- --nocapture --test-threads=1 > "$TEST_OUTPUT" 2>&1 || {
        echo "FAIL: E2E tests failed"
        echo "=== Test Output ==="
        cat "$TEST_OUTPUT"
        exit 1
    }
fi

# Extract test results
echo ""
echo "=== Test Results ==="
grep -E "^(INFO|WARN|ERROR|test result:|running [0-9]+ test)" "$TEST_OUTPUT" || true

# Check for any failures
if grep -q "FAILED" "$TEST_OUTPUT"; then
    echo ""
    echo "FAIL: Some tests failed"
    exit 1
fi

if grep -q "test result: ok" "$TEST_OUTPUT"; then
    echo ""
    echo "PASS: All ${BEAD_ID} E2E tests passed"
else
    echo ""
    echo "WARN: Could not confirm test results"
    exit 1
fi

# Generate summary artifact
SUMMARY_FILE="${ARTIFACT_DIR}/summary.json"
cat > "$SUMMARY_FILE" << EOF
{
    "bead_id": "${BEAD_ID}",
    "title": "Incremental refresh and steady-state lookup cutover off reverse scans",
    "status": "PASS",
    "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
    "tests": [
        "bd_1dp9_6_7_8_2_steady_state_authoritative_lookup",
        "bd_1dp9_6_7_8_2_incremental_refresh_scales_with_new_frames",
        "bd_1dp9_6_7_8_2_generation_reset_invalidates_index",
        "bd_1dp9_6_7_8_2_cross_connection_visibility_with_refresh",
        "bd_1dp9_6_7_8_2_many_pages_authoritative_index_performance"
    ],
    "acceptance_criteria": {
        "authoritative_index_lookup": true,
        "incremental_refresh_scaling": true,
        "generation_reset_handling": true,
        "structured_logging": true
    },
    "replay_command": "cargo test -p fsqlite-e2e --test bd_1dp9_6_7_8_2_incremental_wal_refresh -- --nocapture --test-threads=1",
    "artifact_path": "${ARTIFACT_DIR}"
}
EOF

echo ""
echo "Summary written to: $SUMMARY_FILE"
echo ""
echo "=== Verification Complete ==="
