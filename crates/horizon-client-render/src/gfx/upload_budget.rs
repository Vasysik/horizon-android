//! Frame-local terrain transfer scheduling. This does not change view distance,
//! simulation rate or mesh quality. One oversized section may always advance.
use std::time::Duration;

pub struct UploadBudget {
    count_limit: usize,
    byte_limit: u64,
    time_limit: Option<Duration>,
    count: usize,
    bytes: u64,
}
impl UploadBudget {
    /// Use a little of the previous frame's GPU/display wait when there is
    /// headroom, without turning a slow frame into an unlimited upload batch.
    pub fn with_wait(count_limit: usize, byte_limit: u64, mobile: bool, wait: Duration) -> Self {
        let mut budget = Self::new(count_limit, byte_limit, mobile);
        if mobile {
            budget.time_limit =
                Some((wait / 4).clamp(Duration::from_millis(2), Duration::from_millis(8)));
        }
        budget
    }
    /// Frame-state aware budget for Android: prevents a missed VSync wait or
    /// active camera/menu interaction from triggering multi-millisecond mesh upload stalls.
    pub fn with_frame_state(
        count_limit: usize,
        byte_limit: u64,
        mobile: bool,
        wait: Duration,
        frame_dt: Duration,
        interactive: bool,
    ) -> Self {
        let mut budget = Self::with_wait(count_limit, byte_limit, mobile, wait);
        if mobile {
            let max_upload = if crate::android_runtime::gpu_batching() {
                4 << 20
            } else {
                2 << 20
            };
            let max_time = if crate::android_runtime::gpu_batching() {
                Duration::from_millis(3)
            } else {
                Duration::from_millis(2)
            };
            if frame_dt > Duration::from_millis(22) {
                budget.byte_limit = budget.byte_limit.min(1 << 20);
                budget.time_limit = Some(Duration::from_millis(1));
            } else if frame_dt > Duration::from_millis(18) || interactive {
                budget.byte_limit = budget.byte_limit.min(max_upload / 2);
                budget.time_limit =
                    Some(max_time.clamp(Duration::from_millis(1), Duration::from_millis(2)));
            } else {
                budget.byte_limit = budget.byte_limit.min(max_upload);
                budget.time_limit = Some(max_time);
            }
        }
        budget
    }
    pub fn new(count_limit: usize, byte_limit: u64, mobile: bool) -> Self {
        Self {
            count_limit,
            byte_limit: match (byte_limit, mobile) {
                (0, true) => 8 << 20,
                (bytes, true) => bytes.min(8 << 20),
                (bytes, false) => bytes,
            },
            time_limit: mobile.then_some(Duration::from_millis(2)),
            count: 0,
            bytes: 0,
        }
    }
    pub fn allows(&self, bytes: u64, elapsed: Duration) -> bool {
        self.count == 0
            || ((self.count_limit == 0 || self.count < self.count_limit)
                && (self.byte_limit == 0 || self.bytes.saturating_add(bytes) <= self.byte_limit)
                && self.time_limit.is_none_or(|limit| elapsed < limit))
    }
    pub fn record(&mut self, bytes: u64) {
        self.count += 1;
        self.bytes = self.bytes.saturating_add(bytes);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mobile_stops_a_backlog_without_starving_large_sections() {
        let mut budget = UploadBudget::new(0, 0, true);
        assert!(budget.allows(20 << 20, Duration::from_secs(1)));
        budget.record(20 << 20);
        assert!(!budget.allows(1, Duration::ZERO));
        let next_frame = UploadBudget::new(0, 0, true);
        assert!(next_frame.allows(20 << 20, Duration::ZERO));
    }
    #[test]
    fn elapsed_work_yields_even_with_small_meshes() {
        let mut budget = UploadBudget::new(0, 0, true);
        budget.record(32);
        assert!(budget.allows(32, Duration::from_millis(1)));
        assert!(!budget.allows(32, Duration::from_millis(2)));
    }
    #[test]
    fn gpu_wait_adds_bounded_headroom() {
        let mut budget = UploadBudget::with_wait(0, 0, true, Duration::from_millis(40));
        budget.record(32);
        assert!(budget.allows(32, Duration::from_millis(6)));
        assert!(!budget.allows(32, Duration::from_millis(8)));
    }
    #[test]
    fn user_limits_remain_stricter_and_desktop_unlimited_is_unchanged() {
        let mut count = UploadBudget::new(1, 0, true);
        count.record(8);
        assert!(!count.allows(8, Duration::ZERO));
        let mut bytes = UploadBudget::new(0, 64, true);
        bytes.record(32);
        assert!(!bytes.allows(33, Duration::ZERO));
        let mut desktop = UploadBudget::new(0, 0, false);
        desktop.record(1 << 30);
        assert!(desktop.allows(1 << 30, Duration::from_secs(30)));
    }
}
