use serde::Serialize;

/// Serializable summary of page-cache efficiency counters and compatibility
/// gauges exported by the pager layer.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct PageCacheEfficiencySnapshot {
    pub hits: u64,
    pub misses: u64,
    pub admits: u64,
    pub evictions: u64,
    pub cached_pages: usize,
    pub pool_capacity: usize,
    pub dirty_ratio_pct: u64,
    pub t1_size: usize,
    pub t2_size: usize,
    pub b1_size: usize,
    pub b2_size: usize,
    pub p_target: usize,
    pub mvcc_multi_version_pages: usize,
}

impl PageCacheEfficiencySnapshot {
    /// Total cache probes (`hits + misses`).
    #[must_use]
    pub fn total_accesses(self) -> u64 {
        self.hits.saturating_add(self.misses)
    }

    /// Hit-rate as a percentage in `[0.0, 100.0]`.
    #[must_use]
    pub fn hit_rate_percent(self) -> f64 {
        let total = self.total_accesses();
        if total == 0 {
            0.0
        } else {
            (self.hits as f64 * 100.0) / total as f64
        }
    }

    /// Miss-rate as a percentage in `[0.0, 100.0]`.
    #[must_use]
    pub fn miss_rate_percent(self) -> f64 {
        let total = self.total_accesses();
        if total == 0 {
            0.0
        } else {
            (self.misses as f64 * 100.0) / total as f64
        }
    }

    /// Resident-page occupancy as a percentage of configured pool capacity.
    #[must_use]
    pub fn occupancy_percent(self) -> f64 {
        if self.pool_capacity == 0 {
            0.0
        } else {
            (self.cached_pages as f64 * 100.0) / self.pool_capacity as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PageCacheEfficiencySnapshot;

    #[test]
    fn test_page_cache_efficiency_rates_are_zero_safe() {
        let snapshot = PageCacheEfficiencySnapshot::default();
        assert_eq!(snapshot.total_accesses(), 0);
        assert_eq!(snapshot.hit_rate_percent(), 0.0);
        assert_eq!(snapshot.miss_rate_percent(), 0.0);
        assert_eq!(snapshot.occupancy_percent(), 0.0);
    }

    #[test]
    fn test_page_cache_efficiency_rates_track_raw_counters() {
        let snapshot = PageCacheEfficiencySnapshot {
            hits: 9,
            misses: 3,
            admits: 4,
            evictions: 1,
            cached_pages: 12,
            pool_capacity: 48,
            dirty_ratio_pct: 25,
            t1_size: 7,
            t2_size: 5,
            b1_size: 2,
            b2_size: 1,
            p_target: 6,
            mvcc_multi_version_pages: 0,
        };

        assert_eq!(snapshot.total_accesses(), 12);
        assert!((snapshot.hit_rate_percent() - 75.0).abs() < f64::EPSILON);
        assert!((snapshot.miss_rate_percent() - 25.0).abs() < f64::EPSILON);
        assert!((snapshot.occupancy_percent() - 25.0).abs() < f64::EPSILON);
    }
}
