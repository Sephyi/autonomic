//! Cost tracking for session budget enforcement.
//!
//! Tracks per-session and global costs with a sliding window for
//! rate-based budget enforcement.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use autonomic_core::SessionId;
use dashmap::DashMap;

/// Tracks cost across sessions with a global sliding-window budget.
pub struct CostTracker {
    /// Per-session accumulated costs.
    session_costs: DashMap<SessionId, f64>,
    /// Timestamped cost entries for sliding window calculation.
    cost_history: Mutex<Vec<(Instant, f64)>>,
    /// Maximum cost allowed within the budget window.
    global_budget_usd: f64,
    /// Duration of the sliding budget window.
    budget_window: Duration,
}

impl CostTracker {
    /// Create a new `CostTracker` with the given global budget and window.
    pub fn new(global_budget_usd: f64, budget_window: Duration) -> Self {
        Self {
            session_costs: DashMap::new(),
            cost_history: Mutex::new(Vec::new()),
            global_budget_usd,
            budget_window,
        }
    }

    /// Record a cost entry for a session.
    pub fn record_cost(&self, session_id: &SessionId, cost_usd: f64) {
        self.session_costs
            .entry(session_id.clone())
            .and_modify(|total| *total += cost_usd)
            .or_insert(cost_usd);

        let mut history = self
            .cost_history
            .lock()
            .expect("cost_history lock poisoned");
        history.push((Instant::now(), cost_usd));
    }

    /// Check whether an additional `amount_usd` would stay within the global
    /// budget for the current sliding window.
    pub fn can_afford(&self, amount_usd: f64) -> bool {
        self.window_total() + amount_usd <= self.global_budget_usd
    }

    /// Calculate the total cost within the current sliding window.
    pub fn window_total(&self) -> f64 {
        let history = self
            .cost_history
            .lock()
            .expect("cost_history lock poisoned");
        let cutoff = Instant::now() - self.budget_window;
        history
            .iter()
            .filter(|(ts, _)| *ts >= cutoff)
            .map(|(_, cost)| cost)
            .sum()
    }

    /// Return the all-time total cost across all sessions.
    pub fn total_cost(&self) -> f64 {
        self.session_costs.iter().map(|entry| *entry.value()).sum()
    }

    /// Return the accumulated cost for a specific session.
    pub fn session_cost(&self, session_id: &SessionId) -> f64 {
        self.session_costs
            .get(session_id)
            .map(|entry| *entry.value())
            .unwrap_or(0.0)
    }

    /// Return the number of sessions that have recorded costs.
    pub fn session_count(&self) -> usize {
        self.session_costs.len()
    }
}

impl std::fmt::Debug for CostTracker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CostTracker")
            .field("global_budget_usd", &self.global_budget_usd)
            .field("budget_window", &self.budget_window)
            .field("session_count", &self.session_count())
            .field("total_cost", &self.total_cost())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tracker() -> CostTracker {
        CostTracker::new(10.0, Duration::from_secs(3600))
    }

    #[test]
    fn record_and_retrieve_session_cost() {
        let t = tracker();
        let sid = SessionId::new();
        t.record_cost(&sid, 1.5);
        t.record_cost(&sid, 0.5);
        assert!((t.session_cost(&sid) - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn unknown_session_cost_is_zero() {
        let t = tracker();
        let sid = SessionId::new();
        assert!((t.session_cost(&sid)).abs() < f64::EPSILON);
    }

    #[test]
    fn total_cost_across_sessions() {
        let t = tracker();
        let s1 = SessionId::new();
        let s2 = SessionId::new();
        t.record_cost(&s1, 3.0);
        t.record_cost(&s2, 2.0);
        assert!((t.total_cost() - 5.0).abs() < f64::EPSILON);
    }

    #[test]
    fn can_afford_within_budget() {
        let t = tracker();
        let sid = SessionId::new();
        t.record_cost(&sid, 8.0);
        assert!(t.can_afford(2.0));
        assert!(!t.can_afford(2.01));
    }

    #[test]
    fn session_count_tracks_distinct_sessions() {
        let t = tracker();
        let s1 = SessionId::new();
        let s2 = SessionId::new();
        assert_eq!(t.session_count(), 0);
        t.record_cost(&s1, 1.0);
        assert_eq!(t.session_count(), 1);
        t.record_cost(&s2, 1.0);
        assert_eq!(t.session_count(), 2);
        // Recording to existing session does not increase count.
        t.record_cost(&s1, 1.0);
        assert_eq!(t.session_count(), 2);
    }

    #[test]
    fn window_total_includes_recent_costs() {
        let t = tracker();
        let sid = SessionId::new();
        t.record_cost(&sid, 3.0);
        t.record_cost(&sid, 2.0);
        assert!((t.window_total() - 5.0).abs() < f64::EPSILON);
    }

    #[test]
    fn debug_format_does_not_panic() {
        let t = tracker();
        let _s = format!("{t:?}");
    }

    mod proptests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn can_afford_consistent_with_window_total(
                costs in proptest::collection::vec(0.001_f64..1.0, 1..20),
                query in 0.0_f64..100.0,
            ) {
                let t = CostTracker::new(100.0, Duration::from_secs(3600));
                let sid = SessionId::new();
                for c in &costs {
                    t.record_cost(&sid, *c);
                }
                let total = t.window_total();
                let affordable = t.can_afford(query);
                prop_assert_eq!(affordable, total + query <= 100.0);
            }
        }
    }
}
