//! Rate budget contract shared across all Autonomic subsystems (XD-006).
//!
//! The budget tracks cost in USD over a sliding time window. Subsystems
//! check `can_afford()` before starting work that consumes budget.

use crate::types::{ModelTier, Subsystem};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// A single cost entry in the sliding window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostEntry {
    pub timestamp: DateTime<Utc>,
    pub cost_usd: f64,
    pub subsystem: Subsystem,
    pub model_tier: ModelTier,
    pub session_id: Option<String>,
}

/// Budget allocation percentages per subsystem.
/// Must sum to 100. Validated at construction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetAllocation {
    /// Percentage for interactive/on-demand sessions (default: 55%)
    pub interactive: f64,
    /// Percentage for scheduled jobs (default: 15%)
    pub scheduled: f64,
    /// Percentage for evolution engine (default: 10%)
    pub evolution: f64,
    /// Percentage for monitoring/health checks (default: 15%)
    pub monitoring: f64,
    /// Emergency reserve, not allocatable (default: 5%)
    pub emergency_reserve: f64,
}

impl Default for BudgetAllocation {
    fn default() -> Self {
        Self {
            interactive: 55.0,
            scheduled: 15.0,
            evolution: 10.0,
            monitoring: 15.0,
            emergency_reserve: 5.0,
        }
    }
}

impl BudgetAllocation {
    /// Validate that allocations sum to 100%.
    pub fn validate(&self) -> Result<(), String> {
        let total = self.interactive
            + self.scheduled
            + self.evolution
            + self.monitoring
            + self.emergency_reserve;
        if (total - 100.0).abs() > 0.01 {
            return Err(format!(
                "Budget allocations sum to {total:.2}%, expected 100%"
            ));
        }
        Ok(())
    }

    /// Get the allocation percentage for a subsystem.
    pub fn percentage_for(&self, subsystem: Subsystem) -> f64 {
        match subsystem {
            Subsystem::Interactive => self.interactive,
            Subsystem::Scheduled => self.scheduled,
            Subsystem::Evolution => self.evolution,
            Subsystem::Monitoring => self.monitoring,
        }
    }
}

/// Rate budget tracker with sliding window.
///
/// Thread-safety: This struct is NOT thread-safe. The caller (typically the daemon)
/// must wrap it in a `tokio::sync::Mutex` or similar.
#[derive(Debug)]
pub struct RateBudget {
    /// Maximum budget in USD for the sliding window.
    global_budget_usd: f64,
    /// Duration of the sliding window.
    window_duration: Duration,
    /// Cost entries within the current window.
    entries: VecDeque<CostEntry>,
    /// Budget allocation percentages.
    allocation: BudgetAllocation,
}

impl RateBudget {
    /// Create a new RateBudget with the given ceiling and 5-hour window.
    pub fn new(global_budget_usd: f64, allocation: BudgetAllocation) -> Self {
        Self {
            global_budget_usd,
            window_duration: Duration::hours(5),
            entries: VecDeque::new(),
            allocation,
        }
    }

    /// Create with a custom window duration (for testing).
    pub fn with_window(
        global_budget_usd: f64,
        window: Duration,
        allocation: BudgetAllocation,
    ) -> Self {
        Self {
            global_budget_usd,
            window_duration: window,
            entries: VecDeque::new(),
            allocation,
        }
    }

    /// Record a cost entry.
    pub fn record(&mut self, entry: CostEntry) {
        self.entries.push_back(entry);
        self.prune();
    }

    /// Check whether a subsystem can afford the estimated cost.
    pub fn can_afford(&mut self, subsystem: Subsystem, estimated_cost_usd: f64) -> bool {
        self.prune();
        let subsystem_budget =
            self.global_budget_usd * (self.allocation.percentage_for(subsystem) / 100.0);
        let subsystem_used: f64 = self
            .entries
            .iter()
            .filter(|e| e.subsystem == subsystem)
            .map(|e| e.cost_usd)
            .sum();
        subsystem_used + estimated_cost_usd <= subsystem_budget
    }

    /// Total cost in the current window across all subsystems.
    pub fn window_total(&mut self) -> f64 {
        self.prune();
        self.entries.iter().map(|e| e.cost_usd).sum()
    }

    /// Remaining budget in the current window.
    pub fn remaining(&mut self) -> f64 {
        self.global_budget_usd - self.window_total()
    }

    /// Usage as a percentage (0.0 to 100.0).
    pub fn usage_percent(&mut self) -> f64 {
        if self.global_budget_usd <= 0.0 {
            return 100.0;
        }
        (self.window_total() / self.global_budget_usd) * 100.0
    }

    /// Remove entries older than the window.
    fn prune(&mut self) {
        let cutoff = Utc::now() - self.window_duration;
        while self.entries.front().is_some_and(|e| e.timestamp < cutoff) {
            self.entries.pop_front();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(subsystem: Subsystem, cost: f64, age_minutes: i64) -> CostEntry {
        CostEntry {
            timestamp: Utc::now() - Duration::minutes(age_minutes),
            cost_usd: cost,
            subsystem,
            model_tier: ModelTier::Sonnet,
            session_id: None,
        }
    }

    #[test]
    fn default_allocation_sums_to_100() {
        let alloc = BudgetAllocation::default();
        alloc
            .validate()
            .expect("default allocation should be valid");
    }

    #[test]
    fn invalid_allocation_rejected() {
        let alloc = BudgetAllocation {
            interactive: 50.0,
            scheduled: 50.0,
            evolution: 50.0,
            monitoring: 50.0,
            emergency_reserve: 50.0,
        };
        assert!(alloc.validate().is_err());
    }

    #[test]
    fn empty_budget_can_afford() {
        let mut budget = RateBudget::new(10.0, BudgetAllocation::default());
        // Interactive gets 55% of $10 = $5.50
        assert!(budget.can_afford(Subsystem::Interactive, 5.0));
    }

    #[test]
    fn exhausted_subsystem_cannot_afford() {
        let mut budget = RateBudget::new(10.0, BudgetAllocation::default());
        // Interactive gets 55% of $10 = $5.50
        budget.record(make_entry(Subsystem::Interactive, 5.50, 1));
        assert!(!budget.can_afford(Subsystem::Interactive, 0.01));
    }

    #[test]
    fn other_subsystem_unaffected() {
        let mut budget = RateBudget::new(10.0, BudgetAllocation::default());
        budget.record(make_entry(Subsystem::Interactive, 5.50, 1));
        // Scheduled gets 15% of $10 = $1.50
        assert!(budget.can_afford(Subsystem::Scheduled, 1.0));
    }

    #[test]
    fn old_entries_pruned() {
        let mut budget =
            RateBudget::with_window(10.0, Duration::hours(1), BudgetAllocation::default());
        // Entry from 2 hours ago should be pruned
        budget.record(make_entry(Subsystem::Interactive, 5.50, 120));
        assert!(budget.can_afford(Subsystem::Interactive, 5.0));
    }

    #[test]
    fn usage_percent_correct() {
        let mut budget = RateBudget::new(100.0, BudgetAllocation::default());
        budget.record(make_entry(Subsystem::Interactive, 25.0, 1));
        budget.record(make_entry(Subsystem::Scheduled, 10.0, 1));
        assert!((budget.usage_percent() - 35.0).abs() < 0.01);
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn usage_percent_bounded(budget_usd in 1.0f64..1000.0, cost in 0.001f64..500.0) {
            let mut budget = RateBudget::new(budget_usd, BudgetAllocation::default());
            let entry = CostEntry {
                timestamp: Utc::now(),
                cost_usd: cost,
                subsystem: Subsystem::Interactive,
                model_tier: ModelTier::Sonnet,
                session_id: None,
            };
            budget.record(entry);
            let pct = budget.usage_percent();
            prop_assert!(pct >= 0.0, "usage_percent must be >= 0, got {}", pct);
            // Can exceed 100% if cost > budget — that's valid (overspend detection)
        }

        #[test]
        fn can_afford_monotonically_decreasing(
            budget_usd in 10.0f64..100.0,
            costs in prop::collection::vec(0.1f64..5.0, 1..10),
        ) {
            let mut budget = RateBudget::new(budget_usd, BudgetAllocation::default());
            let mut prev_remaining = budget.remaining();

            for cost in costs {
                let entry = CostEntry {
                    timestamp: Utc::now(),
                    cost_usd: cost,
                    subsystem: Subsystem::Interactive,
                    model_tier: ModelTier::Sonnet,
                    session_id: None,
                };
                budget.record(entry);
                let current_remaining = budget.remaining();
                prop_assert!(
                    current_remaining <= prev_remaining,
                    "remaining budget must not increase: {} -> {}",
                    prev_remaining,
                    current_remaining,
                );
                prev_remaining = current_remaining;
            }
        }

        #[test]
        fn allocation_percentages_consistent(
            interactive in 10.0f64..40.0,
            scheduled in 5.0f64..20.0,
            evolution in 5.0f64..20.0,
        ) {
            // Force sum to 100 by computing monitoring and reserve from remainder
            let reserve = 5.0;
            let monitoring = 100.0 - interactive - scheduled - evolution - reserve;
            prop_assume!(monitoring > 0.0);

            let alloc = BudgetAllocation {
                interactive,
                scheduled,
                evolution,
                monitoring,
                emergency_reserve: reserve,
            };
            prop_assert!(alloc.validate().is_ok());
            // Each subsystem's percentage_for matches its field
            prop_assert!((alloc.percentage_for(Subsystem::Interactive) - interactive).abs() < f64::EPSILON);
            prop_assert!((alloc.percentage_for(Subsystem::Scheduled) - scheduled).abs() < f64::EPSILON);
        }
    }
}
