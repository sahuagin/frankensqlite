//! Targeted fault hooks for group-commit publish verification.

use std::mem;
use std::sync::{LazyLock, Mutex};

use fsqlite_error::{FrankenError, Result};
use tracing::warn;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FaultHookArm {
    pub run_id: String,
    pub scenario_id: String,
    pub invariant_family: String,
}

impl FaultHookArm {
    #[must_use]
    pub fn new(
        run_id: impl Into<String>,
        scenario_id: impl Into<String>,
        invariant_family: impl Into<String>,
    ) -> Self {
        Self {
            run_id: run_id.into(),
            scenario_id: scenario_id.into(),
            invariant_family: invariant_family.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FaultInjectionRecord {
    pub trigger_seq: u64,
    pub point: &'static str,
    pub run_id: String,
    pub scenario_id: String,
    pub invariant_family: String,
    pub detail: String,
}

#[derive(Debug, Default)]
struct PagerFaultHookState {
    next_trigger_seq: u64,
    after_flush_before_publish: Option<FaultHookArm>,
    records: Vec<FaultInjectionRecord>,
}

static PAGER_FAULT_HOOK_STATE: LazyLock<Mutex<PagerFaultHookState>> =
    LazyLock::new(|| Mutex::new(PagerFaultHookState::default()));

pub fn clear() {
    let mut state = PAGER_FAULT_HOOK_STATE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    *state = PagerFaultHookState::default();
}

#[must_use]
pub fn take_records() -> Vec<FaultInjectionRecord> {
    let mut state = PAGER_FAULT_HOOK_STATE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    mem::take(&mut state.records)
}

pub fn arm_after_flush_before_publish(arm: FaultHookArm) {
    let mut state = PAGER_FAULT_HOOK_STATE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    state.after_flush_before_publish = Some(arm);
}

pub(crate) fn maybe_inject_after_flush_before_publish(
    flush_epoch: u64,
    batch_count: usize,
    frame_count: usize,
) -> Result<()> {
    let mut state = PAGER_FAULT_HOOK_STATE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let Some(arm) = state.after_flush_before_publish.take() else {
        return Ok(());
    };

    let detail =
        format!("flush_epoch={flush_epoch} batch_count={batch_count} frame_count={frame_count}");
    record_trigger(&mut state, &arm, "after_flush_before_publish", detail);
    Err(FrankenError::Io(std::io::Error::other(format!(
        "fault_inject:after_flush_before_publish run_id={} scenario_id={} invariant_family={}",
        arm.run_id, arm.scenario_id, arm.invariant_family
    ))))
}

fn record_trigger(
    state: &mut PagerFaultHookState,
    arm: &FaultHookArm,
    point: &'static str,
    detail: String,
) {
    state.next_trigger_seq = state.next_trigger_seq.saturating_add(1);
    let record = FaultInjectionRecord {
        trigger_seq: state.next_trigger_seq,
        point,
        run_id: arm.run_id.clone(),
        scenario_id: arm.scenario_id.clone(),
        invariant_family: arm.invariant_family.clone(),
        detail,
    };
    warn!(
        target: "fsqlite_pager::fault_injection",
        trigger_seq = record.trigger_seq,
        point = record.point,
        run_id = %record.run_id,
        scenario_id = %record.scenario_id,
        invariant_family = %record.invariant_family,
        detail = %record.detail,
        "fault hook fired"
    );
    state.records.push(record);
}
