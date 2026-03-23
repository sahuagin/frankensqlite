//! Targeted fault hooks for batched WAL append and sync verification.

use std::mem;
use std::sync::{LazyLock, Mutex};

use fsqlite_error::{FrankenError, Result};
use fsqlite_types::flags::SyncFlags;
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

#[derive(Debug, Clone)]
struct CountdownFaultArm {
    arm: FaultHookArm,
    remaining_invocations: u32,
}

#[derive(Debug, Default)]
struct WalFaultHookState {
    next_trigger_seq: u64,
    after_append: Option<FaultHookArm>,
    sync_failure: Option<FaultHookArm>,
    append_busy: Option<CountdownFaultArm>,
    records: Vec<FaultInjectionRecord>,
}

static WAL_FAULT_HOOK_STATE: LazyLock<Mutex<WalFaultHookState>> =
    LazyLock::new(|| Mutex::new(WalFaultHookState::default()));

pub fn clear() {
    let mut state = WAL_FAULT_HOOK_STATE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    *state = WalFaultHookState::default();
}

#[must_use]
pub fn take_records() -> Vec<FaultInjectionRecord> {
    let mut state = WAL_FAULT_HOOK_STATE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    mem::take(&mut state.records)
}

pub fn arm_after_append(arm: FaultHookArm) {
    let mut state = WAL_FAULT_HOOK_STATE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    state.after_append = Some(arm);
}

pub fn arm_sync_failure(arm: FaultHookArm) {
    let mut state = WAL_FAULT_HOOK_STATE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    state.sync_failure = Some(arm);
}

pub fn arm_append_busy_countdown(arm: FaultHookArm, fire_on_nth_invocation: u32) {
    let mut state = WAL_FAULT_HOOK_STATE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    state.append_busy = Some(CountdownFaultArm {
        arm,
        remaining_invocations: fire_on_nth_invocation.max(1),
    });
}

pub(crate) fn maybe_inject_append_busy(
    frame_count_before: usize,
    submitted_frames: usize,
) -> Result<()> {
    let mut state = WAL_FAULT_HOOK_STATE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let Some(mut countdown) = state.append_busy.take() else {
        return Ok(());
    };
    if countdown.remaining_invocations > 1 {
        countdown.remaining_invocations -= 1;
        state.append_busy = Some(countdown);
        return Ok(());
    }

    let detail =
        format!("frame_count_before={frame_count_before} submitted_frames={submitted_frames}");
    record_trigger(
        &mut state,
        &countdown.arm,
        "wal_append_busy_countdown",
        detail,
    );
    Err(FrankenError::Busy)
}

pub(crate) fn maybe_inject_after_append(
    frame_count_before: usize,
    appended_frames: usize,
) -> Result<()> {
    let mut state = WAL_FAULT_HOOK_STATE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let Some(arm) = state.after_append.take() else {
        return Ok(());
    };

    let detail =
        format!("frame_count_before={frame_count_before} appended_frames={appended_frames}");
    record_trigger(&mut state, &arm, "wal_after_append", detail);
    Err(fault_error("wal_after_append", &arm))
}

pub(crate) fn maybe_inject_sync_failure(frame_count_before: usize, flags: SyncFlags) -> Result<()> {
    let mut state = WAL_FAULT_HOOK_STATE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let Some(arm) = state.sync_failure.take() else {
        return Ok(());
    };

    let detail = format!("frame_count_before={frame_count_before} flags={flags:?}");
    record_trigger(&mut state, &arm, "wal_sync_failure", detail);
    Err(fault_error("wal_sync_failure", &arm))
}

fn record_trigger(
    state: &mut WalFaultHookState,
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
        target: "fsqlite_wal::fault_injection",
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

fn fault_error(point: &str, arm: &FaultHookArm) -> FrankenError {
    FrankenError::Io(std::io::Error::other(format!(
        "fault_inject:{point} run_id={} scenario_id={} invariant_family={}",
        arm.run_id, arm.scenario_id, arm.invariant_family
    )))
}
