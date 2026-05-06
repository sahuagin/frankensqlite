#!/usr/bin/env bash
# Verification gate for bd-db300.7.3:
# explicit exit criteria for leapfrog claims against SQLite.
#
# Deterministic replay:
#   bash scripts/verify_bd_db300_7_3_leapfrog_exit_criteria.sh

set -euo pipefail

BEAD_ID="bd-db300.7.3"
SCENARIO_ID="LEAPFROG-EXIT-CRITERIA-G3"
SEED=7300
TIMESTAMP_UTC="$(date -u +%Y%m%dT%H%M%SZ)"
RUN_ID="${BEAD_ID}-${TIMESTAMP_UTC}-${SEED}"
TRACE_ID="trace-${RUN_ID}"
ARTIFACT_DIR="artifacts/${BEAD_ID}/${RUN_ID}"
EVENTS_JSONL="${ARTIFACT_DIR}/events.jsonl"
REPORT_JSON="${ARTIFACT_DIR}/manifest.json"
SUMMARY_MD="${ARTIFACT_DIR}/summary.md"
METRIC_DICTIONARY_JSON="${ARTIFACT_DIR}/metric_dictionary.json"
THRESHOLDS_JSON="${ARTIFACT_DIR}/scorecard_thresholds.json"
TMP_REPORT_JSON="${REPORT_JSON}.tmp"
TMP_METRIC_DICTIONARY_JSON="${METRIC_DICTIONARY_JSON}.tmp"
TMP_THRESHOLDS_JSON="${THRESHOLDS_JSON}.tmp"
TEST_LOG="${ARTIFACT_DIR}/cargo-test.log"
CONTRACT_FILE="docs/contracts/leapfrog_exit_criteria.toml"
TEST_FILE="crates/fsqlite-harness/tests/bd_db300_7_3_leapfrog_exit_criteria.rs"

mkdir -p "${ARTIFACT_DIR}"

emit_event() {
  local phase="$1"
  local event_type="$2"
  local outcome="$3"
  local message="$4"
  printf '{"trace_id":"%s","run_id":"%s","scenario_id":"%s","seed":%d,"phase":"%s","event_type":"%s","outcome":"%s","timestamp":"%s","message":"%s"}\n' \
    "${TRACE_ID}" "${RUN_ID}" "${SCENARIO_ID}" "${SEED}" "${phase}" "${event_type}" "${outcome}" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "${message}" \
    >> "${EVENTS_JSONL}"
}

echo "=== ${BEAD_ID}: leapfrog exit criteria verification ==="
echo "run_id=${RUN_ID}"
echo "trace_id=${TRACE_ID}"
echo "scenario_id=${SCENARIO_ID}"

emit_event "bootstrap" "start" "running" "verification started"

if [[ ! -f "${CONTRACT_FILE}" ]]; then
  emit_event "contract_presence" "fail" "fail" "missing ${CONTRACT_FILE}"
  echo "missing ${CONTRACT_FILE}" >&2
  exit 1
fi
if [[ ! -f "${TEST_FILE}" ]]; then
  emit_event "test_presence" "fail" "fail" "missing ${TEST_FILE}"
  echo "missing ${TEST_FILE}" >&2
  exit 1
fi

emit_event "contract_schema" "start" "running" "validating contract schema"
export METRIC_DICTIONARY_JSON
export TMP_METRIC_DICTIONARY_JSON
export THRESHOLDS_JSON
export TMP_THRESHOLDS_JSON
export RUN_ID
export TRACE_ID
export SCENARIO_ID
export CONTRACT_FILE
export TEST_FILE
if ! python3 - <<'PY' > "${TMP_REPORT_JSON}"; then
import json
import os
import tomllib
from pathlib import Path

contract = tomllib.loads(Path(os.environ["CONTRACT_FILE"]).read_text(encoding="utf-8"))

required_meta = {
    "schema_version": "fsqlite-harness.leapfrog_exit_criteria.v1",
    "bead_id": "bd-db300.7.3",
}
required_modes = {
    "sqlite_reference",
    "fsqlite_mvcc",
    "fsqlite_single_writer",
}
required_unit_tests = {
    "test_bd_db300_7_3_contract_schema_and_links",
    "test_bd_db300_7_3_required_campaign_surface_exists",
    "test_bd_db300_7_3_cell_targets_are_monotone",
    "test_bd_db300_7_3_verification_plan_is_actionable",
    "test_bd_db300_7_3_operator_report_contract_is_actionable",
    "test_bd_db300_7_3_transferability_rubric_is_actionable",
    "test_bd_db300_7_3_workload_family_thresholds_are_actionable",
}
required_profiles = {
    "baseline_unpinned",
    "recommended_pinned",
    "adversarial_cross_node",
}
required_cell_suffixes = {"c1", "c4", "c8"}
required_scenarios = {
    "commutative_inserts_disjoint_keys_c1",
    "commutative_inserts_disjoint_keys_c4",
    "commutative_inserts_disjoint_keys_c8",
    "hot_page_contention_c1",
    "hot_page_contention_c4",
    "hot_page_contention_c8",
    "mixed_read_write_c1",
    "mixed_read_write_c4",
    "mixed_read_write_c8",
}
required_metric_families = {
    "throughput",
    "retry",
    "abort",
    "cpu_efficiency",
    "latency",
    "topology",
    "wait",
    "page_touch",
    "split_path",
    "allocator",
    "cache",
    "copy_allocation",
}
required_transferability_classes = [
    "transferable",
    "profile_specific_but_useful",
    "suspicious",
    "non_claimable",
]
required_hardware_classes = {
    "same_host",
    "same_topology_class",
    "cross_hardware_class",
}
required_downstream_beads = {"bd-db300.7.3", "bd-db300.7.4"}
required_operator_source_beads = {"bd-db300.7.5.5", "bd-db300.7.5.6", "bd-db300.7.6.4"}
required_operator_upstream_contracts = [
    "db300_regime_atlas_contract.toml",
    "db300_shadow_oracle_contract.toml",
    "db300_policy_snapshot_contract.toml",
]
required_operator_report_fields = [
    "activation_regime_id",
    "activation_state",
    "rollout_stage",
    "safe_by_default_boundary",
    "shadow_sample_rate",
    "kill_switch_state",
    "fallback_state",
    "rollout_annotation",
    "fallback_annotation",
    "user_visibility",
]
required_operator_bundle_artifacts = [
    "artifacts/{bead_id}/{run_id}/manifest.json",
    "artifacts/{bead_id}/{run_id}/summary.md",
    "artifacts/{bead_id}/{run_id}/scorecard_thresholds.json",
]
required_report_labels = {
    "transferable": "transferable win",
    "profile_specific_but_useful": "lab-specific win",
    "suspicious": "topology-sensitive win",
    "non_claimable": "no-catastrophic-regression failure",
}
for key, expected in required_meta.items():
    actual = contract["meta"].get(key)
    if actual != expected:
        raise SystemExit(f"meta.{key} expected {expected!r}, found {actual!r}")

actual_modes = set(contract["campaign"].get("required_modes", []))
if actual_modes != required_modes:
    raise SystemExit(
        f"campaign.required_modes mismatch expected={sorted(required_modes)!r} actual={sorted(actual_modes)!r}"
    )
actual_profiles = set(contract["campaign"].get("required_placement_profiles", []))
if actual_profiles != required_profiles:
    raise SystemExit(
        f"campaign.required_placement_profiles mismatch expected={sorted(required_profiles)!r} actual={sorted(actual_profiles)!r}"
    )
actual_cell_suffixes = set(contract["campaign"].get("required_cell_suffixes", []))
if actual_cell_suffixes != required_cell_suffixes:
    raise SystemExit(
        f"campaign.required_cell_suffixes mismatch expected={sorted(required_cell_suffixes)!r} actual={sorted(actual_cell_suffixes)!r}"
    )

cell_gates = contract.get("cell_gates", [])
if [gate.get("cell") for gate in cell_gates] != ["c1", "c4", "c8"]:
    raise SystemExit("cell_gates must appear in c1, c4, c8 order")
actual_scenarios = set(contract["verification_plan"].get("e2e_scenarios", []))
if actual_scenarios != required_scenarios:
    raise SystemExit(
        f"verification_plan.e2e_scenarios mismatch expected={sorted(required_scenarios)!r} actual={sorted(actual_scenarios)!r}"
    )
actual_unit_tests = set(contract["verification_plan"].get("unit_tests", []))
if actual_unit_tests != required_unit_tests:
    raise SystemExit(
        f"verification_plan.unit_tests mismatch expected={sorted(required_unit_tests)!r} actual={sorted(actual_unit_tests)!r}"
    )

required_artifacts = {
    "artifacts/{bead_id}/{run_id}/events.jsonl",
    "artifacts/{bead_id}/{run_id}/manifest.json",
    "artifacts/{bead_id}/{run_id}/summary.md",
    "artifacts/{bead_id}/{run_id}/metric_dictionary.json",
    "artifacts/{bead_id}/{run_id}/scorecard_thresholds.json",
    "artifacts/{bead_id}/{run_id}/cell_metrics.jsonl",
    "artifacts/{bead_id}/{run_id}/retry_report.json",
    "artifacts/{bead_id}/{run_id}/topology.json",
}
artifacts = set(contract["verification_plan"].get("logging_artifacts", []))
missing = sorted(required_artifacts - artifacts)
if missing:
    raise SystemExit(f"missing logging artifacts: {missing}")

metric_dictionary = contract.get("metric_dictionary", {})
metrics = metric_dictionary.get("metrics", [])
if not metrics:
    raise SystemExit("metric_dictionary.metrics must not be empty")
metric_ids = {metric["metric_id"] for metric in metrics}
metric_families = {metric["family"] for metric in metrics}
for field in contract["verification_plan"].get("required_log_fields", []):
    if field not in metric_ids:
        raise SystemExit(
            f"verification_plan.required_log_fields references undefined metric {field!r}"
        )
missing_metric_families = sorted(required_metric_families - metric_families)
if missing_metric_families:
    raise SystemExit(
        f"metric_dictionary.metrics missing required families {missing_metric_families!r}"
    )

rubric = contract.get("transferability_rubric", {})
if rubric.get("schema_version") != "fsqlite-harness.transferability_rubric.v1":
    raise SystemExit(
        f"transferability_rubric.schema_version mismatch actual={rubric.get('schema_version')!r}"
    )
if rubric.get("single_writer_role") != "comparison_or_fallback_only":
    raise SystemExit(
        f"transferability_rubric.single_writer_role mismatch actual={rubric.get('single_writer_role')!r}"
    )
actual_rubric_modes = set(rubric.get("required_modes", []))
if actual_rubric_modes != required_modes:
    raise SystemExit(
        f"transferability_rubric.required_modes mismatch expected={sorted(required_modes)!r} actual={sorted(actual_rubric_modes)!r}"
    )
actual_hardware_classes = set(rubric.get("required_hardware_classes", []))
if actual_hardware_classes != required_hardware_classes:
    raise SystemExit(
        f"transferability_rubric.required_hardware_classes mismatch expected={sorted(required_hardware_classes)!r} actual={sorted(actual_hardware_classes)!r}"
    )
if rubric.get("classification_order", []) != required_transferability_classes:
    raise SystemExit(
        f"transferability_rubric.classification_order mismatch expected={required_transferability_classes!r} actual={rubric.get('classification_order', [])!r}"
    )
actual_downstream_beads = set(rubric.get("downstream_beads", []))
if actual_downstream_beads != required_downstream_beads:
    raise SystemExit(
        f"transferability_rubric.downstream_beads mismatch expected={sorted(required_downstream_beads)!r} actual={sorted(actual_downstream_beads)!r}"
    )
classes = rubric.get("classes", [])
if [klass.get("classification_id") for klass in classes] != required_transferability_classes:
    raise SystemExit(
        "transferability_rubric.classes must appear in transferable/profile_specific_but_useful/suspicious/non_claimable order"
    )
covered_profiles = set()
for klass in classes:
    class_id = klass["classification_id"]
    expected_report_label = required_report_labels.get(class_id)
    if expected_report_label is None:
        raise SystemExit(f"unexpected transferability class {class_id!r}")
    if klass.get("final_report_label") != expected_report_label:
        raise SystemExit(
            f"class {class_id!r} final_report_label mismatch expected={expected_report_label!r} actual={klass.get('final_report_label')!r}"
        )
    if not klass.get("summary", "").strip():
        raise SystemExit(f"class {class_id!r} summary must not be blank")
    if not klass.get("placement_rule", "").strip():
        raise SystemExit(f"class {class_id!r} placement_rule must not be blank")
    if not klass.get("mode_rule", "").strip():
        raise SystemExit(f"class {class_id!r} mode_rule must not be blank")
    if not klass.get("hardware_rule", "").strip():
        raise SystemExit(f"class {class_id!r} hardware_rule must not be blank")
    if not klass.get("reporting_requirement", "").strip():
        raise SystemExit(f"class {class_id!r} reporting_requirement must not be blank")
    if not klass.get("example", "").strip():
        raise SystemExit(f"class {class_id!r} example must not be blank")
    example_profiles = set(klass.get("example_profiles", []))
    if not example_profiles:
        raise SystemExit(f"class {class_id!r} must reference example profiles")
    if not example_profiles.issubset(required_profiles):
        raise SystemExit(
            f"class {class_id!r} example_profiles mismatch expected subset of {sorted(required_profiles)!r} actual={sorted(example_profiles)!r}"
        )
    covered_profiles |= example_profiles
if covered_profiles != required_profiles:
    raise SystemExit(
        f"transferability_rubric example profile coverage mismatch expected={sorted(required_profiles)!r} actual={sorted(covered_profiles)!r}"
    )

operator_report = contract.get("operator_report_contract", {})
if operator_report.get("consumer_bead_id") != "bd-db300.7.4":
    raise SystemExit(
        f"operator_report_contract.consumer_bead_id mismatch actual={operator_report.get('consumer_bead_id')!r}"
    )
actual_operator_source_beads = set(operator_report.get("source_contract_beads", []))
if actual_operator_source_beads != required_operator_source_beads:
    raise SystemExit(
        f"operator_report_contract.source_contract_beads mismatch expected={sorted(required_operator_source_beads)!r} actual={sorted(actual_operator_source_beads)!r}"
    )
actual_operator_paths = operator_report.get("upstream_contract_paths", [])
if actual_operator_paths != required_operator_upstream_contracts:
    raise SystemExit(
        f"operator_report_contract.upstream_contract_paths mismatch expected={required_operator_upstream_contracts!r} actual={actual_operator_paths!r}"
    )
for rel_path in actual_operator_paths:
    if not Path(rel_path).exists():
        raise SystemExit(f"missing operator upstream contract {rel_path}")

for field_name in (
    "required_manifest_fields",
    "required_summary_fields",
    "required_threshold_fields",
):
    actual_fields = operator_report.get(field_name, [])
    if actual_fields != required_operator_report_fields:
        raise SystemExit(
            f"operator_report_contract.{field_name} mismatch expected={required_operator_report_fields!r} actual={actual_fields!r}"
        )

actual_operator_bundle_artifacts = operator_report.get("required_bundle_artifacts", [])
if actual_operator_bundle_artifacts != required_operator_bundle_artifacts:
    raise SystemExit(
        f"operator_report_contract.required_bundle_artifacts mismatch expected={required_operator_bundle_artifacts!r} actual={actual_operator_bundle_artifacts!r}"
    )
missing_operator_artifacts = sorted(set(actual_operator_bundle_artifacts) - artifacts)
if missing_operator_artifacts:
    raise SystemExit(
        f"operator_report_contract.required_bundle_artifacts references non-emitted artifacts {missing_operator_artifacts!r}"
    )

regime_contract = tomllib.loads(Path(actual_operator_paths[0]).read_text(encoding="utf-8"))
shadow_contract = tomllib.loads(Path(actual_operator_paths[1]).read_text(encoding="utf-8"))
policy_contract = tomllib.loads(Path(actual_operator_paths[2]).read_text(encoding="utf-8"))
if operator_report.get("default_rollout_stage") != policy_contract["global_defaults"]["default_rollout_stage"]:
    raise SystemExit(
        "operator_report_contract.default_rollout_stage must match policy snapshot default_rollout_stage"
    )
if operator_report.get("default_activation_state") != policy_contract["global_defaults"]["default_activation_state"]:
    raise SystemExit(
        "operator_report_contract.default_activation_state must match policy snapshot default_activation_state"
    )
if shadow_contract["global_defaults"]["default_shadow_mode"] != "off":
    raise SystemExit(
        "shadow oracle default_shadow_mode must stay off for this consumer contract"
    )
if operator_report.get("default_shadow_sample_rate") != "0%":
    raise SystemExit(
        f"operator_report_contract.default_shadow_sample_rate mismatch actual={operator_report.get('default_shadow_sample_rate')!r}"
    )
kill_switch_states = {
    entry["state_id"] for entry in policy_contract.get("kill_switch_state", [])
}
if operator_report.get("default_kill_switch_state") not in kill_switch_states:
    raise SystemExit(
        "operator_report_contract.default_kill_switch_state must be declared in policy snapshot kill_switch_state"
    )
if operator_report.get("default_fallback_state") != "inactive":
    raise SystemExit(
        f"operator_report_contract.default_fallback_state mismatch actual={operator_report.get('default_fallback_state')!r}"
    )
if "fallback_state=active" not in regime_contract["global_defaults"]["safe_mode_policy"]:
    raise SystemExit("regime atlas safe_mode_policy must keep fallback_state=active explicit")
regime_states = {entry["state_id"] for entry in regime_contract.get("activation_state", [])}
if not {"universal_default", "regime_gated_default"}.issubset(regime_states):
    raise SystemExit(
        "regime atlas must declare universal_default and regime_gated_default activation states"
    )
safe_by_default_boundary = operator_report.get("safe_by_default_boundary", "")
if "universal_default" not in safe_by_default_boundary or "regime_gated_default" not in safe_by_default_boundary:
    raise SystemExit(
        "operator_report_contract.safe_by_default_boundary must keep the regime-gated auto-enable boundary explicit"
    )
if not operator_report.get("default_activation_regime_id", "").strip():
    raise SystemExit("operator_report_contract.default_activation_regime_id must not be blank")
if operator_report.get("user_visibility") != "operator_visible_regime_gated_default":
    raise SystemExit(
        f"operator_report_contract.user_visibility mismatch actual={operator_report.get('user_visibility')!r}"
    )
if not operator_report.get("rollout_annotation", "").strip():
    raise SystemExit("operator_report_contract.rollout_annotation must not be blank")
if not operator_report.get("fallback_annotation", "").strip():
    raise SystemExit("operator_report_contract.fallback_annotation must not be blank")

metric_dictionary_path = Path(os.environ["TMP_METRIC_DICTIONARY_JSON"])
metric_dictionary_path.write_text(
    json.dumps(metric_dictionary, indent=2),
    encoding="utf-8",
)

thresholds_path = Path(os.environ["TMP_THRESHOLDS_JSON"])
thresholds_path.write_text(
    json.dumps(
        {
            "cell_gates": contract.get("cell_gates", []),
            "workload_families": contract.get("workload_families", []),
            "required_log_fields": contract["verification_plan"].get("required_log_fields", []),
            "operator_report": {
                "consumer_bead_id": operator_report["consumer_bead_id"],
                "source_contract_beads": operator_report["source_contract_beads"],
                "upstream_contract_paths": operator_report["upstream_contract_paths"],
                "required_fields": operator_report["required_threshold_fields"],
                "activation_regime_id": operator_report["default_activation_regime_id"],
                "activation_state": operator_report["default_activation_state"],
                "rollout_stage": operator_report["default_rollout_stage"],
                "safe_by_default_boundary": operator_report["safe_by_default_boundary"],
                "shadow_sample_rate": operator_report["default_shadow_sample_rate"],
                "kill_switch_state": operator_report["default_kill_switch_state"],
                "fallback_state": operator_report["default_fallback_state"],
                "rollout_annotation": operator_report["rollout_annotation"],
                "fallback_annotation": operator_report["fallback_annotation"],
                "user_visibility": operator_report["user_visibility"],
            },
        },
        indent=2,
    ),
    encoding="utf-8",
)

summary = {
    "bead_id": contract["meta"]["bead_id"],
    "policy_id": contract["meta"]["policy_id"],
    "required_modes": contract["campaign"]["required_modes"],
    "required_profiles": contract["campaign"]["required_placement_profiles"],
    "required_cells": [gate["cell"] for gate in cell_gates],
    "required_scenarios": contract["verification_plan"]["e2e_scenarios"],
    "transferability_classes": rubric["classification_order"],
    "claim_language": contract["scorecard"]["claim_language"],
    "metric_dictionary_path": os.environ["METRIC_DICTIONARY_JSON"],
    "metric_ids": sorted(metric_ids),
    "source_contract_beads": operator_report["source_contract_beads"],
    "activation_regime_id": operator_report["default_activation_regime_id"],
    "activation_state": operator_report["default_activation_state"],
    "rollout_stage": operator_report["default_rollout_stage"],
    "safe_by_default_boundary": operator_report["safe_by_default_boundary"],
    "shadow_sample_rate": operator_report["default_shadow_sample_rate"],
    "kill_switch_state": operator_report["default_kill_switch_state"],
    "fallback_state": operator_report["default_fallback_state"],
    "rollout_annotation": operator_report["rollout_annotation"],
    "fallback_annotation": operator_report["fallback_annotation"],
    "user_visibility": operator_report["user_visibility"],
    "operator_report_contract": {
        "consumer_bead_id": operator_report["consumer_bead_id"],
        "upstream_contract_paths": operator_report["upstream_contract_paths"],
        "required_manifest_fields": operator_report["required_manifest_fields"],
        "required_summary_fields": operator_report["required_summary_fields"],
        "required_threshold_fields": operator_report["required_threshold_fields"],
    },
}
print(json.dumps(summary, indent=2))
PY
  emit_event "contract_schema" "fail" "fail" "contract schema validation failed"
  exit 1
fi
if ! mv "${TMP_REPORT_JSON}" "${REPORT_JSON}"; then
  emit_event "contract_schema" "fail" "fail" "failed to publish manifest.json"
  exit 1
fi
if ! mv "${TMP_METRIC_DICTIONARY_JSON}" "${METRIC_DICTIONARY_JSON}"; then
  emit_event "contract_schema" "fail" "fail" "failed to publish metric_dictionary.json"
  exit 1
fi
if ! mv "${TMP_THRESHOLDS_JSON}" "${THRESHOLDS_JSON}"; then
  emit_event "contract_schema" "fail" "fail" "failed to publish scorecard_thresholds.json"
  exit 1
fi
emit_event "contract_schema" "pass" "pass" "contract schema validated"

export REPORT_JSON
export SUMMARY_MD
python3 - <<'PY'
import json
import os
from pathlib import Path

report = json.loads(Path(os.environ["REPORT_JSON"]).read_text(encoding="utf-8"))
summary_lines = [
    f"# {report['bead_id']} Verification Summary",
    "",
    f"- run_id: `{os.environ['RUN_ID']}`",
    f"- trace_id: `{os.environ['TRACE_ID']}`",
    f"- scenario_id: `{os.environ['SCENARIO_ID']}`",
    f"- contract: `{os.environ['CONTRACT_FILE']}`",
    f"- test: `{os.environ['TEST_FILE']}`",
    f"- report: `{os.environ['REPORT_JSON']}`",
    f"- metric_dictionary: `{os.environ['METRIC_DICTIONARY_JSON']}`",
    f"- scorecard_thresholds: `{os.environ['THRESHOLDS_JSON']}`",
    "",
    "## Operator Report Surface",
    "",
    f"- activation_regime_id: `{report['activation_regime_id']}`",
    f"- activation_state: `{report['activation_state']}`",
    f"- rollout_stage: `{report['rollout_stage']}`",
    f"- safe_by_default_boundary: `{report['safe_by_default_boundary']}`",
    f"- shadow_sample_rate: `{report['shadow_sample_rate']}`",
    f"- kill_switch_state: `{report['kill_switch_state']}`",
    f"- fallback_state: `{report['fallback_state']}`",
    f"- rollout_annotation: `{report['rollout_annotation']}`",
    f"- fallback_annotation: `{report['fallback_annotation']}`",
    f"- user_visibility: `{report['user_visibility']}`",
]
Path(os.environ["SUMMARY_MD"]).write_text("\n".join(summary_lines) + "\n", encoding="utf-8")
PY

emit_event "cargo_test" "start" "running" "running harness test via rch"
if ! rch exec -- cargo test --package fsqlite-harness --test bd_db300_7_3_leapfrog_exit_criteria -- --nocapture 2>&1 | tee "${TEST_LOG}"; then
  emit_event "cargo_test" "fail" "fail" "harness test failed"
  exit 1
fi
emit_event "cargo_test" "pass" "pass" "harness test passed"

emit_event "complete" "finish" "pass" "verification completed"
echo "[GATE PASS] ${BEAD_ID} leapfrog exit criteria verification passed"
