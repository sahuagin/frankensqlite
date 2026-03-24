#!/usr/bin/env bash
# bd-db300.1.7.2: Capture authoritative persistent 8t and 16t phase-attribution packs.
#
# Uses the Criterion bench entrypoint:
#   crates/fsqlite-e2e/benches/concurrent_write_persistent_bench.rs
#
# Capture surface:
#   FSQLITE_PERSISTENT_PHASE_ATTRIBUTION_DIR → provenance.json + samples.jsonl
#
# Thread counts exercised: 8, 16 (the two degraded regimes from 2026-03-20).
#
# Usage:
#   ./scripts/capture_persistent_phase_pack.sh [output_dir]
#
# Output:
#   <output_dir>/
#     provenance/environment.yaml   — machine/build provenance
#     8t/provenance.json            — Criterion bench provenance (auto-generated)
#     8t/samples.jsonl              — per-iteration phase attribution (auto-generated)
#     8t/criterion_stdout.log       — raw Criterion output
#     16t/provenance.json
#     16t/samples.jsonl
#     16t/criterion_stdout.log
#     persistent_scorecard.json     — honest-gate verdicts for 8t / 16t
#     persistent_pack_manifest.json — machine-readable pack summary
#     summary.md                    — human-readable critical-regime surface
#     rerun.sh                      — one-command reproducibility entrypoint
set -euo pipefail

BEAD_ID="${BEAD_ID:-bd-db300.1.7.2}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TIMESTAMP="$(date +%Y%m%d_%H%M%S)"
resolve_output_dir() {
    local requested_path="$1"
    local absolute_path="$requested_path"
    # Criterion benches run from the package directory, so the capture env must be absolute.
    if [[ "$absolute_path" != /* ]]; then
        absolute_path="${PROJECT_ROOT}/${absolute_path}"
    fi
    mkdir -p "$absolute_path"
    (
        cd "$absolute_path"
        pwd -P
    )
}

OUTPUT_DIR="$(resolve_output_dir "${1:-artifacts/persistent_phase_pack_${TIMESTAMP}}")"
PROVENANCE_DIR="${OUTPUT_DIR}/provenance"
SCORECARD_JSON="${OUTPUT_DIR}/persistent_scorecard.json"
MANIFEST_JSON="${OUTPUT_DIR}/persistent_pack_manifest.json"
SUMMARY_MD="${OUTPUT_DIR}/summary.md"
HASHES_TXT="${OUTPUT_DIR}/artifact_hashes.txt"
RERUN_SH="${OUTPUT_DIR}/rerun.sh"

THREAD_COUNTS_CSV="${THREAD_COUNTS:-8,16}"
RENDER_ONLY="${RENDER_ONLY:-0}"
SKIP_RUN="${SKIP_RUN:-0}"
HEALTHY_MARGIN_MIN="${HEALTHY_MARGIN_MIN:-1.10}"
TAIL_COLLAPSE_P95_US="${TAIL_COLLAPSE_P95_US:-250000}"
TAIL_COLLAPSE_P99_US="${TAIL_COLLAPSE_P99_US:-500000}"
TAIL_COLLAPSE_MAX_US="${TAIL_COLLAPSE_MAX_US:-2000000}"
PHASE_B_COLLAPSE_P99_US="${PHASE_B_COLLAPSE_P99_US:-250000}"
WAL_APPEND_COLLAPSE_P99_US="${WAL_APPEND_COLLAPSE_P99_US:-250000}"

mkdir -p "$PROVENANCE_DIR"
IFS=',' read -ra THREAD_COUNTS <<< "$THREAD_COUNTS_CSV"

write_environment_provenance() {
    local cpu_model="unknown"
    cpu_model="$(awk -F: '/model name/ {gsub(/^[ \t]+/, "", $2); print $2; exit}' /proc/cpuinfo 2>/dev/null || true)"
    if [[ -z "${cpu_model}" ]]; then
        cpu_model="unknown"
    fi

    {
        echo "bead_id: ${BEAD_ID}"
        echo "capture_timestamp: ${TIMESTAMP}"
        echo "capture_script: scripts/capture_persistent_phase_pack.sh"
        echo "hostname: $(hostname)"
        echo "uname: $(uname -a)"
        echo "cpu_model: ${cpu_model}"
        echo "cpu_count: $(nproc 2>/dev/null || echo unknown)"
        echo "memory_gb: $(free -g 2>/dev/null | awk '/^Mem:/{print $2}' || echo unknown)"
        echo "numa_nodes: $(ls -d /sys/devices/system/node/node* 2>/dev/null | wc -l || echo 1)"
        echo "load_avg: $(cat /proc/loadavg 2>/dev/null || echo unknown)"
        echo "git_commit: $(git -C "$PROJECT_ROOT" rev-parse HEAD 2>/dev/null || echo unknown)"
        echo "git_branch: $(git -C "$PROJECT_ROOT" branch --show-current 2>/dev/null || echo unknown)"
        echo "git_dirty_files: $(git -C "$PROJECT_ROOT" diff --name-only 2>/dev/null | wc -l || echo unknown)"
        echo "rust_version: $(rustc --version 2>/dev/null || echo unknown)"
        echo "cargo_profile: release-perf"
        echo "bench_entrypoint: crates/fsqlite-e2e/benches/concurrent_write_persistent_bench.rs"
        echo "capture_env: FSQLITE_PERSISTENT_PHASE_ATTRIBUTION_DIR"
        echo "reference_comparator: C SQLite via rusqlite (built-in to bench)"
        echo "thread_counts: [${THREAD_COUNTS_CSV}]"
        echo "warmup_measurement_disclaimer: |"
        echo "  The Criterion harness runs warmup iterations before measurement."
        echo "  FSQLITE_PERSISTENT_PHASE_ATTRIBUTION_DIR captures ALL iterations"
        echo "  (warmup + measurement) in samples.jsonl. The harness does NOT tag"
        echo "  which samples are warmup vs measurement. Consumers should use"
        echo "  Criterion's own throughput estimates for authoritative throughput."
        echo "  The samples.jsonl is authoritative for phase-attribution"
        echo "  distributions and wake-reason accounting."
    } > "$PROVENANCE_DIR/environment.yaml"
}

ensure_bench_binary() {
    echo "--- Building release-perf benchmark binary (local) ---"
    (
        cd "$PROJECT_ROOT"
        cargo bench --profile release-perf -p fsqlite-e2e \
            --bench concurrent_write_persistent_bench --no-run 2>&1 | tail -5
    )
}

run_persistent_bench() {
    local thread_count="$1"
    local label
    local run_dir
    local provenance
    local samples

    label="${thread_count}t"
    run_dir="$OUTPUT_DIR/${label}"
    provenance="$run_dir/provenance.json"
    samples="$run_dir/samples.jsonl"

    mkdir -p "$run_dir"

    echo ""
    echo "=== Capturing ${label} persistent phase pack ==="
    echo "Thread count: $thread_count"
    echo "Phase attribution dir: $run_dir"

    (
        cd "$PROJECT_ROOT"
        FSQLITE_PERSISTENT_PHASE_ATTRIBUTION_DIR="$run_dir" \
        cargo bench --profile release-perf -p fsqlite-e2e \
            --bench concurrent_write_persistent_bench \
            -- "persistent_concurrent_write_${thread_count}t"
    ) 2>&1 | tee "$run_dir/criterion_stdout.log"

    if [[ -f "$provenance" ]]; then
        echo "  provenance.json: $(wc -c < "$provenance") bytes"
    else
        echo "  WARNING: provenance.json not generated"
    fi

    if [[ -f "$samples" ]]; then
        echo "  samples.jsonl: $(wc -l < "$samples") records"
    else
        echo "  WARNING: samples.jsonl not generated"
    fi

    echo "--- ${label} capture complete ---"
}

render_reports() {
    python3 - \
        "$OUTPUT_DIR" \
        "$SCORECARD_JSON" \
        "$MANIFEST_JSON" \
        "$SUMMARY_MD" \
        "$BEAD_ID" \
        "$HEALTHY_MARGIN_MIN" \
        "$TAIL_COLLAPSE_P95_US" \
        "$TAIL_COLLAPSE_P99_US" \
        "$TAIL_COLLAPSE_MAX_US" \
        "$PHASE_B_COLLAPSE_P99_US" \
        "$WAL_APPEND_COLLAPSE_P99_US" \
        "$THREAD_COUNTS_CSV" <<'PY'
import json
import re
import statistics
import sys
from pathlib import Path

output_dir = Path(sys.argv[1])
scorecard_path = Path(sys.argv[2])
manifest_path = Path(sys.argv[3])
summary_path = Path(sys.argv[4])
bead_id = sys.argv[5]
healthy_margin_min = float(sys.argv[6])
tail_collapse_p95_us = int(sys.argv[7])
tail_collapse_p99_us = int(sys.argv[8])
tail_collapse_max_us = int(sys.argv[9])
phase_b_collapse_p99_us = int(sys.argv[10])
wal_append_collapse_p99_us = int(sys.argv[11])
thread_labels = []
for item in sys.argv[12].split(","):
    item = item.strip()
    if not item:
        continue
    thread_labels.append(item if item.endswith("t") else f"{item}t")


def nested_get(mapping, *keys):
    current = mapping
    for key in keys:
        if not isinstance(current, dict):
            return None
        current = current.get(key)
    return current


def median_value(values):
    usable = [value for value in values if value is not None]
    if not usable:
        return None
    return statistics.median(usable)


def ratio(numerator, denominator):
    if numerator is None or denominator in (None, 0):
        return None
    return numerator / denominator


def format_ratio(value):
    if value is None:
        return "n/a"
    return f"{value:.3f}x"


def format_us_triplet(row):
    if row is None:
        return "n/a"
    return f"{row['p50']}/{row['p95']}/{row['p99']}"


def median_nested(rows, *keys):
    return median_value([nested_get(row, *keys) for row in rows])


def format_optional_us(value):
    if value is None:
        return "n/a"
    return f"{int(value)}us"


def format_signed_us(value):
    if value is None:
        return "n/a"
    value = int(value)
    return f"+{value}us" if value > 0 else f"{value}us"


def format_retry_stage_counts(stage_counts):
    if not stage_counts:
        return "n/a"
    values = [
        stage_counts.get("retry_stage_begin_retries_median"),
        stage_counts.get("retry_stage_body_retries_median"),
        stage_counts.get("retry_stage_commit_retries_median"),
        stage_counts.get("retry_stage_duplicate_after_retry_exits_median"),
        stage_counts.get("retry_stage_total_retries_median"),
    ]
    if any(value is None for value in values):
        return "n/a"
    return "/".join(str(int(value)) for value in values)


def parse_interval(body):
    matches = re.findall(r"([0-9]+(?:\.[0-9]+)?)\s*([KMG]?elem/s)", body)
    if len(matches) != 3:
        return None
    scale = {
        "elem/s": 1.0,
        "Kelem/s": 1_000.0,
        "Melem/s": 1_000_000.0,
        "Gelem/s": 1_000_000_000.0,
    }
    values = [float(number) * scale[unit] for number, unit in matches]
    return {
        "low": values[0],
        "mid": values[1],
        "high": values[2],
        "display": body.strip(),
    }


def parse_throughput_interval(log_text, regime_id, bench_name):
    pattern = re.compile(
        rf"Benchmarking {re.escape(regime_id)}/{bench_name}: Analyzing.*?thrpt:\s+\[(?P<body>[^\]]+)\]",
        re.S,
    )
    match = pattern.search(log_text)
    if not match:
        return None
    return parse_interval(match.group("body"))


def load_samples(samples_path):
    if not samples_path.exists():
        return []
    rows = []
    with samples_path.open("r", encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if not line:
                continue
            rows.append(json.loads(line))
    return rows


def classify_throughput(ratio_value):
    if ratio_value is None:
        return None
    if ratio_value < 1.0:
        return "below_parity"
    if ratio_value < healthy_margin_min:
        return "warning"
    return "pass"


critical_regimes = []
for label in thread_labels:
    regime_id = f"persistent_concurrent_write_{label}"
    run_dir = output_dir / label
    artifacts = {
        "provenance_json": f"{label}/provenance.json",
        "samples_jsonl": f"{label}/samples.jsonl",
        "criterion_stdout_log": f"{label}/criterion_stdout.log",
    }
    artifact_paths = {name: output_dir / relpath for name, relpath in artifacts.items()}
    missing_artifacts = [name for name, path in artifact_paths.items() if not path.exists()]
    has_any_artifact = any(path.exists() for path in artifact_paths.values())

    regime = {
        "regime_id": regime_id,
        "thread_label": label,
        "concurrency": int(label[:-1]),
        "artifacts": artifacts,
        "missing_artifacts": missing_artifacts,
        "critical_surface_primary": True,
    }

    if not has_any_artifact:
        regime["coverage_state"] = "no_data"
        regime["verdict"] = "no_data"
        regime["measured_reasons"] = ["no persistent pack artifacts were captured for this regime"]
        critical_regimes.append(regime)
        continue

    sample_rows = load_samples(artifact_paths["samples_jsonl"])
    sqlite_rows = [row for row in sample_rows if row.get("engine") == "sqlite3"]
    fsqlite_rows = [row for row in sample_rows if row.get("engine") == "fsqlite_mvcc"]

    regime["sample_counts"] = {
        "sqlite3": len(sqlite_rows),
        "fsqlite_mvcc": len(fsqlite_rows),
    }

    if not sample_rows:
        regime["coverage_state"] = "incomplete"
        regime["verdict"] = "incomplete"
        regime["measured_reasons"] = ["samples.jsonl is missing or empty, so the regime has no comparable latency surface"]
        critical_regimes.append(regime)
        continue

    if not sqlite_rows or not fsqlite_rows:
        missing_engine = "sqlite3" if not sqlite_rows else "fsqlite_mvcc"
        regime["coverage_state"] = "incomplete"
        regime["verdict"] = "incomplete"
        regime["measured_reasons"] = [f"samples.jsonl is missing comparator rows for `{missing_engine}`"]
        critical_regimes.append(regime)
        continue

    sqlite_latency = {
        metric: median_value([nested_get(row, "latency_us", f"{metric}_us") for row in sqlite_rows])
        for metric in ("p50", "p95", "p99", "max")
    }
    fsqlite_latency = {
        metric: median_value([nested_get(row, "latency_us", f"{metric}_us") for row in fsqlite_rows])
        for metric in ("p50", "p95", "p99", "max")
    }
    latency_ratios = {
        metric: ratio(fsqlite_latency[metric], sqlite_latency[metric])
        for metric in ("p50", "p95", "p99")
    }
    phase_metric_rows = [row for row in fsqlite_rows if isinstance(row.get("phase_metrics"), dict)]
    wall_audit_rows = [
        row for row in fsqlite_rows
        if isinstance(row.get("operation_wall_time_audit"), dict)
    ]
    phase_metrics_medians = {
        "arrival_wait_p95_us": median_nested(phase_metric_rows, "phase_metrics", "hist_arrival_wait", "p95"),
        "wal_backend_lock_wait_p99_us": median_nested(phase_metric_rows, "phase_metrics", "hist_wal_backend_lock_wait", "p99"),
        "wal_append_p99_us": median_nested(phase_metric_rows, "phase_metrics", "hist_wal_append", "p99"),
        "phase_b_p99_us": median_nested(phase_metric_rows, "phase_metrics", "hist_phase_b", "p99"),
        "waiter_epoch_wait_p99_us": median_nested(phase_metric_rows, "phase_metrics", "hist_waiter_epoch_wait", "p99"),
        "wake_timeout_median": median_nested(phase_metric_rows, "phase_metrics", "wake_reasons", "timeout"),
        "wake_flusher_takeover_median": median_nested(phase_metric_rows, "phase_metrics", "wake_reasons", "flusher_takeover"),
        "wake_notify_median": median_nested(phase_metric_rows, "phase_metrics", "wake_reasons", "notify"),
        "lock_topology_limited_sample_count": sum(1 for row in fsqlite_rows if row.get("lock_topology_limited") is True),
    }
    operation_wall_time_audit_medians = {
        "wall_avg_us_per_operation": median_nested(wall_audit_rows, "operation_wall_time_audit", "wall_time", "avg_us_per_operation"),
        "begin_retry_handoff_avg_us_per_operation": median_nested(wall_audit_rows, "operation_wall_time_audit", "begin_retry_handoff", "avg_us_per_operation"),
        "statement_execute_body_avg_us_per_operation": median_nested(wall_audit_rows, "operation_wall_time_audit", "statement_execute_body", "avg_us_per_operation"),
        "commit_roundtrip_avg_us_per_operation": median_nested(wall_audit_rows, "operation_wall_time_audit", "commit_roundtrip", "avg_us_per_operation"),
        "rollback_cleanup_avg_us_per_operation": median_nested(wall_audit_rows, "operation_wall_time_audit", "rollback_cleanup", "avg_us_per_operation"),
        "retry_backoff_sleep_avg_us_per_operation": median_nested(wall_audit_rows, "operation_wall_time_audit", "retry_backoff_sleep", "avg_us_per_operation"),
        "commit_center_avg_us_per_recorded_commit": median_nested(
            wall_audit_rows,
            "operation_wall_time_audit",
            "measured_commit_sub_buckets",
            "commit_center",
            "avg_us_per_recorded_commit",
        ),
        "post_commit_cleanup_publish_avg_us_per_recorded_commit": median_nested(
            wall_audit_rows,
            "operation_wall_time_audit",
            "measured_commit_sub_buckets",
            "post_commit_cleanup_publish",
            "avg_us_per_recorded_commit",
        ),
        "measured_commit_roundtrip_gap_avg_us_per_recorded_commit": median_nested(
            wall_audit_rows,
            "operation_wall_time_audit",
            "measured_commit_roundtrip_gap",
            "avg_us_per_recorded_commit",
        ),
        "measured_commit_roundtrip_gap_abs_fraction_basis_points": median_nested(
            wall_audit_rows,
            "operation_wall_time_audit",
            "measured_commit_roundtrip_gap",
            "abs_fraction_basis_points",
        ),
        "residual_avg_us_per_operation": median_nested(
            wall_audit_rows,
            "operation_wall_time_audit",
            "residual",
            "avg_us_per_operation",
        ),
        "residual_abs_fraction_basis_points": median_nested(
            wall_audit_rows,
            "operation_wall_time_audit",
            "residual",
            "abs_fraction_basis_points",
        ),
        "retry_stage_begin_retries_median": median_nested(
            wall_audit_rows, "operation_wall_time_audit", "retry_stage_counts", "begin_retries"
        ),
        "retry_stage_body_retries_median": median_nested(
            wall_audit_rows, "operation_wall_time_audit", "retry_stage_counts", "body_retries"
        ),
        "retry_stage_commit_retries_median": median_nested(
            wall_audit_rows, "operation_wall_time_audit", "retry_stage_counts", "commit_retries"
        ),
        "retry_stage_duplicate_after_retry_exits_median": median_nested(
            wall_audit_rows,
            "operation_wall_time_audit",
            "retry_stage_counts",
            "duplicate_after_retry_exits",
        ),
        "retry_stage_total_retries_median": median_nested(
            wall_audit_rows, "operation_wall_time_audit", "retry_stage_counts", "total_retries"
        ),
    }

    regime["latency_medians_us"] = {
        "sqlite3": sqlite_latency,
        "fsqlite_mvcc": fsqlite_latency,
    }
    regime["latency_ratio_vs_sqlite"] = latency_ratios
    regime["phase_metrics_medians"] = phase_metrics_medians
    regime["operation_wall_time_audit_sample_count"] = len(wall_audit_rows)
    regime["operation_wall_time_audit_medians"] = operation_wall_time_audit_medians

    throughput_ratio = None
    comparator_state = "missing_criterion_comparator"
    if artifact_paths["criterion_stdout_log"].exists():
        log_text = artifact_paths["criterion_stdout_log"].read_text(encoding="utf-8")
        sqlite_interval = parse_throughput_interval(log_text, regime_id, "csqlite_concurrent_persistent")
        fsqlite_interval = parse_throughput_interval(log_text, regime_id, "frankensqlite_concurrent_persistent")
        if sqlite_interval is not None and fsqlite_interval is not None:
            throughput_ratio = ratio(fsqlite_interval["mid"], sqlite_interval["mid"])
            comparator_state = "same_pack_sqlite3_available"
            regime["throughput_midpoint_elem_per_sec"] = {
                "sqlite3": sqlite_interval["mid"],
                "fsqlite_mvcc": fsqlite_interval["mid"],
            }
            regime["throughput_interval_elem_per_sec"] = {
                "sqlite3": sqlite_interval,
                "fsqlite_mvcc": fsqlite_interval,
            }
        else:
            regime["throughput_midpoint_elem_per_sec"] = {
                "sqlite3": sqlite_interval["mid"] if sqlite_interval else None,
                "fsqlite_mvcc": fsqlite_interval["mid"] if fsqlite_interval else None,
            }

    throughput_band = classify_throughput(throughput_ratio)
    regime["throughput_ratio_vs_sqlite"] = throughput_ratio
    regime["throughput_band"] = throughput_band
    regime["comparator_state"] = comparator_state

    collapse_reasons = []
    if fsqlite_latency["p95"] is not None and fsqlite_latency["p95"] >= tail_collapse_p95_us:
        collapse_reasons.append(f"median p95 latency {int(fsqlite_latency['p95'])}us >= collapse threshold {tail_collapse_p95_us}us")
    if fsqlite_latency["p99"] is not None and fsqlite_latency["p99"] >= tail_collapse_p99_us:
        collapse_reasons.append(f"median p99 latency {int(fsqlite_latency['p99'])}us >= collapse threshold {tail_collapse_p99_us}us")
    if fsqlite_latency["max"] is not None and fsqlite_latency["max"] >= tail_collapse_max_us:
        collapse_reasons.append(f"median max latency {int(fsqlite_latency['max'])}us >= collapse threshold {tail_collapse_max_us}us")
    if phase_metrics_medians["wal_append_p99_us"] is not None and phase_metrics_medians["wal_append_p99_us"] >= wal_append_collapse_p99_us:
        collapse_reasons.append(
            f"wal_append p99 median {int(phase_metrics_medians['wal_append_p99_us'])}us >= collapse threshold {wal_append_collapse_p99_us}us"
        )
    if phase_metrics_medians["phase_b_p99_us"] is not None and phase_metrics_medians["phase_b_p99_us"] >= phase_b_collapse_p99_us:
        collapse_reasons.append(
            f"phase_B p99 median {int(phase_metrics_medians['phase_b_p99_us'])}us >= collapse threshold {phase_b_collapse_p99_us}us"
        )
    regime["collapse_override_reasons"] = collapse_reasons
    regime["collapse_override_applies"] = bool(collapse_reasons)

    measured_reasons = [
        f"throughput midpoint ratio vs same-pack sqlite3 is {format_ratio(throughput_ratio)}" if throughput_ratio is not None else "same-pack Criterion throughput comparator is missing",
        f"median p50 latency is {format_ratio(latency_ratios['p50'])} vs same-pack sqlite3",
        f"median p95 latency is {format_ratio(latency_ratios['p95'])} vs same-pack sqlite3",
        f"median p99 latency is {format_ratio(latency_ratios['p99'])} vs same-pack sqlite3",
        f"wal_append p99 median {int(phase_metrics_medians['wal_append_p99_us'])}us" if phase_metrics_medians["wal_append_p99_us"] is not None else "wal_append p99 median unavailable",
        f"phase_B p99 median {int(phase_metrics_medians['phase_b_p99_us'])}us" if phase_metrics_medians["phase_b_p99_us"] is not None else "phase_B p99 median unavailable",
        (
            f"lock_topology_limited remained false in all captured MVCC samples"
            if phase_metrics_medians["lock_topology_limited_sample_count"] == 0
            else f"lock_topology_limited was true in {phase_metrics_medians['lock_topology_limited_sample_count']} captured MVCC samples"
        ),
    ]
    if wall_audit_rows:
        measured_reasons.append(
            "end-to-end wall avg/op median {} with begin/retry-handoff {}, statement body {}, commit roundtrip {}, rollback cleanup {}, retry backoff {}, residual {} ({} bp of wall)".format(
                format_optional_us(operation_wall_time_audit_medians["wall_avg_us_per_operation"]),
                format_optional_us(operation_wall_time_audit_medians["begin_retry_handoff_avg_us_per_operation"]),
                format_optional_us(operation_wall_time_audit_medians["statement_execute_body_avg_us_per_operation"]),
                format_optional_us(operation_wall_time_audit_medians["commit_roundtrip_avg_us_per_operation"]),
                format_optional_us(operation_wall_time_audit_medians["rollback_cleanup_avg_us_per_operation"]),
                format_optional_us(operation_wall_time_audit_medians["retry_backoff_sleep_avg_us_per_operation"]),
                format_signed_us(operation_wall_time_audit_medians["residual_avg_us_per_operation"]),
                "n/a"
                if operation_wall_time_audit_medians["residual_abs_fraction_basis_points"] is None
                else int(operation_wall_time_audit_medians["residual_abs_fraction_basis_points"]),
            )
        )
        measured_reasons.append(
            "measured commit center/post-commit medians are {commit_center}/{post_commit} per recorded commit; commit roundtrip gap is {commit_gap} ({commit_gap_bp} bp of roundtrip), which captures commit-side wall time the old commit-center-only view hid".format(
                commit_center=format_optional_us(
                    operation_wall_time_audit_medians["commit_center_avg_us_per_recorded_commit"]
                ),
                post_commit=format_optional_us(
                    operation_wall_time_audit_medians["post_commit_cleanup_publish_avg_us_per_recorded_commit"]
                ),
                commit_gap=format_signed_us(
                    operation_wall_time_audit_medians["measured_commit_roundtrip_gap_avg_us_per_recorded_commit"]
                ),
                commit_gap_bp="n/a"
                if operation_wall_time_audit_medians["measured_commit_roundtrip_gap_abs_fraction_basis_points"] is None
                else int(operation_wall_time_audit_medians["measured_commit_roundtrip_gap_abs_fraction_basis_points"]),
            )
        )
        measured_reasons.append(
            "retry stage count medians begin/body/commit/duplicate-after-retry/total = {}".format(
                format_retry_stage_counts(operation_wall_time_audit_medians)
            )
        )
    else:
        measured_reasons.append("operation wall-time audit is unavailable in captured MVCC samples")
    if missing_artifacts:
        regime["coverage_state"] = "incomplete"
        regime["verdict"] = "incomplete"
        measured_reasons.append(f"expected pack artifacts are missing: {', '.join(missing_artifacts)}")
    elif throughput_ratio is None:
        regime["coverage_state"] = "incomplete"
        regime["verdict"] = "incomplete"
        measured_reasons.append("criterion comparator missing keeps the regime non-green even if samples.jsonl exists")
    else:
        regime["coverage_state"] = "complete"
        if throughput_band == "below_parity":
            regime["verdict"] = "below_parity"
        elif collapse_reasons:
            regime["verdict"] = "collapse_red"
            measured_reasons.append("raw throughput alone is not sufficient because collapse override applies")
        elif throughput_band == "warning":
            regime["verdict"] = "warning"
        else:
            regime["verdict"] = "pass"
    regime["measured_reasons"] = measured_reasons
    critical_regimes.append(regime)

pack_verdict = "pass"
lane_verdicts = [regime["verdict"] for regime in critical_regimes]
if not critical_regimes or all(verdict == "no_data" for verdict in lane_verdicts):
    pack_verdict = "no_data"
elif any(verdict in {"no_data", "incomplete"} for verdict in lane_verdicts):
    pack_verdict = "incomplete"
elif any(verdict in {"below_parity", "collapse_red"} for verdict in lane_verdicts):
    pack_verdict = "fail"
elif any(verdict == "warning" for verdict in lane_verdicts):
    pack_verdict = "warning"

honest_gate_summary = {
    "verdict": pack_verdict,
    "critical_regime_count": len(critical_regimes),
    "complete_regime_count": sum(1 for regime in critical_regimes if regime["coverage_state"] == "complete"),
    "incomplete_regime_count": sum(1 for regime in critical_regimes if regime["coverage_state"] == "incomplete"),
    "no_data_regime_count": sum(1 for regime in critical_regimes if regime["coverage_state"] == "no_data"),
    "red_regimes": [regime["regime_id"] for regime in critical_regimes if regime["verdict"] in {"below_parity", "collapse_red"}],
    "incomplete_regimes": [regime["regime_id"] for regime in critical_regimes if regime["verdict"] == "incomplete"],
    "no_data_regimes": [regime["regime_id"] for regime in critical_regimes if regime["verdict"] == "no_data"],
    "rule": "8t and 16t stay individually visible; incomplete or comparator-missing packs are non-green and throughput does not override collapse",
}

scorecard = {
    "schema_version": "bd-db300.persistent_phase_pack_scorecard.v3",
    "bead_id": bead_id,
    "run_id": output_dir.name,
    "entrypoint": "scripts/capture_persistent_phase_pack.sh",
    "pack_role": "honest_gate_phase_pack",
    "baseline_comparator": "sqlite3_same_pack",
    "shadow_lineage": "none",
    "critical_surface_primary": True,
    "aggregate_views_secondary_only": True,
    "healthy_margin_min": healthy_margin_min,
    "comparator_contract": {
        "baseline_comparator": "sqlite3_same_pack",
        "comparator_engine": "sqlite3",
        "comparator_scope": "same thread regime, same pack",
        "aggregate_rows_are_secondary": True,
    },
    "operation_wall_time_audit_disclosure": {
        "avg_us_per_operation": "median_of_per_sample_averages",
        "avg_us_per_recorded_commit": "median_of_per_sample_averages",
        "retry_stage_counts": "median_of_per_sample_counts",
        "not_a_per_operation_quantile": True,
    },
    "honest_gate_summary": honest_gate_summary,
    "critical_regimes": critical_regimes,
    "warmup_measurement_disclosure": {
        "samples_include_warmup": True,
        "samples_include_measurement": True,
        "authoritative_for": [
            "phase-attribution distributions",
            "wake-reason distributions",
            "same-pack sqlite3 versus fsqlite_mvcc regime comparison",
        ],
        "not_authoritative_for": [
            "headline throughput without Criterion comparator coverage",
        ],
    },
}
scorecard_path.write_text(json.dumps(scorecard, indent=2, sort_keys=True) + "\n", encoding="utf-8")

manifest = {
    "schema_version": "bd-db300.persistent_phase_pack_manifest.v2",
    "bead_id": bead_id,
    "run_id": output_dir.name,
    "entrypoint": "scripts/capture_persistent_phase_pack.sh",
    "scorecard_json": scorecard_path.name,
    "summary_md": summary_path.name,
    "honest_gate_summary": honest_gate_summary,
    "critical_regimes": [
        {
            "regime_id": regime["regime_id"],
            "verdict": regime["verdict"],
            "coverage_state": regime["coverage_state"],
            "artifacts": regime["artifacts"],
            "missing_artifacts": regime["missing_artifacts"],
        }
        for regime in critical_regimes
    ],
}
manifest_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")

summary_lines = [
    f"# {bead_id} Persistent Phase Pack",
    "",
    f"- run_id: `{output_dir.name}`",
    "- baseline_comparator: same-pack `sqlite3` Criterion + `samples.jsonl` rows when present",
    f"- critical_regimes: `{', '.join(regime['regime_id'] for regime in critical_regimes)}`",
    "- disclosure: `samples.jsonl` mixes warmup and measurement; use it for phase and wake-reason truth, not headline throughput without the Criterion comparator",
    "",
    "## Honest Gate Summary",
    "",
    f"- verdict: `{pack_verdict}`",
    f"- complete_regime_count: `{honest_gate_summary['complete_regime_count']}`",
    f"- incomplete_regime_count: `{honest_gate_summary['incomplete_regime_count']}`",
    f"- no_data_regime_count: `{honest_gate_summary['no_data_regime_count']}`",
    "- critical 8t/16t regimes remain individually visible; no aggregate green can hide a red or incomplete regime",
    "- comparator-missing or partial packs are non-green and report `no_data`/`incomplete` instead of silently grading the regime",
    "- throughput > 1.0x is not sufficient when the collapse override applies",
    "",
    "## Critical Regimes (Primary Surface)",
    "",
    "| Regime | Verdict | Coverage | Throughput vs sqlite | Throughput band | Collapse override | SQLite p50/p95/p99 (us) | FrankenSQLite p50/p95/p99 (us) | WAL append p99 (us) | Phase B p99 (us) | lock_topology_limited true samples |",
    "|--------|---------|----------|----------------------|-----------------|-------------------|-------------------------|---------------------------------|---------------------|------------------|------------------------------------|",
]

for regime in critical_regimes:
    collapse_override = "yes" if regime.get("collapse_override_applies") else "no"
    phase_metrics = regime.get("phase_metrics_medians", {})
    summary_lines.append(
        "| {regime_id} | {verdict} | {coverage} | {throughput_ratio} | {throughput_band} | {collapse_override} | {sqlite_latency} | {fsqlite_latency} | {wal_append_p99} | {phase_b_p99} | {lock_topology_count} |".format(
            regime_id=regime["regime_id"],
            verdict=regime["verdict"],
            coverage=regime["coverage_state"],
            throughput_ratio=format_ratio(regime.get("throughput_ratio_vs_sqlite")),
            throughput_band=regime.get("throughput_band") or "n/a",
            collapse_override=collapse_override,
            sqlite_latency=format_us_triplet(regime.get("latency_medians_us", {}).get("sqlite3")),
            fsqlite_latency=format_us_triplet(regime.get("latency_medians_us", {}).get("fsqlite_mvcc")),
            wal_append_p99="n/a" if phase_metrics.get("wal_append_p99_us") is None else int(phase_metrics["wal_append_p99_us"]),
            phase_b_p99="n/a" if phase_metrics.get("phase_b_p99_us") is None else int(phase_metrics["phase_b_p99_us"]),
            lock_topology_count=phase_metrics.get("lock_topology_limited_sample_count", "n/a"),
        )
    )

summary_lines.extend([
    "",
    "## End-to-End Wall-Time Audit (FrankenSQLite Median-of-Per-Sample Averages)",
    "",
    "- disclosure: `avg/op` and `avg/recorded` columns below are medians of per-sample averages from captured MVCC samples; they are not per-operation p50/p95 quantiles.",
    "",
    "| Regime | Wall avg/op | Begin/retry-handoff avg/op | Commit roundtrip avg/op | Commit center avg/recorded | Post-commit avg/recorded | Commit roundtrip gap avg/recorded | Rollback avg/op | Backoff avg/op | Residual avg/op | Retry medians begin/body/commit/dup/total |",
    "|--------|-------------|----------------------------|-------------------------|----------------------------|--------------------------|-----------------------------------|-----------------|----------------|-----------------|-------------------------------------------|",
])
for regime in critical_regimes:
    wall_audit = regime.get("operation_wall_time_audit_medians", {})
    summary_lines.append(
        "| {regime_id} | {wall_avg} | {begin_retry_handoff} | {commit_roundtrip} | {commit_center} | {post_commit} | {commit_gap} | {rollback} | {backoff} | {residual} | {retry_counts} |".format(
            regime_id=regime["regime_id"],
            wall_avg=format_optional_us(wall_audit.get("wall_avg_us_per_operation")),
            begin_retry_handoff=format_optional_us(
                wall_audit.get("begin_retry_handoff_avg_us_per_operation")
            ),
            commit_roundtrip=format_optional_us(wall_audit.get("commit_roundtrip_avg_us_per_operation")),
            commit_center=format_optional_us(wall_audit.get("commit_center_avg_us_per_recorded_commit")),
            post_commit=format_optional_us(wall_audit.get("post_commit_cleanup_publish_avg_us_per_recorded_commit")),
            commit_gap=format_signed_us(wall_audit.get("measured_commit_roundtrip_gap_avg_us_per_recorded_commit")),
            rollback=format_optional_us(wall_audit.get("rollback_cleanup_avg_us_per_operation")),
            backoff=format_optional_us(wall_audit.get("retry_backoff_sleep_avg_us_per_operation")),
            residual=format_signed_us(wall_audit.get("residual_avg_us_per_operation")),
            retry_counts=format_retry_stage_counts(wall_audit),
        )
    )

summary_lines.extend(["", "## Regime Notes", ""])
for regime in critical_regimes:
    summary_lines.append(f"- `{regime['regime_id']}`: {regime['verdict']} ({regime['coverage_state']})")
    for note in regime["measured_reasons"]:
        summary_lines.append(f"  - {note}")
    if regime.get("missing_artifacts"):
        summary_lines.append(f"  - missing_artifacts: {', '.join(regime['missing_artifacts'])}")

summary_path.write_text("\n".join(summary_lines) + "\n", encoding="utf-8")
PY
}

hash_artifacts() {
    (
        cd "$OUTPUT_DIR"
        find . -type f ! -name "$(basename "$HASHES_TXT")" -print0 \
            | sort -z \
            | xargs -0 sha256sum > "$(basename "$HASHES_TXT")"
    )
}

write_rerun_script() {
    cat > "$RERUN_SH" <<RERUN_EOF
#!/usr/bin/env bash
# One-command rerun of the persistent phase-attribution pack.
# Original capture: ${TIMESTAMP}
# Bead: ${BEAD_ID}
set -euo pipefail
cd "$PROJECT_ROOT"
exec ./scripts/capture_persistent_phase_pack.sh "\${1:-$OUTPUT_DIR.rerun_\$(date +%Y%m%d_%H%M%S)}"
RERUN_EOF
    chmod +x "$RERUN_SH"
}

main() {
    echo "=== ${BEAD_ID}: Authoritative Persistent Phase-Attribution Pack ==="
    echo "Output: $OUTPUT_DIR"
    echo "Timestamp: $TIMESTAMP"

    if [[ "$RENDER_ONLY" != "1" ]]; then
        echo "--- Capturing environment provenance ---"
        write_environment_provenance
        ensure_bench_binary

        echo ""
        echo "--- Pre-flight load check ---"
        LOAD_1MIN="$(awk '{print $1}' /proc/loadavg 2>/dev/null || echo 0)"
        CPU_COUNT="$(nproc 2>/dev/null || echo 0)"
        echo "Load average (1min): $LOAD_1MIN"
        echo "CPU count: $CPU_COUNT"
        if awk "BEGIN { exit !($LOAD_1MIN > $CPU_COUNT * 0.8) }" 2>/dev/null; then
            echo "WARNING: Load is >80% of CPU count. Results may be noisy."
            echo "Proceeding anyway — noise will remain visible in the capture."
        fi

        if [[ "$SKIP_RUN" != "1" ]]; then
            for thread_count in "${THREAD_COUNTS[@]}"; do
                run_persistent_bench "$thread_count"
            done
        fi
    fi

    echo ""
    echo "--- Rendering honest-gate reports ---"
    render_reports
    hash_artifacts
    write_rerun_script

    echo ""
    echo "=== Pack capture complete ==="
    echo "Output directory: $OUTPUT_DIR"
    echo "Scorecard: $SCORECARD_JSON"
    echo "Summary: $SUMMARY_MD"
    echo "Manifest: $MANIFEST_JSON"
    echo "Rerun: $RERUN_SH"
}

main "$@"
