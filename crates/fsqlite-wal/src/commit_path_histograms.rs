//! Supplementary tests for commit-path histograms and wake-reason
//! accounting (bd-db300.3.8.1).
//!
//! The canonical types live in `group_commit.rs` (`PhaseHistogram`,
//! `PhasePercentiles`, `WakeReasonCounters`, `WakeReasonSnapshot`).
//! This module adds edge-case tests that the main test suite doesn't cover:
//! bimodal distributions, overflow behavior, snapshot consistency under
//! concurrent writes, and wake-reason total invariants.

#[cfg(test)]
mod tests {
    use crate::group_commit::{
        PhaseHistogram, WakeReasonCounters, GLOBAL_CONSOLIDATION_METRICS,
    };

    #[test]
    fn histogram_bimodal_p50_below_p99() {
        let h = PhaseHistogram::new();
        // 90 fast samples, 10 slow samples — bimodal.
        for _ in 0..90 {
            h.record(5);
        }
        for _ in 0..10 {
            h.record(5000);
        }
        let p = h.percentiles();
        assert_eq!(p.count, 100);
        assert!(
            p.p99 > p.p50,
            "bimodal: p99={} should exceed p50={}",
            p.p99,
            p.p50
        );
        assert_eq!(p.max, 5000);
    }

    #[test]
    fn histogram_all_same_value() {
        let h = PhaseHistogram::new();
        for _ in 0..200 {
            h.record(42);
        }
        let p = h.percentiles();
        assert_eq!(p.p50, 42);
        assert_eq!(p.p95, 42);
        assert_eq!(p.p99, 42);
        assert_eq!(p.max, 42);
        assert_eq!(p.mean_us, 42);
    }

    #[test]
    fn histogram_zero_samples_only() {
        let h = PhaseHistogram::new();
        for _ in 0..50 {
            h.record(0);
        }
        let p = h.percentiles();
        assert_eq!(p.count, 50);
        assert_eq!(p.p50, 0);
        assert_eq!(p.p99, 0);
        assert_eq!(p.max, 0);
        assert_eq!(p.mean_us, 0);
    }

    #[test]
    fn histogram_large_values_tracked() {
        let h = PhaseHistogram::new();
        h.record(1_000_000); // 1 second in microseconds
        let p = h.percentiles();
        assert_eq!(p.max, 1_000_000);
        assert_eq!(p.count, 1);
    }

    #[test]
    fn wake_reason_total_is_sum_of_all_fields() {
        let w = WakeReasonCounters::new();
        w.notify.fetch_add(10, std::sync::atomic::Ordering::Relaxed);
        w.timeout.fetch_add(3, std::sync::atomic::Ordering::Relaxed);
        w.flusher_takeover
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        w.failed_epoch
            .fetch_add(2, std::sync::atomic::Ordering::Relaxed);
        w.busy_retry
            .fetch_add(5, std::sync::atomic::Ordering::Relaxed);

        let s = w.snapshot();
        assert_eq!(
            s.total(),
            s.notify + s.timeout + s.flusher_takeover + s.failed_epoch + s.busy_retry,
            "total() must equal sum of individual fields"
        );
    }

    #[test]
    fn wake_reason_concurrent_increments_no_loss() {
        use std::sync::Arc;
        let w = Arc::new(WakeReasonCounters::new());
        let barrier = Arc::new(std::sync::Barrier::new(4));
        let mut handles = Vec::new();

        for _ in 0..4 {
            let w = Arc::clone(&w);
            let b = Arc::clone(&barrier);
            handles.push(std::thread::spawn(move || {
                b.wait();
                for _ in 0..1000 {
                    w.notify.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    w.timeout
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
            }));
        }
        for h in handles {
            h.join().expect("thread panicked");
        }

        let s = w.snapshot();
        assert_eq!(s.notify, 4000, "4 threads × 1000 increments");
        assert_eq!(s.timeout, 4000);
    }

    #[test]
    fn global_metrics_histograms_accessible() {
        // Verify the global singleton's histogram fields are accessible
        // and recordable without panic.
        GLOBAL_CONSOLIDATION_METRICS.hist_phase_b.record(100);
        GLOBAL_CONSOLIDATION_METRICS.hist_wal_append.record(50);
        GLOBAL_CONSOLIDATION_METRICS
            .wake_reasons
            .notify
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let s = GLOBAL_CONSOLIDATION_METRICS.snapshot();
        assert!(s.hist_phase_b.count >= 1);
        assert!(s.hist_wal_append.count >= 1);
        assert!(s.wake_reasons.notify >= 1);
    }

    #[test]
    fn histogram_percentile_monotonicity() {
        let h = PhaseHistogram::new();
        // Ascending values.
        for i in 1..=1000u64 {
            h.record(i);
        }
        let p = h.percentiles();
        assert!(p.p50 <= p.p95, "p50 <= p95");
        assert!(p.p95 <= p.p99, "p95 <= p99");
        assert!(p.p99 <= p.max, "p99 <= max");
    }

    #[test]
    fn histogram_reset_then_record_works() {
        let h = PhaseHistogram::new();
        for i in 0..100 {
            h.record(i);
        }
        h.reset();
        h.record(999);
        let p = h.percentiles();
        assert_eq!(p.count, 1);
        assert_eq!(p.max, 999);
        assert_eq!(p.p50, 999);
    }
}
