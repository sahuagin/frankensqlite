#!/usr/bin/env bash
set -euo pipefail

BEAD_ID="bd-1dp9.6.7.9.2"
RUN_ID="${BEAD_ID}-$(date -u +%Y%m%dT%H%M%SZ)"
ARTIFACT_DIR="artifacts/${BEAD_ID}/${RUN_ID}"
TARGET_DIR="${TMPDIR:-/tmp}/rch_target_bd_1dp9_6_7_9_2"
COORD_LOG="${ARTIFACT_DIR}/checkpoint_coordination_tests.log"
BUSY_LOG="${ARTIFACT_DIR}/checkpoint_busy_tests.log"
SHA_LOG="${ARTIFACT_DIR}/sha256.txt"
REPORT_JSON="${ARTIFACT_DIR}/gate_report.json"

mkdir -p "${ARTIFACT_DIR}"

coord_cmd=(
  rch exec -- env "CARGO_TARGET_DIR=${TARGET_DIR}"
  cargo test -p fsqlite-pager checkpoint_coordination -- --nocapture
)
busy_cmd=(
  rch exec -- env "CARGO_TARGET_DIR=${TARGET_DIR}"
  cargo test -p fsqlite-pager checkpoint_busy_with_active_ -- --nocapture
)

printf '+'
printf ' %q' "${coord_cmd[@]}"
printf '\n'
"${coord_cmd[@]}" 2>&1 | tee "${COORD_LOG}"

printf '+'
printf ' %q' "${busy_cmd[@]}"
printf '\n'
"${busy_cmd[@]}" 2>&1 | tee "${BUSY_LOG}"

rg -q 'checkpoint_phase="active_gate".*foreground_phase="begin".*interaction_rule="checkpoint_excludes_new_transactions"' "${COORD_LOG}"
rg -q 'checkpoint_phase="backend_owned".*foreground_phase="foreground_idle".*foreground_action="checkpoint_runs"' "${COORD_LOG}"
rg -q 'checkpoint_phase="complete".*foreground_action="checkpoint_complete".*pending_batch_count=0' "${COORD_LOG}"
rg -q 'checkpoint_phase="error".*foreground_action="checkpoint_failed".*pending_batch_count=0' "${COORD_LOG}"
rg -q 'checkpoint_phase="writer_gate".*foreground_phase="checkpoint_begin".*interaction_rule="checkpoint_waits_for_foreground_writers_to_quiesce".*pending_batch_count=0' "${BUSY_LOG}"

sha256sum "${COORD_LOG}" "${BUSY_LOG}" | tee "${SHA_LOG}" >/dev/null

jq -n \
  --arg bead_id "${BEAD_ID}" \
  --arg run_id "${RUN_ID}" \
  --arg coord_log "${COORD_LOG}" \
  --arg busy_log "${BUSY_LOG}" \
  --arg coord_cmd "${coord_cmd[*]}" \
  --arg busy_cmd "${busy_cmd[*]}" \
  --arg sha_log "${SHA_LOG}" \
  '{
    bead_id: $bead_id,
    run_id: $run_id,
    coordination_log: $coord_log,
    busy_gate_log: $busy_log,
    replay_commands: {
      checkpoint_coordination: $coord_cmd,
      checkpoint_busy_gate: $busy_cmd
    },
    artifacts: {
      sha256: $sha_log
    },
    assertions: [
      "checkpoint_active gate blocks new foreground writers",
      "checkpoint acquires backend ownership only after the foreground lane is idle",
      "checkpoint success path reports idle queue and completion fields",
      "checkpoint error path preserves idle queue and exposes failure context",
      "active foreground writers block checkpoint entry with explicit writer_gate tracing"
    ]
  }' > "${REPORT_JSON}"

echo "[GATE PASS] ${BEAD_ID} checkpoint decoupling verification passed"
echo "Artifacts: ${ARTIFACT_DIR}"
