//! Estimates for one counted phase, never for unmeasured finishing work.
use std::collections::VecDeque;

#[derive(Debug, Default)]
pub struct ProgressEstimate {
    samples: VecDeque<(f64, u64)>,
}

impl ProgressEstimate {
    /// Return (recent units/second, seconds remaining), after five seconds.
    /// Call with a new estimator when the phase changes. Time is monotonic
    /// elapsed seconds supplied by the worker, so queued GUI delivery is harmless.
    pub fn update(&mut self, elapsed: f64, completed: u64, total: u64) -> Option<(f64, f64)> {
        if !elapsed.is_finite() || elapsed < 0.0 || total == 0 || completed >= total {
            self.samples.clear();
            return None;
        }
        if self
            .samples
            .back()
            .is_some_and(|&(t, n)| elapsed < t || completed < n)
        {
            self.samples.clear();
        }
        if self.samples.back().is_some_and(|&(t, _)| elapsed == t) {
            return None;
        }
        self.samples.push_back((elapsed, completed));
        // Keep approximately the last 30 seconds, plus a strict size bound.
        while self.samples.len() > 2
            && (self.samples[1].0 <= elapsed - 30.0 || self.samples.len() > 512)
        {
            self.samples.pop_front();
        }
        let &(start, count) = self.samples.front()?;
        let duration = elapsed - start;
        if duration < 5.0 || completed <= count {
            return None;
        }
        let rate = (completed - count) as f64 / duration;
        Some((rate, (total - completed) as f64 / rate))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn waits_for_observations_and_excludes_previous_phase_time() {
        let mut estimate = ProgressEstimate::default();
        assert_eq!(estimate.update(100.0, 0, 100), None);
        assert_eq!(estimate.update(102.0, 20, 100), None);
        assert_eq!(estimate.update(105.0, 50, 100), Some((10.0, 5.0)));
        assert_eq!(estimate.update(110.0, 100, 100), None);
    }

    #[test]
    fn adapts_to_recent_speed_instead_of_entire_history() {
        let mut estimate = ProgressEstimate::default();
        for t in 0..=40 {
            estimate.update(t as f64, t * 10, 10000);
        }
        let mut value = None;
        for t in 41..=80 {
            value = estimate.update(t as f64, 400 + (t - 40) * 2, 10000);
        }
        assert_eq!(value, Some((2.0, 4760.0)));
    }

    #[test]
    fn restart_and_unknown_totals_do_not_reuse_old_rate() {
        let mut estimate = ProgressEstimate::default();
        estimate.update(0.0, 0, 100);
        assert!(estimate.update(5.0, 50, 100).is_some());
        assert_eq!(estimate.update(6.0, 0, 100), None);
        assert_eq!(estimate.update(12.0, 20, 0), None);
        assert_eq!(estimate.update(f64::NAN, 20, 100), None);
    }
}
