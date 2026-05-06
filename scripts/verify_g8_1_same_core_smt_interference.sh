#!/usr/bin/env bash
# Verification gate for bd-db300.7.8.1:
# same-core / SMT-sibling / same-LLC interference suite contract and first
# replayable case-resolution slice.

set -euo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BEAD_ID="bd-db300.7.8.1"
CONTRACT_PATH="${WORKSPACE_ROOT}/docs/contracts/db300_topology_interference_contract.toml"
TOPOLOGY_BUNDLE_SCRIPT="${WORKSPACE_ROOT}/scripts/verify_bd_db300_1_6_1_topology_bundle.sh"
SCENARIO_ID="${SCENARIO_ID:-G8-1-TOPOLOGY-INTERFERENCE}"
SEED="${SEED:-781}"
TIMESTAMP_UTC="$(date -u +%Y%m%dT%H%M%SZ)"
RUN_ID="${RUN_ID:-${BEAD_ID}-${TIMESTAMP_UTC}-${SEED}}"
TRACE_ID="${TRACE_ID:-trace-${RUN_ID}}"
PLACEMENT_PROFILE_ID="${PLACEMENT_PROFILE_ID:-recommended_pinned}"
HARDWARE_CLASS_ID="${HARDWARE_CLASS_ID:-linux_x86_64_many_core_numa}"
PRIMITIVE_CLASS="${PRIMITIVE_CLASS:-topology_interference_smoke}"
ARTIFACT_DIR="${ARTIFACT_DIR:-${WORKSPACE_ROOT}/artifacts/${BEAD_ID}/${RUN_ID}}"
ARTIFACT_RELPATH="${ARTIFACT_RELPATH:-${ARTIFACT_DIR#"${WORKSPACE_ROOT}/"}}"
TOPOLOGY_DIR="${ARTIFACT_DIR}/topology_bundle"
CASE_MATRIX_JSON="${ARTIFACT_DIR}/case_matrix.json"
STRUCTURED_LOGS_JSONL="${ARTIFACT_DIR}/structured_logs.ndjson"
SUMMARY_MD="${ARTIFACT_DIR}/summary.md"
MANIFEST_JSON="${ARTIFACT_DIR}/manifest.json"
RERUN_SH="${ARTIFACT_DIR}/rerun_entrypoint.sh"
FIRST_FAILURE_JSON="${ARTIFACT_DIR}/first_failure.json"
TEST_LOG="${ARTIFACT_DIR}/contract_test.log"
USE_RCH="${USE_RCH:-0}"
SKIP_CONTRACT_TEST="${SKIP_CONTRACT_TEST:-0}"
CARGO_TARGET_DIR_BASE="${CARGO_TARGET_DIR_BASE:-${WORKSPACE_ROOT}/.codex-target/g8_1_topology_interference}"

default_source_revision() {
  git -C "${WORKSPACE_ROOT}" rev-parse --short=12 HEAD 2>/dev/null || printf 'unknown'
}

default_beads_hash() {
  if [[ -f "${WORKSPACE_ROOT}/.beads/issues.jsonl" ]]; then
    sha256sum "${WORKSPACE_ROOT}/.beads/issues.jsonl" | awk '{print $1}'
  else
    printf 'unavailable'
  fi
}

SOURCE_REVISION="${SOURCE_REVISION:-$(default_source_revision)}"
BEADS_HASH="${BEADS_HASH:-$(default_beads_hash)}"
CLAIM_ID="${CLAIM_ID:-claim-g8_1-topology-interference}"
EVIDENCE_ROOT="${EVIDENCE_ROOT:-${ARTIFACT_RELPATH}}"

mkdir -p "${ARTIFACT_DIR}" "${TOPOLOGY_DIR}"
: > "${STRUCTURED_LOGS_JSONL}"

export NO_COLOR="${NO_COLOR:-1}"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"

run_contract_test() {
  local started finished elapsed
  local -a cmd=(
    env
    "CARGO_TARGET_DIR=${CARGO_TARGET_DIR_BASE}"
    "CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS}"
    "NO_COLOR=${NO_COLOR}"
    cargo test -p fsqlite-harness --test bd_db300_7_8_1_same_core_smt_interference_contract -- --nocapture
  )
  if [[ "${USE_RCH}" == "1" ]]; then
    cmd=(rch exec -- "${cmd[@]}")
  fi

  started="$(date +%s%3N)"
  if (
    cd "${WORKSPACE_ROOT}"
    "${cmd[@]}"
  ) 2>&1 | tee "${TEST_LOG}"; then
    finished="$(date +%s%3N)"
    elapsed="$((finished - started))"
    if ! grep -Eq '^test result: ok\.' "${TEST_LOG}"; then
      printf '{"schema_version":"fsqlite.db300.topology_interference_event.v1","trace_id":"%s","scenario_id":"%s","bead_id":"%s","run_id":"%s","phase":"contract_test","event_type":"fail","outcome":"fail","timestamp":"%s","message":"contract test completed without a passing test result","surface_id":"g8_1_topology_interference_suite","pillar_id":"G8","event_family":"verification_bundle_summary","claim_id":"%s","evidence_id":"contract_test","evidence_root":"%s","placement_profile_id":"%s","hardware_class_id":"%s","hardware_signature":"pending","source_revision":"%s","beads_data_hash":"%s"}\n' \
        "${TRACE_ID}" "${SCENARIO_ID}" "${BEAD_ID}" "${RUN_ID}" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
        "${CLAIM_ID}" "${EVIDENCE_ROOT}" "${PLACEMENT_PROFILE_ID}" "${HARDWARE_CLASS_ID}" \
        "${SOURCE_REVISION}" "${BEADS_HASH}" >> "${STRUCTURED_LOGS_JSONL}"
      return 1
    fi
    printf 'contract_test_elapsed_ms=%s\n' "${elapsed}" > "${ARTIFACT_DIR}/contract_test_metrics.txt"
  else
    return 1
  fi
}

capture_topology_bundle() {
  (
    cd "${WORKSPACE_ROOT}"
    RUN_ID="${RUN_ID}" \
    SOURCE_REVISION="${SOURCE_REVISION}" \
    BEADS_HASH="${BEADS_HASH}" \
    PLACEMENT_PROFILE_ID="${PLACEMENT_PROFILE_ID}" \
    HARDWARE_CLASS_ID="${HARDWARE_CLASS_ID}" \
    ARTIFACT_BUNDLE_DIR="${TOPOLOGY_DIR}" \
    ARTIFACT_BUNDLE_RELPATH="${TOPOLOGY_DIR#"${WORKSPACE_ROOT}/"}" \
    bash "${TOPOLOGY_BUNDLE_SCRIPT}"
  )
}

render_contract_artifacts() {
  python3 - "${CONTRACT_PATH}" "${TOPOLOGY_DIR}/hardware_discovery_bundle.json" \
    "${CASE_MATRIX_JSON}" "${STRUCTURED_LOGS_JSONL}" "${SUMMARY_MD}" "${MANIFEST_JSON}" \
    "${RERUN_SH}" "${FIRST_FAILURE_JSON}" "${TRACE_ID}" "${SCENARIO_ID}" "${RUN_ID}" \
    "${BEAD_ID}" "${PLACEMENT_PROFILE_ID}" "${HARDWARE_CLASS_ID}" "${PRIMITIVE_CLASS}" \
    "${SOURCE_REVISION}" "${BEADS_HASH}" "${CLAIM_ID}" "${EVIDENCE_ROOT}" "${ARTIFACT_RELPATH}" <<'PY'
from __future__ import annotations

import json
import sys
import tomllib
from datetime import datetime, timezone
from pathlib import Path


def expand_cpu_list(raw: str | None) -> list[int]:
    if raw is None:
        return []
    cpus: list[int] = []
    normalized = raw.replace("\n", ",").replace("\t", ",").replace(" ", ",")
    for chunk in normalized.split(","):
        part = chunk.strip()
        if not part:
            continue
        if "-" in part:
            start_raw, end_raw = part.split("-", 1)
            if start_raw.isdigit() and end_raw.isdigit():
                cpus.extend(range(int(start_raw), int(end_raw) + 1))
        elif part.isdigit():
            cpus.append(int(part))
    return sorted(set(cpus))


def payload_value(container: dict[str, object], *keys: str):
    current: object = container
    for key in keys:
        if not isinstance(current, dict):
            return None
        current = current.get(key)
    if not isinstance(current, dict):
        return None
    return current.get("value")


def llc_for_cpu(llc_domains: list[dict[str, object]], cpu: int) -> str | None:
    for domain in llc_domains:
        raw = domain.get("shared_cpu_list")
        if isinstance(raw, str) and cpu in expand_cpu_list(raw):
            return raw
    return None


def llc_for_pair(llc_domains: list[dict[str, object]], cpu_a: int, cpu_b: int) -> str | None:
    for domain in llc_domains:
        raw = domain.get("shared_cpu_list")
        if not isinstance(raw, str):
            continue
        cpus = expand_cpu_list(raw)
        if cpu_a in cpus and cpu_b in cpus:
            return raw
    return None


def build_sibling_maps(entries: list[dict[str, object]]) -> tuple[dict[int, tuple[int, ...]], list[tuple[int, ...]]]:
    cpu_to_group: dict[int, tuple[int, ...]] = {}
    groups: list[tuple[int, ...]] = []
    for entry in entries:
        raw = entry.get("cpus")
        if not isinstance(raw, str):
            continue
        group = tuple(expand_cpu_list(raw))
        if not group:
            continue
        groups.append(group)
        for cpu in group:
            cpu_to_group[cpu] = group
    return cpu_to_group, groups


(
    contract_path_raw,
    topology_bundle_path_raw,
    case_matrix_path_raw,
    logs_path_raw,
    summary_path_raw,
    manifest_path_raw,
    rerun_path_raw,
    first_failure_path_raw,
    trace_id,
    scenario_id,
    run_id,
    bead_id,
    placement_profile_id,
    hardware_class_id,
    primitive_class,
    source_revision,
    beads_hash,
    claim_id,
    evidence_root,
    artifact_bundle_relpath,
) = sys.argv[1:21]

contract_path = Path(contract_path_raw)
topology_bundle_path = Path(topology_bundle_path_raw)
case_matrix_path = Path(case_matrix_path_raw)
logs_path = Path(logs_path_raw)
summary_path = Path(summary_path_raw)
manifest_path = Path(manifest_path_raw)
rerun_path = Path(rerun_path_raw)
first_failure_path = Path(first_failure_path_raw)

with contract_path.open("rb") as handle:
    contract = tomllib.load(handle)
with topology_bundle_path.open("r", encoding="utf-8") as handle:
    topology_bundle = json.load(handle)

hardware_signature = str(topology_bundle["hardware_signature"])
online_cpus = expand_cpu_list(payload_value(topology_bundle, "cpu", "topology", "online_cpus"))
thread_siblings_raw = payload_value(topology_bundle, "cpu", "topology", "thread_siblings")
llc_domains_raw = payload_value(topology_bundle, "cpu", "topology", "llc_domains")
thread_siblings = thread_siblings_raw if isinstance(thread_siblings_raw, list) else []
llc_domains = llc_domains_raw if isinstance(llc_domains_raw, list) else []
cpu_to_sibling_group, sibling_groups = build_sibling_maps(thread_siblings)

cases: list[dict[str, object]] = []

if online_cpus:
    cpu = online_cpus[0]
    cases.append(
        {
            "case_id": "same_core_serialized",
            "case_kind": "same_core",
            "availability": "available",
            "unavailability_reason": None,
            "cpu_pair": [cpu, cpu],
            "cpu_pair_signature": f"{cpu}:{cpu}",
            "llc_domain": llc_for_cpu(llc_domains, cpu),
            "smt_relationship": "same_logical_cpu",
        }
    )
else:
    cases.append(
        {
            "case_id": "same_core_serialized",
            "case_kind": "same_core",
            "availability": "unavailable",
            "unavailability_reason": "no_online_cpus_in_topology_bundle",
            "cpu_pair": [],
            "cpu_pair_signature": None,
            "llc_domain": None,
            "smt_relationship": "same_logical_cpu",
        }
    )

available_smt: dict[str, object] | None = None
for group in sibling_groups:
    if len(group) >= 2:
        cpu_a, cpu_b = group[0], group[1]
        available_smt = {
            "case_id": "smt_sibling_pair",
            "case_kind": "smt_sibling",
            "availability": "available",
            "unavailability_reason": None,
            "cpu_pair": [cpu_a, cpu_b],
            "cpu_pair_signature": f"{cpu_a}:{cpu_b}",
            "llc_domain": llc_for_pair(llc_domains, cpu_a, cpu_b),
            "smt_relationship": "siblings_same_core",
        }
        break
if available_smt is None:
    available_smt = {
        "case_id": "smt_sibling_pair",
        "case_kind": "smt_sibling",
        "availability": "unavailable",
        "unavailability_reason": "no_thread_siblings_pair_with_two_distinct_cpus",
        "cpu_pair": [],
        "cpu_pair_signature": None,
        "llc_domain": None,
        "smt_relationship": "siblings_same_core",
    }
cases.append(available_smt)

available_llc: dict[str, object] | None = None
for domain in llc_domains:
    raw = domain.get("shared_cpu_list")
    if not isinstance(raw, str):
        continue
    domain_cpus = expand_cpu_list(raw)
    for index, cpu_a in enumerate(domain_cpus):
        for cpu_b in domain_cpus[index + 1 :]:
            group_a = cpu_to_sibling_group.get(cpu_a)
            group_b = cpu_to_sibling_group.get(cpu_b)
            if group_a is not None and group_b is not None and group_a == group_b:
                continue
            available_llc = {
                "case_id": "same_llc_diff_core_pair",
                "case_kind": "same_llc",
                "availability": "available",
                "unavailability_reason": None,
                "cpu_pair": [cpu_a, cpu_b],
                "cpu_pair_signature": f"{cpu_a}:{cpu_b}",
                "llc_domain": raw,
                "smt_relationship": "not_siblings_same_llc",
            }
            break
        if available_llc is not None:
            break
    if available_llc is not None:
        break
if available_llc is None:
    available_llc = {
        "case_id": "same_llc_diff_core_pair",
        "case_kind": "same_llc",
        "availability": "unavailable",
        "unavailability_reason": "no_same_llc_non_sibling_cpu_pair",
        "cpu_pair": [],
        "cpu_pair_signature": None,
        "llc_domain": None,
        "smt_relationship": "not_siblings_same_llc",
    }
cases.append(available_llc)

case_matrix = {
    "schema_version": contract["global_defaults"]["case_matrix_schema_id"],
    "bead_id": bead_id,
    "run_id": run_id,
    "trace_id": trace_id,
    "scenario_id": scenario_id,
    "placement_profile_id": placement_profile_id,
    "hardware_class_id": hardware_class_id,
    "hardware_signature": hardware_signature,
    "primitive_class": primitive_class,
    "cases": cases,
}
case_matrix_path.write_text(json.dumps(case_matrix, indent=2, sort_keys=True) + "\n", encoding="utf-8")

common_fields = list(contract["structured_log_common_fields"]["required_fields"])
case_fields = list(contract["structured_log_case_fields"]["required_fields"])
measurement_fields = list(contract["structured_log_measurement_fields"]["required_fields"])

events: list[dict[str, object]] = []
timestamp = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
for case in cases:
    event = {
        "schema_version": contract["global_defaults"]["event_schema_id"],
        "trace_id": trace_id,
        "scenario_id": scenario_id,
        "bead_id": bead_id,
        "run_id": run_id,
        "phase": "case_resolution",
        "event_type": "case_resolved",
        "outcome": "pass" if case["availability"] == "available" else "unavailable",
        "timestamp": timestamp,
        "message": f"resolved {case['case_id']}",
        "surface_id": contract["global_defaults"]["default_surface_id"],
        "pillar_id": contract["global_defaults"]["default_pillar_id"],
        "event_family": "topology_case_resolution",
        "claim_id": claim_id,
        "evidence_id": f"evidence-{case['case_id']}",
        "evidence_root": evidence_root,
        "placement_profile_id": placement_profile_id,
        "hardware_class_id": hardware_class_id,
        "hardware_signature": hardware_signature,
        "source_revision": source_revision,
        "beads_data_hash": beads_hash,
        "case_id": case["case_id"],
        "case_kind": case["case_kind"],
        "primitive_class": primitive_class,
        "cpu_pair": case["cpu_pair"],
        "cpu_pair_signature": case["cpu_pair_signature"],
        "llc_domain": case["llc_domain"],
        "smt_relationship": case["smt_relationship"],
        "availability": case["availability"],
    }
    if case["unavailability_reason"] is not None:
        event["unavailability_reason"] = case["unavailability_reason"]
    events.append(event)

summary_event = {
    "schema_version": contract["global_defaults"]["event_schema_id"],
    "trace_id": trace_id,
    "scenario_id": scenario_id,
    "bead_id": bead_id,
    "run_id": run_id,
    "phase": "bundle_summary",
    "event_type": "bundle_complete",
    "outcome": "pass",
    "timestamp": timestamp,
    "message": "rendered g8.1 topology case-resolution bundle",
    "surface_id": contract["global_defaults"]["default_surface_id"],
    "pillar_id": contract["global_defaults"]["default_pillar_id"],
    "event_family": "verification_bundle_summary",
    "claim_id": claim_id,
    "evidence_id": "evidence-bundle-summary",
    "evidence_root": evidence_root,
    "placement_profile_id": placement_profile_id,
    "hardware_class_id": hardware_class_id,
    "hardware_signature": hardware_signature,
    "source_revision": source_revision,
    "beads_data_hash": beads_hash,
}
events.append(summary_event)
logs_path.write_text(
    "".join(json.dumps(event, sort_keys=True) + "\n" for event in events),
    encoding="utf-8",
)

manifest = {
    "schema_version": contract["global_defaults"]["artifact_manifest_schema_id"],
    "bead_id": bead_id,
    "run_id": run_id,
    "trace_id": trace_id,
    "scenario_id": scenario_id,
    "operator_entrypoint": contract["meta"]["operator_entrypoint"],
    "contract_path": f"docs/contracts/{contract_path.name}",
    "topology_bundle_script": contract["meta"]["topology_bundle_contract_ref"],
    "case_matrix_path": case_matrix_path.name,
    "structured_logs_path": logs_path.name,
    "summary_path": summary_path.name,
    "rerun_entrypoint_path": rerun_path.name,
    "artifact_bundle_relpath": artifact_bundle_relpath,
    "hardware_signature": hardware_signature,
    "required_log_families": list(contract["first_slice"]["required_event_families"]),
    "required_log_fields": {
        "topology_case_resolution": common_fields + case_fields,
        "topology_interference_measurement": common_fields + case_fields + measurement_fields,
        "verification_bundle_summary": common_fields,
    },
    "artifact_names": list(contract["artifact_layout"]["required_files"]),
}
manifest_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")

summary_lines = [
    f"# {bead_id} Topology Interference Case Resolution",
    "",
    f"- run_id: `{run_id}`",
    f"- trace_id: `{trace_id}`",
    f"- scenario_id: `{scenario_id}`",
    f"- primitive_class: `{primitive_class}`",
    f"- placement_profile_id: `{placement_profile_id}`",
    f"- hardware_class_id: `{hardware_class_id}`",
    f"- hardware_signature: `{hardware_signature}`",
    "",
    "## Case Matrix",
    "",
]
for case in cases:
    cpu_pair = ",".join(str(cpu) for cpu in case["cpu_pair"]) if case["cpu_pair"] else "unresolved"
    llc_domain = case["llc_domain"] if case["llc_domain"] is not None else "unresolved"
    line = (
        f"- `{case['case_id']}`: availability=`{case['availability']}`, "
        f"cpu_pair=`{cpu_pair}`, llc_domain=`{llc_domain}`, "
        f"smt_relationship=`{case['smt_relationship']}`"
    )
    if case["unavailability_reason"] is not None:
        line += f", reason=`{case['unavailability_reason']}`"
    summary_lines.append(line)
summary_lines.extend(
    [
        "",
        "## Replay",
        "",
        f"- rerun_entrypoint: `{rerun_path.name}`",
        f"- topology_bundle: `topology_bundle/{topology_bundle_path.name}`",
        f"- structured_logs: `{logs_path.name}`",
    ]
)
summary_path.write_text("\n".join(summary_lines) + "\n", encoding="utf-8")

rerun_lines = [
    "#!/usr/bin/env bash",
    "set -euo pipefail",
    'WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"',
    f'export PLACEMENT_PROFILE_ID="${placement_profile_id}"',
    f'export HARDWARE_CLASS_ID="{hardware_class_id}"',
    f'export PRIMITIVE_CLASS="{primitive_class}"',
    f'export SOURCE_REVISION="{source_revision}"',
    f'export BEADS_HASH="{beads_hash}"',
    'exec "${WORKSPACE_ROOT}/scripts/verify_g8_1_same_core_smt_interference.sh" "$@"',
]
rerun_path.write_text("\n".join(rerun_lines) + "\n", encoding="utf-8")
rerun_path.chmod(0o755)

failures = [case for case in cases if case["availability"] == "unavailable"]
if failures:
    first_failure = {
        "schema_version": contract["global_defaults"]["artifact_manifest_schema_id"],
        "trace_id": trace_id,
        "scenario_id": scenario_id,
        "bead_id": bead_id,
        "run_id": run_id,
        "first_failure_summary": f"{failures[0]['case_id']} unavailable on this host",
        "first_failure_stage": "case_resolution",
        "first_failure_artifact": case_matrix_path.name,
        "diagnostic_json_pointer": f"/cases/{cases.index(failures[0])}",
        "replay_command": "bash scripts/verify_g8_1_same_core_smt_interference.sh",
    }
    first_failure_path.write_text(
        json.dumps(first_failure, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
PY
}

if [[ "${SKIP_CONTRACT_TEST}" != "1" ]]; then
  run_contract_test
fi
capture_topology_bundle
render_contract_artifacts

printf 'manifest_json=%s\n' "${MANIFEST_JSON}"
printf 'case_matrix_json=%s\n' "${CASE_MATRIX_JSON}"
printf 'structured_logs=%s\n' "${STRUCTURED_LOGS_JSONL}"
printf 'summary_md=%s\n' "${SUMMARY_MD}"
