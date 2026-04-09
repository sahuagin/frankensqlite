//! Conflict-topology-aware writer-routing telemetry inputs.
//!
//! Track E5.1 does not introduce a second telemetry stack. Instead, it pins
//! the existing MVCC/VDBE conflict signals to a stable contract so later
//! routing beads can consume the same hot-path evidence without reopening the
//! capture design.

use fsqlite_types::{CommitSeq, PageNumber, TxnId, TxnToken};
use smallvec::SmallVec;

use crate::ssi_validation::DiscoveredEdge;

/// Stable signal identifiers for writer-routing telemetry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WriterRoutingTelemetrySignal {
    TieredWriteCounts,
    ReadPages,
    WriteSetPages,
    HeldLockPages,
    ConflictOnlyPages,
    MetadataExemptPages,
    SamePageConflictPages,
    PageLockWait,
    BusyRetry,
    StaleSnapshotReject,
    PageOneConflictOnly,
    PendingSurfaceClear,
    LockHolderClues,
    SerializableConflictEdges,
}

/// High-level grouping for routing inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WriterRoutingTelemetryClass {
    TouchSurface,
    ConflictHistory,
    OwnershipLineage,
}

/// Phase that currently produces the signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WriterRoutingTelemetryPhase {
    StatementExecution,
    FirstTouchLockAcquire,
    CommitPlanning,
    CommitFinalize,
    RetryLoop,
}

/// Payload shape exposed by the source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WriterRoutingTelemetryShape {
    Counter,
    DurationCounter,
    PageSet,
    OwnershipSet,
    EdgeSet,
}

/// Capture-cost rule for the current hot path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WriterRoutingTelemetryCaptureCost {
    /// Reuse an already-maintained counter or timer.
    ExistingCounter,
    /// Reuse an already-maintained in-memory page/txn set.
    ExistingSet,
    /// Clone the data once at prepare/finalize, not per page-touch.
    PrepareBoundaryClone,
    /// Fold existing telemetry after the hot path has completed.
    DeferredFold,
}

/// Design-time source contract for one routing signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WriterRoutingTelemetrySourceSpec {
    /// Stable signal identifier.
    pub signal: WriterRoutingTelemetrySignal,
    /// Touch-surface vs conflict-history vs ownership-lineage grouping.
    pub class: WriterRoutingTelemetryClass,
    /// Phase that owns the signal today.
    pub phase: WriterRoutingTelemetryPhase,
    /// Counter/page-set/edge-set payload shape.
    pub shape: WriterRoutingTelemetryShape,
    /// Concrete code touchpoint producing the evidence.
    pub touchpoint: &'static str,
    /// Existing runtime artifact or counter family to reuse.
    pub current_artifact: &'static str,
    /// Allowed capture budget on the hot path.
    pub hot_path_cost: WriterRoutingTelemetryCaptureCost,
    /// Why a routing policy cares about this signal.
    pub routing_use: &'static str,
}

/// Stable routing-input inventory for Track E5.1.
pub const WRITER_ROUTING_TELEMETRY_SOURCES: [WriterRoutingTelemetrySourceSpec; 14] = [
    WriterRoutingTelemetrySourceSpec {
        signal: WriterRoutingTelemetrySignal::TieredWriteCounts,
        class: WriterRoutingTelemetryClass::TouchSurface,
        phase: WriterRoutingTelemetryPhase::StatementExecution,
        shape: WriterRoutingTelemetryShape::Counter,
        touchpoint: "fsqlite-vdbe/src/engine.rs::SharedTxnPageIo::{classify_concurrent_write_tier,write_page_data}",
        current_artifact: "VDBE mvcc_write_path snapshot tier{0,1,2}_*_writes_total",
        hot_path_cost: WriterRoutingTelemetryCaptureCost::ExistingCounter,
        routing_use: "Distinguish already-owned writes from first-touch and commit-surface expansion pressure.",
    },
    WriterRoutingTelemetrySourceSpec {
        signal: WriterRoutingTelemetrySignal::ReadPages,
        class: WriterRoutingTelemetryClass::TouchSurface,
        phase: WriterRoutingTelemetryPhase::CommitPlanning,
        shape: WriterRoutingTelemetryShape::PageSet,
        touchpoint: "fsqlite-mvcc/src/begin_concurrent.rs::ConcurrentHandle::read_set / PreparedConcurrentCommit::read_pages",
        current_artifact: "ConcurrentHandle read_set summarized into PreparedConcurrentCommit::read_pages()",
        hot_path_cost: WriterRoutingTelemetryCaptureCost::PrepareBoundaryClone,
        routing_use: "Identify readers that repeatedly pivot into conflicting write surfaces.",
    },
    WriterRoutingTelemetrySourceSpec {
        signal: WriterRoutingTelemetrySignal::WriteSetPages,
        class: WriterRoutingTelemetryClass::TouchSurface,
        phase: WriterRoutingTelemetryPhase::CommitPlanning,
        shape: WriterRoutingTelemetryShape::PageSet,
        touchpoint: "fsqlite-mvcc/src/begin_concurrent.rs::ConcurrentHandle::write_set_pages / PreparedConcurrentCommit::write_set_pages",
        current_artifact: "Sorted write-set pages already materialized for FCW/SSI prepare",
        hot_path_cost: WriterRoutingTelemetryCaptureCost::PrepareBoundaryClone,
        routing_use: "Feed same-page conflict history and writer-home locality decisions.",
    },
    WriterRoutingTelemetrySourceSpec {
        signal: WriterRoutingTelemetrySignal::HeldLockPages,
        class: WriterRoutingTelemetryClass::TouchSurface,
        phase: WriterRoutingTelemetryPhase::CommitPlanning,
        shape: WriterRoutingTelemetryShape::PageSet,
        touchpoint: "fsqlite-mvcc/src/begin_concurrent.rs::ConcurrentHandle::held_lock_pages / PreparedConcurrentCommit::held_lock_pages",
        current_artifact: "Tracked held page locks already used for commit finalization and release",
        hot_path_cost: WriterRoutingTelemetryCaptureCost::PrepareBoundaryClone,
        routing_use: "Reveal ownership concentration and lock reuse for later writer placement.",
    },
    WriterRoutingTelemetrySourceSpec {
        signal: WriterRoutingTelemetrySignal::ConflictOnlyPages,
        class: WriterRoutingTelemetryClass::TouchSurface,
        phase: WriterRoutingTelemetryPhase::CommitPlanning,
        shape: WriterRoutingTelemetryShape::PageSet,
        touchpoint: "fsqlite-mvcc/src/begin_concurrent.rs::PageTxnState::is_conflict_only",
        current_artifact: "Synthetic conflict-tracking state embedded in ConcurrentHandle page_states",
        hot_path_cost: WriterRoutingTelemetryCaptureCost::DeferredFold,
        routing_use: "Separate structural conflict surfaces from direct row/page ownership.",
    },
    WriterRoutingTelemetrySourceSpec {
        signal: WriterRoutingTelemetrySignal::MetadataExemptPages,
        class: WriterRoutingTelemetryClass::TouchSurface,
        phase: WriterRoutingTelemetryPhase::CommitPlanning,
        shape: WriterRoutingTelemetryShape::PageSet,
        touchpoint: "fsqlite-mvcc/src/begin_concurrent.rs::PageTxnState::metadata_exempt",
        current_artifact: "Metadata-exempt page marks carried in ConcurrentHandle page_states",
        hot_path_cost: WriterRoutingTelemetryCaptureCost::DeferredFold,
        routing_use: "Prevent routing from overreacting to page-one/freelist metadata that is intentionally conflict-exempt.",
    },
    WriterRoutingTelemetrySourceSpec {
        signal: WriterRoutingTelemetrySignal::SamePageConflictPages,
        class: WriterRoutingTelemetryClass::ConflictHistory,
        phase: WriterRoutingTelemetryPhase::CommitPlanning,
        shape: WriterRoutingTelemetryShape::PageSet,
        touchpoint: "fsqlite-mvcc/src/begin_concurrent.rs::PreparedConcurrentCommit::conflict_pages / validate_first_committer_wins",
        current_artifact: "PreparedConcurrentCommit conflict pages plus FCW conflicting page set",
        hot_path_cost: WriterRoutingTelemetryCaptureCost::PrepareBoundaryClone,
        routing_use: "Measure repeated same-page collisions, the primary topology signal for writer routing.",
    },
    WriterRoutingTelemetrySourceSpec {
        signal: WriterRoutingTelemetrySignal::PageLockWait,
        class: WriterRoutingTelemetryClass::ConflictHistory,
        phase: WriterRoutingTelemetryPhase::FirstTouchLockAcquire,
        shape: WriterRoutingTelemetryShape::DurationCounter,
        touchpoint: "fsqlite-vdbe/src/engine.rs::wait_for_page_lock_holder_change / fsqlite-mvcc/src/core_types.rs::InProcessPageLockTable::wait_for_holder_change",
        current_artifact: "VDBE mvcc_write_path snapshot page_lock_waits_total + page_lock_wait_time_ns_total",
        hot_path_cost: WriterRoutingTelemetryCaptureCost::ExistingCounter,
        routing_use: "Quantify how often ownership handoff blocks first-touch progress.",
    },
    WriterRoutingTelemetrySourceSpec {
        signal: WriterRoutingTelemetrySignal::BusyRetry,
        class: WriterRoutingTelemetryClass::ConflictHistory,
        phase: WriterRoutingTelemetryPhase::RetryLoop,
        shape: WriterRoutingTelemetryShape::Counter,
        touchpoint: "fsqlite-vdbe/src/engine.rs wait/busy loop + fsqlite-core/src/connection.rs begin busy handoff",
        current_artifact: "VDBE mvcc_write_path snapshot write_busy_retries_total + write_busy_timeouts_total",
        hot_path_cost: WriterRoutingTelemetryCaptureCost::ExistingCounter,
        routing_use: "Expose retried lock conflicts separately from hard stale-snapshot aborts.",
    },
    WriterRoutingTelemetrySourceSpec {
        signal: WriterRoutingTelemetrySignal::StaleSnapshotReject,
        class: WriterRoutingTelemetryClass::ConflictHistory,
        phase: WriterRoutingTelemetryPhase::RetryLoop,
        shape: WriterRoutingTelemetryShape::Counter,
        touchpoint: "fsqlite-vdbe/src/engine.rs stale-snapshot rejection sites + fsqlite-mvcc/src/begin_concurrent.rs::validate_first_committer_wins",
        current_artifact: "VDBE mvcc_write_path snapshot stale_snapshot_rejects_total",
        hot_path_cost: WriterRoutingTelemetryCaptureCost::ExistingCounter,
        routing_use: "Tell routing when conflicts are snapshot-age driven rather than raw lock ownership.",
    },
    WriterRoutingTelemetrySourceSpec {
        signal: WriterRoutingTelemetrySignal::PageOneConflictOnly,
        class: WriterRoutingTelemetryClass::ConflictHistory,
        phase: WriterRoutingTelemetryPhase::CommitPlanning,
        shape: WriterRoutingTelemetryShape::DurationCounter,
        touchpoint: "fsqlite-vdbe/src/engine.rs::track_concurrent_conflict_only_page",
        current_artifact: "VDBE mvcc_write_path snapshot page_one_conflict_tracks_total + page_one_conflict_track_time_ns_total",
        hot_path_cost: WriterRoutingTelemetryCaptureCost::ExistingCounter,
        routing_use: "Separate structural page-one expansion from genuine data-page overlap.",
    },
    WriterRoutingTelemetrySourceSpec {
        signal: WriterRoutingTelemetrySignal::PendingSurfaceClear,
        class: WriterRoutingTelemetryClass::ConflictHistory,
        phase: WriterRoutingTelemetryPhase::CommitPlanning,
        shape: WriterRoutingTelemetryShape::DurationCounter,
        touchpoint: "fsqlite-vdbe/src/engine.rs::SharedTxnPageIo::clear_stale_synthetic_pending_commit_surface",
        current_artifact: "VDBE mvcc_write_path snapshot pending_commit_surface_clears_total + pending_commit_surface_clear_time_ns_total",
        hot_path_cost: WriterRoutingTelemetryCaptureCost::ExistingCounter,
        routing_use: "Show how often synthetic structural state is cleared before routing blames hot pages.",
    },
    WriterRoutingTelemetrySourceSpec {
        signal: WriterRoutingTelemetrySignal::LockHolderClues,
        class: WriterRoutingTelemetryClass::OwnershipLineage,
        phase: WriterRoutingTelemetryPhase::FirstTouchLockAcquire,
        shape: WriterRoutingTelemetryShape::OwnershipSet,
        touchpoint: "fsqlite-mvcc/src/core_types.rs::InProcessPageLockTable::{try_acquire,holder}",
        current_artifact: "Page-lock holder TxnId returned on contention and available through holder(page)",
        hot_path_cost: WriterRoutingTelemetryCaptureCost::ExistingSet,
        routing_use: "Capture remote-ownership clues for the page currently blocking a writer.",
    },
    WriterRoutingTelemetrySourceSpec {
        signal: WriterRoutingTelemetrySignal::SerializableConflictEdges,
        class: WriterRoutingTelemetryClass::OwnershipLineage,
        phase: WriterRoutingTelemetryPhase::CommitPlanning,
        shape: WriterRoutingTelemetryShape::EdgeSet,
        touchpoint: "fsqlite-mvcc/src/begin_concurrent.rs::PreparedConcurrentCommit::{incoming_edges,outgoing_edges,conflicting_txns}",
        current_artifact: "Prepared SSI edge sets and conflicting_txns() result",
        hot_path_cost: WriterRoutingTelemetryCaptureCost::PrepareBoundaryClone,
        routing_use: "Preserve lineage from lock-holder clues to committed serialization conflicts.",
    },
];

/// Per-tier counts for the local MVCC write path.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WriterTierSurfaceCounts {
    pub tier0_already_owned: u64,
    pub tier1_first_touch: u64,
    pub tier2_commit_surface_rare: u64,
}

/// Per-attempt page surfaces relevant to routing.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WriterTouchSurfaceTelemetry {
    /// Pages read by the transaction before it became a writer.
    pub read_pages: SmallVec<[PageNumber; 16]>,
    /// Pages directly written or freed by the transaction.
    pub write_set_pages: SmallVec<[PageNumber; 16]>,
    /// Pages whose locks are currently or were recently held by the transaction.
    pub held_lock_pages: SmallVec<[PageNumber; 16]>,
    /// Synthetic conflict-only pages added for structural safety.
    pub conflict_only_pages: SmallVec<[PageNumber; 8]>,
    /// Pages intentionally excluded from FCW conflict tracking.
    pub metadata_exempt_pages: SmallVec<[PageNumber; 4]>,
    /// Pages that actually collided during FCW/SSI prepare.
    pub same_page_conflict_pages: SmallVec<[PageNumber; 8]>,
    /// Aggregate write-path classification counts to combine with the page sets.
    pub tier_counts: WriterTierSurfaceCounts,
}

/// Stable retry-cause labels for second-pass routing telemetry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WriterRetryCause {
    PageLockContention,
    StructuralPageOne,
    PendingSurfaceExpansion,
    PublicationAdvance,
    StaleSnapshot,
    BusyTimeout,
}

/// One retry-cause bucket tied to a small page sample.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriterRetryAttribution {
    pub cause: WriterRetryCause,
    pub count: u64,
    pub wait_nanos: u64,
    pub pages: SmallVec<[PageNumber; 4]>,
}

/// Aggregate conflict-frequency inputs for routing.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WriterConflictHistoryTelemetry {
    pub same_page_conflict_count: u64,
    pub page_lock_wait_count: u64,
    pub page_lock_wait_nanos: u64,
    pub busy_retry_count: u64,
    pub busy_timeout_count: u64,
    pub stale_snapshot_reject_count: u64,
    pub page_one_conflict_only_count: u64,
    pub page_one_conflict_only_nanos: u64,
    pub pending_surface_clear_count: u64,
    pub pending_surface_clear_nanos: u64,
    pub retry_attributions: SmallVec<[WriterRetryAttribution; 4]>,
}

/// Immediate ownership clue returned by the page-lock table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WriterLockHolderClue {
    pub page: PageNumber,
    pub holder: TxnId,
}

/// Ownership lineage inputs spanning lock holders and SSI edges.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WriterOwnershipLineageTelemetry {
    /// Active lock holders currently blocking the writer.
    pub lock_holder_clues: SmallVec<[WriterLockHolderClue; 8]>,
    /// Distinct txns discovered as conflicting during prepare/finalize.
    pub conflicting_txns: SmallVec<[TxnToken; 8]>,
    /// Incoming rw-antidependencies discovered during prepare.
    pub incoming_edges: SmallVec<[DiscoveredEdge; 4]>,
    /// Outgoing rw-antidependencies discovered during prepare.
    pub outgoing_edges: SmallVec<[DiscoveredEdge; 4]>,
}

/// Routing input bundle assembled from the existing MVCC/VDBE telemetry planes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriterRoutingTelemetryInput {
    pub session_id: Option<u64>,
    pub txn_token: TxnToken,
    pub begin_seq: CommitSeq,
    pub planned_commit_seq: Option<CommitSeq>,
    pub touch_surface: WriterTouchSurfaceTelemetry,
    pub conflict_history: WriterConflictHistoryTelemetry,
    pub ownership_lineage: WriterOwnershipLineageTelemetry,
}

#[cfg(test)]
mod tests {
    use super::{
        WRITER_ROUTING_TELEMETRY_SOURCES, WriterRoutingTelemetryCaptureCost,
        WriterRoutingTelemetryClass, WriterRoutingTelemetrySignal,
    };

    fn has_signal(signal: WriterRoutingTelemetrySignal) -> bool {
        WRITER_ROUTING_TELEMETRY_SOURCES
            .iter()
            .any(|source| source.signal == signal)
    }

    #[test]
    fn test_writer_routing_sources_cover_required_first_pass_signals() {
        assert!(has_signal(WriterRoutingTelemetrySignal::TieredWriteCounts));
        assert!(has_signal(WriterRoutingTelemetrySignal::PageLockWait));
        assert!(has_signal(WriterRoutingTelemetrySignal::BusyRetry));
        assert!(has_signal(WriterRoutingTelemetrySignal::StaleSnapshotReject));
        assert!(has_signal(WriterRoutingTelemetrySignal::PageOneConflictOnly));
        assert!(has_signal(WriterRoutingTelemetrySignal::PendingSurfaceClear));
    }

    #[test]
    fn test_writer_routing_sources_cover_same_page_conflicts_and_ownership() {
        assert!(has_signal(WriterRoutingTelemetrySignal::WriteSetPages));
        assert!(has_signal(WriterRoutingTelemetrySignal::SamePageConflictPages));
        assert!(has_signal(WriterRoutingTelemetrySignal::LockHolderClues));
        assert!(has_signal(WriterRoutingTelemetrySignal::SerializableConflictEdges));
    }

    #[test]
    fn test_writer_routing_hot_path_budget_reuses_existing_planes() {
        let allowed = [
            WriterRoutingTelemetryCaptureCost::ExistingCounter,
            WriterRoutingTelemetryCaptureCost::ExistingSet,
            WriterRoutingTelemetryCaptureCost::PrepareBoundaryClone,
            WriterRoutingTelemetryCaptureCost::DeferredFold,
        ];
        assert!(
            WRITER_ROUTING_TELEMETRY_SOURCES
                .iter()
                .all(|source| allowed.contains(&source.hot_path_cost)),
            "routing telemetry must only reuse existing hot-path state or fold it after the fact"
        );
        assert!(
            WRITER_ROUTING_TELEMETRY_SOURCES.iter().any(
                |source| source.class == WriterRoutingTelemetryClass::OwnershipLineage
            ),
            "routing contract must include ownership lineage, not just counters"
        );
    }
}
