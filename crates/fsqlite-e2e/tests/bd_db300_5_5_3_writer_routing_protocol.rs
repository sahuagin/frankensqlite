//! Synthetic writer-routing protocol evidence for `bd-db300.5.5.3`.

use std::sync::Mutex;

use fsqlite_mvcc::{
    WriterRoutingDecisionConfig, WriterRoutingPlacementProfile, WriterRoutingSyntheticComparison,
    WriterRoutingSyntheticConfig, WriterRoutingSyntheticSummary, WriterRoutingSyntheticWorkload,
    compare_writer_routing_synthetic_workload,
};
use serde::Serialize;
use serde_json::Value;

const BEAD_ID: &str = "bd-db300.5.5.3";
const REPLAY_COMMAND: &str = "cargo test -p fsqlite-e2e --test bd_db300_5_5_3_writer_routing_protocol -- --nocapture --test-threads=1";

static WRITER_ROUTING_PROTOCOL_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, Serialize)]
struct RoutingProtocolFairnessSummary {
    lane_writer_counts: Vec<u64>,
    max_minus_min_assignments: u64,
    jain_index: f64,
}

#[derive(Debug, Clone, Serialize)]
struct RoutingProtocolMetadataPublicationSummary {
    publication_retry_rate: f64,
    publication_retry_total: u64,
    visibility_handoff_nanos_total: u64,
    stale_snapshot_reject_rate: f64,
    stale_snapshot_rejects_total: u64,
}

#[derive(Debug, Clone, Serialize)]
struct RoutingProtocolCaseEvidence {
    trace_id: String,
    scenario_id: String,
    routing_mode: String,
    placement_profile: String,
    workload_shape: String,
    conflict_rate: f64,
    retry_rate: f64,
    fallback_rate: f64,
    remote_ownership_events: u64,
    ops_per_sec: f64,
    p50_ns: u64,
    p95_ns: u64,
    p99_ns: u64,
    fairness_summary: RoutingProtocolFairnessSummary,
    metadata_publication: RoutingProtocolMetadataPublicationSummary,
}

#[derive(Debug, Clone, Serialize)]
struct RoutingProtocolComparisonEvidence {
    trace_id: String,
    scenario_id: String,
    placement_profile: String,
    workload_shape: String,
    conflict_rate_reduction: f64,
    retry_rate_reduction: f64,
    fallback_rate_reduction: f64,
    publication_retry_rate_reduction: f64,
    remote_ownership_events_reduction: u64,
    ops_per_sec_improvement: f64,
    fairness_jain_delta: f64,
}

#[derive(Debug, Clone, Serialize)]
struct RoutingProtocolArtifact {
    bead_id: &'static str,
    trace_id: String,
    scenario_id: String,
    replay_command: &'static str,
    comparison: RoutingProtocolComparisonEvidence,
    cases: Vec<RoutingProtocolCaseEvidence>,
}

fn synthetic_config(
    scenario_id: &str,
    workload: WriterRoutingSyntheticWorkload,
    placement_profile: WriterRoutingPlacementProfile,
) -> WriterRoutingSyntheticConfig {
    WriterRoutingSyntheticConfig {
        scenario_id: scenario_id.to_owned(),
        workload,
        placement_profile,
        lane_count: 4,
        iterations: 16,
        writers_per_iteration: 4,
    }
}

fn case_evidence(
    trace_id: &str,
    summary: &WriterRoutingSyntheticSummary,
) -> RoutingProtocolCaseEvidence {
    RoutingProtocolCaseEvidence {
        trace_id: trace_id.to_owned(),
        scenario_id: summary.scenario_id.clone(),
        routing_mode: summary.routing_mode.as_str().to_owned(),
        placement_profile: summary.placement_profile.as_str().to_owned(),
        workload_shape: summary.workload.as_str().to_owned(),
        conflict_rate: summary.conflict_rate(),
        retry_rate: summary.retry_rate(),
        fallback_rate: summary.fallback_rate(),
        remote_ownership_events: summary.remote_ownership_events,
        ops_per_sec: summary.ops_per_sec(),
        p50_ns: summary.p50_latency_ns(),
        p95_ns: summary.p95_latency_ns(),
        p99_ns: summary.p99_latency_ns(),
        fairness_summary: RoutingProtocolFairnessSummary {
            lane_writer_counts: summary.fairness.lane_writer_counts.clone(),
            max_minus_min_assignments: summary.fairness.max_minus_min_assignments,
            jain_index: summary.fairness.jain_fairness_index(),
        },
        metadata_publication: RoutingProtocolMetadataPublicationSummary {
            publication_retry_rate: summary.publication_retry_rate(),
            publication_retry_total: summary.publication_retry_total,
            visibility_handoff_nanos_total: summary.visibility_handoff_nanos_total,
            stale_snapshot_reject_rate: summary.stale_snapshot_rate(),
            stale_snapshot_rejects_total: summary.stale_snapshot_rejects_total,
        },
    }
}

fn protocol_artifact(
    trace_id: &str,
    comparison: &WriterRoutingSyntheticComparison,
) -> RoutingProtocolArtifact {
    let baseline = case_evidence(trace_id, &comparison.baseline);
    let routed = case_evidence(trace_id, &comparison.routed);
    RoutingProtocolArtifact {
        bead_id: BEAD_ID,
        trace_id: trace_id.to_owned(),
        scenario_id: comparison.routed.scenario_id.clone(),
        replay_command: REPLAY_COMMAND,
        comparison: RoutingProtocolComparisonEvidence {
            trace_id: trace_id.to_owned(),
            scenario_id: comparison.routed.scenario_id.clone(),
            placement_profile: comparison.routed.placement_profile.as_str().to_owned(),
            workload_shape: comparison.routed.workload.as_str().to_owned(),
            conflict_rate_reduction: comparison.baseline.conflict_rate()
                - comparison.routed.conflict_rate(),
            retry_rate_reduction: comparison.baseline.retry_rate() - comparison.routed.retry_rate(),
            fallback_rate_reduction: comparison.baseline.fallback_rate()
                - comparison.routed.fallback_rate(),
            publication_retry_rate_reduction: comparison.baseline.publication_retry_rate()
                - comparison.routed.publication_retry_rate(),
            remote_ownership_events_reduction: comparison
                .baseline
                .remote_ownership_events
                .saturating_sub(comparison.routed.remote_ownership_events),
            ops_per_sec_improvement: comparison.routed.ops_per_sec()
                - comparison.baseline.ops_per_sec(),
            fairness_jain_delta: comparison.routed.fairness.jain_fairness_index()
                - comparison.baseline.fairness.jain_fairness_index(),
        },
        cases: vec![baseline, routed],
    }
}

fn assert_case_fields(case: &Value) {
    for field in [
        "trace_id",
        "scenario_id",
        "routing_mode",
        "placement_profile",
        "workload_shape",
        "conflict_rate",
        "retry_rate",
        "fallback_rate",
        "remote_ownership_events",
        "ops_per_sec",
        "p50_ns",
        "p95_ns",
        "p99_ns",
        "fairness_summary",
        "metadata_publication",
    ] {
        assert!(
            case.get(field).is_some(),
            "protocol case should include required field `{field}`"
        );
    }
    assert!(
        case["fairness_summary"].get("lane_writer_counts").is_some(),
        "fairness summary should include lane counts"
    );
    assert!(
        case["metadata_publication"]
            .get("publication_retry_rate")
            .is_some(),
        "metadata publication summary should include publication retry rate"
    );
}

fn assert_protocol_artifact_fields(artifact: &RoutingProtocolArtifact) {
    let artifact_json = serde_json::to_value(artifact).expect("protocol artifact should serialize");
    for field in [
        "bead_id",
        "trace_id",
        "scenario_id",
        "replay_command",
        "comparison",
        "cases",
    ] {
        assert!(
            artifact_json.get(field).is_some(),
            "protocol artifact should include required field `{field}`"
        );
    }
    for field in [
        "trace_id",
        "scenario_id",
        "placement_profile",
        "workload_shape",
        "conflict_rate_reduction",
        "retry_rate_reduction",
        "fallback_rate_reduction",
        "publication_retry_rate_reduction",
        "remote_ownership_events_reduction",
        "ops_per_sec_improvement",
        "fairness_jain_delta",
    ] {
        assert!(
            artifact_json["comparison"].get(field).is_some(),
            "protocol comparison should include required field `{field}`"
        );
    }
    let cases = artifact_json["cases"]
        .as_array()
        .expect("protocol cases should serialize as an array");
    assert_eq!(
        cases.len(),
        2,
        "protocol artifact should carry baseline and routed cases"
    );
    for case in cases {
        assert_case_fields(case);
    }
}

fn emit_protocol_artifact(test_name: &str, artifact: &RoutingProtocolArtifact) {
    eprintln!(
        "WRITER_ROUTING_PROTOCOL:{}",
        serde_json::json!({
            "test_name": test_name,
            "artifact": artifact,
        })
    );
}

#[test]
fn bd_db300_5_5_3_disjoint_workload_protocol_is_conflict_free() {
    let _guard = WRITER_ROUTING_PROTOCOL_LOCK
        .lock()
        .expect("writer routing protocol lock");

    let comparison = compare_writer_routing_synthetic_workload(
        &synthetic_config(
            "writer_routing.protocol.disjoint",
            WriterRoutingSyntheticWorkload::DisjointPages,
            WriterRoutingPlacementProfile::BaselineUnpinned,
        ),
        WriterRoutingDecisionConfig::default(),
    );
    let artifact = protocol_artifact("trace-bd-db300.5.5.3-disjoint", &comparison);

    assert_eq!(comparison.baseline.conflicts_total, 0);
    assert_eq!(comparison.routed.conflicts_total, 0);
    assert_eq!(comparison.baseline.retries_total, 0);
    assert_eq!(comparison.routed.retries_total, 0);
    assert_protocol_artifact_fields(&artifact);
    emit_protocol_artifact(
        "bd_db300_5_5_3_disjoint_workload_protocol_is_conflict_free",
        &artifact,
    );
}

#[test]
fn bd_db300_5_5_3_overlapping_workload_protocol_shows_conflict_reduction() {
    let _guard = WRITER_ROUTING_PROTOCOL_LOCK
        .lock()
        .expect("writer routing protocol lock");

    let comparison = compare_writer_routing_synthetic_workload(
        &synthetic_config(
            "writer_routing.protocol.overlap",
            WriterRoutingSyntheticWorkload::OverlappingPages,
            WriterRoutingPlacementProfile::HomeLanePinned,
        ),
        WriterRoutingDecisionConfig::default(),
    );
    let artifact = protocol_artifact("trace-bd-db300.5.5.3-overlap", &comparison);

    assert!(artifact.comparison.conflict_rate_reduction > 0.0);
    assert!(artifact.comparison.retry_rate_reduction > 0.0);
    assert!(artifact.comparison.publication_retry_rate_reduction > 0.0);
    assert!(artifact.comparison.remote_ownership_events_reduction > 0);
    assert_protocol_artifact_fields(&artifact);
    emit_protocol_artifact(
        "bd_db300_5_5_3_overlapping_workload_protocol_shows_conflict_reduction",
        &artifact,
    );
}

#[test]
fn bd_db300_5_5_3_hot_page_workload_protocol_shows_publication_relief() {
    let _guard = WRITER_ROUTING_PROTOCOL_LOCK
        .lock()
        .expect("writer routing protocol lock");

    let comparison = compare_writer_routing_synthetic_workload(
        &synthetic_config(
            "writer_routing.protocol.hot_page",
            WriterRoutingSyntheticWorkload::HotPageContention,
            WriterRoutingPlacementProfile::HomeNodePinned,
        ),
        WriterRoutingDecisionConfig::default(),
    );
    let artifact = protocol_artifact("trace-bd-db300.5.5.3-hot-page", &comparison);

    assert!(artifact.comparison.conflict_rate_reduction > 0.0);
    assert!(artifact.comparison.retry_rate_reduction > 0.0);
    assert!(artifact.comparison.publication_retry_rate_reduction > 0.0);
    assert!(artifact.comparison.ops_per_sec_improvement > 0.0);
    assert!(
        comparison.routed.visibility_handoff_nanos_total
            < comparison.baseline.visibility_handoff_nanos_total
    );
    assert_protocol_artifact_fields(&artifact);
    emit_protocol_artifact(
        "bd_db300_5_5_3_hot_page_workload_protocol_shows_publication_relief",
        &artifact,
    );
}
