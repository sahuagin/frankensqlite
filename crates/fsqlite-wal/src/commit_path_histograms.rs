//! Supplementary edge-case tests for commit-path histograms and wake-reason
//! accounting (bd-db300.3.8.1).
//!
//! The canonical types (`PhaseHistogram`, `PhasePercentiles`,
//! `WakeReasonCounters`, `WakeReasonSnapshot`) live in `group_commit.rs`.
//! This module adds edge-case coverage via the global singleton.

#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering;

    use crate::group_commit::GLOBAL_CONSOLIDATION_METRICS;

    // ── Global histogram recording and snapshot ─────────────────────

    #[test]
    fn global_hist_phase_b_records_and_snapshots() {
        GLOBAL_CONSOLIDATION_METRICS.hist_phase_b.record(100);
        GLOBAL_CONSOLIDATION_METRICS.hist_phase_b.record(200);
        let s = GLOBAL_CONSOLIDATION_METRICS.snapshot();
        assert!(s.hist_phase_b.count >= 2, "at least 2 samples recorded");
        assert!(s.hist_phase_b.max >= 200, "max should track largest");
    }

    #[test]
    fn global_hist_wal_append_records() {
        GLOBAL_CONSOLIDATION_METRICS.hist_wal_append.record(50);
        let s = GLOBAL_CONSOLIDATION_METRICS.snapshot();
        assert!(s.hist_wal_append.count >= 1);
    }

    #[test]
    fn global_hist_exclusive_lock_records() {
        GLOBAL_CONSOLIDATION_METRICS.hist_exclusive_lock.record(10);
        let s = GLOBAL_CONSOLIDATION_METRICS.snapshot();
        assert!(s.hist_exclusive_lock.count >= 1);
    }

    #[test]
    fn global_hist_consolidator_lock_wait_records() {
        GLOBAL_CONSOLIDATION_METRICS
            .hist_consolidator_lock_wait
            .record(5);
        let s = GLOBAL_CONSOLIDATION_METRICS.snapshot();
        assert!(s.hist_consolidator_lock_wait.count >= 1);
    }

    #[test]
    fn global_hist_waiter_epoch_wait_records() {
        GLOBAL_CONSOLIDATION_METRICS
            .hist_waiter_epoch_wait
            .record(200);
        let s = GLOBAL_CONSOLIDATION_METRICS.snapshot();
        assert!(s.hist_waiter_epoch_wait.count >= 1);
    }

    #[test]
    fn global_hist_arrival_wait_records() {
        GLOBAL_CONSOLIDATION_METRICS.hist_arrival_wait.record(15);
        let s = GLOBAL_CONSOLIDATION_METRICS.snapshot();
        assert!(s.hist_arrival_wait.count >= 1);
    }

    #[test]
    fn global_hist_wal_sync_records() {
        GLOBAL_CONSOLIDATION_METRICS.hist_wal_sync.record(80);
        let s = GLOBAL_CONSOLIDATION_METRICS.snapshot();
        assert!(s.hist_wal_sync.count >= 1);
    }

    #[test]
    fn global_hist_full_commit_records() {
        GLOBAL_CONSOLIDATION_METRICS.hist_full_commit.record(300);
        let s = GLOBAL_CONSOLIDATION_METRICS.snapshot();
        assert!(s.hist_full_commit.count >= 1);
    }

    // ── Wake-reason global counters ─────────────────────────────────

    #[test]
    fn global_wake_reason_notify_increments() {
        GLOBAL_CONSOLIDATION_METRICS
            .wake_reasons
            .notify
            .fetch_add(1, Ordering::Relaxed);
        let s = GLOBAL_CONSOLIDATION_METRICS.snapshot();
        assert!(s.wake_reasons.notify >= 1);
    }

    #[test]
    fn global_wake_reason_timeout_increments() {
        GLOBAL_CONSOLIDATION_METRICS
            .wake_reasons
            .timeout
            .fetch_add(1, Ordering::Relaxed);
        let s = GLOBAL_CONSOLIDATION_METRICS.snapshot();
        assert!(s.wake_reasons.timeout >= 1);
    }

    #[test]
    fn global_wake_reason_flusher_takeover_increments() {
        GLOBAL_CONSOLIDATION_METRICS
            .wake_reasons
            .flusher_takeover
            .fetch_add(1, Ordering::Relaxed);
        let s = GLOBAL_CONSOLIDATION_METRICS.snapshot();
        assert!(s.wake_reasons.flusher_takeover >= 1);
    }

    #[test]
    fn global_wake_reason_failed_epoch_increments() {
        GLOBAL_CONSOLIDATION_METRICS
            .wake_reasons
            .failed_epoch
            .fetch_add(1, Ordering::Relaxed);
        let s = GLOBAL_CONSOLIDATION_METRICS.snapshot();
        assert!(s.wake_reasons.failed_epoch >= 1);
    }

    #[test]
    fn global_wake_reason_busy_retry_increments() {
        GLOBAL_CONSOLIDATION_METRICS
            .wake_reasons
            .busy_retry
            .fetch_add(1, Ordering::Relaxed);
        let s = GLOBAL_CONSOLIDATION_METRICS.snapshot();
        assert!(s.wake_reasons.busy_retry >= 1);
    }

    #[test]
    fn global_wake_reason_total_is_nonnegative() {
        let s = GLOBAL_CONSOLIDATION_METRICS.snapshot();
        // total() must never be less than any individual field.
        assert!(s.wake_reasons.total() >= s.wake_reasons.notify);
        assert!(s.wake_reasons.total() >= s.wake_reasons.timeout);
        assert!(s.wake_reasons.total() >= s.wake_reasons.flusher_takeover);
        assert!(s.wake_reasons.total() >= s.wake_reasons.failed_epoch);
        assert!(s.wake_reasons.total() >= s.wake_reasons.busy_retry);
    }

    // ── Histogram percentile structure ──────────────────────────────

    #[test]
    fn global_hist_percentiles_are_ordered() {
        // Record a spread of values into a fresh-ish histogram.
        for i in 1..=100u64 {
            GLOBAL_CONSOLIDATION_METRICS
                .hist_wal_backend_lock_wait
                .record(i);
        }
        let s = GLOBAL_CONSOLIDATION_METRICS.snapshot();
        let p = s.hist_wal_backend_lock_wait;
        assert!(p.count >= 100);
        // Percentiles from a sorted ring must be monotone.
        // (The global may have prior samples from other tests, so we just
        // check the structural invariant.)
        assert!(p.p50 <= p.p95 || p.count == 0, "p50 <= p95");
        assert!(p.p95 <= p.p99 || p.count == 0, "p95 <= p99");
    }
}
